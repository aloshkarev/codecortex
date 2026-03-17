use crate::GraphClient;
use cortex_core::Result;
use serde_json::Value;

/// Query builder that operates across multiple repositories.
#[derive(Debug, Clone)]
pub struct CrossProjectQueryBuilder {
    /// If None, queries all repositories. If Some, only these.
    repositories: Option<Vec<String>>,
    /// Optional branch filter. If None, queries all branches.
    branch_filter: Option<String>,
}

impl CrossProjectQueryBuilder {
    pub fn all() -> Self {
        Self {
            repositories: None,
            branch_filter: None,
        }
    }

    pub fn with_repositories(repositories: Vec<String>) -> Self {
        Self {
            repositories: Some(repositories),
            branch_filter: None,
        }
    }

    pub fn with_branch(mut self, branch: String) -> Self {
        self.branch_filter = Some(branch);
        self
    }

    /// Build the WHERE clause for repository/branch filtering.
    pub fn build_scope_where(&self, node_var: &str) -> String {
        let mut conditions = Vec::new();
        if let Some(repos) = &self.repositories {
            let quoted = repos
                .iter()
                .map(|r| format!("'{}'", escape_cypher_string(r)))
                .collect::<Vec<_>>()
                .join(", ");
            conditions.push(format!("{node_var}.repository_path IN [{quoted}]"));
        }
        if let Some(branch) = &self.branch_filter {
            conditions.push(format!(
                "{node_var}.branch = '{}'",
                escape_cypher_string(branch)
            ));
        }
        if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        }
    }

    /// Find symbols by name across all (or filtered) repositories.
    pub async fn find_by_name(&self, client: &GraphClient, name: &str) -> Result<Vec<Value>> {
        let where_clause = self.build_scope_where("n");
        let name_condition = if where_clause.is_empty() {
            format!("WHERE n.name CONTAINS '{}'", escape_cypher_string(name))
        } else {
            format!("AND n.name CONTAINS '{}'", escape_cypher_string(name))
        };

        let cypher = format!(
            "MATCH (n:CodeNode)
             {where_clause} {name_condition}
             RETURN n.name AS name, n.kind AS kind, n.path AS path,
                    n.line_number AS line, n.repository_path AS repository,
                    n.branch AS branch, coalesce(n.lang, '') AS language
             ORDER BY n.repository_path, n.name
             LIMIT 200"
        );
        client.raw_query(&cypher).await
    }

    /// Find dead code across repositories in scope.
    pub async fn dead_code_cross_project(&self, client: &GraphClient) -> Result<Vec<Value>> {
        let where_clause = self.build_scope_where("f");
        let cypher = format!(
            "MATCH (f:Function)
             {where_clause}
             OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
             WITH f, count(caller) AS incoming
             WHERE incoming = 0
               AND NOT f.name IN ['main', '__init__', '__main__', 'new', 'default', 'drop']
             RETURN f.name AS function, f.path AS path,
                    f.repository_path AS repository, f.branch AS branch,
                    f.line_number AS line, 'no incoming calls' AS reason
             ORDER BY f.repository_path, f.path, f.name
             LIMIT 500"
        );
        client.raw_query(&cypher).await
    }

    /// Find functions with the same name across different repositories.
    pub async fn find_similar_across_repos(
        &self,
        client: &GraphClient,
        min_repos: usize,
    ) -> Result<Vec<Value>> {
        let where_clause = self.build_scope_where("f");
        let cypher = format!(
            "MATCH (f:Function)
             {where_clause}
             WITH f.name AS name, collect(DISTINCT f.repository_path) AS repos,
                  collect({{path: f.path, repo: f.repository_path, line: f.line_number}}) AS locations
             WHERE size(repos) >= {min_repos}
             RETURN name, size(repos) AS repo_count, repos, locations
             ORDER BY repo_count DESC, name
             LIMIT 100"
        );
        client.raw_query(&cypher).await
    }

    /// Compare two repositories: find functions that exist in both.
    pub async fn compare_repositories(
        &self,
        client: &GraphClient,
        repo_a: &str,
        repo_b: &str,
    ) -> Result<Vec<Value>> {
        let cypher = "MATCH (a:Function {repository_path: $repo_a})
             MATCH (b:Function {repository_path: $repo_b})
             WHERE a.name = b.name
             RETURN a.name AS function_name,
                    a.path AS path_a, a.line_number AS line_a,
                    b.path AS path_b, b.line_number AS line_b,
                    coalesce(a.lang, '') AS language
             ORDER BY a.name
             LIMIT 200";
        client
            .query_with_params(
                cypher,
                vec![
                    ("repo_a", repo_a.to_string()),
                    ("repo_b", repo_b.to_string()),
                ],
            )
            .await
    }

    /// List all indexed repositories with symbol counts.
    pub async fn list_repositories_with_stats(&self, client: &GraphClient) -> Result<Vec<Value>> {
        let where_clause = self.build_scope_where("n");
        let cypher = format!(
            "MATCH (n:CodeNode)
             {where_clause}
             WITH n.repository_path AS repo, n.branch AS branch,
                  count(n) AS symbol_count,
                  count(CASE WHEN n.kind = 'FUNCTION' THEN 1 END) AS functions,
                  count(CASE WHEN n.kind = 'CLASS' THEN 1 END) AS classes
             RETURN repo, branch, symbol_count, functions, classes
             ORDER BY repo, branch"
        );
        client.raw_query(&cypher).await
    }
}

fn escape_cypher_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::CrossProjectQueryBuilder;

    #[test]
    fn test_cross_project_query_builder_all() {
        let builder = CrossProjectQueryBuilder::all();
        assert!(builder.build_scope_where("n").is_empty());
    }

    #[test]
    fn test_cross_project_query_builder_filtered() {
        let builder = CrossProjectQueryBuilder::with_repositories(vec![
            "/repo/a".to_string(),
            "/repo/b".to_string(),
        ]);
        let where_clause = builder.build_scope_where("n");
        assert!(where_clause.contains("n.repository_path IN"));
        assert!(where_clause.contains("/repo/a"));
        assert!(where_clause.contains("/repo/b"));
    }

    #[test]
    fn test_cross_project_query_builder_with_branch() {
        let builder = CrossProjectQueryBuilder::all().with_branch("main".to_string());
        let where_clause = builder.build_scope_where("n");
        assert!(where_clause.contains("n.branch = 'main'"));
    }
}
