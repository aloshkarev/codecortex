//! Optional MCP/graph services injected by `cortex-mcp` to avoid circular deps.

use crate::envelope::A2aEnvelope;
use crate::payload::RiskLevel;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;

/// Scoped request context for graph-backed intelligence (MCP + A2A parity).
#[derive(Debug, Clone)]
pub struct IntelligenceRequest {
    pub task: String,
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub target_symbol: Option<String>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    pub mode: Option<String>,
    pub budget_tokens: u32,
    pub repo_path: Option<String>,
    pub target_path: Option<String>,
    pub workflow: Option<String>,
}

impl IntelligenceRequest {
    pub fn from_role_context(ctx: &crate::runtime::RoleContext) -> Self {
        Self {
            task: ctx.task.clone(),
            include_paths: ctx.include_paths.clone(),
            exclude_paths: ctx.exclude_paths.clone(),
            target_symbol: ctx.target_symbol.clone(),
            source_branch: ctx.source_branch.clone(),
            target_branch: ctx.target_branch.clone(),
            mode: ctx.mode.clone(),
            budget_tokens: ctx.budget_tokens,
            repo_path: ctx.repo_root.as_ref().map(|p| p.display().to_string()),
            target_path: Some(ctx.target_path()),
            workflow: None,
        }
    }

    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflow = Some(workflow.into());
        self
    }

    pub fn target_path_or_default(&self) -> String {
        self.target_path
            .clone()
            .or_else(|| self.include_paths.first().cloned())
            .unwrap_or_else(|| "src/lib.rs".to_string())
    }
}

/// MCP tools a host agent should call after spawning a workflow session.
pub fn spawn_tool_hints(workflow: &str) -> Vec<String> {
    match workflow {
        "consensus_review" => vec![
            "get_patch_context".to_string(),
            "get_impact_graph".to_string(),
            "get_delta_context".to_string(),
        ],
        "patch_plan" => vec![
            "get_patch_context".to_string(),
            "get_api_contract".to_string(),
            "get_test_context".to_string(),
        ],
        "impact_review" => vec![
            "get_impact_graph".to_string(),
            "find_all_usages".to_string(),
            "analyze_code_relationships".to_string(),
        ],
        "pr_review" => vec![
            "get_delta_context".to_string(),
            "pr_review".to_string(),
            "get_impact_graph".to_string(),
        ],
        _ => vec!["get_patch_context".to_string()],
    }
}

/// Patch planning context returned to A2A workflows.
#[derive(Debug, Clone)]
pub struct PatchContextCapsule {
    pub capsule_uri: String,
    pub summary: String,
    pub include_paths: Vec<String>,
    pub freshness: String,
    pub warnings: Vec<String>,
    pub suggested_next_tools: Vec<String>,
    pub data_json: Option<Value>,
}

/// Impact analysis summary for consensus review.
#[derive(Debug, Clone)]
pub struct ImpactSummary {
    pub target: String,
    pub risk_level: RiskLevel,
    pub summary: String,
    pub has_cycle_risk: bool,
    pub freshness: String,
    pub warnings: Vec<String>,
    pub suggested_next_tools: Vec<String>,
    pub data_json: Option<Value>,
}

/// Validation outcome from `cargo check` or similar.
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub passed: bool,
    pub summary: String,
}

/// API contract hints for analyzer roles.
#[derive(Debug, Clone)]
pub struct ApiContractSummary {
    pub symbol: String,
    pub contracts_json: Value,
}

/// Test context for validator / patch workflows.
#[derive(Debug, Clone)]
pub struct TestContextSummary {
    pub symbol: String,
    pub tests_json: Value,
}

/// Branch delta context for PR workflows.
#[derive(Debug, Clone)]
pub struct DeltaContextSummary {
    pub source_branch: String,
    pub target_branch: String,
    pub delta_json: Value,
}

/// Structured PR review pack for hub workflows.
#[derive(Debug, Clone)]
pub struct PrReviewSummary {
    pub capsule_uri: String,
    pub impact_summary: String,
    pub risk_level: String,
    pub delta_json: Value,
    pub status_hint: String,
    pub freshness: String,
    pub warnings: Vec<String>,
    pub suggested_next_tools: Vec<String>,
    pub data_json: Option<Value>,
}

/// Context capsule for NL discovery workflows.
#[derive(Debug, Clone)]
pub struct ContextCapsuleSummary {
    pub query: String,
    pub item_count: usize,
    pub freshness: String,
    pub warnings: Vec<String>,
    pub suggested_next_tools: Vec<String>,
    pub data_json: Value,
}

/// Index freshness label for spawn responses.
#[derive(Debug, Clone)]
pub struct IndexFreshnessLabel {
    pub label: String,
}

/// Facade implemented by `cortex-mcp` for real role runners.
#[async_trait]
pub trait A2aServices: Send + Sync {
    async fn index_freshness_for_paths(&self, paths: &[String]) -> Result<IndexFreshnessLabel>;

    async fn get_patch_context(&self, req: &IntelligenceRequest) -> Result<PatchContextCapsule>;

    async fn analyze_impact(&self, req: &IntelligenceRequest) -> Result<ImpactSummary>;

    async fn validate_build(&self, repo_root: Option<&str>) -> Result<ValidationSummary>;

    async fn get_api_contract(
        &self,
        symbol: &str,
        budget_tokens: u32,
    ) -> Result<ApiContractSummary>;

    async fn get_test_context(
        &self,
        symbol: &str,
        budget_tokens: u32,
    ) -> Result<TestContextSummary>;

    async fn get_delta_context(&self, req: &IntelligenceRequest) -> Result<DeltaContextSummary>;

    async fn get_pr_review_summary(&self, req: &IntelligenceRequest) -> Result<PrReviewSummary>;

    async fn get_context_capsule(
        &self,
        req: &IntelligenceRequest,
        query: &str,
    ) -> Result<ContextCapsuleSummary>;

    async fn publish_graph_mutation(
        &self,
        session_id: &str,
        conversation_id: &str,
        affected_files: Vec<String>,
    ) -> Result<()>;
}

pub type SharedA2aServices = Arc<dyn A2aServices>;

/// No-op services for tests and stdio-only hubs.
pub struct NullA2aServices;

#[async_trait]
impl A2aServices for NullA2aServices {
    async fn index_freshness_for_paths(&self, _paths: &[String]) -> Result<IndexFreshnessLabel> {
        Ok(IndexFreshnessLabel {
            label: "unknown".to_string(),
        })
    }

    async fn get_patch_context(&self, req: &IntelligenceRequest) -> Result<PatchContextCapsule> {
        Ok(PatchContextCapsule {
            capsule_uri: format!("codecortex://session/in-memory/{}", uuid::Uuid::new_v4()),
            summary: req.task.clone(),
            include_paths: req.include_paths.clone(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: spawn_tool_hints(req.workflow.as_deref().unwrap_or("patch_plan")),
            data_json: None,
        })
    }

    async fn analyze_impact(&self, req: &IntelligenceRequest) -> Result<ImpactSummary> {
        Ok(ImpactSummary {
            target: req.target_path_or_default(),
            risk_level: RiskLevel::Low,
            summary: "null services: no graph analysis".to_string(),
            has_cycle_risk: false,
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: spawn_tool_hints("impact_review"),
            data_json: None,
        })
    }

    async fn validate_build(&self, _repo_root: Option<&str>) -> Result<ValidationSummary> {
        Ok(ValidationSummary {
            passed: true,
            summary: "skipped (null services)".to_string(),
        })
    }

    async fn get_api_contract(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> Result<ApiContractSummary> {
        Ok(ApiContractSummary {
            symbol: symbol.to_string(),
            contracts_json: Value::Array(vec![]),
        })
    }

    async fn get_test_context(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> Result<TestContextSummary> {
        Ok(TestContextSummary {
            symbol: symbol.to_string(),
            tests_json: Value::Array(vec![]),
        })
    }

    async fn get_delta_context(&self, req: &IntelligenceRequest) -> Result<DeltaContextSummary> {
        Ok(DeltaContextSummary {
            source_branch: req
                .source_branch
                .clone()
                .unwrap_or_else(|| "HEAD".to_string()),
            target_branch: req
                .target_branch
                .clone()
                .unwrap_or_else(|| "main".to_string()),
            delta_json: json!({"warnings": ["null services: no delta context"]}),
        })
    }

    async fn get_pr_review_summary(&self, req: &IntelligenceRequest) -> Result<PrReviewSummary> {
        Ok(PrReviewSummary {
            capsule_uri: format!("codecortex://session/in-memory/{}", uuid::Uuid::new_v4()),
            impact_summary: req.task.clone(),
            risk_level: "low".to_string(),
            delta_json: json!({
                "source_branch": req.source_branch,
                "target_branch": req.target_branch,
                "include_paths": req.include_paths,
            }),
            status_hint: "review".to_string(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: spawn_tool_hints("pr_review"),
            data_json: None,
        })
    }

    async fn get_context_capsule(
        &self,
        req: &IntelligenceRequest,
        query: &str,
    ) -> Result<ContextCapsuleSummary> {
        Ok(ContextCapsuleSummary {
            query: query.to_string(),
            item_count: 0,
            freshness: "unknown".to_string(),
            warnings: vec!["null services: no context capsule".to_string()],
            suggested_next_tools: vec!["get_patch_context".to_string()],
            data_json: json!({"query": query, "task": req.task}),
        })
    }

    async fn publish_graph_mutation(
        &self,
        _session_id: &str,
        _conversation_id: &str,
        _affected_files: Vec<String>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Record an envelope on the blackboard when a writer is available.
pub async fn blackboard_from_envelope(
    writer: Option<&cortex_graph::BlackboardWriter>,
    session_id: &str,
    envelope: &A2aEnvelope,
) -> Result<()> {
    let Some(writer) = writer else {
        return Ok(());
    };
    use chrono::Utc;
    use cortex_graph::{AgentInsightRecord, insight_id};

    match &envelope.payload {
        crate::payload::A2aPayload::GraphMutationSignal {
            event_type,
            affected_files,
        } => {
            let conversation_id = envelope.conversation_id.to_string();
            for path in affected_files {
                writer
                    .write_mutation_hint(session_id, &conversation_id, path, event_type)
                    .await?;
            }
            return Ok(());
        }
        crate::payload::A2aPayload::FinalResult { data } => {
            let status = data
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let role = envelope.sender.as_str().to_string();
            let id = insight_id(session_id, &role, "", &status);
            let insight = AgentInsightRecord {
                id,
                session_id: session_id.to_string(),
                conversation_id: envelope.conversation_id.to_string(),
                role,
                summary: status,
                target_qualified_name: String::new(),
                risk_level: "low".to_string(),
                suggested_action: "final_result".to_string(),
                created_at: Utc::now(),
            };
            writer.write_insight(&insight).await?;
            return Ok(());
        }
        crate::payload::A2aPayload::TaskDelegation {
            task_description,
            context_capsule_uri,
        } => {
            let role = envelope.sender.as_str().to_string();
            let id = insight_id(session_id, &role, context_capsule_uri, task_description);
            let insight = AgentInsightRecord {
                id,
                session_id: session_id.to_string(),
                conversation_id: envelope.conversation_id.to_string(),
                role,
                summary: task_description.clone(),
                target_qualified_name: context_capsule_uri.clone(),
                risk_level: "low".to_string(),
                suggested_action: "task_delegation".to_string(),
                created_at: Utc::now(),
            };
            writer.write_insight(&insight).await?;
            return Ok(());
        }
        crate::payload::A2aPayload::StrategyProposal {
            estimated_complexity,
            required_sub_nodes,
        } => {
            let role = envelope.sender.as_str().to_string();
            let summary = format!(
                "complexity={estimated_complexity} nodes={}",
                required_sub_nodes.join(",")
            );
            let id = insight_id(session_id, &role, "", &summary);
            let insight = AgentInsightRecord {
                id,
                session_id: session_id.to_string(),
                conversation_id: envelope.conversation_id.to_string(),
                role,
                summary,
                target_qualified_name: required_sub_nodes.join(","),
                risk_level: if *estimated_complexity > 5 {
                    "high".to_string()
                } else {
                    "low".to_string()
                },
                suggested_action: "strategy_proposal".to_string(),
                created_at: Utc::now(),
            };
            writer.write_insight(&insight).await?;
            return Ok(());
        }
        _ => {}
    }

    let (summary, target, risk, action, role) = match &envelope.payload {
        crate::payload::A2aPayload::CodeInsight {
            summary,
            target_qualified_name,
            risk_level,
            suggested_action,
        } => (
            summary.clone(),
            target_qualified_name.clone(),
            format!("{:?}", risk_level).to_lowercase(),
            suggested_action.clone(),
            envelope.sender.as_str().to_string(),
        ),
        crate::payload::A2aPayload::Reject { reason } => (
            reason.clone(),
            String::new(),
            "high".to_string(),
            "reject".to_string(),
            envelope.sender.as_str().to_string(),
        ),
        crate::payload::A2aPayload::Accept => (
            "accepted".to_string(),
            String::new(),
            "low".to_string(),
            "accept".to_string(),
            envelope.sender.as_str().to_string(),
        ),
        _ => return Ok(()),
    };

    let id = insight_id(session_id, &role, &target, &summary);
    let insight = AgentInsightRecord {
        id,
        session_id: session_id.to_string(),
        conversation_id: envelope.conversation_id.to_string(),
        role,
        summary,
        target_qualified_name: target,
        risk_level: risk,
        suggested_action: action,
        created_at: Utc::now(),
    };
    writer.write_insight(&insight).await?;
    Ok(())
}

/// Extract final JSON from workflow envelopes.
pub fn final_result_from_events(events: &[A2aEnvelope]) -> Option<Value> {
    events.iter().find_map(|e| {
        if let crate::payload::A2aPayload::FinalResult { data } = &e.payload {
            Some(data.clone())
        } else {
            None
        }
    })
}
