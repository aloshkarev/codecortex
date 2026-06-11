//! Per-envelope execution context for role runners.

use crate::manifest::RoleManifestRegistry;
use crate::services::SharedA2aServices;
use cortex_core::A2aConfig;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct RoleContext {
    pub config: A2aConfig,
    pub services: SharedA2aServices,
    pub manifests: Arc<RoleManifestRegistry>,
    pub session_id: String,
    pub conversation_id: Uuid,
    pub task_id: Uuid,
    pub task: String,
    pub budget_tokens: u32,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub target_symbol: Option<String>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    pub mode: Option<String>,
    pub repo_root: Option<PathBuf>,
    pub blackboard: Option<Arc<cortex_graph::BlackboardWriter>>,
    /// When true, `RoleGateway::dispatch_sync` runs in-process runners (hub workflows).
    pub force_in_process: bool,
}

impl RoleContext {
    pub fn target_path(&self) -> String {
        self.include_paths
            .first()
            .cloned()
            .unwrap_or_else(|| "src/lib.rs".to_string())
    }
}
