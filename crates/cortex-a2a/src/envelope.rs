use crate::payload::A2aPayload;
use crate::roles::AgentRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Internal envelope for agent-to-agent routing inside CodeCortex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aEnvelope {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub task_id: Option<Uuid>,
    pub parent_message_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub sender: AgentRole,
    pub receiver: AgentRole,
    pub payload: A2aPayload,
    pub timestamp: u64,
    #[serde(default)]
    pub priority: u8,
}

impl A2aEnvelope {
    pub fn new(
        conversation_id: Uuid,
        sender: AgentRole,
        receiver: AgentRole,
        payload: A2aPayload,
    ) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            conversation_id,
            task_id: None,
            parent_message_id: None,
            correlation_id: None,
            sender,
            receiver,
            payload,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            priority: 0,
        }
    }

    pub fn with_task_id(mut self, task_id: Uuid) -> Self {
        self.task_id = Some(task_id);
        self
    }
}
