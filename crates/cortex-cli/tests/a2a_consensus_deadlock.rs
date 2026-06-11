//! Consensus deadlock workflow: analyzer rejects naive spin_lock, accepts ordered_mutex fix.

use cortex_a2a::{A2aHub, A2aPayload, SpawnSessionRequest};
use cortex_core::{A2aConfig, CortexConfig, McpToolsConfig};
use serde_json::Value;

fn test_config() -> CortexConfig {
    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        consensus_max_rounds: 3,
        ..A2aConfig::default()
    };
    config.a2a.workflows.consensus_review.demo_fixture = true;
    config.mcp.tools = McpToolsConfig {
        a2a_spawn_session: true,
        ..McpToolsConfig::default()
    };
    config
}

#[tokio::test]
async fn consensus_rejects_spin_lock_then_completes_with_ordered_mutex() {
    let config = test_config();
    let hub = A2aHub::new(config.a2a.clone());

    let mut spawn = SpawnSessionRequest::with_scope(
        "Fix deadlock in src/transport.rs",
        "consensus_review",
        vec!["src/transport.rs".to_string()],
        6000,
    );
    spawn.wait_for_completion = true;

    let resp = hub
        .spawn_session_async(spawn)
        .await
        .expect("spawn with wait_for_completion");

    let events = hub.events_snapshot().await;

    let rejects: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.payload, A2aPayload::Reject { .. }))
        .collect();
    assert!(
        !rejects.is_empty(),
        "analyzer should reject naive spin_lock attempt before accept"
    );
    let reject_reason = match &rejects[0].payload {
        A2aPayload::Reject { reason } => reason.clone(),
        _ => String::new(),
    };
    assert!(
        reject_reason.contains("spin_lock") || reject_reason.contains("lock"),
        "reject reason: {reject_reason}"
    );

    let accepts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.payload, A2aPayload::Accept))
        .collect();
    assert!(
        !accepts.is_empty(),
        "expected Accept after revision (bus may record analyzer/validator accepts)"
    );

    let finals: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.payload, A2aPayload::FinalResult { .. }))
        .collect();
    assert!(!finals.is_empty(), "expected FinalResult artifact");

    let task = hub.get_task_wire(&resp.task_id).expect("task");
    assert!(matches!(
        task.status.state,
        cortex_a2a::wire::TaskStateWire::TaskStateCompleted
    ));

    let artifact: Value = resp
        .result
        .clone()
        .or_else(|| {
            task.artifacts.iter().find_map(|a| {
                a.parts.first().and_then(|p| p.data.clone()).filter(|d| {
                    d.get("patch").is_some()
                        || d.get("status")
                            .and_then(Value::as_str)
                            .is_some_and(|s| s == "completed")
                })
            })
        })
        .expect("completed task should expose artifact JSON");

    let strategy = artifact
        .get("patch")
        .and_then(|p| p.get("strategy"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        strategy.contains("ordered_mutex"),
        "final patch strategy should be ordered_mutex, got: {strategy:?} full={artifact}"
    );
    assert!(
        !strategy.contains("spin_lock"),
        "must not ship naive_spin_lock strategy: {strategy}"
    );
}
