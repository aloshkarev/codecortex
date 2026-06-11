//! MCP tool cooperation routing for A2A workflows and intelligence packs.

use super::pack::IntelligencePack;
use serde_json::Value;

/// Initial MCP tools a host agent should call after spawning an A2A session.
pub fn spawn_tools_for_workflow(workflow: &str) -> Vec<String> {
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

/// Follow-on tools after a specific intelligence tool completes.
pub fn next_tools(
    tool_id: &str,
    pack: &IntelligencePack,
    workflow: Option<&str>,
    _role: Option<&str>,
) -> Vec<String> {
    let mut out = match tool_id {
        "get_patch_context" => vec![
            "get_api_contract".to_string(),
            "get_test_context".to_string(),
            "get_delta_context".to_string(),
        ],
        "get_impact_graph" => vec![
            "find_all_usages".to_string(),
            "analyze_code_relationships".to_string(),
        ],
        "get_delta_context" => {
            let mut tools = vec!["get_impact_graph".to_string()];
            if pack
                .data
                .get("source_branch")
                .and_then(Value::as_str)
                .is_some()
            {
                tools.push("pr_review".to_string());
            }
            tools
        }
        "pr_review" => vec!["get_test_context".to_string()],
        "get_context_capsule" => vec!["get_api_contract".to_string(), "get_skeleton".to_string()],
        _ => Vec::new(),
    };

    if out.is_empty() {
        if let Some(wf) = workflow {
            out = spawn_tools_for_workflow(wf);
        }
    }

    if !pack.meta.suggested_next_tools.is_empty() {
        for t in &pack.meta.suggested_next_tools {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::FreshnessState;
    use crate::intelligence::pack::{IntelligenceMeta, IntelligencePack};
    use serde_json::json;

    #[test]
    fn impact_review_spawn_tools_non_empty() {
        let tools = spawn_tools_for_workflow("impact_review");
        assert!(tools.contains(&"get_impact_graph".to_string()));
    }

    #[test]
    fn patch_context_suggests_contracts() {
        let pack = IntelligencePack::new(json!({}), IntelligenceMeta::default());
        let next = next_tools("get_patch_context", &pack, None, None);
        assert!(next.contains(&"get_api_contract".to_string()));
    }
}
