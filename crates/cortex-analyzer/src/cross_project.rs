use cortex_core::Result;
use cortex_graph::{CrossProjectQueryBuilder, GraphClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossProjectMatch {
    pub function_name: String,
    pub repositories: Vec<String>,
    pub locations: Vec<CrossProjectLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossProjectLocation {
    pub repository: String,
    pub path: String,
    pub line_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDependency {
    pub module_name: String,
    pub repositories: Vec<String>,
    pub usage_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSurfaceComparison {
    pub repo_a: String,
    pub repo_b: String,
    pub shared_functions: Vec<String>,
    pub unique_to_a: Vec<String>,
    pub unique_to_b: Vec<String>,
    pub similarity_score: f64,
}

pub struct CrossProjectAnalyzer {
    graph: GraphClient,
}

impl CrossProjectAnalyzer {
    pub fn new(graph: GraphClient) -> Self {
        Self { graph }
    }

    /// Find symbols that exist in multiple repositories.
    pub async fn find_similar_symbols(
        &self,
        symbol_filter: Option<&str>,
        min_repos: usize,
    ) -> Result<Vec<CrossProjectMatch>> {
        let builder = CrossProjectQueryBuilder::all();
        let rows = builder
            .find_similar_across_repos(&self.graph, min_repos)
            .await?;
        let mut matches = parse_cross_project_matches(rows);
        if let Some(filter) = symbol_filter {
            let needle = filter.to_ascii_lowercase();
            matches.retain(|m| m.function_name.to_ascii_lowercase().contains(&needle));
        }
        Ok(matches)
    }

    /// Find shared import dependencies between repositories.
    pub async fn find_shared_dependencies(
        &self,
        repos: Option<&[String]>,
    ) -> Result<Vec<SharedDependency>> {
        let rows = self
            .graph
            .raw_query(
                "MATCH (f:File)-[:IMPORTS]->(m)
                 WITH m.name AS module, collect(DISTINCT f.repository_path) AS repos
                 WHERE size(repos) >= 2
                 RETURN module, repos, size(repos) AS repo_count
                 ORDER BY repo_count DESC
                 LIMIT 100",
            )
            .await?;
        Ok(parse_shared_dependencies(rows, repos))
    }

    /// Compare public API surface between two repositories.
    pub async fn compare_api_surface(
        &self,
        repo_a: &str,
        repo_b: &str,
    ) -> Result<ApiSurfaceComparison> {
        let builder = CrossProjectQueryBuilder::all();
        let shared = builder
            .compare_repositories(&self.graph, repo_a, repo_b)
            .await?;

        let funcs_a = self.get_public_functions(repo_a).await?;
        let funcs_b = self.get_public_functions(repo_b).await?;

        let shared_names: Vec<String> = shared
            .iter()
            .filter_map(|v| v.get("function_name").and_then(|n| n.as_str()))
            .map(String::from)
            .collect();
        let shared_set: HashSet<&str> = shared_names.iter().map(|s| s.as_str()).collect();

        let unique_a: Vec<String> = funcs_a
            .iter()
            .filter(|f| !shared_set.contains(f.as_str()))
            .cloned()
            .collect();
        let unique_b: Vec<String> = funcs_b
            .iter()
            .filter(|f| !shared_set.contains(f.as_str()))
            .cloned()
            .collect();

        let total = funcs_a.len() + funcs_b.len();
        let similarity = if total > 0 {
            (shared_names.len() as f64 * 2.0) / total as f64
        } else {
            0.0
        };

        Ok(ApiSurfaceComparison {
            repo_a: repo_a.to_string(),
            repo_b: repo_b.to_string(),
            shared_functions: shared_names,
            unique_to_a: unique_a,
            unique_to_b: unique_b,
            similarity_score: similarity,
        })
    }

    async fn get_public_functions(&self, repo: &str) -> Result<Vec<String>> {
        let rows = self
            .graph
            .query_with_param(
                "MATCH (f:Function {repository_path: $repo})
                 WHERE f.visibility IS NULL OR f.visibility = 'public' OR f.visibility = 'pub'
                 RETURN f.name AS name
                 ORDER BY f.name",
                "repo",
                repo,
            )
            .await?;

        Ok(rows
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
            .map(String::from)
            .collect())
    }
}

fn parse_cross_project_matches(rows: Vec<Value>) -> Vec<CrossProjectMatch> {
    rows.into_iter()
        .filter_map(|row| {
            let function_name = row.get("name")?.as_str()?.to_string();
            let repositories = row
                .get("repos")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let locations = row
                .get("locations")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|loc| {
                            Some(CrossProjectLocation {
                                repository: loc.get("repo")?.as_str()?.to_string(),
                                path: loc.get("path")?.as_str()?.to_string(),
                                line_number: loc.get("line")?.as_u64()? as u32,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(CrossProjectMatch {
                function_name,
                repositories,
                locations,
            })
        })
        .collect()
}

fn parse_shared_dependencies(rows: Vec<Value>, repos: Option<&[String]>) -> Vec<SharedDependency> {
    rows.into_iter()
        .filter_map(|row| {
            let module_name = row.get("module")?.as_str()?.to_string();
            let repositories = row
                .get("repos")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some(filter_repos) = repos
                && !repositories
                    .iter()
                    .any(|repo| filter_repos.iter().any(|needle| needle == repo))
            {
                return None;
            }
            Some(SharedDependency {
                module_name,
                usage_count: repositories.len(),
                repositories,
            })
        })
        .collect()
}
