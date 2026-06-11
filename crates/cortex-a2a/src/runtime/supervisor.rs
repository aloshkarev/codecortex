//! Background consumers for per-role bus inboxes.

use crate::bus::A2aBus;
use crate::envelope::A2aEnvelope;
use crate::manifest::RoleManifestRegistry;
use crate::payload::A2aPayload;
use crate::roles::AgentRole;
use crate::runtime::context::RoleContext;
use crate::runtime::runners::RoleRunner;
use crate::services::SharedA2aServices;
use cortex_core::A2aConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct BusSupervisor;

impl BusSupervisor {
    /// Spawn one tokio task per role inbox that has a registered runner.
    pub fn spawn(
        bus: A2aBus,
        runners: Vec<Arc<dyn RoleRunner>>,
        config: A2aConfig,
        services: SharedA2aServices,
        manifests: Arc<RoleManifestRegistry>,
        repo_root: Option<PathBuf>,
        blackboard: Option<Arc<cortex_graph::BlackboardWriter>>,
        role_receivers: HashMap<AgentRole, mpsc::Receiver<Arc<A2aEnvelope>>>,
    ) {
        let runners: HashMap<AgentRole, Arc<dyn RoleRunner>> =
            runners.into_iter().map(|r| (r.role(), r)).collect();

        for (role, mut rx) in role_receivers {
            let Some(runner) = runners.get(&role).cloned() else {
                continue;
            };
            let bus = bus.clone();
            let config = config.clone();
            let services = services.clone();
            let manifests = manifests.clone();
            let repo_root = repo_root.clone();
            let blackboard = blackboard.clone();

            let role = role;
            let spawn_loop = async move {
                while let Some(env) = rx.recv().await {
                    let ctx = role_context_from_envelope(
                        &env,
                        &config,
                        services.clone(),
                        manifests.clone(),
                        repo_root.clone(),
                        blackboard.clone(),
                        true,
                    );
                    match runner.handle((*env).clone(), &ctx).await {
                        Ok(replies) => {
                            for reply in replies {
                                bus.publish(reply).await;
                            }
                        }
                        Err(e) => tracing::warn!("bus supervisor {role}: {e}"),
                    }
                }
            };

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(spawn_loop);
            } else {
                tracing::debug!(
                    "bus supervisor for {role}: no tokio runtime; inbox consumer not started"
                );
            }
        }
    }
}

fn role_context_from_envelope(
    env: &A2aEnvelope,
    config: &A2aConfig,
    services: SharedA2aServices,
    manifests: Arc<RoleManifestRegistry>,
    repo_root: Option<PathBuf>,
    blackboard: Option<Arc<cortex_graph::BlackboardWriter>>,
    force_in_process: bool,
) -> RoleContext {
    let task_id = env.task_id.unwrap_or_else(Uuid::new_v4);
    let session_id = if env.sender == AgentRole::Indexer
        || matches!(env.payload, A2aPayload::GraphMutationSignal { .. })
    {
        "codecortex-index".to_string()
    } else {
        task_id.to_string()
    };
    let include_paths = match &env.payload {
        A2aPayload::GraphMutationSignal { affected_files, .. } => affected_files.clone(),
        _ => Vec::new(),
    };

    RoleContext {
        config: config.clone(),
        services,
        manifests,
        session_id,
        conversation_id: env.conversation_id,
        task_id,
        task: String::new(),
        budget_tokens: 4000,
        include_paths,
        exclude_paths: Vec::new(),
        target_symbol: None,
        source_branch: None,
        target_branch: None,
        mode: None,
        repo_root,
        blackboard,
        force_in_process,
    }
}
