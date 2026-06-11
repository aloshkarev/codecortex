//! Stdio MCP graph-backed A2A hub (requires live graph backend).

use cortex_core::CortexConfig;
use cortex_mcp::a2a_services::try_build_a2a_hub;

#[tokio::test]
#[ignore = "requires graph backend at CORTEX_TEST_GRAPH=1"]
async fn stdio_handler_uses_graph_hub_when_connect_succeeds() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }
    let mut config = CortexConfig::default();
    config.a2a.enabled = true;
    let hub = try_build_a2a_hub(&config).await;
    assert!(hub.is_some());
    assert!(hub.unwrap().blackboard().is_some());
}
