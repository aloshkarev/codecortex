//! Token-bounded patch context (shared by get_patch_context and A2A patch planner).

use super::filters::{ScopeFilters, path_matches_scope};
use super::pack::{IntelligenceMeta, IntelligencePack, parse_freshness_label};
use super::types::{detect_intent, redact_secrets};
use super::{freshness::path_freshness, tool_router};
use cortex_analyzer::Analyzer;
use cortex_core::SearchKind;
use cortex_graph::GraphClient;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct PatchContextParams {
    pub task: String,
    pub mode: Option<String>,
    pub budget_tokens: u32,
    pub scope: ScopeFilters,
}

#[derive(Debug, Clone)]
pub struct PatchContextData {
    pub mode: String,
    pub task: String,
    pub targets: Vec<Value>,
    pub contracts: Vec<Value>,
    pub likely_tests: Vec<Value>,
    pub estimated_tokens: usize,
    pub budget_tokens: u32,
    pub capsule_uri: String,
    pub summary: String,
    pub include_paths: Vec<String>,
}

pub async fn compute_patch_context(
    analyzer: &Analyzer,
    params: &PatchContextParams,
) -> PatchContextData {
    let mode = params
        .mode
        .clone()
        .unwrap_or_else(|| detect_intent(&params.task).to_string());
    let budget = params.budget_tokens;
    let rows = analyzer
        .find_code(&params.task, SearchKind::Pattern, None)
        .await
        .unwrap_or_default();

    let mut targets = Vec::new();
    let mut contracts = Vec::new();
    let mut target_names = Vec::new();
    let mut summary_parts = Vec::new();
    let mut estimated_tokens = 256usize;

    for row in rows.into_iter().take(20) {
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
        let excerpt = redact_secrets(&source.lines().take(8).collect::<Vec<_>>().join("\n"));
        estimated_tokens += excerpt.len() / 4 + 48;
        if estimated_tokens > budget as usize {
            break;
        }
        let name = node
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if !name.is_empty() {
            target_names.push(name.clone());
            summary_parts.push(format!("{name} @ {path}"));
            if contracts.len() < 8 {
                contracts.push(json!({
                    "symbol": name,
                    "path": path,
                    "signature_hint": source.lines().next().unwrap_or_default(),
                    "contract_kind": node.get("kind").cloned().unwrap_or(Value::Null)
                }));
            }
        }
        targets.push(json!({
            "kind": node.get("kind").cloned().unwrap_or(Value::Null),
            "name": node.get("name").cloned().unwrap_or(Value::Null),
            "path": path,
            "line_number": node.get("line_number").cloned().unwrap_or(Value::Null),
            "context_kind": "target",
            "excerpt": excerpt,
            "why": "lexical match for patch task"
        }));
    }

    let mut likely_tests = Vec::new();
    for name in target_names.iter().take(5) {
        for test in analyzer
            .find_tests_for(name)
            .await
            .unwrap_or_default()
            .into_iter()
            .take(3)
        {
            likely_tests.push(test);
        }
    }

    let target_count = targets.len();
    let summary = if summary_parts.is_empty() {
        params.task.clone()
    } else {
        format!("{} ({} graph targets)", params.task, target_count)
    };

    let include_paths = if params.scope.include_paths.is_empty() {
        vec!["src".to_string()]
    } else {
        params.scope.include_paths.clone()
    };

    PatchContextData {
        mode,
        task: params.task.clone(),
        targets,
        contracts,
        likely_tests,
        estimated_tokens,
        budget_tokens: budget,
        capsule_uri: format!(
            "codecortex://session/capsule/{}?budget={budget}",
            uuid::Uuid::new_v4()
        ),
        summary,
        include_paths,
    }
}

pub fn patch_capsule_from_data(data: &PatchContextData) -> (String, String, Vec<String>) {
    (
        data.capsule_uri.clone(),
        data.summary.clone(),
        data.include_paths.clone(),
    )
}

pub fn patch_context_json(data: &PatchContextData) -> Value {
    json!({
        "mode": data.mode,
        "task": data.task,
        "targets": data.targets,
        "contracts": data.contracts,
        "likely_tests": data.likely_tests,
        "risks": ["verify index_status freshness before high-confidence edits"],
        "estimated_tokens": data.estimated_tokens,
        "budget_tokens": data.budget_tokens,
        "capsule_uri": data.capsule_uri,
        "summary": data.summary,
        "include_paths": data.include_paths,
    })
}

pub async fn build_patch_pack(
    client: &GraphClient,
    analyzer: &Analyzer,
    repo_path: &str,
    params: &PatchContextParams,
    workflow: Option<&str>,
) -> IntelligencePack {
    let data = compute_patch_context(analyzer, params).await;
    let freshness_label = path_freshness(client, repo_path, &params.scope.include_paths)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    let mut warnings = Vec::new();
    if freshness_label != "fresh" {
        warnings.push(format!("index freshness is {freshness_label}"));
    }
    let omitted = if data.estimated_tokens >= params.budget_tokens as usize {
        warnings.push("budget_exhausted_before_all_candidates".to_string());
        true
    } else {
        false
    };
    if omitted {
        let _ = omitted;
    }
    let json_data = patch_context_json(&data);
    let mut pack = IntelligencePack::new(
        json_data,
        IntelligenceMeta {
            freshness: parse_freshness_label(&freshness_label),
            warnings,
            budget_tokens: params.budget_tokens,
            estimated_tokens: data.estimated_tokens,
            suggested_next_tools: tool_router::next_tools(
                "get_patch_context",
                &IntelligencePack::new(Value::Null, IntelligenceMeta::default()),
                workflow,
                None,
            ),
            ..IntelligenceMeta::default()
        },
    )
    .with_tool("get_patch_context")
    .with_capsule_uri(data.capsule_uri.clone());
    pack.meta.suggested_next_tools =
        tool_router::next_tools("get_patch_context", &pack, workflow, None);
    pack
}
