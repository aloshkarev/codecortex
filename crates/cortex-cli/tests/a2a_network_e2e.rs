//! Network hub E2E with graph-backed services (requires FalkorDB when enabled).

use cortex_a2a::wire::TaskStateWire;
use cortex_a2a::{A2aHub, SpawnSessionRequest};
use cortex_core::{A2aConfig, CortexConfig, McpToolsConfig};
use std::time::Duration;

fn test_config() -> CortexConfig {
    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: false,
        ..A2aConfig::default()
    };
    config.mcp.tools = McpToolsConfig {
        a2a_spawn_session: true,
        ..McpToolsConfig::default()
    };
    config
}

async fn poll_until_completed(hub: &A2aHub, task_id: &str) -> cortex_a2a::wire::TaskWire {
    for _ in 0..80 {
        if let Ok(task) = hub.get_task_wire(task_id) {
            if matches!(task.status.state, TaskStateWire::TaskStateCompleted) {
                return task;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    hub.get_task_wire(task_id).expect("task should exist")
}

#[tokio::test]
#[ignore = "requires graph backend at CORTEX_TEST_GRAPH=1"]
async fn network_hub_consensus_with_graph_services() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }

    let config = test_config();
    let hub = cortex_mcp::a2a_services::try_build_a2a_hub(&config)
        .await
        .expect("graph-backed network hub");

    let resp = hub
        .spawn_session_async({
            let mut req = SpawnSessionRequest::with_scope(
                "Fix deadlock in src/transport.rs",
                "consensus_review",
                vec!["crates/cortex-a2a/src/hub.rs".to_string()],
                6000,
            );
            req.wait_for_completion = true;
            req
        })
        .await
        .expect("spawn");

    let task = poll_until_completed(&hub, &resp.task_id).await;
    assert!(matches!(
        task.status.state,
        TaskStateWire::TaskStateCompleted
    ));
    assert!(
        resp.subscribe_url.is_some() || !resp.task_id.is_empty(),
        "network spawn should expose subscribe_url or task_id for host polling"
    );
}
