//! E2E workflow completion tests for patch_plan, impact_review, and pr_review.

use cortex_a2a::wire::TaskStateWire;
use cortex_a2a::{A2aHub, A2aPayload, SpawnSessionRequest};
use cortex_core::{A2aConfig, CortexConfig, McpToolsConfig};
use serde_json::Value;
use std::time::Duration;

fn test_config() -> CortexConfig {
    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        ..A2aConfig::default()
    };
    config.mcp.tools = McpToolsConfig {
        a2a_spawn_session: true,
        ..McpToolsConfig::default()
    };
    config
}

async fn poll_until_completed(hub: &A2aHub, task_id: &str) -> cortex_a2a::wire::TaskWire {
    for _ in 0..50 {
        if let Ok(task) = hub.get_task_wire(task_id) {
            if matches!(task.status.state, TaskStateWire::TaskStateCompleted) {
                return task;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    hub.get_task_wire(task_id).expect("task should exist")
}

fn artifact_result(task: &cortex_a2a::wire::TaskWire) -> &Value {
    for artifact in &task.artifacts {
        if let Some(data) = artifact.parts.first().and_then(|p| p.data.as_ref()) {
            if data.get("workflow").and_then(Value::as_str).is_some() {
                return data;
            }
            if data.get("status").and_then(Value::as_str) == Some("completed") {
                return data;
            }
        }
    }
    task.artifacts
        .iter()
        .find_map(|a| a.parts.first().and_then(|p| p.data.as_ref()))
        .expect("workflow artifact with data part")
}

#[tokio::test]
async fn patch_plan_completes() {
    let config = test_config();
    let hub = A2aHub::new(config.a2a.clone());

    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Plan patch for src/hub.rs",
            "patch_plan",
            vec!["crates/cortex-a2a/src/hub.rs".to_string()],
            6000,
        ))
        .expect("spawn");

    let task = poll_until_completed(&hub, &resp.task_id).await;
    assert!(matches!(
        task.status.state,
        TaskStateWire::TaskStateCompleted
    ));

    let result = artifact_result(&task);
    assert_eq!(
        result.get("workflow").and_then(Value::as_str),
        Some("patch_plan")
    );
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert!(result.get("capsule_uri").and_then(Value::as_str).is_some());
}

#[tokio::test]
async fn impact_review_completes() {
    let config = test_config();
    let hub = A2aHub::new(config.a2a.clone());

    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Review impact on hub",
            "impact_review",
            vec!["crates/cortex-a2a/src/hub.rs".to_string()],
            4000,
        ))
        .expect("spawn");

    let task = poll_until_completed(&hub, &resp.task_id).await;
    assert!(matches!(
        task.status.state,
        TaskStateWire::TaskStateCompleted
    ));

    let result = artifact_result(&task);
    assert_eq!(
        result.get("workflow").and_then(Value::as_str),
        Some("impact_review")
    );
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert!(result.get("summary").and_then(Value::as_str).is_some());
}

#[tokio::test]
async fn pr_review_completes() {
    let config = test_config();
    let hub = A2aHub::new(config.a2a.clone());

    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "review branch",
            "pr_review",
            vec!["crates/cortex-a2a".to_string()],
            6000,
        ))
        .expect("spawn");

    let task = poll_until_completed(&hub, &resp.task_id).await;
    assert!(matches!(
        task.status.state,
        TaskStateWire::TaskStateCompleted
    ));

    let result = artifact_result(&task);
    assert_eq!(
        result.get("workflow").and_then(Value::as_str),
        Some("pr_review")
    );
    assert!(
        result.get("delta_context").is_some(),
        "artifact should include delta_context: {result}"
    );
    assert!(
        result
            .get("delta_context")
            .and_then(|d| d.get("capsule_uri"))
            .and_then(Value::as_str)
            .is_some(),
        "delta_context should include capsule_uri"
    );

    let events = hub.events_snapshot().await;
    let pr_insights: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(e.payload, A2aPayload::CodeInsight { .. })
                && e.sender == cortex_a2a::AgentRole::PrReviewer
        })
        .collect();
    assert!(
        !pr_insights.is_empty(),
        "PrReviewer should emit CodeInsight"
    );

    let finals: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.payload, A2aPayload::FinalResult { .. }))
        .collect();
    assert!(!finals.is_empty(), "pr_review should emit FinalResult");
}

#[tokio::test]
#[ignore = "requires graph backend at CORTEX_TEST_GRAPH=1"]
async fn impact_review_returns_graph_backed_summary_when_available() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }

    let config = test_config();
    let hub = cortex_mcp::a2a_services::try_build_a2a_hub(&config)
        .await
        .expect("graph-backed hub");

    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Review impact on A2aHub",
            "impact_review",
            vec!["crates/cortex-a2a/src/hub.rs".to_string()],
            4000,
        ))
        .expect("spawn");

    let task = poll_until_completed(&hub, &resp.task_id).await;
    assert!(matches!(
        task.status.state,
        TaskStateWire::TaskStateCompleted
    ));

    let result = artifact_result(&task);
    let summary = result
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or_default();

    assert!(
        !summary.contains("null services: no graph analysis"),
        "expected graph-backed impact, got stub summary: {summary}"
    );
    assert!(
        summary.contains("direct callers"),
        "expected get_impact_graph-style summary, got: {summary}"
    );
}
