//! Host MCP call budget contract for A2A spawn_session.
//!
//! A single `cortex_a2a_spawn_session` with `return_immediately: true` is one host
//! call that initiates the workflow. Polling task status (`cortex_a2a_get_task` or
//! HTTP GET) is optional client-side responsibility — not counted as part of the
//! spawn budget enforced here.

use cortex_a2a::{A2aHub, SpawnSessionRequest};
use cortex_core::{A2aConfig, CortexConfig, McpToolsConfig};

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

#[tokio::test]
async fn spawn_session_response_includes_poll_hint() {
    let config = test_config();
    let hub = A2aHub::new(config.a2a.clone());

    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Budget contract check",
            "impact_review",
            vec!["crates/cortex-a2a/src/hub.rs".to_string()],
            4000,
        ))
        .expect("spawn should succeed with return_immediately");

    assert_eq!(
        resp.poll, "get_task",
        "spawn response must tell the host to poll via get_task (client-side, not part of spawn call count)"
    );
    assert!(
        !resp.task_id.is_empty(),
        "spawn must return task_id for client polling"
    );
    assert!(
        !resp.context_id.is_empty(),
        "spawn must return context_id for session correlation"
    );
}
