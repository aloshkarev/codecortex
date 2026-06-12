//! Lazy MCP tool discovery: hot eager tools, deferred catalog, and promotion tracking.

use crate::tool_names;
use rmcp::model::Tool;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};

/// Tools always listed when lazy discovery is enabled (routing + discovery itself).
pub const ALWAYS_LIVE_TOOLS: &[&str] = &["tools_search", "tool_profile"];

/// High-frequency tools listed eagerly when `CORTEX_LAZY_TOOLS=1`.
pub const HOT_EAGER_TOOLS: &[&str] = &[
    "check_health",
    "index_status",
    "recommend_tools",
    "get_tool_guidance",
    "find_code",
    "go_to_definition",
    "find_all_usages",
    "get_context_capsule",
    "get_patch_context",
    "get_delta_context",
    "get_test_context",
    "get_impact_graph",
    "get_skeleton",
    "get_signature",
    "analyze_code_relationships",
    "pr_review",
    "manage_codecortex",
    "explain_index_freshness",
    "ctx_stats",
    "ctx_grep",
];

/// Whether lazy tool discovery is enabled (`CORTEX_LAZY_TOOLS=1`).
pub fn lazy_tools_enabled() -> bool {
    matches!(
        std::env::var("CORTEX_LAZY_TOOLS")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

/// Names that should appear in `tools/list` for the current promotion state.
pub fn live_tool_names(promoted: &HashSet<String>) -> HashSet<String> {
    let mut live = HashSet::new();
    let exported: HashSet<&str> = tool_names().iter().copied().collect();
    for name in ALWAYS_LIVE_TOOLS
        .iter()
        .chain(HOT_EAGER_TOOLS.iter())
        .copied()
    {
        if exported.contains(name) {
            live.insert(name.to_string());
        }
    }
    for name in promoted {
        if exported.contains(name.as_str()) {
            live.insert(name.clone());
        }
    }
    live
}

/// Deferred tools (exported but not currently live).
/// Names of tools not yet promoted in the current session (lazy-load routing).
#[allow(dead_code)]
pub fn deferred_tool_names(promoted: &HashSet<String>) -> Vec<String> {
    let exported: HashSet<&str> = tool_names().iter().copied().collect();
    let live = live_tool_names(promoted);
    let mut deferred: Vec<String> = exported
        .iter()
        .filter(|name| !live.contains(**name))
        .map(|name| (*name).to_string())
        .collect();
    deferred.sort();
    deferred
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolProfileReport {
    pub lazy_enabled: bool,
    pub live_count: usize,
    pub deferred_count: usize,
    pub promoted_count: usize,
    pub total_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolProfileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolProfileEntry {
    pub name: String,
    pub live: bool,
    pub hot_eager: bool,
    pub promoted: bool,
}

/// Summarize lazy discovery state; optional `tool` reports membership for one tool.
pub fn tool_profile(promoted: &HashSet<String>, tool: Option<&str>) -> ToolProfileReport {
    let lazy_enabled = lazy_tools_enabled();
    let live = live_tool_names(promoted);
    let total_count = tool_names().len();
    let live_count = live.len();
    let deferred_count = total_count.saturating_sub(live_count);
    let tool_entry = tool.map(|name| {
        let hot_eager = HOT_EAGER_TOOLS.contains(&name) || ALWAYS_LIVE_TOOLS.contains(&name);
        ToolProfileEntry {
            name: name.to_string(),
            live: live.contains(name),
            hot_eager,
            promoted: promoted.contains(name),
        }
    });
    ToolProfileReport {
        lazy_enabled,
        live_count,
        deferred_count,
        promoted_count: promoted.len(),
        total_count,
        tool: tool_entry,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSearchMatch {
    pub name: String,
    pub score: f64,
    pub schema: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolSearchResult {
    pub query: String,
    pub lazy_enabled: bool,
    pub matches: Vec<ToolSearchMatch>,
    pub promoted: Vec<String>,
}

/// Fuzzy-search deferred tools and return MCP schemas for matches.
pub fn tools_search(
    all_tools: &[Tool],
    promoted: &mut HashSet<String>,
    query: &str,
    max_results: usize,
    promote: bool,
) -> ToolSearchResult {
    let lazy_enabled = lazy_tools_enabled();
    let live = live_tool_names(promoted);
    let q = query.trim().to_ascii_lowercase();
    let limit = max_results.clamp(1, 32);

    let mut scored: Vec<(String, f64, Tool)> = all_tools
        .iter()
        .filter(|tool| !live.contains(tool.name.as_ref()))
        .filter_map(|tool| {
            let score = fuzzy_tool_score(&q, tool.name.as_ref());
            if score > 0.0 {
                Some((tool.name.to_string(), score, tool.clone()))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    scored.truncate(limit);

    let mut promoted_names = Vec::new();
    if promote {
        for (name, _, _) in &scored {
            if promoted.insert(name.clone()) {
                promoted_names.push(name.clone());
            }
        }
    }

    let matches = scored
        .into_iter()
        .map(|(name, score, tool)| ToolSearchMatch {
            name,
            score,
            schema: tool_to_schema_value(&tool),
        })
        .collect();

    ToolSearchResult {
        query: query.to_string(),
        lazy_enabled,
        matches,
        promoted: promoted_names,
    }
}

fn tool_to_schema_value(tool: &Tool) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": tool.input_schema,
    })
}

/// Subsequence + token overlap fuzzy score in `[0, 1]`.
fn fuzzy_tool_score(query: &str, tool_name: &str) -> f64 {
    if query.is_empty() {
        return 0.0;
    }
    let name = tool_name.to_ascii_lowercase();
    if name == query {
        return 1.0;
    }
    if name.contains(query) {
        return 0.92;
    }
    if query.contains(&name) {
        return 0.85;
    }

    let mut qi = query.chars().peekable();
    let mut matched = 0usize;
    for ch in name.chars() {
        if qi.peek() == Some(&ch) {
            matched += 1;
            qi.next();
        }
    }
    if matched == query.chars().count() {
        let subseq = matched as f64 / name.chars().count().max(1) as f64;
        return (0.55 + subseq * 0.35).min(0.88);
    }

    let query_tokens: Vec<&str> = query.split('_').filter(|t| !t.is_empty()).collect();
    if query_tokens.is_empty() {
        return 0.0;
    }
    let name_tokens: HashSet<&str> = name.split('_').collect();
    let overlap = query_tokens
        .iter()
        .filter(|t| name_tokens.contains(*t))
        .count();
    if overlap == 0 {
        return 0.0;
    }
    (overlap as f64 / query_tokens.len() as f64) * 0.75
}

/// Shared promotion set for handler clones.
pub type PromotedTools = std::sync::Arc<Mutex<HashSet<String>>>;

pub fn new_promoted_tools() -> PromotedTools {
    std::sync::Arc::new(Mutex::new(HashSet::new()))
}

pub fn lock_promoted(promoted: &PromotedTools) -> MutexGuard<'_, HashSet<String>> {
    promoted
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ToolAnnotations;

    fn sample_tool(name: &str) -> Tool {
        let mut schema = serde_json::Map::new();
        schema.insert("type".into(), json!("object"));
        Tool::new(
            name.to_string(),
            format!("Tool {name}"),
            std::sync::Arc::new(schema),
        )
        .annotate(ToolAnnotations::default())
    }

    #[test]
    fn lazy_tools_enabled_reads_env() {
        unsafe {
            std::env::remove_var("CORTEX_LAZY_TOOLS");
        }
        assert!(!lazy_tools_enabled());
        unsafe {
            std::env::set_var("CORTEX_LAZY_TOOLS", "1");
        }
        assert!(lazy_tools_enabled());
        unsafe {
            std::env::remove_var("CORTEX_LAZY_TOOLS");
        }
    }

    #[test]
    fn live_tools_include_hot_eager_and_discovery_tools() {
        let promoted = HashSet::new();
        let live = live_tool_names(&promoted);
        assert!(live.contains("check_health"));
        assert!(live.contains("tools_search"));
        assert!(live.contains("get_patch_context"));
        assert!(!live.contains("vector_search"));
    }

    #[test]
    fn promotion_expands_live_set() {
        let mut promoted = HashSet::new();
        promoted.insert("vector_search".to_string());
        let live = live_tool_names(&promoted);
        assert!(live.contains("vector_search"));
        let deferred = deferred_tool_names(&promoted);
        assert!(!deferred.contains(&"vector_search".to_string()));
    }

    #[test]
    fn tools_search_ranks_substring_matches() {
        let tools = vec![
            sample_tool("vector_search"),
            sample_tool("vector_search_hybrid"),
            sample_tool("find_code"),
        ];
        let mut promoted = HashSet::new();
        let result = tools_search(&tools, &mut promoted, "vector", 5, false);
        assert!(!result.matches.is_empty());
        assert_eq!(result.matches[0].name, "vector_search");
        assert!(result.promoted.is_empty());
    }

    #[test]
    fn tools_search_promote_tracks_names() {
        let tools = vec![sample_tool("find_dead_code")];
        let mut promoted = HashSet::new();
        let result = tools_search(&tools, &mut promoted, "dead", 3, true);
        assert_eq!(result.promoted, vec!["find_dead_code".to_string()]);
        assert!(promoted.contains("find_dead_code"));
    }

    #[test]
    fn tool_profile_reports_counts() {
        let mut promoted = HashSet::new();
        promoted.insert("vector_search".to_string());
        let report = tool_profile(&promoted, Some("vector_search"));
        assert!(report.lazy_enabled || !report.lazy_enabled);
        assert!(report.total_count >= report.live_count);
        let entry = report.tool.expect("tool entry");
        assert!(entry.promoted);
        assert!(entry.live);
    }

    #[test]
    fn fuzzy_tool_score_prefers_exact_match() {
        assert_eq!(fuzzy_tool_score("find_code", "find_code"), 1.0);
        assert!(fuzzy_tool_score("patch", "get_patch_context") > 0.5);
        assert_eq!(fuzzy_tool_score("zzz", "find_code"), 0.0);
    }
}
