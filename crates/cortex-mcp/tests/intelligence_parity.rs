//! Unit tests for intelligence pack and tool router cooperation.

use cortex_mcp::intelligence::{
    IntelligenceMeta, IntelligencePack, next_tools, spawn_tools_for_workflow,
};
use serde_json::json;

#[test]
fn spawn_tools_for_impact_review_non_empty() {
    let tools = spawn_tools_for_workflow("impact_review");
    assert!(tools.contains(&"get_impact_graph".to_string()));
}

#[test]
fn patch_router_suggests_follow_on_tools() {
    let pack = IntelligencePack::new(json!({}), IntelligenceMeta::default());
    let next = next_tools("get_patch_context", &pack, None, None);
    assert!(next.contains(&"get_api_contract".to_string()));
}

#[test]
fn cooperation_artifact_metadata_outside_data() {
    let pack = IntelligencePack::new(json!({"summary": "test"}), IntelligenceMeta::default())
        .with_tool("get_patch_context");
    let art = pack.to_cooperation_artifact("task-99");
    let wire = art.to_wire();
    assert_eq!(
        wire.metadata.as_ref().and_then(|m| m.get("mcpToolId")),
        Some(&json!("get_patch_context"))
    );
}
