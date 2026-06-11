//! Shared symbol resolution: repo scope, path normalization, and graph lookups.

use crate::navigation::{DefinitionConfidence, DefinitionResult, NavigationEngine};
use cortex_core::{EntityKind, Result, graph_repository_path_for_index};
use cortex_graph::GraphClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

/// Graph node hit used by signature and logic-flow tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHit {
    pub name: String,
    pub qualified_name: Option<String>,
    pub kind: String,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub source: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolveSymbolInput<'a> {
    pub symbol: &'a str,
    pub qualified_name: Option<&'a str>,
    pub from_file: Option<&'a str>,
    pub from_line: Option<u32>,
    pub repo_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResolveOutcome {
    Found {
        hits: Vec<SymbolHit>,
        tried_strategies: Vec<String>,
    },
    NotFound {
        tried_strategies: Vec<String>,
        suggestions: Vec<String>,
    },
    Ambiguous {
        candidates: Vec<SymbolHit>,
        tried_strategies: Vec<String>,
    },
}

/// Canonical graph `repository_path` for a scan path.
pub fn normalize_repo_scope(scan_path: &Path) -> String {
    graph_repository_path_for_index(scan_path, None)
}

/// Convert absolute or messy `from_file` to repo-relative path for graph `File.path` / `CodeNode.path`.
pub fn normalize_repo_relative_file(repo_scope: &str, from_file: &str) -> String {
    let file = from_file.replace('\\', "/");
    if file.is_empty() {
        return file;
    }
    let scope = repo_scope.replace('\\', "/");
    let scope_trim = scope.trim_end_matches('/');
    if !scope_trim.is_empty() && file.starts_with(scope_trim) {
        let rel = file[scope_trim.len()..].trim_start_matches('/');
        if !rel.is_empty() {
            return rel.to_string();
        }
    }
    // Common monorepo layout: absolute path contains /crates/...
    if let Some(pos) = file.find("/crates/") {
        return file[pos + 1..].to_string();
    }
    if let Some(pos) = file.find("crates/") {
        return file[pos..].to_string();
    }
    file
}

/// Whether a graph kind string is a definitional symbol (not Parameter/Property noise).
pub fn is_definitional_kind(kind: &str) -> bool {
    matches!(
        kind.to_ascii_uppercase().as_str(),
        "FUNCTION"
            | "METHOD"
            | "CLASS"
            | "STRUCT"
            | "TRAIT"
            | "ENUM"
            | "INTERFACE"
            | "TYPE_ALIAS"
            | "MODULE"
            | "IMPL"
            | "CONSTANT"
            | "VARIABLE"
    )
}

/// Whether a graph kind can participate in call-graph traversal.
pub fn is_callable_kind(kind: &str) -> bool {
    matches!(
        kind.to_ascii_uppercase().as_str(),
        "FUNCTION" | "METHOD" | "CLASS"
    )
}

/// Cypher `IN` list for callable kinds (PascalCase as stored in graph).
pub fn callable_kinds_cypher_list() -> &'static str {
    "['Function', 'Method', 'Class', 'FUNCTION', 'METHOD', 'CLASS']"
}

/// Cypher predicate: node kind is definitional (PascalCase + UPPERCASE indexer variants).
pub fn definitional_kind_cypher_predicate(node_var: &str) -> String {
    format!(
        "{node_var}.kind IN ['FUNCTION', 'Function', 'METHOD', 'Method', 'CLASS', 'Class', \
         'STRUCT', 'Struct', 'TRAIT', 'Trait', 'INTERFACE', 'Interface', 'ENUM', 'Enum', \
         'TYPE_ALIAS', 'TypeAlias', 'MODULE', 'Module', 'IMPL', 'Impl', 'CONSTANT', 'Constant', \
         'VARIABLE', 'Variable']"
    )
}

pub fn entity_kind_from_graph_kind(kind: &str) -> EntityKind {
    match kind.to_ascii_lowercase().as_str() {
        "function" => EntityKind::Function,
        "method" => EntityKind::Method,
        "struct" => EntityKind::Struct,
        "enum" => EntityKind::Enum,
        "trait" => EntityKind::Trait,
        "interface" => EntityKind::Interface,
        "class" => EntityKind::Class,
        "module" | "impl" => EntityKind::Module,
        _ => EntityKind::Function,
    }
}

pub struct SymbolResolver {
    graph: GraphClient,
    repo_scope: String,
    branch: Option<String>,
}

impl SymbolResolver {
    pub fn new(graph: GraphClient, repo_scope: String, branch: Option<String>) -> Self {
        Self {
            graph,
            repo_scope,
            branch,
        }
    }

    pub fn from_scan_path(graph: GraphClient, scan_path: &Path, branch: Option<String>) -> Self {
        Self::new(graph, normalize_repo_scope(scan_path), branch)
    }

    pub fn repo_scope(&self) -> &str {
        &self.repo_scope
    }

    /// Resolve definitions using the navigation pipeline (qualified → imports → module → global).
    pub async fn resolve_definitions(
        &self,
        input: &ResolveSymbolInput<'_>,
    ) -> Result<ResolveOutcome> {
        let mut tried = Vec::new();
        let from_file = input
            .from_file
            .map(|f| normalize_repo_relative_file(&self.repo_scope, f))
            .unwrap_or_default();

        if let Some(qn) = input.qualified_name.filter(|s| !s.is_empty()) {
            tried.push("qualified_name".to_string());
            let hits = self.lookup_by_qualified_name(qn).await?;
            if !hits.is_empty() {
                return Ok(ResolveOutcome::Found {
                    hits,
                    tried_strategies: tried,
                });
            }
        }

        let nav = NavigationEngine::new(
            self.graph.clone(),
            self.repo_scope.clone(),
            self.branch.clone(),
        );
        tried.push("navigation_engine".to_string());
        let defs = nav
            .go_to_definition(input.symbol, &from_file, input.from_line)
            .await?;
        if defs.is_empty() {
            tried.push("exact_definitional".to_string());
            let exact = self.resolve_exact_definitional(input.symbol, 10).await?;
            if !exact.is_empty() {
                let mut hits = exact;
                if !from_file.is_empty() {
                    hits.sort_by(|a, b| {
                        let a_match = a.file_path == from_file || a.file_path.ends_with(&from_file);
                        let b_match = b.file_path == from_file || b.file_path.ends_with(&from_file);
                        b_match.cmp(&a_match)
                    });
                }
                return Ok(ResolveOutcome::Found {
                    hits,
                    tried_strategies: tried,
                });
            }
            let suggestions = self.suggest_names(input.symbol, 8).await?;
            return Ok(ResolveOutcome::NotFound {
                tried_strategies: tried,
                suggestions,
            });
        }
        if defs.len() > 1
            && defs
                .iter()
                .any(|d| d.confidence != DefinitionConfidence::Exact)
        {
            let mut candidates: Vec<SymbolHit> = defs.into_iter().map(definition_to_hit).collect();
            if !from_file.is_empty() {
                candidates.sort_by(|a, b| {
                    let a_match = a.file_path == from_file || a.file_path.ends_with(&from_file);
                    let b_match = b.file_path == from_file || b.file_path.ends_with(&from_file);
                    b_match.cmp(&a_match)
                });
                let preferred: Vec<_> = candidates
                    .iter()
                    .filter(|h| h.file_path == from_file || h.file_path.ends_with(&from_file))
                    .collect();
                if preferred.len() == 1 {
                    return Ok(ResolveOutcome::Found {
                        hits: vec![preferred[0].clone()],
                        tried_strategies: tried,
                    });
                }
            }
            return Ok(ResolveOutcome::Ambiguous {
                candidates,
                tried_strategies: tried,
            });
        }
        Ok(ResolveOutcome::Found {
            hits: defs.into_iter().map(definition_to_hit).collect(),
            tried_strategies: tried,
        })
    }

    /// Exact name match on CodeNode (definitional kinds only).
    pub async fn resolve_exact_definitional(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<SymbolHit>> {
        let branch_clause = self.branch_clause("n");
        let cypher = format!(
            "MATCH (n:CodeNode)
             WHERE n.repository_path = $repo
               AND n.name = $symbol
               {branch_clause}
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line, n.source AS source, n.lang AS lang
             LIMIT {limit}"
        );
        let mut params = vec![
            ("repo", self.repo_scope.clone()),
            ("symbol", symbol.to_string()),
        ];
        if let Some(ref b) = self.branch {
            params.push(("branch", b.clone()));
        }
        let rows = self.graph.query_with_params(&cypher, params).await?;
        Ok(rows
            .iter()
            .filter_map(|r| row_to_hit(r))
            .filter(|h| is_definitional_kind(&h.kind))
            .collect())
    }

    /// Prefix / contains fallback for signature lookup after exact miss.
    pub async fn resolve_fuzzy_definitional(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<SymbolHit>> {
        let branch_clause = self.branch_clause("n");
        let cypher = format!(
            "MATCH (n:CodeNode)
             WHERE n.repository_path = $repo
               AND n.name CONTAINS $symbol
               {branch_clause}
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line, n.source AS source, n.lang AS lang
             LIMIT {limit}"
        );
        let mut params = vec![
            ("repo", self.repo_scope.clone()),
            ("symbol", symbol.to_string()),
        ];
        if let Some(ref b) = self.branch {
            params.push(("branch", b.clone()));
        }
        let rows = self.graph.query_with_params(&cypher, params).await?;
        Ok(rows
            .iter()
            .filter_map(|r| row_to_hit(r))
            .filter(|h| is_definitional_kind(&h.kind))
            .collect())
    }

    pub async fn suggest_names(&self, symbol: &str, limit: usize) -> Result<Vec<String>> {
        let cypher = format!(
            "MATCH (n:CodeNode)
             WHERE n.repository_path = $repo AND n.name CONTAINS $symbol
             RETURN DISTINCT n.name AS name LIMIT {limit}"
        );
        let rows = self
            .graph
            .query_with_params(
                &cypher,
                vec![
                    ("repo", self.repo_scope.clone()),
                    ("symbol", symbol.to_string()),
                ],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()).map(String::from))
            .collect())
    }

    async fn lookup_by_qualified_name(&self, qualified_name: &str) -> Result<Vec<SymbolHit>> {
        let branch_clause = self.branch_clause("n");
        let cypher = format!(
            "MATCH (n:CodeNode)
             WHERE n.repository_path = $repo AND n.qualified_name = $qn
               {branch_clause}
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line, n.source AS source, n.lang AS lang
             LIMIT 10"
        );
        let mut params = vec![
            ("repo", self.repo_scope.clone()),
            ("qn", qualified_name.to_string()),
        ];
        if let Some(ref b) = self.branch {
            params.push(("branch", b.clone()));
        }
        let rows = self.graph.query_with_params(&cypher, params).await?;
        Ok(rows.iter().filter_map(|r| row_to_hit(r)).collect())
    }

    fn branch_clause(&self, node_var: &str) -> String {
        if self.branch.is_some() {
            format!("AND {node_var}.branch = $branch")
        } else {
            String::new()
        }
    }
}

fn definition_to_hit(d: DefinitionResult) -> SymbolHit {
    SymbolHit {
        name: d.name,
        qualified_name: d.qualified_name,
        kind: d.kind,
        file_path: d.file_path,
        line_number: Some(d.line_number),
        source: d.source_preview,
        lang: None,
    }
}

fn row_to_hit(row: &Value) -> Option<SymbolHit> {
    Some(SymbolHit {
        name: row.get("name")?.as_str()?.to_string(),
        qualified_name: row
            .get("qualified_name")
            .and_then(|v| v.as_str())
            .map(String::from),
        kind: row.get("kind")?.as_str()?.to_string(),
        file_path: row.get("path")?.as_str()?.to_string(),
        line_number: row.get("line").and_then(|v| v.as_u64()).map(|n| n as u32),
        source: row.get("source").and_then(|v| v.as_str()).map(String::from),
        lang: row.get("lang").and_then(|v| v.as_str()).map(String::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_repo_prefix() {
        let scope = "/data/myproject";
        let file = "/data/myproject/crates/foo/src/lib.rs";
        assert_eq!(
            normalize_repo_relative_file(scope, file),
            "crates/foo/src/lib.rs"
        );
    }

    #[test]
    fn normalize_finds_crates_suffix() {
        let file = "/home/user/ws/64-codecortex/crates/cortex-mcp/src/handler.rs";
        assert_eq!(
            normalize_repo_relative_file("/other", file),
            "crates/cortex-mcp/src/handler.rs"
        );
    }

    #[test]
    fn definitional_kinds() {
        assert!(is_definitional_kind("Function"));
        assert!(is_definitional_kind("METHOD"));
        assert!(!is_definitional_kind("Parameter"));
        let pred = definitional_kind_cypher_predicate("n");
        assert!(pred.contains("'Function'"));
        assert!(pred.contains("'FUNCTION'"));
    }

    #[test]
    fn callable_kinds() {
        assert!(is_callable_kind("Method"));
        assert!(!is_callable_kind("Struct"));
    }
}
