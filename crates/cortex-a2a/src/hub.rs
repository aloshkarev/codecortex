use crate::bus::A2aBus;
use crate::codec::envelope_to_message;
use crate::cooperation::{CooperationArtifact, task_cooperation_metadata};
use crate::envelope::A2aEnvelope;
use crate::manifest::RoleManifestRegistry;
use crate::payload::{A2aPayload, RiskLevel};
use crate::push::PushDelivery;
use crate::roles::AgentRole;
use crate::runtime::{BusSupervisor, RoleContext, RoleGateway, build_runners};
use crate::services::{
    IntelligenceRequest, NullA2aServices, SharedA2aServices, blackboard_from_envelope,
    spawn_tool_hints,
};
use crate::session::{A2aTaskRecord, TaskState, TaskStore};
use crate::task_events::TaskEventHub;
use crate::task_store::SledTaskStore;
use crate::wire::A2aMessage;
use crate::wire::{
    ArtifactWire, ListTasksResponseWire, SendMessageRequestWire, SendMessageResponseWire,
    StreamResponseWire, TaskArtifactUpdateWire, TaskStatusUpdateWire, TaskWire,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use cortex_core::A2aConfig;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use uuid::Uuid;

/// In-memory event ring cap (oldest evicted first).
const MAX_HUB_EVENTS: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionRequest {
    pub task: String,
    pub workflow: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub exclude_globs: Vec<String>,
    #[serde(default)]
    pub target_symbol: Option<String>,
    #[serde(default)]
    pub source_branch: Option<String>,
    #[serde(default)]
    pub target_branch: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub return_immediately: bool,
    #[serde(default)]
    pub wait_for_completion: bool,
    #[serde(default)]
    pub budget_tokens: u32,
}

impl SpawnSessionRequest {
    /// Fill optional scope fields with defaults for internal/test spawns.
    pub fn with_scope(
        task: impl Into<String>,
        workflow: impl Into<String>,
        include_paths: Vec<String>,
        budget_tokens: u32,
    ) -> Self {
        Self {
            task: task.into(),
            workflow: workflow.into(),
            roles: vec![],
            include_paths,
            exclude_paths: vec![],
            exclude_globs: vec![],
            target_symbol: None,
            source_branch: None,
            target_branch: None,
            mode: None,
            return_immediately: true,
            wait_for_completion: false,
            budget_tokens,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSessionResponse {
    pub task_id: String,
    pub context_id: String,
    pub poll: String,
    pub freshness: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscribe_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_next_tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_count: Option<usize>,
}

/// Shared A2A orchestration hub (in-process bus + task store + role gateway).
pub struct A2aHub {
    pub config: A2aConfig,
    pub bus: A2aBus,
    pub tasks: TaskStore,
    pub manifests: Arc<RoleManifestRegistry>,
    gateway: RoleGateway,
    services: SharedA2aServices,
    blackboard: Option<Arc<cortex_graph::BlackboardWriter>>,
    repo_root: Option<PathBuf>,
    push: PushDelivery,
    task_events: TaskEventHub,
    events: Arc<RwLock<Vec<A2aEnvelope>>>,
    sled_store: Option<Arc<SledTaskStore>>,
}

impl A2aHub {
    pub fn new(config: A2aConfig) -> Self {
        Self::with_options(config, Arc::new(NullA2aServices), None, None)
    }

    pub fn with_options(
        config: A2aConfig,
        services: SharedA2aServices,
        blackboard: Option<Arc<cortex_graph::BlackboardWriter>>,
        repo_root: Option<PathBuf>,
    ) -> Self {
        let manifests = Arc::new(RoleManifestRegistry::load(&config, repo_root.as_deref()));
        let bus = A2aBus::new();
        let mut role_receivers: HashMap<AgentRole, mpsc::Receiver<Arc<A2aEnvelope>>> =
            HashMap::new();
        for role in AgentRole::all() {
            role_receivers.insert(*role, bus.register_role(*role, 64));
        }
        let runners = build_runners();
        let gateway = RoleGateway::new(config.clone(), bus.clone(), runners.clone());
        let hub = Self {
            push: PushDelivery::new(config.push.clone()),
            config: config.clone(),
            bus: bus.clone(),
            tasks: TaskStore::new(),
            manifests: manifests.clone(),
            gateway,
            services: services.clone(),
            blackboard: blackboard.clone(),
            repo_root: repo_root.clone(),
            task_events: TaskEventHub::default(),
            events: Arc::new(RwLock::new(Vec::new())),
            sled_store: None,
        };
        let hub = hub.with_task_store_backend(&config);
        BusSupervisor::spawn(
            bus,
            runners,
            config,
            services,
            manifests,
            repo_root,
            blackboard,
            role_receivers,
        );
        hub
    }

    fn with_task_store_backend(mut self, config: &A2aConfig) -> Self {
        if config.task_store == cortex_core::A2aTaskStoreKind::Sled {
            match SledTaskStore::open(&config.task_store_path) {
                Ok(store) => {
                    self.tasks = store.task_store().clone();
                    self.sled_store = Some(Arc::new(store));
                }
                Err(e) => tracing::warn!("a2a sled task store unavailable: {e}"),
            }
        }
        self
    }

    fn subscribe_url_for(&self, task_id: &str) -> Option<String> {
        if !self.config.server.http_enabled {
            return None;
        }
        Some(format!(
            "{}/tasks/{task_id}/subscribe",
            self.config.server.base_path.trim_end_matches('/')
        ))
    }

    fn persist_task_insert(&self, record: A2aTaskRecord) {
        self.tasks.insert(record.clone());
        if let Some(store) = &self.sled_store {
            let _ = store.insert(record);
        }
    }

    fn persist_task_update<F>(&self, id: &Uuid, f: F)
    where
        F: FnOnce(&mut A2aTaskRecord),
    {
        if let Some(store) = &self.sled_store {
            let _ = store.update(id, f);
        } else {
            let _ = self.tasks.update(id, f);
        }
    }

    pub fn push(&self) -> &PushDelivery {
        &self.push
    }

    pub fn services(&self) -> &SharedA2aServices {
        &self.services
    }

    pub fn blackboard(&self) -> Option<&Arc<cortex_graph::BlackboardWriter>> {
        self.blackboard.as_ref()
    }

    pub fn subscribe_task(
        &self,
        task_id: &Uuid,
    ) -> tokio::sync::broadcast::Receiver<StreamResponseWire> {
        self.task_events.subscribe(task_id)
    }

    pub async fn record_event(&self, envelope: A2aEnvelope) {
        if let Some(store) = &self.sled_store {
            if let Some(task_id) = envelope.task_id {
                let _ = store.append_event(&task_id, &envelope);
            }
        }
        if let Ok(mut guard) = self.events.write() {
            guard.push(envelope.clone());
            if guard.len() > MAX_HUB_EVENTS {
                let drop = guard.len() - MAX_HUB_EVENTS;
                guard.drain(0..drop);
            }
        }
        self.bus.publish(envelope).await;
    }

    pub async fn events_snapshot(&self) -> Vec<A2aEnvelope> {
        self.events
            .read()
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    fn history_messages_for_task(&self, task_id: &Uuid) -> Vec<A2aMessage> {
        let Ok(events) = self.events.read() else {
            return Vec::new();
        };
        events
            .iter()
            .filter(|e| e.task_id.as_ref() == Some(task_id))
            .map(envelope_to_message)
            .collect()
    }

    fn emit_task_wire(&self, task_id: &Uuid) {
        if let Ok(wire) = self.get_task_wire(&task_id.to_string()) {
            let stream = StreamResponseWire {
                task: Some(wire.clone()),
                status_update: Some(TaskStatusUpdateWire {
                    task_id: wire.id.clone(),
                    context_id: wire.context_id.clone().unwrap_or_default(),
                    status: wire.status.clone(),
                }),
                artifact_update: None,
            };
            self.task_events.publish(task_id, stream.clone());
            self.push.deliver_task_update(&wire);
        }
    }

    fn publish_artifact_update(
        &self,
        task_id: &Uuid,
        artifact: ArtifactWire,
        append: bool,
        last_chunk: bool,
    ) {
        let Ok(wire) = self.get_task_wire(&task_id.to_string()) else {
            return;
        };
        let stream = StreamResponseWire {
            task: None,
            status_update: None,
            artifact_update: Some(TaskArtifactUpdateWire {
                task_id: task_id.to_string(),
                context_id: wire.context_id.clone().unwrap_or_default(),
                artifact,
                append,
                last_chunk,
            }),
        };
        self.task_events.publish(task_id, stream.clone());
        if let Ok(task) = self.get_task_wire(&task_id.to_string()) {
            self.push.deliver_task_update(&task);
        }
    }

    fn append_cooperation_artifact(
        &self,
        task_id: &Uuid,
        artifact: CooperationArtifact,
        stream: bool,
    ) {
        let wire = artifact.to_wire();
        let value = artifact.to_value();
        self.persist_task_update(task_id, |t| t.artifacts.push(value));
        if stream {
            self.publish_artifact_update(task_id, wire, false, true);
        }
    }

    pub fn validate_workflow(&self, workflow: &str) -> Result<()> {
        if !self.config.workflows.is_enabled(workflow) {
            return Err(anyhow!(
                "workflow '{workflow}' is disabled or unknown (known: {:?})",
                cortex_core::A2aWorkflowsConfig::known_workflows()
            ));
        }
        Ok(())
    }

    pub fn spawn_session(&self, req: SpawnSessionRequest) -> Result<SpawnSessionResponse> {
        if !self.config.enabled {
            return Err(anyhow!("a2a.enabled is false in config.toml"));
        }
        self.validate_workflow(&req.workflow)?;
        let context_id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let session_id = task_id.to_string();
        let record = A2aTaskRecord {
            id: task_id,
            context_id,
            state: TaskState::Submitted,
            workflow: req.workflow.clone(),
            goal: req.task.clone(),
            artifacts: Vec::new(),
            metadata: Some(task_cooperation_metadata(
                &req.workflow,
                &req.include_paths,
                &spawn_tool_hints(&req.workflow),
                "unknown",
            )),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            result: None,
            error: None,
        };
        self.persist_task_insert(record);
        let delegations = self.build_external_delegations(&req, &task_id.to_string());
        for delegation in delegations {
            self.append_cooperation_artifact(&task_id, delegation, true);
        }
        self.emit_task_wire(&task_id);

        let hub = self.clone_for_spawn();
        let req_clone = req.clone();
        tokio::spawn(async move {
            if let Err(e) = hub
                .run_workflow(task_id, context_id, session_id, req_clone)
                .await
            {
                tracing::error!("a2a workflow failed: {e}");
                hub.persist_task_update(&task_id, |t| {
                    t.state = TaskState::Failed;
                    t.error = Some(e.to_string());
                });
                hub.emit_task_wire(&task_id);
            }
        });

        Ok(SpawnSessionResponse {
            task_id: task_id.to_string(),
            context_id: context_id.to_string(),
            poll: "get_task".to_string(),
            subscribe_url: self.subscribe_url_for(&task_id.to_string()),
            freshness: "unknown".to_string(),
            status: Some("working".to_string()),
            result: None,
            warnings: Vec::new(),
            suggested_next_tools: spawn_tool_hints(&req.workflow),
            artifact_count: Some(0),
        })
    }

    /// Spawn and optionally block until the workflow reaches a terminal state.
    pub async fn spawn_session_async(
        &self,
        req: SpawnSessionRequest,
    ) -> Result<SpawnSessionResponse> {
        let wait = req.wait_for_completion;
        let mut resp = self.spawn_session(req)?;
        if wait {
            self.wait_task_terminal(&resp.task_id).await?;
            if let Ok(wire) = self.get_task_wire(&resp.task_id) {
                resp.status = Some(format!("{:?}", wire.status.state).to_lowercase());
                resp.result = primary_workflow_artifact(&wire.artifacts);
                resp.artifact_count = Some(wire.artifacts.len());
                if let Some(tools) = resp
                    .result
                    .as_ref()
                    .and_then(|r| r.get("suggested_next_tools"))
                    .and_then(|v| v.as_array())
                {
                    resp.suggested_next_tools = tools
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect();
                }
            }
        }
        Ok(resp)
    }

    async fn wait_task_terminal(&self, task_id: &str) -> Result<()> {
        let uuid = Uuid::parse_str(task_id)?;
        let mut rx = self.subscribe_task(&uuid);
        for _ in 0..400 {
            if let Ok(task) = self.get_task_wire(task_id) {
                if matches!(
                    task.status.state,
                    crate::wire::TaskStateWire::TaskStateCompleted
                        | crate::wire::TaskStateWire::TaskStateFailed
                        | crate::wire::TaskStateWire::TaskStateCanceled
                        | crate::wire::TaskStateWire::TaskStateRejected
                ) {
                    return Ok(());
                }
            }
            if let Ok(stream) = rx.try_recv() {
                if let Some(task) = stream.task {
                    if matches!(
                        task.status.state,
                        crate::wire::TaskStateWire::TaskStateCompleted
                            | crate::wire::TaskStateWire::TaskStateFailed
                            | crate::wire::TaskStateWire::TaskStateCanceled
                            | crate::wire::TaskStateWire::TaskStateRejected
                    ) {
                        return Ok(());
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Err(anyhow!("timeout waiting for task {task_id}"))
    }

    pub async fn freshness_for_paths(&self, paths: &[String]) -> String {
        self.services
            .index_freshness_for_paths(paths)
            .await
            .map(|f| f.label)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Publish a graph mutation signal after index promote (watch / add_code_to_graph).
    pub async fn notify_index_promotion(&self, affected_path: &str) {
        if !self.config.enabled {
            return;
        }
        let conversation_id = Uuid::new_v4();
        let session_id = "codecortex-index".to_string();
        let task_id = Uuid::new_v4();
        let signal = A2aEnvelope::new(
            conversation_id,
            AgentRole::Gateway,
            AgentRole::Analyzer,
            A2aPayload::GraphMutationSignal {
                event_type: "index_promoted".to_string(),
                affected_files: vec![affected_path.to_string()],
            },
        )
        .with_task_id(task_id);
        let req = SpawnSessionRequest::with_scope(
            format!("index promoted: {affected_path}"),
            "impact_review",
            vec![affected_path.to_string()],
            4000,
        );
        let ctx = self.role_ctx(&session_id, conversation_id, task_id, &req);
        let _ = self.dispatch_and_record(signal, &ctx).await;

        let _ = self
            .services
            .publish_graph_mutation(
                &session_id,
                &conversation_id.to_string(),
                vec![affected_path.to_string()],
            )
            .await;
    }

    fn clone_for_spawn(&self) -> Self {
        Self {
            config: self.config.clone(),
            bus: self.bus.clone(),
            tasks: self.tasks.clone(),
            manifests: self.manifests.clone(),
            gateway: RoleGateway::new(self.config.clone(), self.bus.clone(), build_runners()),
            services: self.services.clone(),
            blackboard: self.blackboard.clone(),
            repo_root: self.repo_root.clone(),
            push: self.push.clone(),
            task_events: self.task_events.clone(),
            events: self.events.clone(),
            sled_store: self.sled_store.clone(),
        }
    }

    fn role_ctx(
        &self,
        session_id: &str,
        conversation_id: Uuid,
        task_id: Uuid,
        req: &SpawnSessionRequest,
    ) -> RoleContext {
        RoleContext {
            config: self.config.clone(),
            services: self.services.clone(),
            manifests: self.manifests.clone(),
            session_id: session_id.to_string(),
            conversation_id,
            task_id,
            task: req.task.clone(),
            budget_tokens: req.budget_tokens,
            include_paths: req.include_paths.clone(),
            exclude_paths: req.exclude_paths.clone(),
            target_symbol: req.target_symbol.clone(),
            source_branch: req.source_branch.clone(),
            target_branch: req.target_branch.clone(),
            mode: req.mode.clone(),
            repo_root: self.repo_root.clone(),
            blackboard: self.blackboard.clone(),
            force_in_process: self.config.force_in_process,
        }
    }

    fn build_external_delegations(
        &self,
        req: &SpawnSessionRequest,
        task_id: &str,
    ) -> Vec<CooperationArtifact> {
        let mut out = Vec::new();
        for (role, cfg) in &self.config.roles {
            if cfg.mode != cortex_core::A2aRoleMode::External {
                continue;
            }
            let Some(url) = cfg.agent_card_url.as_ref() else {
                continue;
            };
            let tools = if cfg.mcp_tools.is_empty() {
                spawn_tool_hints(&req.workflow)
            } else {
                cfg.mcp_tools.clone()
            };
            out.push(CooperationArtifact::from_tool_delegation(
                task_id,
                role,
                url,
                tools,
                json!({
                    "includePaths": req.include_paths,
                    "excludePaths": req.exclude_paths,
                    "targetSymbol": req.target_symbol,
                }),
                "unknown",
            ));
        }
        out
    }

    fn intelligence_req(&self, req: &SpawnSessionRequest, workflow: &str) -> IntelligenceRequest {
        IntelligenceRequest {
            task: req.task.clone(),
            include_paths: req.include_paths.clone(),
            exclude_paths: req.exclude_paths.clone(),
            target_symbol: req.target_symbol.clone(),
            source_branch: req.source_branch.clone(),
            target_branch: req.target_branch.clone(),
            mode: req.mode.clone(),
            budget_tokens: req.budget_tokens,
            repo_path: self.repo_root.as_ref().map(|p| p.display().to_string()),
            target_path: req.include_paths.first().cloned(),
            workflow: Some(workflow.to_string()),
        }
    }

    async fn run_workflow(
        &self,
        task_id: Uuid,
        context_id: Uuid,
        session_id: String,
        req: SpawnSessionRequest,
    ) -> Result<()> {
        if let Some(bb) = self.blackboard.as_ref() {
            let _ = bb
                .upsert_session(&session_id, &context_id.to_string(), "working")
                .await;
            if self.config.insight_ttl_secs > 0 {
                let _ = bb
                    .prune_session_insights(&session_id, self.config.insight_ttl_secs)
                    .await;
            }
        }

        self.persist_task_update(&task_id, |t| t.state = TaskState::Working);
        self.emit_task_wire(&task_id);

        let result = match req.workflow.as_str() {
            "consensus_review" => {
                self.run_consensus_review(task_id, context_id, &session_id, &req)
                    .await?
            }
            "patch_plan" => {
                self.run_patch_plan(task_id, context_id, &session_id, &req)
                    .await?
            }
            "impact_review" => {
                self.run_impact_review(task_id, context_id, &session_id, &req)
                    .await?
            }
            "pr_review" => {
                self.run_pr_review(task_id, context_id, &session_id, &req)
                    .await?
            }
            other => return Err(anyhow!("unknown a2a workflow: {other}")),
        };

        self.persist_task_update(&task_id, |t| {
            t.state = TaskState::Completed;
            t.result = Some(result.clone());
            let freshness = result
                .get("freshness")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            t.metadata = Some(task_cooperation_metadata(
                &req.workflow,
                &req.include_paths,
                &result
                    .get("suggested_next_tools")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| spawn_tool_hints(&req.workflow)),
                freshness,
            ));
        });
        let workflow_art = CooperationArtifact::from_workflow_result(
            &task_id.to_string(),
            &req.workflow,
            result.clone(),
        );
        self.append_cooperation_artifact(&task_id, workflow_art, true);
        self.emit_task_wire(&task_id);
        Ok(())
    }

    async fn dispatch_and_record(
        &self,
        envelope: A2aEnvelope,
        ctx: &RoleContext,
    ) -> Result<Vec<A2aEnvelope>> {
        self.record_event(envelope.clone()).await;
        let replies = self.gateway.dispatch_sync(envelope, ctx).await?;
        for reply in &replies {
            self.record_event(reply.clone()).await;
            if let Some(w) = ctx.blackboard.as_deref() {
                let _ = blackboard_from_envelope(Some(w), &ctx.session_id, reply).await;
            }
        }
        Ok(replies)
    }

    async fn dispatch_parallel_and_record(
        &self,
        envelopes: Vec<A2aEnvelope>,
        ctx: &RoleContext,
    ) -> Result<Vec<Vec<A2aEnvelope>>> {
        let hub = Arc::new(self.clone_for_spawn());
        let mut set = tokio::task::JoinSet::new();
        for envelope in envelopes {
            let hub = hub.clone();
            let ctx = ctx.clone();
            set.spawn(async move { hub.dispatch_and_record(envelope, &ctx).await });
        }
        let mut out = Vec::new();
        while let Some(res) = set.join_next().await {
            out.push(res??);
        }
        Ok(out)
    }

    async fn run_consensus_review(
        &self,
        task_id: Uuid,
        context_id: Uuid,
        session_id: &str,
        req: &SpawnSessionRequest,
    ) -> Result<Value> {
        let max_rounds = self.config.consensus_max_rounds.max(1);
        let max_negotiation = self.config.max_negotiation_rounds.max(1);
        let ctx = self.role_ctx(session_id, context_id, task_id, req);

        let mut patch = json!({
            "strategy": "naive_spin_lock",
            "file": ctx.target_path(),
            "change": "initial proposal",
        });

        for round in 0..max_rounds {
            let planner_env = A2aEnvelope::new(
                context_id,
                AgentRole::Gateway,
                AgentRole::PatchPlanner,
                if round == 0 {
                    A2aPayload::TaskDelegation {
                        task_description: req.task.clone(),
                        context_capsule_uri: format!("codecortex://session/{session_id}/capsule"),
                    }
                } else {
                    A2aPayload::Reject {
                        reason: "revise patch after analyzer feedback".to_string(),
                    }
                },
            )
            .with_task_id(task_id);

            let planner_replies = self.dispatch_and_record(planner_env, &ctx).await?;

            for reply in &planner_replies {
                if let A2aPayload::CodeInsight {
                    suggested_action, ..
                } = &reply.payload
                {
                    patch["strategy"] = json!(suggested_action);
                }
            }

            let mut negotiation_rounds = 0;
            while negotiation_rounds < max_negotiation {
                let Some(proposal) = planner_replies.iter().find_map(|r| {
                    if let A2aPayload::StrategyProposal {
                        estimated_complexity,
                        required_sub_nodes,
                    } = &r.payload
                    {
                        Some((estimated_complexity, required_sub_nodes.clone()))
                    } else {
                        None
                    }
                }) else {
                    break;
                };

                let (complexity, sub_nodes) = proposal;
                let negotiate_env = A2aEnvelope::new(
                    context_id,
                    AgentRole::PatchPlanner,
                    AgentRole::Analyzer,
                    A2aPayload::StrategyProposal {
                        estimated_complexity: *complexity,
                        required_sub_nodes: sub_nodes,
                    },
                )
                .with_task_id(task_id);

                let negotiate_replies = self.dispatch_and_record(negotiate_env, &ctx).await?;

                let rejected = negotiate_replies
                    .iter()
                    .any(|r| matches!(r.payload, A2aPayload::Reject { .. }));
                if !rejected {
                    break;
                }
                negotiation_rounds += 1;
            }

            let insight_env = A2aEnvelope::new(
                context_id,
                AgentRole::PatchPlanner,
                AgentRole::Analyzer,
                A2aPayload::CodeInsight {
                    summary: format!("proposed patch round {}", round + 1),
                    target_qualified_name: ctx.target_path(),
                    risk_level: if patch
                        .get("strategy")
                        .and_then(|v| v.as_str())
                        .map(|s| s.contains("spin"))
                        .unwrap_or(false)
                    {
                        RiskLevel::Critical
                    } else {
                        RiskLevel::Low
                    },
                    suggested_action: patch
                        .get("strategy")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                },
            )
            .with_task_id(task_id);

            let validator_enabled =
                self.config.role_mode("validator") != cortex_core::A2aRoleMode::Disabled;
            let mut accepted = false;

            if validator_enabled {
                let accept_env = A2aEnvelope::new(
                    context_id,
                    AgentRole::Gateway,
                    AgentRole::Validator,
                    A2aPayload::Accept,
                )
                .with_task_id(task_id);

                let parallel = self
                    .dispatch_parallel_and_record(vec![insight_env.clone(), accept_env], &ctx)
                    .await?;

                for (i, replies) in parallel.into_iter().enumerate() {
                    for reply in replies {
                        if i == 0 {
                            match &reply.payload {
                                A2aPayload::Accept => accepted = true,
                                A2aPayload::Reject { reason } => {
                                    patch = json!({
                                        "strategy": "ordered_mutex",
                                        "file": ctx.target_path(),
                                        "change": reason,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                let analyzer_replies = self.dispatch_and_record(insight_env, &ctx).await?;
                for reply in analyzer_replies {
                    match &reply.payload {
                        A2aPayload::Accept => accepted = true,
                        A2aPayload::Reject { reason } => {
                            patch = json!({
                                "strategy": "ordered_mutex",
                                "file": ctx.target_path(),
                                "change": reason,
                            });
                        }
                        _ => {}
                    }
                }
            }

            if !accepted {
                continue;
            }

            let validation = self
                .services
                .validate_build(ctx.repo_root.as_deref().and_then(|p| p.to_str()))
                .await
                .map(|v| v.summary)
                .ok();

            let final_env = self.gateway.finalize_consensus(
                &ctx,
                patch.clone(),
                round + 1,
                &req.task,
                validation,
            );
            self.record_event(final_env).await;
            return Ok(json!({
                "status": "completed",
                "patch": patch,
                "rounds": round + 1,
            }));
        }

        Err(anyhow!("consensus_review exceeded max rounds"))
    }

    async fn run_patch_plan(
        &self,
        task_id: Uuid,
        context_id: Uuid,
        session_id: &str,
        req: &SpawnSessionRequest,
    ) -> Result<Value> {
        let ctx = self.role_ctx(session_id, context_id, task_id, req);
        let intel = self.intelligence_req(req, "patch_plan");
        let capsule = self.services.get_patch_context(&intel).await?;
        let contracts = self
            .services
            .get_api_contract(ctx.target_path().as_str(), req.budget_tokens.min(4000))
            .await?;

        let planner_env = A2aEnvelope::new(
            context_id,
            AgentRole::Gateway,
            AgentRole::PatchPlanner,
            A2aPayload::TaskDelegation {
                task_description: req.task.clone(),
                context_capsule_uri: capsule.capsule_uri.clone(),
            },
        )
        .with_task_id(task_id);
        for reply in self.gateway.dispatch_sync(planner_env, &ctx).await? {
            self.record_event(reply).await;
        }

        if self.config.role_mode("validator") != cortex_core::A2aRoleMode::Disabled {
            let accept_env = A2aEnvelope::new(
                context_id,
                AgentRole::Gateway,
                AgentRole::Validator,
                A2aPayload::Accept,
            )
            .with_task_id(task_id);
            for reply in self.gateway.dispatch_sync(accept_env, &ctx).await? {
                self.record_event(reply).await;
            }
        }

        Ok(json!({
            "status": "completed",
            "workflow": "patch_plan",
            "capsule_uri": capsule.capsule_uri,
            "summary": capsule.summary,
            "include_paths": capsule.include_paths,
            "contracts": contracts.contracts_json,
            "freshness": capsule.freshness,
            "mcp_tool_id": "get_patch_context",
            "suggested_next_tools": capsule.suggested_next_tools,
            "intelligence_pack": json!({
                "artifact_kind": "intelligence_pack",
                "mcp_tool_id": "get_patch_context",
                "freshness": capsule.freshness,
                "data": capsule.data_json,
                "suggested_next_tools": capsule.suggested_next_tools,
            }),
        }))
    }

    async fn run_impact_review(
        &self,
        task_id: Uuid,
        context_id: Uuid,
        session_id: &str,
        req: &SpawnSessionRequest,
    ) -> Result<Value> {
        let ctx = self.role_ctx(session_id, context_id, task_id, req);
        let intel = self.intelligence_req(req, "impact_review");
        let impact = self.services.analyze_impact(&intel).await?;

        let pack_art = CooperationArtifact::from_intelligence_pack(
            &task_id.to_string(),
            "get_impact_graph",
            impact.data_json.clone().unwrap_or(json!({})),
            &impact.freshness,
            impact.suggested_next_tools.clone(),
            req.budget_tokens,
            impact
                .data_json
                .as_ref()
                .map(|d| d.to_string().len() / 4)
                .unwrap_or(512),
            None,
        );
        self.append_cooperation_artifact(&task_id, pack_art, true);

        let insight_env = A2aEnvelope::new(
            context_id,
            AgentRole::Gateway,
            AgentRole::Analyzer,
            A2aPayload::CodeInsight {
                summary: impact.summary.clone(),
                target_qualified_name: impact.target.clone(),
                risk_level: impact.risk_level,
                suggested_action: if impact.has_cycle_risk {
                    "reject_cycle".to_string()
                } else {
                    "proceed".to_string()
                },
            },
        )
        .with_task_id(task_id);
        for reply in self.gateway.dispatch_sync(insight_env, &ctx).await? {
            self.record_event(reply).await;
        }

        Ok(json!({
            "status": "completed",
            "workflow": "impact_review",
            "target": impact.target,
            "risk_level": format!("{:?}", impact.risk_level),
            "summary": impact.summary,
            "has_cycle_risk": impact.has_cycle_risk,
            "freshness": impact.freshness,
            "mcp_tool_id": "get_impact_graph",
            "suggested_next_tools": impact.suggested_next_tools,
            "intelligence_pack": json!({
                "artifact_kind": "intelligence_pack",
                "mcp_tool_id": "get_impact_graph",
                "freshness": impact.freshness,
                "data": impact.data_json,
                "suggested_next_tools": impact.suggested_next_tools,
            }),
        }))
    }

    async fn run_pr_review(
        &self,
        task_id: Uuid,
        context_id: Uuid,
        session_id: &str,
        req: &SpawnSessionRequest,
    ) -> Result<Value> {
        let ctx = self.role_ctx(session_id, context_id, task_id, req);
        let intel = self.intelligence_req(req, "pr_review");
        let capsule = self.services.get_patch_context(&intel).await?;
        let impact_preview = self.services.analyze_impact(&intel).await?;

        let delta_json = if req.source_branch.is_some() && req.target_branch.is_some() {
            Some(self.services.get_delta_context(&intel).await?.delta_json)
        } else {
            None
        };

        let reviewer_env = A2aEnvelope::new(
            context_id,
            AgentRole::Gateway,
            AgentRole::PrReviewer,
            A2aPayload::TaskDelegation {
                task_description: req.task.clone(),
                context_capsule_uri: capsule.capsule_uri.clone(),
            },
        )
        .with_task_id(task_id);

        let mut review_summary = impact_preview.summary.clone();
        let mut risk_level = impact_preview.risk_level;
        let mut accepted = false;
        let mut rejected = false;
        let mut reject_reason = String::new();

        for reply in self.gateway.dispatch_sync(reviewer_env, &ctx).await? {
            self.record_event(reply.clone()).await;
            match &reply.payload {
                A2aPayload::CodeInsight {
                    summary,
                    risk_level: rl,
                    ..
                } => {
                    review_summary = summary.clone();
                    risk_level = *rl;
                }
                A2aPayload::Accept => accepted = true,
                A2aPayload::Reject { reason } => {
                    rejected = true;
                    reject_reason = reason.clone();
                }
                _ => {}
            }
        }

        let mut analyzer_confirmed =
            self.config.role_mode("analyzer") == cortex_core::A2aRoleMode::Disabled;
        if !analyzer_confirmed {
            let insight_env = A2aEnvelope::new(
                context_id,
                AgentRole::PrReviewer,
                AgentRole::Analyzer,
                A2aPayload::CodeInsight {
                    summary: impact_preview.summary.clone(),
                    target_qualified_name: impact_preview.target.clone(),
                    risk_level: impact_preview.risk_level,
                    suggested_action: if impact_preview.has_cycle_risk {
                        "reject_cycle".to_string()
                    } else {
                        "proceed".to_string()
                    },
                },
            )
            .with_task_id(task_id);
            for reply in self.gateway.dispatch_sync(insight_env, &ctx).await? {
                self.record_event(reply.clone()).await;
                if matches!(reply.payload, A2aPayload::Accept) {
                    analyzer_confirmed = true;
                }
            }
        }

        let status = if rejected {
            "rejected"
        } else if accepted && analyzer_confirmed {
            "approved"
        } else {
            "reviewed"
        };

        let result = json!({
            "status": status,
            "workflow": "pr_review",
            "capsule_uri": capsule.capsule_uri,
            "summary": review_summary,
            "risk_level": format!("{:?}", risk_level),
            "target": impact_preview.target,
            "delta_context": delta_json.unwrap_or(json!({
                "capsule_uri": capsule.capsule_uri,
                "include_paths": capsule.include_paths,
                "impact_summary": impact_preview.summary,
            })),
            "analyzer_confirmed": analyzer_confirmed,
            "reject_reason": if rejected { Some(reject_reason) } else { None },
        });

        let final_env = A2aEnvelope::new(
            context_id,
            AgentRole::Gateway,
            AgentRole::Gateway,
            A2aPayload::FinalResult {
                data: result.clone(),
            },
        )
        .with_task_id(task_id);
        self.record_event(final_env).await;

        Ok(result)
    }

    pub fn get_task_wire(&self, id: &str) -> Result<TaskWire> {
        self.get_task_wire_with_history(id, None)
    }

    pub fn get_task_wire_with_history(
        &self,
        id: &str,
        history_length: Option<i32>,
    ) -> Result<TaskWire> {
        self.get_task_wire_with_options(id, history_length, true)
    }

    pub fn get_task_wire_with_options(
        &self,
        id: &str,
        history_length: Option<i32>,
        include_artifacts: bool,
    ) -> Result<TaskWire> {
        let uuid = Uuid::parse_str(id)?;
        let task = self
            .tasks
            .get(&uuid)
            .ok_or_else(|| anyhow!("task not found: {id}"))?;
        let history = self.history_messages_for_task(&uuid);
        Ok(task.to_wire_with_options(&history, history_length, include_artifacts))
    }

    pub fn mcp_tools_for_role(&self, role: &str) -> Vec<String> {
        self.manifests
            .get(role)
            .map(|m| m.mcp_tools.clone())
            .unwrap_or_default()
    }

    pub fn list_tasks_wire(&self, context_id: Option<&str>) -> Result<ListTasksResponseWire> {
        let tasks = if let Some(cid) = context_id {
            let uuid = Uuid::parse_str(cid)?;
            self.tasks.list_by_context(&uuid)
        } else {
            self.tasks.list_all()
        };
        let wires: Vec<_> = tasks.into_iter().map(|t| t.to_wire()).collect();
        let total = wires.len() as i32;
        Ok(ListTasksResponseWire {
            tasks: wires,
            next_page_token: None,
            page_size: Some(total),
            total_size: Some(total),
        })
    }

    pub fn cancel_task(&self, id: &str) -> Result<TaskWire> {
        let uuid = Uuid::parse_str(id)?;
        self.tasks
            .update(&uuid, |t| t.state = TaskState::Canceled)
            .ok_or_else(|| anyhow!("task not found: {id}"))?;
        self.emit_task_wire(&uuid);
        self.get_task_wire(id)
    }

    pub fn send_message(&self, req: SendMessageRequestWire) -> Result<SendMessageResponseWire> {
        self.send_message_with_options(req, None, &[])
    }

    pub fn send_message_with_options(
        &self,
        req: SendMessageRequestWire,
        workflow: Option<&str>,
        include_paths: &[String],
    ) -> Result<SendMessageResponseWire> {
        let return_immediately = req
            .configuration
            .as_ref()
            .map(|c| c.return_immediately)
            .unwrap_or(false);
        let goal = req
            .message
            .parts
            .first()
            .and_then(|p| p.text.clone())
            .unwrap_or_else(|| "A2A message".to_string());

        let workflow_name = workflow.unwrap_or("consensus_review").to_string();
        let budget_tokens = match workflow_name.as_str() {
            "patch_plan" => self.config.workflows.patch_plan.default_budget_tokens,
            "impact_review" => self.config.workflows.impact_review.default_budget_tokens,
            "pr_review" => self.config.workflows.pr_review.default_budget_tokens,
            _ => self.config.workflows.consensus_review.default_budget_tokens,
        };

        let mut spawn = SpawnSessionRequest::with_scope(
            goal,
            workflow_name,
            include_paths.to_vec(),
            budget_tokens,
        );
        spawn.return_immediately = return_immediately;

        let resp = self.spawn_session(spawn)?;
        let wire = self.get_task_wire(&resp.task_id)?;
        Ok(SendMessageResponseWire {
            task: Some(wire),
            message: None,
        })
    }

    /// Integration-test helper: build role context for dispatch benchmarks.
    #[doc(hidden)]
    pub fn test_role_ctx(
        &self,
        session_id: &str,
        conversation_id: Uuid,
        task_id: Uuid,
        req: &SpawnSessionRequest,
    ) -> RoleContext {
        self.role_ctx(session_id, conversation_id, task_id, req)
    }

    /// Integration-test helper: single dispatch with event recording.
    #[doc(hidden)]
    pub async fn test_dispatch_and_record(
        &self,
        envelope: A2aEnvelope,
        ctx: &RoleContext,
    ) -> Result<Vec<A2aEnvelope>> {
        self.dispatch_and_record(envelope, ctx).await
    }

    /// Integration-test helper: parallel dispatch (consensus analyzer+validator round).
    #[doc(hidden)]
    pub async fn test_dispatch_parallel_and_record(
        &self,
        envelopes: Vec<A2aEnvelope>,
        ctx: &RoleContext,
    ) -> Result<Vec<Vec<A2aEnvelope>>> {
        self.dispatch_parallel_and_record(envelopes, ctx).await
    }
}

fn primary_workflow_artifact(artifacts: &[ArtifactWire]) -> Option<Value> {
    for artifact in artifacts {
        let Some(data) = artifact.parts.first().and_then(|p| p.data.as_ref()) else {
            continue;
        };
        let kind = artifact
            .metadata
            .as_ref()
            .and_then(|m| m.get("artifactKind").or_else(|| m.get("artifact_kind")))
            .and_then(Value::as_str)
            .or_else(|| data.get("artifact_kind").and_then(Value::as_str));
        if kind == Some("tool_delegation") {
            continue;
        }
        if data.get("workflow").is_some() || data.get("patch").is_some() {
            return Some(data.clone());
        }
        if data.get("status").and_then(Value::as_str) == Some("completed") {
            return Some(data.clone());
        }
    }
    artifacts
        .first()
        .and_then(|a| a.parts.first())
        .and_then(|p| p.data.clone())
}
