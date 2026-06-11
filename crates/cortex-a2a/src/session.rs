use crate::cooperation::CooperationArtifact;
use crate::wire::{A2aMessage, TaskStateWire, TaskStatusWire, TaskWire};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
    Rejected,
}

impl From<TaskState> for TaskStateWire {
    fn from(s: TaskState) -> Self {
        match s {
            TaskState::Submitted => TaskStateWire::TaskStateSubmitted,
            TaskState::Working => TaskStateWire::TaskStateWorking,
            TaskState::Completed => TaskStateWire::TaskStateCompleted,
            TaskState::Failed => TaskStateWire::TaskStateFailed,
            TaskState::Canceled => TaskStateWire::TaskStateCanceled,
            TaskState::Rejected => TaskStateWire::TaskStateRejected,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aTaskRecord {
    pub id: Uuid,
    pub context_id: Uuid,
    pub state: TaskState,
    pub workflow: String,
    pub goal: String,
    pub artifacts: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl A2aTaskRecord {
    pub fn to_wire(&self) -> TaskWire {
        self.to_wire_impl(&[], None, true)
    }

    pub fn to_wire_with_history(
        &self,
        history_messages: &[A2aMessage],
        history_length: Option<i32>,
    ) -> TaskWire {
        self.to_wire_impl(history_messages, history_length, true)
    }

    pub fn to_wire_with_options(
        &self,
        history_messages: &[A2aMessage],
        history_length: Option<i32>,
        include_artifacts: bool,
    ) -> TaskWire {
        self.to_wire_impl(history_messages, history_length, include_artifacts)
    }

    fn to_wire_impl(
        &self,
        history_messages: &[A2aMessage],
        history_length: Option<i32>,
        include_artifacts: bool,
    ) -> TaskWire {
        let artifacts = if include_artifacts {
            self.artifacts
                .iter()
                .map(|v| CooperationArtifact::from_value(v).to_wire())
                .collect()
        } else {
            Vec::new()
        };
        let mut wire = TaskWire {
            id: self.id.to_string(),
            context_id: Some(self.context_id.to_string()),
            status: TaskStatusWire {
                state: self.state.into(),
                message: None,
            },
            artifacts,
            history: history_messages.to_vec(),
            metadata: self.metadata.clone(),
        };
        if let Some(n) = history_length {
            let n = n.max(0) as usize;
            if wire.history.len() > n {
                wire.history = wire.history.split_off(wire.history.len() - n);
            }
        }
        wire
    }
}

#[derive(Clone, Default)]
pub struct TaskStore {
    tasks: Arc<DashMap<Uuid, A2aTaskRecord>>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, task: A2aTaskRecord) {
        self.tasks.insert(task.id, task);
    }

    pub fn get(&self, id: &Uuid) -> Option<A2aTaskRecord> {
        self.tasks.get(id).map(|e| e.clone())
    }

    pub fn update<F>(&self, id: &Uuid, f: F) -> Option<A2aTaskRecord>
    where
        F: FnOnce(&mut A2aTaskRecord),
    {
        let mut entry = self.tasks.get_mut(id)?;
        f(&mut entry);
        entry.updated_at = Utc::now();
        Some(entry.clone())
    }

    pub fn list_by_context(&self, context_id: &Uuid) -> Vec<A2aTaskRecord> {
        self.tasks
            .iter()
            .filter(|e| e.context_id == *context_id)
            .map(|e| e.clone())
            .collect()
    }

    pub fn list_all(&self) -> Vec<A2aTaskRecord> {
        self.tasks.iter().map(|e| e.clone()).collect()
    }
}
