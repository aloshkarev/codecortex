#[tokio::test]
async fn spawn_response_includes_workflow_tool_hints() {
    use cortex_a2a::{A2aHub, SpawnSessionRequest, services::spawn_tool_hints};
    use cortex_core::A2aConfig;
    use std::sync::Arc;

    let config = A2aConfig {
        enabled: true,
        force_in_process: true,
        ..A2aConfig::default()
    };
    let hub = A2aHub::with_options(config, Arc::new(cortex_a2a::NullA2aServices), None, None);
    let req = SpawnSessionRequest::with_scope(
        "tool hints",
        "impact_review",
        vec!["crates/cortex-mcp".to_string()],
        4000,
    );
    let resp = hub.spawn_session(req).expect("spawn");
    assert_eq!(resp.suggested_next_tools, spawn_tool_hints("impact_review"));
    assert!(!resp.suggested_next_tools.is_empty());
}
