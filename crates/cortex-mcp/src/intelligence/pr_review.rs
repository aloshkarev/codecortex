//! PR review intelligence (shared by MCP pr_review and A2A workflows).

use super::filters::ScopeFilters;
use super::impact::{ImpactGraphParams, compute_impact_graph, impact_summary_for_a2a};
use super::pack::{IntelligenceMeta, IntelligencePack, parse_freshness_label};
use super::patch::{PatchContextParams, compute_patch_context, patch_context_json};
use super::{compute_delta_context, freshness::path_freshness, resolve_symbol};
use cortex_analyzer::Analyzer;
use cortex_graph::GraphClient;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct PrReviewParams {
    pub task: String,
    pub repo_path: String,
    pub source_branch: String,
    pub target_branch: String,
    pub scope: ScopeFilters,
    pub budget_tokens: u32,
    pub target_symbol: Option<String>,
}

pub async fn compute_pr_review_pack(
    client: &GraphClient,
    params: &PrReviewParams,
) -> IntelligencePack {
    let analyzer = Analyzer::new(client.clone());
    let mut warnings = Vec::new();

    let patch_data = compute_patch_context(
        &analyzer,
        &PatchContextParams {
            task: params.task.clone(),
            mode: Some("review".to_string()),
            budget_tokens: params.budget_tokens,
            scope: params.scope.clone(),
        },
    )
    .await;

    let target = params
        .scope
        .include_paths
        .first()
        .map(String::as_str)
        .unwrap_or("src/lib.rs");
    let symbol = resolve_symbol(&analyzer, target, params.target_symbol.as_deref()).await;

    let (impact_payload, impact_warnings, _) = compute_impact_graph(
        client,
        &analyzer,
        &ImpactGraphParams {
            symbol,
            depth: 4,
            include_importers: true,
            budget_tokens: params.budget_tokens,
            symbol_type: "auto".to_string(),
        },
    )
    .await;
    warnings.extend(impact_warnings);

    let impact = impact_summary_for_a2a(target, &params.scope.include_paths, &impact_payload, None);

    let delta = compute_delta_context(
        client,
        &params.repo_path,
        &params.source_branch,
        &params.target_branch,
        params.budget_tokens,
        &params.scope,
    )
    .await;

    if let Some(w) = delta.get("warnings").and_then(Value::as_array) {
        for item in w {
            if let Some(s) = item.as_str() {
                warnings.push(s.to_string());
            }
        }
    }

    let freshness_label = path_freshness(client, &params.repo_path, &params.scope.include_paths)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    if freshness_label != "fresh" {
        warnings.push(format!("index freshness is {freshness_label}"));
    }

    let status_hint = if impact.has_cycle_risk {
        "reject_high_blast_radius"
    } else {
        "review"
    };

    let estimated = patch_data.estimated_tokens + 256;
    let data = json!({
        "capsule_uri": patch_data.capsule_uri,
        "patch": patch_context_json(&patch_data),
        "impact_summary": impact.summary,
        "risk_level": format!("{:?}", impact.risk_level).to_lowercase(),
        "delta": delta,
        "status_hint": status_hint,
    });

    IntelligencePack::new(
        data,
        IntelligenceMeta {
            freshness: parse_freshness_label(&freshness_label),
            warnings,
            budget_tokens: params.budget_tokens,
            estimated_tokens: estimated,
            suggested_next_tools: vec![
                "get_test_context".to_string(),
                "get_delta_context".to_string(),
            ],
            ..IntelligenceMeta::default()
        },
    )
    .with_tool("pr_review")
    .with_capsule_uri(patch_data.capsule_uri)
}
