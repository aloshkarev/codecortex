//! Shared graph-backed intelligence for MCP tools and A2A role services.

mod capsule;
mod filters;
mod freshness;
mod impact;
mod pack;
mod patch;
mod pr_review;
mod tool_router;
mod types;

pub use capsule::{CapsuleParams, compute_context_capsule, merge_hybrid_targets};
pub use filters::{ScopeFilters, path_matches_scope};
pub use freshness::path_freshness;
pub use impact::{
    ImpactGraphParams, build_impact_pack, compute_impact_graph, impact_summary_for_a2a,
};
pub use pack::{IntelligenceMeta, IntelligencePack, parse_freshness_label};
pub use patch::{
    PatchContextParams, build_patch_pack, compute_patch_context, patch_capsule_from_data,
    patch_context_json,
};
pub use pr_review::{PrReviewParams, compute_pr_review_pack};
pub use tool_router::{next_tools, spawn_tools_for_workflow};
pub use types::{detect_intent, redact_secrets, symbol_from_path};

use cortex_analyzer::Analyzer;
use cortex_analyzer::NavigationEngine;
use cortex_analyzer::navigation::BranchStructuralDiff;
use cortex_graph::GraphClient;
use serde_json::{Value, json};
use std::path::Path;

/// API contract rows for a symbol (shared by MCP and A2A).
pub async fn compute_api_contract(analyzer: &Analyzer, symbol: &str, max_rows: usize) -> Value {
    use cortex_core::SearchKind;

    let rows = analyzer
        .find_code(symbol, SearchKind::Pattern, None)
        .await
        .unwrap_or_default();

    let mut contracts = Vec::new();
    for row in rows.into_iter().take(max_rows) {
        let Some(node) = row.get("n") else {
            continue;
        };
        let name = node.get("name").and_then(Value::as_str).unwrap_or(symbol);
        let path = node.get("path").and_then(Value::as_str).unwrap_or_default();
        let source = node
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or_default();
        contracts.push(json!({
            "symbol": name,
            "path": path,
            "signature_hint": source.lines().next().unwrap_or_default(),
            "kind": node.get("kind").cloned().unwrap_or(Value::Null),
        }));
    }
    Value::Array(contracts)
}

/// Test context rows for a symbol.
pub async fn compute_test_context(analyzer: &Analyzer, symbol: &str, max_rows: usize) -> Value {
    let tests = analyzer.find_tests_for(symbol).await.unwrap_or_default();
    Value::Array(tests.into_iter().take(max_rows).collect())
}

/// Resolve symbol: explicit target, graph lookup by path, or file stem fallback.
pub async fn resolve_symbol(
    analyzer: &Analyzer,
    target_path: &str,
    explicit: Option<&str>,
) -> String {
    if let Some(s) = explicit.filter(|s| !s.trim().is_empty()) {
        return s.to_string();
    }
    if let Ok(rows) = analyzer
        .find_code(
            &Path::new(target_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(target_path),
            cortex_core::SearchKind::Pattern,
            None,
        )
        .await
    {
        for row in rows.into_iter().take(5) {
            let Some(node) = row.get("n") else {
                continue;
            };
            let path = node.get("path").and_then(Value::as_str).unwrap_or_default();
            if path.contains(target_path) || target_path.contains(path) {
                if let Some(name) = node.get("name").and_then(Value::as_str) {
                    return name.to_string();
                }
            }
        }
    }
    symbol_from_path(target_path)
}

/// Branch/worktree delta context with structural diff when graph supports it.
pub async fn compute_delta_context(
    client: &GraphClient,
    repo_path: &str,
    source_branch: &str,
    target_branch: &str,
    budget_tokens: u32,
    scope: &ScopeFilters,
) -> Value {
    let nav = NavigationEngine::new(
        client.clone(),
        repo_path.to_string(),
        Some(source_branch.to_string()),
    );
    let mut warnings = Vec::new();

    match nav
        .branch_structural_diff(source_branch, target_branch)
        .await
    {
        Ok(diff) => {
            let scoped = scope_diff(&diff, scope);
            let freshness = path_freshness(client, repo_path, &scope.include_paths)
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            if freshness != "fresh" {
                warnings.push(format!("index freshness is {freshness} for scoped paths"));
            }
            let mut out = serde_json::to_value(scoped).unwrap_or(Value::Null);
            if let Some(obj) = out.as_object_mut() {
                obj.insert("freshness".to_string(), json!(freshness));
                obj.insert("budget_tokens".to_string(), json!(budget_tokens));
                obj.insert("warnings".to_string(), json!(warnings));
            }
            return out;
        }
        Err(e) => {
            warnings.push(format!("branch_structural_diff unavailable: {e}"));
        }
    }

    let freshness = path_freshness(client, repo_path, &scope.include_paths)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    if freshness != "fresh" {
        warnings.push(format!("index freshness is {freshness} for scoped paths"));
    }

    json!({
        "repo_path": repo_path,
        "source_branch": source_branch,
        "target_branch": target_branch,
        "changed_symbols": [],
        "removed_or_renamed_symbols": [],
        "affected_callers": [],
        "likely_tests": [],
        "warnings": warnings,
        "freshness": freshness,
        "budget_tokens": budget_tokens,
        "estimated_tokens": 512
    })
}

fn scope_diff(diff: &BranchStructuralDiff, scope: &ScopeFilters) -> BranchStructuralDiff {
    let mut scoped = diff.clone();
    if !scope.include_paths.is_empty() {
        scoped
            .added_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .removed_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .modified_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .impact
            .retain(|i| path_matches_scope(&i.affected_file, scope));
    }
    if !scope.exclude_paths.is_empty() {
        scoped
            .added_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .removed_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .modified_symbols
            .retain(|s| path_matches_scope(&s.file_path, scope));
        scoped
            .impact
            .retain(|i| path_matches_scope(&i.affected_file, scope));
    }
    scoped
}

/// Build an IntelligencePack for branch/worktree delta context.
pub async fn build_delta_pack(
    client: &GraphClient,
    repo_path: &str,
    source_branch: &str,
    target_branch: &str,
    budget_tokens: u32,
    scope: &ScopeFilters,
    workflow: Option<&str>,
) -> IntelligencePack {
    let body = compute_delta_context(
        client,
        repo_path,
        source_branch,
        target_branch,
        budget_tokens,
        scope,
    )
    .await;
    let freshness_label = body
        .get("freshness")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let warnings: Vec<String> = body
        .get("warnings")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let estimated = (body.to_string().len() / 4).min(budget_tokens as usize);
    let mut pack = IntelligencePack::new(
        body,
        IntelligenceMeta {
            freshness: parse_freshness_label(&freshness_label),
            warnings,
            budget_tokens,
            estimated_tokens: estimated,
            ..IntelligenceMeta::default()
        },
    )
    .with_tool("get_delta_context");
    pack.meta.suggested_next_tools =
        tool_router::next_tools("get_delta_context", &pack, workflow, None);
    pack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_from_path_stem() {
        assert_eq!(symbol_from_path("crates/foo/src/bar.rs"), "bar");
    }

    #[test]
    fn redact_secrets_masks_tokens() {
        let out = redact_secrets("api_key = secret123");
        assert!(out.contains("[REDACTED"));
    }
}
