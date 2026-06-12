//! A2A validator failure mode: `validate_build` failure emits `fix_build` insight.
//!
//! Does not require graph — uses a temp repo with a failing `build.sh`.

use cortex_a2a::{A2aHub, A2aPayload, SpawnSessionRequest};
use cortex_core::{A2aConfig, A2aValidateConfig, CortexConfig};
use cortex_a2a::services::{A2aServices, ValidationSummary};
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

struct FilesystemValidateServices {
    validate: A2aValidateConfig,
}

#[async_trait]
impl A2aServices for FilesystemValidateServices {
    async fn index_freshness_for_paths(
        &self,
        _paths: &[String],
    ) -> anyhow::Result<cortex_a2a::services::IndexFreshnessLabel> {
        Ok(cortex_a2a::services::IndexFreshnessLabel {
            label: "unknown".to_string(),
        })
    }

    async fn get_patch_context(
        &self,
        req: &cortex_a2a::services::IntelligenceRequest,
    ) -> anyhow::Result<cortex_a2a::services::PatchContextCapsule> {
        Ok(cortex_a2a::services::PatchContextCapsule {
            capsule_uri: "codecortex://test".to_string(),
            summary: req.task.clone(),
            include_paths: req.include_paths.clone(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn analyze_impact(
        &self,
        req: &cortex_a2a::services::IntelligenceRequest,
    ) -> anyhow::Result<cortex_a2a::services::ImpactSummary> {
        Ok(cortex_a2a::services::ImpactSummary {
            target: req.target_path_or_default(),
            risk_level: cortex_a2a::payload::RiskLevel::Low,
            summary: "stub impact".to_string(),
            has_cycle_risk: false,
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn validate_build(&self, repo_root: Option<&str>) -> anyhow::Result<ValidationSummary> {
        let root = repo_root
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let plan = self
            .validate
            .resolve(&root)
            .ok_or_else(|| anyhow::anyhow!("no build plan"))?;
        let output = tokio::process::Command::new(&plan.program)
            .args(&plan.args)
            .current_dir(&plan.cwd)
            .output()
            .await?;
        Ok(ValidationSummary {
            passed: output.status.success(),
            summary: if output.status.success() {
                format!("{} passed", plan.label)
            } else {
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .chain(String::from_utf8_lossy(&output.stdout).lines())
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("build failed")
                    .to_string()
            },
        })
    }

    async fn get_api_contract(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> anyhow::Result<cortex_a2a::services::ApiContractSummary> {
        Ok(cortex_a2a::services::ApiContractSummary {
            symbol: symbol.to_string(),
            contracts_json: serde_json::json!([]),
        })
    }

    async fn get_test_context(
        &self,
        symbol: &str,
        _budget_tokens: u32,
    ) -> anyhow::Result<cortex_a2a::services::TestContextSummary> {
        Ok(cortex_a2a::services::TestContextSummary {
            symbol: symbol.to_string(),
            tests_json: serde_json::json!([]),
        })
    }

    async fn get_delta_context(
        &self,
        req: &cortex_a2a::services::IntelligenceRequest,
    ) -> anyhow::Result<cortex_a2a::services::DeltaContextSummary> {
        Ok(cortex_a2a::services::DeltaContextSummary {
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
        req: &cortex_a2a::services::IntelligenceRequest,
    ) -> anyhow::Result<cortex_a2a::services::PrReviewSummary> {
        Ok(cortex_a2a::services::PrReviewSummary {
            capsule_uri: "codecortex://test".to_string(),
            impact_summary: req.task.clone(),
            risk_level: "low".to_string(),
            delta_json: serde_json::json!({}),
            status_hint: "review".to_string(),
            freshness: "unknown".to_string(),
            warnings: Vec::new(),
            suggested_next_tools: Vec::new(),
            data_json: None,
        })
    }

    async fn get_context_capsule(
        &self,
        req: &cortex_a2a::services::IntelligenceRequest,
        query: &str,
    ) -> anyhow::Result<cortex_a2a::services::ContextCapsuleSummary> {
        Ok(cortex_a2a::services::ContextCapsuleSummary {
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

fn failing_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let repo = dir.path().to_path_buf();
    fs::write(
        repo.join("CMakeLists.txt"),
        "cmake_minimum_required(VERSION 3.16)\n",
    )
    .expect("cmake");
    fs::write(
        repo.join("build.sh"),
        "#!/bin/sh\necho 'simulated compile error' >&2\nexit 1\n",
    )
    .expect("build.sh");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(repo.join("build.sh"))
            .expect("meta")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(repo.join("build.sh"), perms).expect("chmod");
    }
    (dir, repo)
}

#[tokio::test]
async fn validator_build_failure_emits_fix_build_insight() {
    let (_dir, repo) = failing_repo();
    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        ..A2aConfig::default()
    };

    let hub = A2aHub::with_options(
        config.a2a.clone(),
        Arc::new(FilesystemValidateServices {
            validate: A2aValidateConfig::default(),
        }),
        None,
        Some(repo.clone()),
    );

    let mut req = SpawnSessionRequest::with_scope(
        "Ship patch with failing TWAG-style build",
        "consensus_review",
        vec!["components/cp/src/orchestrator.cpp".to_string()],
        4000,
    );
    req.wait_for_completion = true;
    req.return_immediately = false;

    let resp = hub
        .spawn_session_async(req)
        .await
        .expect("spawn consensus_review");

    let events = hub.events_snapshot().await;
    let build_failures: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                &e.payload,
                A2aPayload::CodeInsight {
                    suggested_action,
                    ..
                } if suggested_action == "fix_build"
            )
        })
        .collect();

    assert!(
        !build_failures.is_empty(),
        "validator should emit fix_build CodeInsight when build.sh fails; events={events:?}"
    );

    let task = hub.get_task_wire(&resp.task_id).expect("task");
    assert!(
        matches!(
            task.status.state,
            cortex_a2a::wire::TaskStateWire::TaskStateCompleted
                | cortex_a2a::wire::TaskStateWire::TaskStateWorking
        ),
        "consensus may complete or still be working after build failure insight: {:?}",
        task.status.state
    );
}

#[test]
fn failing_twag_layout_resolves_build_sh() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.16)\n")
        .unwrap();
    fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 1\n").unwrap();
    let plan = A2aValidateConfig::default()
        .resolve(dir.path())
        .expect("plan");
    assert_eq!(plan.program, "./build.sh");
}
