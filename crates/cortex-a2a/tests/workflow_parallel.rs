//! `dispatch_parallel_and_record` runs both roles and returns independent reply batches.

use cortex_a2a::payload::RiskLevel;
use cortex_a2a::services::{
    A2aServices, ApiContractSummary, ContextCapsuleSummary, DeltaContextSummary, ImpactSummary,
    IndexFreshnessLabel, IntelligenceRequest, PatchContextCapsule, PrReviewSummary,
    TestContextSummary, ValidationSummary,
};
use cortex_a2a::{A2aEnvelope, A2aHub, A2aPayload, AgentRole, SpawnSessionRequest};
use cortex_core::A2aConfig;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

struct SlowValidatorServices {
    delay: Duration,
}

#[async_trait::async_trait]
impl A2aServices for SlowValidatorServices {
    async fn index_freshness_for_paths(
        &self,
        _paths: &[String],
    ) -> anyhow::Result<IndexFreshnessLabel> {
        Ok(IndexFreshnessLabel {
            label: "unknown".to_string(),
        })
    }

    async fn get_patch_context(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<PatchContextCapsule> {
        Ok(PatchContextCapsule {
            capsule_uri: "codecortex://test".to_string(),
            summary: req.task.clone(),
            include_paths: req.include_paths.clone(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn analyze_impact(&self, req: &IntelligenceRequest) -> anyhow::Result<ImpactSummary> {
        tokio::time::sleep(self.delay).await;
        Ok(ImpactSummary {
            target: req.target_path_or_default(),
            risk_level: RiskLevel::Low,
            summary: "null services: no graph analysis".to_string(),
            has_cycle_risk: false,
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn validate_build(&self, _repo_root: Option<&str>) -> anyhow::Result<ValidationSummary> {
        tokio::time::sleep(self.delay).await;
        Ok(ValidationSummary {
            passed: true,
            summary: "ok".to_string(),
        })
    }

    async fn get_api_contract(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> anyhow::Result<ApiContractSummary> {
        Ok(ApiContractSummary {
            symbol: symbol.to_string(),
            contracts_json: serde_json::json!([]),
        })
    }

    async fn get_test_context(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> anyhow::Result<TestContextSummary> {
        Ok(TestContextSummary {
            symbol: symbol.to_string(),
            tests_json: serde_json::json!([]),
        })
    }

    async fn get_delta_context(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<DeltaContextSummary> {
        Ok(DeltaContextSummary {
            source_branch: req
                .source_branch
                .clone()
                .unwrap_or_else(|| "HEAD".to_string()),
            target_branch: req
                .target_branch
                .clone()
                .unwrap_or_else(|| "main".to_string()),
            delta_json: serde_json::json!({}),
        })
    }

    async fn get_pr_review_summary(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<PrReviewSummary> {
        Ok(PrReviewSummary {
            capsule_uri: "codecortex://test".to_string(),
            impact_summary: req.task.clone(),
            risk_level: "low".to_string(),
            delta_json: serde_json::json!({
                "source_branch": req.source_branch,
                "target_branch": req.target_branch,
            }),
            status_hint: "review".to_string(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn get_context_capsule(
        &self,
        req: &IntelligenceRequest,
        query: &str,
    ) -> anyhow::Result<ContextCapsuleSummary> {
        Ok(ContextCapsuleSummary {
            query: query.to_string(),
            item_count: 0,
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: serde_json::json!({"task": req.task}),
        })
    }

    async fn publish_graph_mutation(
        &self,
        _session_id: &str,
        _conversation_id: &str,
        _affected_files: Vec<String>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

fn slow_hub() -> A2aHub {
    let config = A2aConfig {
        enabled: true,
        force_in_process: true,
        ..A2aConfig::default()
    };
    A2aHub::with_options(
        config,
        Arc::new(SlowValidatorServices {
            delay: Duration::from_millis(50),
        }),
        None,
        None,
    )
}

fn parallel_envelopes(conversation_id: Uuid, task_id: Uuid) -> (A2aEnvelope, A2aEnvelope) {
    let insight = A2aEnvelope::new(
        conversation_id,
        AgentRole::PatchPlanner,
        AgentRole::Analyzer,
        A2aPayload::CodeInsight {
            summary: "proposed patch".to_string(),
            target_qualified_name: "src/transport.rs".to_string(),
            risk_level: RiskLevel::Low,
            suggested_action: "ordered_mutex".to_string(),
        },
    )
    .with_task_id(task_id);
    let accept = A2aEnvelope::new(
        conversation_id,
        AgentRole::Gateway,
        AgentRole::Validator,
        A2aPayload::Accept,
    )
    .with_task_id(task_id);
    (insight, accept)
}

#[tokio::test]
async fn parallel_dispatch_returns_both_role_batches() {
    let hub = slow_hub();
    let req = SpawnSessionRequest::with_scope(
        "parallel bench",
        "consensus_review",
        vec!["src/transport.rs".to_string()],
        4000,
    );
    let conversation_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let ctx = hub.test_role_ctx("bench-session", conversation_id, task_id, &req);
    let (insight, accept) = parallel_envelopes(conversation_id, task_id);

    let batches = hub
        .test_dispatch_parallel_and_record(vec![insight, accept], &ctx)
        .await
        .expect("parallel dispatch");

    assert_eq!(batches.len(), 2);
    assert!(
        batches.iter().any(|b| !b.is_empty()),
        "at least one role should reply"
    );
}
