//! Gateway dispatch: routes envelopes to in-process runners or external HTTP.

use crate::bus::A2aBus;
use crate::envelope::A2aEnvelope;
use crate::payload::A2aPayload;
use crate::roles::AgentRole;
use crate::runtime::context::RoleContext;
use crate::runtime::external::{api_base_from_agent_card_url, send_and_collect_replies};
use crate::runtime::runners::RoleRunner;
use anyhow::Result;
use cortex_core::{A2aConfig, A2aRoleMode};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

pub struct RoleGateway {
    runners: HashMap<AgentRole, Arc<dyn RoleRunner>>,
    semaphore: Arc<Semaphore>,
    bus: A2aBus,
    config: A2aConfig,
    http_client: reqwest::Client,
}

impl RoleGateway {
    pub fn new(config: A2aConfig, bus: A2aBus, runners: Vec<Arc<dyn RoleRunner>>) -> Self {
        let max = config.max_parallel_roles.max(1);
        let runners = runners.into_iter().map(|r| (r.role(), r)).collect();
        Self {
            runners,
            semaphore: Arc::new(Semaphore::new(max)),
            bus,
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// In-process / external dispatch that waits for replies (consensus workflows).
    pub async fn dispatch_sync(
        &self,
        envelope: A2aEnvelope,
        ctx: &RoleContext,
    ) -> Result<Vec<A2aEnvelope>> {
        let mode = if ctx.force_in_process {
            A2aRoleMode::InProcess
        } else {
            self.config.role_mode(envelope.receiver.as_str())
        };
        match mode {
            A2aRoleMode::Disabled => Ok(vec![]),
            A2aRoleMode::External => {
                let role_name = envelope.receiver.as_str();
                let Some(role_cfg) = self.config.roles.get(role_name) else {
                    return Ok(vec![]);
                };
                let Some(agent_card_url) = role_cfg.agent_card_url.as_deref() else {
                    tracing::warn!("external role {role_name} missing agent_card_url");
                    return Ok(vec![]);
                };
                let base = api_base_from_agent_card_url(agent_card_url);
                let timeout = Duration::from_secs(role_cfg.reply_timeout_secs.max(1));
                let replies =
                    send_and_collect_replies(&self.http_client, &base, &envelope, timeout).await?;
                for reply in &replies {
                    self.bus.publish(reply.clone()).await;
                }
                Ok(replies)
            }
            A2aRoleMode::InProcess => {
                let Some(runner) = self.runners.get(&envelope.receiver) else {
                    return Ok(vec![]);
                };
                let _permit = self.semaphore.acquire().await?;
                let replies = runner.handle(envelope.clone(), ctx).await?;
                for reply in &replies {
                    self.bus.publish(reply.clone()).await;
                }
                Ok(replies)
            }
        }
    }

    pub async fn dispatch(
        &self,
        envelope: A2aEnvelope,
        ctx: RoleContext,
        record: impl Fn(A2aEnvelope) + Send + Sync + 'static,
    ) -> Result<()> {
        let mode = self.config.role_mode(envelope.receiver.as_str());

        match mode {
            A2aRoleMode::Disabled => Ok(()),
            A2aRoleMode::External => self.dispatch_external(envelope, &ctx).await,
            A2aRoleMode::InProcess => {
                let runner = match self.runners.get(&envelope.receiver) {
                    Some(r) => r.clone(),
                    None => return Ok(()),
                };
                let permit = self.semaphore.clone().acquire_owned().await?;
                let bus = self.bus.clone();
                let env = envelope.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    match runner.handle(env.clone(), &ctx).await {
                        Ok(replies) => {
                            for reply in replies {
                                record(reply.clone());
                                bus.publish(reply).await;
                            }
                        }
                        Err(e) => tracing::warn!("role runner {:?}: {e}", env.receiver),
                    }
                });
                Ok(())
            }
        }
    }

    async fn dispatch_external(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<()> {
        let role_name = envelope.receiver.as_str();
        let Some(role_cfg) = self.config.roles.get(role_name) else {
            return Ok(());
        };
        let Some(url) = role_cfg.agent_card_url.as_deref() else {
            tracing::warn!("external role {role_name} missing agent_card_url");
            return Ok(());
        };
        let send_url = format!("{}/a2a/v1/message:send", url.trim_end_matches('/'));
        let wire_msg = crate::codec::envelope_to_message(&envelope);
        let body = serde_json::json!({
            "message": wire_msg,
            "configuration": { "returnImmediately": true }
        });
        let _resp = self.http_client.post(send_url).json(&body).send().await?;
        let _ = ctx;
        Ok(())
    }

    pub fn finalize_consensus(
        &self,
        ctx: &RoleContext,
        patch: serde_json::Value,
        rounds: u32,
        task_summary: &str,
        validation_summary: Option<String>,
    ) -> A2aEnvelope {
        A2aEnvelope::new(
            ctx.conversation_id,
            AgentRole::Gateway,
            AgentRole::Gateway,
            A2aPayload::FinalResult {
                data: serde_json::json!({
                    "status": "completed",
                    "patch": patch,
                    "rounds": rounds,
                    "task": task_summary,
                    "validation": validation_summary,
                }),
            },
        )
        .with_task_id(ctx.task_id)
    }
}
