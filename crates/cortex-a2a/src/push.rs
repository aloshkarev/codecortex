//! Task push notification webhook delivery (spec §3.5).

use crate::wire::{StreamResponseWire, TaskStatusUpdateWire, TaskWire};
use cortex_core::a2a_config::A2aPushConfig;
use dashmap::DashMap;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs;
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    /// Push config id (distinct from task id per proto).
    #[serde(default)]
    pub id: String,
    pub task_id: String,
    #[serde(alias = "callbackUrl")]
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl TaskPushNotificationConfig {
    pub fn ensure_id(mut self) -> Self {
        if self.id.is_empty() {
            self.id = Uuid::new_v4().to_string();
        }
        self
    }
}

#[derive(Clone)]
pub struct PushDelivery {
    config: A2aPushConfig,
    configs: Arc<DashMap<String, TaskPushNotificationConfig>>,
    client: reqwest::Client,
    signing_secret: Option<Vec<u8>>,
}

impl PushDelivery {
    pub fn new(config: A2aPushConfig) -> Self {
        let signing_secret = fs::read(&config.signing_secret_path).ok();
        Self {
            config,
            configs: Arc::new(DashMap::new()),
            client: reqwest::Client::new(),
            signing_secret,
        }
    }

    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn create_config(&self, cfg: TaskPushNotificationConfig) -> TaskPushNotificationConfig {
        let cfg = cfg.ensure_id();
        self.configs.insert(cfg.id.clone(), cfg.clone());
        cfg
    }

    pub fn delete_config(&self, config_id: &str) -> bool {
        self.configs.remove(config_id).is_some()
    }

    pub fn get_config(&self, config_id: &str) -> Option<TaskPushNotificationConfig> {
        self.configs.get(config_id).map(|e| e.clone())
    }

    pub fn list_for_task(&self, task_id: &str) -> Vec<TaskPushNotificationConfig> {
        self.configs
            .iter()
            .filter(|e| e.task_id == task_id)
            .map(|e| e.clone())
            .collect()
    }

    pub fn list_all(&self) -> Vec<TaskPushNotificationConfig> {
        self.configs.iter().map(|e| e.clone()).collect()
    }

    pub fn spawn_deliver(&self, task_id: &str, stream: StreamResponseWire) {
        if !self.config.enabled {
            return;
        }
        let configs: Vec<_> = self.list_for_task(task_id);
        if configs.is_empty() {
            return;
        }
        let client = self.client.clone();
        let timeout = self.config.default_callback_timeout_secs;
        let max_attempts = self.config.retry.max_attempts.max(1);
        let backoff = self.config.retry.backoff_ms;
        let signing_secret = self.signing_secret.clone();
        tokio::spawn(async move {
            let body = match serde_json::to_vec(&stream) {
                Ok(b) => b,
                Err(_) => return,
            };
            let signature = signing_secret.as_ref().and_then(|secret| {
                let mut mac = HmacSha256::new_from_slice(secret).ok()?;
                mac.update(&body);
                Some(hex::encode(mac.finalize().into_bytes()))
            });
            for cfg in configs {
                for attempt in 0..max_attempts {
                    let mut req = client
                        .post(&cfg.url)
                        .header("Content-Type", "application/json")
                        .timeout(std::time::Duration::from_secs(timeout.max(1)));
                    if let Some(token) = &cfg.token {
                        req = req.bearer_auth(token);
                    }
                    if let Some(sig) = &signature {
                        req = req.header("X-A2A-Signature", format!("sha256={sig}"));
                    }
                    match req.body(body.clone()).send().await {
                        Ok(resp) if resp.status().is_success() => break,
                        _ => {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                backoff.saturating_mul(attempt as u64 + 1),
                            ))
                            .await;
                        }
                    }
                }
            }
        });
    }

    pub fn deliver_task_update(&self, task: &TaskWire) {
        let stream = StreamResponseWire {
            task: Some(task.clone()),
            status_update: Some(TaskStatusUpdateWire {
                task_id: task.id.clone(),
                context_id: task.context_id.clone().unwrap_or_default(),
                status: task.status.clone(),
            }),
            artifact_update: None,
        };
        self.spawn_deliver(&task.id, stream);
    }
}

pub fn parse_task_id(id: &str) -> Option<Uuid> {
    Uuid::parse_str(id).ok()
}
