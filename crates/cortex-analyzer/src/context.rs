use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cortex_core::Result;
use cortex_graph::GraphClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolLocation {
    pub file_path: String,
    pub line_number: u32,
    pub kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectSymbolIndex {
    pub definitions: HashMap<String, Vec<SymbolLocation>>,
    pub callers: HashMap<String, HashSet<String>>,
    pub callees: HashMap<String, HashSet<String>>,
    pub imports: HashMap<String, Vec<String>>,
    pub module_dependents: HashMap<String, HashSet<String>>,
    pub module_dependencies: HashMap<String, HashSet<String>>,
    pub all_functions: HashSet<String>,
    pub all_types: HashSet<String>,
}

impl ProjectSymbolIndex {
    pub fn has_callers(&self, function_name: &str) -> bool {
        self.callers
            .get(function_name)
            .is_some_and(|callers| !callers.is_empty())
    }

    pub fn caller_count(&self, function_name: &str) -> usize {
        self.callers.get(function_name).map_or(0, HashSet::len)
    }

    pub fn is_defined(&self, function_name: &str) -> bool {
        self.all_functions.contains(function_name)
    }

    pub fn importers_of(&self, module_name: &str) -> Vec<&str> {
        self.imports
            .iter()
            .filter(|(_, modules)| modules.iter().any(|m| m == module_name))
            .map(|(path, _)| path.as_str())
            .collect()
    }

    pub fn dependents_of(&self, module_name: &str) -> Option<&HashSet<String>> {
        self.module_dependents.get(module_name)
    }
}

#[derive(Clone)]
pub struct ProjectAnalysisContext {
    graph: Option<GraphClient>,
    repository_path: String,
    branch: Option<String>,
    symbols: Arc<ProjectSymbolIndex>,
}

impl ProjectAnalysisContext {
    pub async fn build(
        graph: GraphClient,
        repository_path: String,
        branch: Option<String>,
    ) -> Result<Self> {
        let symbols =
            Arc::new(Self::load_symbol_index(&graph, &repository_path, branch.as_deref()).await?);
        Ok(Self {
            graph: Some(graph),
            repository_path,
            branch,
            symbols,
        })
    }

    pub fn symbols(&self) -> &ProjectSymbolIndex {
        &self.symbols
    }

    pub fn repository_path(&self) -> &str {
        &self.repository_path
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn graph(&self) -> Option<&GraphClient> {
        self.graph.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn from_symbols_for_tests(symbols: ProjectSymbolIndex) -> Self {
        Self {
            graph: None,
            repository_path: "test-repo".to_string(),
            branch: Some("test".to_string()),
            symbols: Arc::new(symbols),
        }
    }

    async fn load_symbol_index(
        graph: &GraphClient,
        repository_path: &str,
        branch: Option<&str>,
    ) -> Result<ProjectSymbolIndex> {
        let mut index = ProjectSymbolIndex::default();

        let (definitions_rows, type_rows, call_rows, import_rows, module_rows) = tokio::try_join!(
            query_definitions(graph, repository_path, branch),
            query_type_definitions(graph, repository_path, branch),
            query_calls(graph, repository_path, branch),
            query_imports(graph, repository_path, branch),
            query_module_dependencies(graph, repository_path, branch),
        )?;

        for row in definitions_rows {
            let Some(name) = row.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let file_path = row
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let line_number = row
                .get("line")
                .and_then(|v| v.as_u64())
                .map_or(0, |n| n as u32);
            let kind = row
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if !name.is_empty() {
                index.all_functions.insert(name.to_string());
                index
                    .definitions
                    .entry(name.to_string())
                    .or_default()
                    .push(SymbolLocation {
                        file_path,
                        line_number,
                        kind,
                    });
            }
        }

        for row in type_rows {
            let Some(name) = row.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            if !name.is_empty() {
                index.all_types.insert(name.to_string());
            }
        }

        for row in call_rows {
            let Some(caller_name) = row.get("caller_name").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(callee_name) = row.get("callee_name").and_then(|v| v.as_str()) else {
                continue;
            };

            index
                .callers
                .entry(callee_name.to_string())
                .or_default()
                .insert(caller_name.to_string());
            index
                .callees
                .entry(caller_name.to_string())
                .or_default()
                .insert(callee_name.to_string());
        }

        for row in import_rows {
            let Some(file_path) = row.get("file_path").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(module_name) = row.get("module_name").and_then(|v| v.as_str()) else {
                continue;
            };

            index
                .imports
                .entry(file_path.to_string())
                .or_default()
                .push(module_name.to_string());
        }

        for row in module_rows {
            let Some(from_module) = row.get("from_module").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(to_module) = row.get("to_module").and_then(|v| v.as_str()) else {
                continue;
            };

            index
                .module_dependencies
                .entry(from_module.to_string())
                .or_default()
                .insert(to_module.to_string());
            index
                .module_dependents
                .entry(to_module.to_string())
                .or_default()
                .insert(from_module.to_string());
        }

        Ok(index)
    }
}

async fn query_definitions(
    graph: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let cypher_no_branch = "MATCH (f:Function)
         WHERE f.repository_path = $repo
         RETURN f.name AS name, f.path AS path, f.line_number AS line, coalesce(f.kind, 'FUNCTION') AS kind";
    let cypher_with_branch = "MATCH (f:Function)
         WHERE f.repository_path = $repo AND f.branch = $branch
         RETURN f.name AS name, f.path AS path, f.line_number AS line, coalesce(f.kind, 'FUNCTION') AS kind";

    match branch {
        Some(br) => {
            graph
                .query_with_params(
                    cypher_with_branch,
                    vec![
                        ("repo", repository_path.to_string()),
                        ("branch", br.to_string()),
                    ],
                )
                .await
        }
        None => {
            graph
                .query_with_param(cypher_no_branch, "repo", repository_path)
                .await
        }
    }
}

async fn query_type_definitions(
    graph: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let cypher_no_branch = "MATCH (t:CodeNode)
         WHERE t.repository_path = $repo
           AND t.kind IN ['CLASS', 'STRUCT', 'TRAIT', 'INTERFACE', 'ENUM']
         RETURN t.name AS name, t.path AS path, t.line_number AS line, t.kind AS kind";
    let cypher_with_branch = "MATCH (t:CodeNode)
         WHERE t.repository_path = $repo
           AND t.branch = $branch
           AND t.kind IN ['CLASS', 'STRUCT', 'TRAIT', 'INTERFACE', 'ENUM']
         RETURN t.name AS name, t.path AS path, t.line_number AS line, t.kind AS kind";

    match branch {
        Some(br) => {
            graph
                .query_with_params(
                    cypher_with_branch,
                    vec![
                        ("repo", repository_path.to_string()),
                        ("branch", br.to_string()),
                    ],
                )
                .await
        }
        None => {
            graph
                .query_with_param(cypher_no_branch, "repo", repository_path)
                .await
        }
    }
}

async fn query_calls(
    graph: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let cypher_no_branch = "MATCH (caller:Function)-[:CALLS]->(callee:Function)
         WHERE caller.repository_path = $repo
         RETURN caller.name AS caller_name, callee.name AS callee_name";
    let cypher_with_branch = "MATCH (caller:Function)-[:CALLS]->(callee:Function)
         WHERE caller.repository_path = $repo
           AND caller.branch = $branch
           AND callee.branch = $branch
         RETURN caller.name AS caller_name, callee.name AS callee_name";

    match branch {
        Some(br) => {
            graph
                .query_with_params(
                    cypher_with_branch,
                    vec![
                        ("repo", repository_path.to_string()),
                        ("branch", br.to_string()),
                    ],
                )
                .await
        }
        None => {
            graph
                .query_with_param(cypher_no_branch, "repo", repository_path)
                .await
        }
    }
}

async fn query_imports(
    graph: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let cypher_no_branch = "MATCH (f:File)-[:IMPORTS]->(m)
         WHERE f.repository_path = $repo
         RETURN f.path AS file_path, m.name AS module_name";
    let cypher_with_branch = "MATCH (f:File)-[:IMPORTS]->(m)
         WHERE f.repository_path = $repo AND f.branch = $branch
         RETURN f.path AS file_path, m.name AS module_name";

    match branch {
        Some(br) => {
            graph
                .query_with_params(
                    cypher_with_branch,
                    vec![
                        ("repo", repository_path.to_string()),
                        ("branch", br.to_string()),
                    ],
                )
                .await
        }
        None => {
            graph
                .query_with_param(cypher_no_branch, "repo", repository_path)
                .await
        }
    }
}

async fn query_module_dependencies(
    graph: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
) -> Result<Vec<serde_json::Value>> {
    let cypher_no_branch = "MATCH (a:Module)-[:IMPORTS]->(b:Module)
         WHERE a.repository_path = $repo
         RETURN a.name AS from_module, b.name AS to_module";
    let cypher_with_branch = "MATCH (a:Module)-[:IMPORTS]->(b:Module)
         WHERE a.repository_path = $repo AND a.branch = $branch
         RETURN a.name AS from_module, b.name AS to_module";

    match branch {
        Some(br) => {
            graph
                .query_with_params(
                    cypher_with_branch,
                    vec![
                        ("repo", repository_path.to_string()),
                        ("branch", br.to_string()),
                    ],
                )
                .await
        }
        None => {
            graph
                .query_with_param(cypher_no_branch, "repo", repository_path)
                .await
        }
    }
}
