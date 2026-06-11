//! Async workflow orchestration: send → await replies → blackboard merge.

use crate::envelope::A2aEnvelope;
use crate::runtime::{RoleContext, RoleGateway};
use crate::services::blackboard_from_envelope;
use anyhow::Result;

pub struct WorkflowEngine;

impl WorkflowEngine {
    /// Record envelope, dispatch synchronously, record replies and blackboard writes.
    pub async fn send_and_collect<F>(
        gateway: &RoleGateway,
        record: &mut F,
        envelope: A2aEnvelope,
        ctx: &RoleContext,
    ) -> Result<Vec<A2aEnvelope>>
    where
        F: FnMut(A2aEnvelope),
    {
        record(envelope.clone());
        let replies = gateway.dispatch_sync(envelope, ctx).await?;
        Self::record_replies(record, ctx, &replies).await;
        Ok(replies)
    }

    /// Dispatch multiple envelopes (sequential collection; hub uses true parallel dispatch).
    pub async fn send_parallel<F>(
        gateway: &RoleGateway,
        record: F,
        envelopes: Vec<A2aEnvelope>,
        ctx: &RoleContext,
    ) -> Result<Vec<Vec<A2aEnvelope>>>
    where
        F: FnMut(A2aEnvelope),
    {
        let mut out = Vec::with_capacity(envelopes.len());
        let mut record = record;
        for envelope in envelopes {
            out.push(Self::send_and_collect(gateway, &mut record, envelope, ctx).await?);
        }
        Ok(out)
    }

    async fn record_replies<F>(record: &mut F, ctx: &RoleContext, replies: &[A2aEnvelope])
    where
        F: FnMut(A2aEnvelope),
    {
        for reply in replies {
            record(reply.clone());
            if let Some(w) = ctx.blackboard.as_deref() {
                let _ = blackboard_from_envelope(Some(w), &ctx.session_id, reply).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::A2aBus;
    use crate::payload::A2aPayload;
    use crate::roles::AgentRole;
    use crate::runtime::build_runners;
    use crate::services::NullA2aServices;
    use cortex_core::A2aConfig;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn send_and_collect_records_replies() {
        let config = A2aConfig {
            enabled: true,
            force_in_process: true,
            ..Default::default()
        };
        let bus = A2aBus::new();
        let gateway = RoleGateway::new(config, bus, build_runners());
        let ctx = RoleContext {
            config: A2aConfig::default(),
            services: Arc::new(NullA2aServices),
            manifests: Arc::new(crate::manifest::RoleManifestRegistry::load(
                &A2aConfig::default(),
                None,
            )),
            session_id: "test".to_string(),
            conversation_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            task: "fix transport".to_string(),
            budget_tokens: 4000,
            include_paths: vec!["src/transport.rs".to_string()],
            exclude_paths: Vec::new(),
            target_symbol: None,
            source_branch: None,
            target_branch: None,
            mode: None,
            repo_root: None,
            blackboard: None,
            force_in_process: true,
        };
        let mut recorded = Vec::new();
        let env = A2aEnvelope::new(
            ctx.conversation_id,
            AgentRole::Gateway,
            AgentRole::PatchPlanner,
            A2aPayload::TaskDelegation {
                task_description: "fix".to_string(),
                context_capsule_uri: "codecortex://test".to_string(),
            },
        )
        .with_task_id(ctx.task_id);
        let replies =
            WorkflowEngine::send_and_collect(&gateway, &mut |e| recorded.push(e), env, &ctx)
                .await
                .expect("dispatch");
        assert!(!replies.is_empty());
        assert!(recorded.len() > 1);
    }
}
