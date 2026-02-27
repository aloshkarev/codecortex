use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: String,
    pub state: JobState,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct JobRegistry {
    jobs: Arc<DashMap<String, JobInfo>>,
}

impl JobRegistry {
    pub fn upsert(&self, info: JobInfo) {
        self.jobs.insert(info.id.clone(), info);
    }

    pub fn get(&self, id: &str) -> Option<JobInfo> {
        self.jobs.get(id).map(|e| e.clone())
    }

    pub fn list(&self) -> Vec<JobInfo> {
        self.jobs.iter().map(|e| e.clone()).collect()
    }

    pub fn mark_running(&self, id: &str, message: impl Into<String>) {
        self.upsert(JobInfo {
            id: id.to_string(),
            state: JobState::Running,
            message: message.into(),
        });
    }

    pub fn mark_completed(&self, id: &str, message: impl Into<String>) {
        self.upsert(JobInfo {
            id: id.to_string(),
            state: JobState::Completed,
            message: message.into(),
        });
    }

    pub fn mark_failed(&self, id: &str, message: impl Into<String>) {
        self.upsert(JobInfo {
            id: id.to_string(),
            state: JobState::Failed,
            message: message.into(),
        });
    }
}
