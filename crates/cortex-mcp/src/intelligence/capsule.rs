//! Bounded context capsule (graph-backed; optional hybrid enrichment).

use super::filters::{ScopeFilters, path_matches_scope};
use super::freshness::path_freshness;
use super::pack::{IntelligenceMeta, IntelligencePack, parse_freshness_label};
use super::types::{detect_intent, redact_secrets};
use cortex_analyzer::Analyzer;
use cortex_core::SearchKind;
use cortex_graph::GraphClient;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct CapsuleParams {
    pub query: String,
    pub task_intent: Option<String>,
    pub budget_tokens: u32,
    pub max_items: usize,
    pub scope: ScopeFilters,
    pub repo_path: String,
}

/// Graph-first context capsule; vector hybrid rows may be merged via `extra_items`.
pub async fn compute_context_capsule(
    client: &GraphClient,
    analyzer: &Analyzer,
    params: &CapsuleParams,
    extra_items: Vec<Value>,
) -> IntelligencePack {
    let intent = params
        .task_intent
        .clone()
        .unwrap_or_else(|| detect_intent(&params.query).to_string());
    let max_items = params.max_items.clamp(1, 100);
    let budget = params.budget_tokens;
    let mut warnings = Vec::new();
    let mut items = extra_items;
    let mut token_estimate = 0usize;

    let rows = analyzer
        .find_code(&params.query, SearchKind::Pattern, None)
        .await
        .unwrap_or_default();

    for row in rows.into_iter().take(max_items) {
        let Some(node) = row.get("n") else {
            continue;
        };
        let path = node.get("path").and_then(Value::as_str).unwrap_or_default();
        if !path_matches_scope(path, &params.scope) {
            continue;
        }
        let source = node
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let excerpt: String = source.lines().take(8).collect::<Vec<_>>().join("\n");
        let est = excerpt.len() / 4 + 32;
        if token_estimate + est > budget as usize {
            warnings.push("budget_exhausted_before_all_candidates".to_string());
            break;
        }
        token_estimate += est;
        items.push(json!({
            "kind": node.get("kind").cloned().unwrap_or(Value::Null),
            "name": node.get("name").cloned().unwrap_or(Value::Null),
            "path": path,
            "excerpt": redact_secrets(&excerpt),
            "context_kind": "graph_match",
            "score": 0.7,
        }));
    }

    if items.is_empty() {
        warnings.push("no graph matches for query".to_string());
    }

    let freshness_label = path_freshness(client, &params.repo_path, &params.scope.include_paths)
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    let data = json!({
        "query": params.query,
        "task_intent": intent,
        "items": items,
        "item_count": items.len(),
        "estimated_tokens": token_estimate,
        "budget_tokens": budget,
    });

    IntelligencePack::new(
        data,
        IntelligenceMeta {
            freshness: parse_freshness_label(&freshness_label),
            warnings,
            budget_tokens: budget,
            estimated_tokens: token_estimate,
            suggested_next_tools: vec!["get_api_contract".to_string(), "get_skeleton".to_string()],
            ..IntelligenceMeta::default()
        },
    )
    .with_tool("get_context_capsule")
}

/// Optional hybrid target enrichment for patch planning when vector index exists.
pub fn merge_hybrid_targets(
    mut targets: Vec<Value>,
    hybrid_rows: Vec<Value>,
    budget: u32,
) -> Vec<Value> {
    let max = (budget / 200).clamp(3, 20) as usize;
    for row in hybrid_rows.into_iter().take(max) {
        if !targets.iter().any(|t| t.get("path") == row.get("path")) {
            targets.push(row);
        }
    }
    targets
}
