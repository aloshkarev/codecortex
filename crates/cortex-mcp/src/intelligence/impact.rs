//! Impact graph computation (shared by get_impact_graph and A2A analyzer).

use super::pack::{IntelligenceMeta, IntelligencePack, parse_freshness_label};
use super::{freshness::path_freshness, tool_router};
use cortex_a2a::payload::RiskLevel;
use cortex_a2a::services::ImpactSummary;
use cortex_analyzer::Analyzer;
use cortex_graph::{GraphClient, GraphParam};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ImpactGraphParams {
    pub symbol: String,
    pub depth: usize,
    pub include_importers: bool,
    pub budget_tokens: u32,
    pub symbol_type: String,
}

#[derive(Debug, Clone)]
struct ReachProps {
    reach_d1_count: usize,
    reach_d3_ids: Vec<String>,
    reach_truncated: bool,
}

async fn fetch_reach_properties(graph: &GraphClient, symbol: &str) -> Option<ReachProps> {
    let rows = graph
        .query_with_param(
            "MATCH (n:CodeNode {name: $name})
             WHERE n.reach_d1_count IS NOT NULL
             RETURN n.reach_d1_count AS reach_d1_count,
                    n.reach_d3_ids AS reach_d3_ids,
                    coalesce(n.reach_truncated, false) AS reach_truncated
             LIMIT 1",
            "name",
            symbol,
        )
        .await
        .ok()?;
    let row = rows.first()?;
    let reach_d1_count = row
        .get("reach_d1_count")
        .and_then(Value::as_u64)
        .map(|n| n as usize)?;
    let reach_d3_ids = parse_reach_id_list(row.get("reach_d3_ids")?);
    let reach_truncated = row
        .get("reach_truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(ReachProps {
        reach_d1_count,
        reach_d3_ids,
        reach_truncated,
    })
}

fn parse_reach_id_list(value: &Value) -> Vec<String> {
    if let Some(arr) = value.as_array() {
        return arr
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
    }
    if let Some(text) = value.as_str() {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(text) {
            return arr;
        }
    }
    Vec::new()
}

async fn fetch_callers_by_ids(graph: &GraphClient, ids: &[String]) -> Vec<Value> {
    if ids.is_empty() {
        return Vec::new();
    }
    let batch: Vec<GraphParam> = ids
        .iter()
        .map(|id| {
            let mut item = HashMap::new();
            item.insert("id".to_string(), GraphParam::String(id.clone()));
            GraphParam::Map(item)
        })
        .collect();
    let mut params = HashMap::new();
    params.insert("batch".to_string(), GraphParam::List(batch));
    let rows = graph
        .raw_query_with_param_map(
            "UNWIND $batch AS item
             MATCH (caller:CodeNode {id: item.id})
             RETURN caller",
            params,
        )
        .await
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| {
            row.get("caller").map(|caller| {
                json!({
                    "caller": caller,
                })
            })
        })
        .collect()
}

fn direct_caller_ids(rows: &[Value]) -> HashSet<String> {
    rows.iter()
        .filter_map(|row| {
            row.get("caller")
                .and_then(|c| c.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

pub async fn compute_impact_graph(
    graph: &GraphClient,
    analyzer: &Analyzer,
    params: &ImpactGraphParams,
) -> (Value, Vec<String>, bool) {
    let depth = params.depth.clamp(1, 8);
    let reach_fast_path = depth <= 3;
    let reach_props = if reach_fast_path {
        fetch_reach_properties(graph, params.symbol.as_str()).await
    } else {
        None
    };
    let mut reach_index_used = false;

    let direct = analyzer
        .callers(params.symbol.as_str())
        .await
        .unwrap_or_default();

    let transitive = if let Some(reach) = reach_props.as_ref() {
        reach_index_used = true;
        if depth == 1 {
            analyzer
                .who_calls(params.symbol.as_str(), Some(1))
                .await
                .unwrap_or_default()
        } else {
            let direct_ids = direct_caller_ids(&direct);
            let transitive_ids: Vec<String> = reach
                .reach_d3_ids
                .iter()
                .filter(|id| !direct_ids.contains(*id))
                .cloned()
                .collect();
            let mut rows = fetch_callers_by_ids(graph, &transitive_ids).await;
            if reach.reach_truncated {
                rows.push(json!({
                    "reach_truncated": true,
                    "reach_d1_count": reach.reach_d1_count,
                }));
            }
            rows
        }
    } else {
        analyzer
            .who_calls(params.symbol.as_str(), Some(depth))
            .await
            .unwrap_or_default()
    };

    let importers = if params.include_importers {
        analyzer
            .find_importers(params.symbol.as_str())
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let blast = if transitive.len() > 20 {
        "high"
    } else if transitive.len() > 5 {
        "medium"
    } else {
        "low"
    };

    let budget = params.budget_tokens;
    let max_rows = (budget / 80).clamp(10, 500) as usize;
    let mut warnings = Vec::new();
    let mut partial = false;
    let total = direct.len() + transitive.len() + importers.len();
    let (direct_out, transitive_out, importers_out) = if total > max_rows {
        partial = true;
        warnings.push("truncated_for_budget".to_string());
        let d = direct.len().min(max_rows / 3);
        let t = transitive.len().min(max_rows / 3);
        let i = importers.len().min(max_rows / 3);
        (
            direct.into_iter().take(d).collect(),
            transitive.into_iter().take(t).collect(),
            importers.into_iter().take(i).collect(),
        )
    } else {
        (direct, transitive, importers)
    };

    let mut summary = json!({
        "direct_callers": direct_out.len(),
        "transitive_callers": transitive_out.len(),
        "importers": importers_out.len(),
        "dependents": direct_out.len() + importers_out.len(),
        "blast_radius": blast,
        "depth_used": depth,
        "budget_tokens": budget
    });
    if reach_index_used {
        summary["reach_index_used"] = json!(true);
    }

    let payload = json!({
        "root": {
            "name": params.symbol,
            "symbol_type": params.symbol_type
        },
        "direct_callers": direct_out,
        "transitive_callers": transitive_out,
        "importers": importers_out,
        "summary": summary
    });

    (payload, warnings, partial)
}

/// Convert impact graph JSON to A2A ImpactSummary.
pub fn impact_summary_for_a2a(
    target: &str,
    include_paths: &[String],
    graph_payload: &Value,
    graph_error: Option<&str>,
) -> ImpactSummary {
    let uses_spin = target.contains("spin") || include_paths.iter().any(|p| p.contains("spin"));

    if let Some(err) = graph_error {
        return ImpactSummary {
            target: target.to_string(),
            risk_level: if uses_spin {
                RiskLevel::Critical
            } else {
                RiskLevel::Medium
            },
            summary: format!("graph unavailable: {err}"),
            has_cycle_risk: uses_spin,
            freshness: "unknown".to_string(),
            warnings: vec![err.to_string()],
            suggested_next_tools: Vec::new(),
            data_json: None,
        };
    }

    let summary_obj = graph_payload.get("summary").cloned().unwrap_or(json!({}));
    let direct = summary_obj
        .get("direct_callers")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let transitive = summary_obj
        .get("transitive_callers")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let blast = summary_obj
        .get("blast_radius")
        .and_then(Value::as_str)
        .unwrap_or("low");

    let risk_level = if uses_spin {
        RiskLevel::Critical
    } else {
        match blast {
            "high" => RiskLevel::High,
            "medium" => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    };

    let has_cycle_risk = uses_spin || (direct > 3 && transitive > 15);

    ImpactSummary {
        target: target.to_string(),
        risk_level,
        summary: format!(
            "get_impact_graph: {} direct callers, {} transitive (blast_radius={})",
            direct, transitive, blast
        ),
        has_cycle_risk,
        freshness: "unknown".to_string(),
        warnings: Vec::new(),
        suggested_next_tools: Vec::new(),
        data_json: Some(graph_payload.clone()),
    }
}

pub async fn build_impact_pack(
    client: &GraphClient,
    analyzer: &Analyzer,
    repo_path: &str,
    include_paths: &[String],
    params: &ImpactGraphParams,
    workflow: Option<&str>,
) -> IntelligencePack {
    let (payload, mut warnings, partial) = compute_impact_graph(client, analyzer, params).await;
    if partial {
        warnings.push("partial_response".to_string());
    }
    let freshness_label = path_freshness(client, repo_path, include_paths)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    if freshness_label != "fresh" {
        warnings.push(format!("index freshness is {freshness_label}"));
    }
    let estimated = (payload.to_string().len() / 4).min(params.budget_tokens as usize);
    let mut pack = IntelligencePack::new(
        payload,
        IntelligenceMeta {
            freshness: parse_freshness_label(&freshness_label),
            warnings,
            budget_tokens: params.budget_tokens,
            estimated_tokens: estimated,
            ..IntelligenceMeta::default()
        },
    )
    .with_tool("get_impact_graph");
    pack.meta.suggested_next_tools =
        tool_router::next_tools("get_impact_graph", &pack, workflow, None);
    pack
}
