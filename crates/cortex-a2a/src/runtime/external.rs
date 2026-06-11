//! External A2A role dispatch: send message and poll task for replies.

use crate::codec::{envelope_to_message, message_to_envelope};
use crate::envelope::A2aEnvelope;
use crate::wire::{
    A2aMessage, A2aPart, EXTENSION_BLACKBOARD, SendMessageConfigurationWire,
    SendMessageRequestWire, SendMessageResponseWire, TaskStateWire, TaskWire,
};
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::{Duration, Instant};
use uuid::Uuid;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Derive the HTTP API base URL from an agent-card URL.
pub fn api_base_from_agent_card_url(agent_card_url: &str) -> String {
    if let Some(idx) = agent_card_url.find("/.well-known/") {
        agent_card_url[..idx].to_string()
    } else {
        agent_card_url.trim_end_matches('/').to_string()
    }
}

pub fn terminal_state(state: &TaskStateWire) -> bool {
    matches!(
        state,
        TaskStateWire::TaskStateCompleted
            | TaskStateWire::TaskStateFailed
            | TaskStateWire::TaskStateCanceled
            | TaskStateWire::TaskStateRejected
    )
}

/// Decode blackboard extension envelopes from task history and artifact parts.
pub fn decode_task_replies(task: &TaskWire) -> Vec<A2aEnvelope> {
    let mut replies = Vec::new();

    for msg in &task.history {
        if let Ok(env) = message_to_envelope(msg) {
            replies.push(env);
        }
    }

    for artifact in &task.artifacts {
        for part in &artifact.parts {
            if let Some(data) = &part.data {
                if let Some(env) = decode_part_data(data, &task.id, task.context_id.as_deref()) {
                    replies.push(env);
                }
            }
        }
    }

    replies
}

fn decode_part_data(
    data: &serde_json::Value,
    task_id: &str,
    context_id: Option<&str>,
) -> Option<A2aEnvelope> {
    let parsed_task_id = Uuid::parse_str(task_id).ok().map(|u| u.to_string());
    let msg = A2aMessage {
        message_id: Uuid::new_v4().to_string(),
        context_id: context_id.and_then(|id| Uuid::parse_str(id).ok().map(|u| u.to_string())),
        task_id: parsed_task_id,
        role: "agent".to_string(),
        parts: vec![A2aPart {
            text: None,
            data: Some(data.clone()),
            metadata: None,
            media_type: Some("application/vnd.codecortex.a2a+json".to_string()),
        }],
        metadata: None,
        extensions: vec![EXTENSION_BLACKBOARD.to_string()],
    };
    message_to_envelope(&msg).ok()
}

/// POST `message:send`, poll `tasks/{id}` until terminal, decode replies.
pub async fn send_and_collect_replies(
    client: &Client,
    base_url: &str,
    envelope: &A2aEnvelope,
    timeout: Duration,
) -> Result<Vec<A2aEnvelope>> {
    let base = base_url.trim_end_matches('/');
    let send_url = format!("{base}/a2a/v1/message:send");

    let body = SendMessageRequestWire {
        message: envelope_to_message(envelope),
        configuration: Some(SendMessageConfigurationWire {
            return_immediately: true,
            history_length: None,
        }),
    };

    let send_resp: SendMessageResponseWire = client
        .post(&send_url)
        .json(&body)
        .send()
        .await
        .context("message:send request failed")?
        .error_for_status()
        .context("message:send HTTP error")?
        .json()
        .await
        .context("parse SendMessageResponseWire")?;

    let task_id = send_resp
        .task
        .as_ref()
        .map(|t| t.id.clone())
        .or_else(|| send_resp.message.as_ref().and_then(|m| m.task_id.clone()))
        .context("SendMessageResponse missing task id")?;

    let get_url = format!("{base}/a2a/v1/tasks/{task_id}");
    let deadline = Instant::now() + timeout;

    loop {
        let task: TaskWire = client
            .get(&get_url)
            .send()
            .await
            .context("get task request failed")?
            .error_for_status()
            .context("get task HTTP error")?
            .json()
            .await
            .context("parse TaskWire")?;

        if terminal_state(&task.status.state) {
            return Ok(decode_task_replies(&task));
        }

        if Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for external task {task_id} (state: {:?})",
                task.status.state
            );
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{A2aPayload, RiskLevel};
    use crate::roles::AgentRole;
    use serde_json::json;

    #[test]
    fn api_base_strips_well_known_suffix() {
        assert_eq!(
            api_base_from_agent_card_url(
                "http://127.0.0.1:3001/.well-known/agents/patch-planner.json"
            ),
            "http://127.0.0.1:3001"
        );
    }

    #[test]
    fn decode_artifact_code_insight() {
        let task = TaskWire {
            id: "task-1".to_string(),
            context_id: Some(Uuid::new_v4().to_string()),
            status: crate::wire::TaskStatusWire {
                state: TaskStateWire::TaskStateCompleted,
                message: None,
            },
            artifacts: vec![crate::wire::ArtifactWire {
                artifact_id: "a1".to_string(),
                name: None,
                description: None,
                parts: vec![A2aPart {
                    text: None,
                    data: Some(json!({
                        "codecortexRole": "patch_planner",
                        "targetRole": "analyzer",
                        "payload": {
                            "type": "code_insight",
                            "summary": "test",
                            "target_qualified_name": "src/lib.rs",
                            "risk_level": "low",
                            "suggested_action": "fix"
                        }
                    })),
                    metadata: None,
                    media_type: None,
                }],
                metadata: None,
                extensions: vec![],
            }],
            history: vec![],
            metadata: None,
        };
        let replies = decode_task_replies(&task);
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].sender, AgentRole::PatchPlanner);
        assert!(matches!(
            replies[0].payload,
            A2aPayload::CodeInsight {
                risk_level: RiskLevel::Low,
                ..
            }
        ));
    }
}
