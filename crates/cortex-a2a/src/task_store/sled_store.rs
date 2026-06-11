//! Sled-backed task and event persistence for A2A hub restart recovery.

use crate::envelope::A2aEnvelope;
use crate::session::{A2aTaskRecord, TaskStore};
use anyhow::{Context, Result};
use cortex_core::A2aTaskStoreKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const TASK_PREFIX: &str = "task:";
const EVENT_PREFIX: &str = "event:";

pub struct SledTaskStore {
    db: sled::Db,
    memory: TaskStore,
}

impl SledTaskStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = sled::open(path).context("open sled task store")?;
        let memory = TaskStore::new();
        let store = Self { db, memory };
        store.reload_tasks()?;
        Ok(store)
    }

    pub fn from_config(kind: A2aTaskStoreKind, path: PathBuf) -> Result<Arc<Self>> {
        match kind {
            A2aTaskStoreKind::Memory => Err(anyhow::anyhow!("memory store — use TaskStore::new()")),
            A2aTaskStoreKind::Sled => Ok(Arc::new(Self::open(&path)?)),
        }
    }

    fn reload_tasks(&self) -> Result<()> {
        for item in self.db.scan_prefix(TASK_PREFIX.as_bytes()) {
            let (_, value) = item?;
            if let Ok(task) = serde_json::from_slice::<A2aTaskRecord>(&value) {
                self.memory.insert(task);
            }
        }
        Ok(())
    }

    pub fn task_store(&self) -> &TaskStore {
        &self.memory
    }

    pub fn insert(&self, task: A2aTaskRecord) -> Result<()> {
        let key = format!("{TASK_PREFIX}{}", task.id);
        self.db
            .insert(key.as_bytes(), serde_json::to_vec(&task)?)
            .context("sled insert task")?;
        self.memory.insert(task);
        Ok(())
    }

    pub fn update<F>(&self, id: &Uuid, f: F) -> Option<A2aTaskRecord>
    where
        F: FnOnce(&mut A2aTaskRecord),
    {
        let updated = self.memory.update(id, f)?;
        let key = format!("{TASK_PREFIX}{id}");
        if let Ok(bytes) = serde_json::to_vec(&updated) {
            let _ = self.db.insert(key.as_bytes(), bytes);
        }
        Some(updated)
    }

    pub fn append_event(&self, task_id: &Uuid, envelope: &A2aEnvelope) -> Result<()> {
        let key = format!("{EVENT_PREFIX}{task_id}:{}", envelope.message_id);
        self.db
            .insert(key.as_bytes(), serde_json::to_vec(envelope)?)
            .context("sled append event")?;
        Ok(())
    }

    pub fn events_for_task(&self, task_id: &Uuid) -> Result<Vec<A2aEnvelope>> {
        let prefix = format!("{EVENT_PREFIX}{task_id}:");
        let mut events = Vec::new();
        for item in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            if let Ok(env) = serde_json::from_slice::<A2aEnvelope>(&value) {
                events.push(env);
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::TaskState;
    use chrono::Utc;

    #[test]
    fn sled_persists_task_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        let task_id = Uuid::new_v4();
        {
            let store = SledTaskStore::open(&path).unwrap();
            store
                .insert(A2aTaskRecord {
                    id: task_id,
                    context_id: Uuid::new_v4(),
                    state: TaskState::Working,
                    workflow: "consensus_review".to_string(),
                    goal: "test".to_string(),
                    artifacts: vec![],
                    metadata: None,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    result: None,
                    error: None,
                })
                .unwrap();
        }
        let store2 = SledTaskStore::open(&path).unwrap();
        assert!(store2.task_store().get(&task_id).is_some());
    }
}
