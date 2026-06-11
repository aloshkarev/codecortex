use crate::envelope::A2aEnvelope;
use crate::payload::A2aPayload;
use crate::roles::AgentRole;
use crate::wire::{A2aMessage, A2aPart, EXTENSION_BLACKBOARD};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use uuid::Uuid;

pub fn envelope_to_message(envelope: &A2aEnvelope) -> A2aMessage {
    let payload_json = serde_json::to_value(&envelope.payload).unwrap_or(json!({}));
    let data = json!({
        "codecortexRole": envelope.sender.as_str(),
        "targetRole": envelope.receiver.as_str(),
        "payload": payload_json,
    });
    A2aMessage {
        message_id: envelope.message_id.to_string(),
        context_id: Some(envelope.conversation_id.to_string()),
        task_id: envelope.task_id.map(|t| t.to_string()),
        role: if envelope.sender == AgentRole::Gateway {
            "user".to_string()
        } else {
            "agent".to_string()
        },
        parts: vec![A2aPart {
            text: None,
            data: Some(data),
            metadata: None,
            media_type: Some("application/vnd.codecortex.a2a+json".to_string()),
        }],
        metadata: Some(json!({
            "correlationId": envelope.correlation_id.map(|c| c.to_string()),
            "parentMessageId": envelope.parent_message_id.map(|c| c.to_string()),
        })),
        extensions: vec![EXTENSION_BLACKBOARD.to_string()],
    }
}

pub fn message_to_envelope(message: &A2aMessage) -> Result<A2aEnvelope> {
    let message_id = Uuid::parse_str(&message.message_id).context("invalid message_id uuid")?;
    let conversation_id = message
        .context_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .context("invalid context_id uuid")?
        .unwrap_or_else(Uuid::new_v4);
    let task_id = message
        .task_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .context("invalid task_id uuid")?;

    let part = message.parts.first().context("message missing parts")?;
    let data = part.data.as_ref().context("part missing data")?;
    let sender = data
        .get("codecortexRole")
        .and_then(|v| v.as_str())
        .unwrap_or("gateway")
        .parse::<AgentRole>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let receiver = data
        .get("targetRole")
        .and_then(|v| v.as_str())
        .unwrap_or("gateway")
        .parse::<AgentRole>()
        .map_err(|e| anyhow::anyhow!(e))?;
    let payload: A2aPayload =
        serde_json::from_value(data.get("payload").cloned().unwrap_or(Value::Null))
            .context("invalid payload")?;

    let meta = message.metadata.as_ref();
    let correlation_id = meta
        .and_then(|m| m.get("correlationId"))
        .and_then(|v| v.as_str())
        .map(Uuid::parse_str)
        .transpose()
        .context("invalid correlationId")?;
    let parent_message_id = meta
        .and_then(|m| m.get("parentMessageId"))
        .and_then(|v| v.as_str())
        .map(Uuid::parse_str)
        .transpose()
        .context("invalid parentMessageId")?;

    Ok(A2aEnvelope {
        message_id,
        conversation_id,
        task_id,
        parent_message_id,
        correlation_id,
        sender,
        receiver,
        payload,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        priority: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::RiskLevel;

    #[test]
    fn envelope_roundtrip() {
        let env = A2aEnvelope::new(
            Uuid::new_v4(),
            AgentRole::PatchPlanner,
            AgentRole::Analyzer,
            A2aPayload::CodeInsight {
                summary: "cycle detected".to_string(),
                target_qualified_name: "crate::transport".to_string(),
                risk_level: RiskLevel::High,
                suggested_action: "reorder locks".to_string(),
            },
        );
        let msg = envelope_to_message(&env);
        let back = message_to_envelope(&msg).expect("roundtrip");
        assert_eq!(back.sender, AgentRole::PatchPlanner);
        assert_eq!(back.receiver, AgentRole::Analyzer);
        match back.payload {
            A2aPayload::CodeInsight { summary, .. } => assert_eq!(summary, "cycle detected"),
            _ => panic!("wrong payload"),
        }
    }
}
