//! Branch-scoped query utilities for multi-project support.
//!
//! This module provides query builders that automatically scope queries
//! to a specific project and branch context.
//!
//! ## Overview
//!
//! - [`ScopedQueryBuilder`]: Builder for creating branch-scoped Cypher queries
//! - [`QueryScope`]: Defines the scope (repository + branch) for queries
//! - [`ScopedResult`]: Results from scoped queries with context metadata
//!
//! ## Example
//!
//! ```rust,ignore
//! use cortex_graph::scoped_query::{ScopedQueryBuilder, QueryScope};
//!
//! let scope = QueryScope::new("/path/to/repo", "main");
//! let builder = ScopedQueryBuilder::new(scope);
//!
//! // Find functions scoped to the current branch
//! let query = builder.find_by_name("UserRepository", Some("Function"));
//! ```

use cortex_core::EntityKind;
use serde::{Deserialize, Serialize};

/// Defines the scope for branch-scoped queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryScope {
    /// Repository path (root directory of the project)
    pub repository_path: String,
    /// Branch name to scope queries to
    pub branch: String,
    /// Optional commit hash for exact matching
    pub commit: Option<String>,
}

impl QueryScope {
    /// Create a new query scope
    pub fn new(repository_path: &str, branch: &str) -> Self {
        Self {
            repository_path: repository_path.to_string(),
            branch: branch.to_string(),
            commit: None,
        }
    }

    /// Create a scope with a specific commit
    pub fn with_commit(repository_path: &str, branch: &str, commit: &str) -> Self {
        Self {
            repository_path: repository_path.to_string(),
            branch: branch.to_string(),
            commit: Some(commit.to_string()),
        }
    }

    /// Check if this scope matches a node's repository and branch
    pub fn matches(&self, node_repo: &str, node_branch: &str) -> bool {
        self.repository_path == node_repo && self.branch == node_branch
    }
}

impl Default for QueryScope {
    fn default() -> Self {
        Self {
            repository_path: String::new(),
            branch: "main".to_string(),
            commit: None,
        }
    }
}

/// Result from a scoped query with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopedResult<T> {
    /// The query result data
    pub data: T,
    /// The scope that was used for the query
    pub scope: QueryScope,
    /// Number of results before applying scope
    pub total_unscoped: usize,
    /// Number of results after applying scope
    pub total_scoped: usize,
}

impl<T> ScopedResult<T> {
    /// Create a new scoped result
    pub fn new(data: T, scope: QueryScope, total_unscoped: usize, total_scoped: usize) -> Self {
        Self {
            data,
            scope,
            total_unscoped,
            total_scoped,
        }
    }
}

/// Builder for creating branch-scoped Cypher queries
#[derive(Debug, Clone)]
pub struct ScopedQueryBuilder {
    scope: QueryScope,
}

impl ScopedQueryBuilder {
    /// Create a new scoped query builder
    pub fn new(scope: QueryScope) -> Self {
        Self { scope }
    }

    /// Get the current scope
    pub fn scope(&self) -> &QueryScope {
        &self.scope
    }

    /// Set a new scope
    pub fn with_scope(mut self, scope: QueryScope) -> Self {
        self.scope = scope;
        self
    }

    /// Build a WHERE clause for scoping to repository and branch
    pub fn build_scope_where(&self, node_var: &str) -> String {
        format!(
            "{}.repository_path = $repository_path AND {}.branch = $branch",
            node_var, node_var
        )
    }

    /// Build a query to find nodes by name, scoped to the current branch
    pub fn find_by_name(&self, _name: &str, entity_kind: Option<EntityKind>) -> String {
        let label = entity_kind
            .map(|k| format!(":{}", k.cypher_label()))
            .unwrap_or_default();

        format!(
            r#"
            MATCH (n{}:CodeNode)
            WHERE n.name = $name
              AND n.repository_path = $repository_path
              AND n.branch = $branch
            RETURN n
            ORDER BY n.name
            "#,
            label
        )
    }

    /// Build a query to find nodes by pattern (regex), scoped to the current branch
    pub fn find_by_pattern(&self, _pattern: &str, entity_kind: Option<EntityKind>) -> String {
        let label = entity_kind
            .map(|k| format!(":{}", k.cypher_label()))
            .unwrap_or_default();

        format!(
            r#"
            MATCH (n{}:CodeNode)
            WHERE n.name =~ $pattern
              AND n.repository_path = $repository_path
              AND n.branch = $branch
            RETURN n
            ORDER BY n.name
            "#,
            label
        )
    }

    /// Build a query to find nodes by path prefix, scoped to the current branch
    pub fn find_by_path_prefix(&self, _path_prefix: &str) -> String {
        r#"
            MATCH (n:CodeNode)
            WHERE n.path STARTS WITH $path_prefix
              AND n.repository_path = $repository_path
              AND n.branch = $branch
            RETURN n
            ORDER BY n.path
            "#.to_string()
    }

    /// Build a query to get callers of a function, scoped to the current branch
    pub fn find_callers(&self, _target_id: &str) -> String {
        r#"
            MATCH (caller:Function)-[:CALLS]->(target {id: $target_id})
            WHERE caller.repository_path = $repository_path
              AND caller.branch = $branch
            RETURN caller
            ORDER BY caller.name
            "#.to_string()
    }

    /// Build a query to get callees of a function, scoped to the current branch
    pub fn find_callees(&self, _source_id: &str) -> String {
        r#"
            MATCH (source {id: $source_id})-[:CALLS]->(callee:Function)
            WHERE callee.repository_path = $repository_path
              AND callee.branch = $branch
            RETURN callee
            ORDER BY callee.name
            "#.to_string()
    }

    /// Build a query to get the class hierarchy, scoped to the current branch
    pub fn find_class_hierarchy(&self, _class_id: &str) -> String {
        r#"
            MATCH path = (superclass)-[:EXTENDS*]->(subclass {id: $class_id})
            WHERE subclass.repository_path = $repository_path
              AND subclass.branch = $branch
              AND superclass.branch = $branch
            RETURN nodes(path) as hierarchy
            "#.to_string()
    }

    /// Build a query to find tests for a symbol, scoped to the current branch
    pub fn find_tests(&self, _symbol_name: &str) -> String {
        r#"
            MATCH (test:Function)
            WHERE (test.name CONTAINS 'test_' OR test.name CONTAINS '_test' OR test.name STARTS WITH 'test')
              AND test.repository_path = $repository_path
              AND test.branch = $branch
            OPTIONAL MATCH (test)-[:CALLS]->(symbol {name: $symbol_name})
            WHERE symbol.repository_path = $repository_path AND symbol.branch = $branch
            WITH test, symbol
            WHERE symbol IS NOT NULL
               OR test.docstring CONTAINS $symbol_name
               OR test.source CONTAINS $symbol_name
            RETURN DISTINCT test
            ORDER BY test.name
            "#.to_string()
    }

    /// Build a query to get impact graph (blast radius), scoped to the current branch
    pub fn find_impact_graph(&self, _symbol_id: &str, depth: usize) -> String {
        format!(
            r#"
            MATCH path = (start {{id: $symbol_id}})-[:CALLS|EXTENDS|IMPLEMENTS|IMPORTS*1..{}]->(impacted)
            WHERE start.repository_path = $repository_path
              AND start.branch = $branch
              AND impacted.branch = $branch
            RETURN DISTINCT impacted, path
            ORDER BY size(path)
            "#,
            depth
        )
    }

    /// Build a query to find dead code (unreachable nodes), scoped to the current branch
    pub fn find_dead_code(&self) -> String {
        r#"
            MATCH (n:Function)
            WHERE n.repository_path = $repository_path
              AND n.branch = $branch
              AND NOT (n.name STARTS WITH 'main')
              AND NOT (n.name STARTS WITH 'test_')
              AND NOT (n.name STARTS WITH '_')
              AND NOT ()-[:CALLS]->(n)
            RETURN n
            ORDER BY n.name
            "#.to_string()
    }

    /// Build a query to get statistics for the scoped branch
    pub fn get_branch_stats(&self) -> String {
        r#"
            MATCH (n:CodeNode)
            WHERE n.repository_path = $repository_path
              AND n.branch = $branch
            RETURN
              count(n) as total_nodes,
              count(CASE WHEN n:Function THEN 1 END) as function_count,
              count(CASE WHEN n:Class THEN 1 END) as class_count,
              count(CASE WHEN n:Variable THEN 1 END) as variable_count,
              count(CASE WHEN n:File THEN 1 END) as file_count
            "#.to_string()
    }

    /// Build a query to count nodes by kind, scoped to the current branch
    pub fn count_by_kind(&self) -> String {
        r#"
            MATCH (n:CodeNode)
            WHERE n.repository_path = $repository_path
              AND n.branch = $branch
            RETURN n.kind as kind, count(n) as count
            ORDER BY count DESC
            "#.to_string()
    }

    /// Build a query to find the most complex functions, scoped to the current branch
    pub fn find_most_complex(&self, limit: usize) -> String {
        format!(
            r#"
            MATCH (n:Function)
            WHERE n.repository_path = $repository_path
              AND n.branch = $branch
              AND n.cyclomatic_complexity > 1
            RETURN n
            ORDER BY n.cyclomatic_complexity DESC
            LIMIT {}
            "#,
            limit
        )
    }

    /// Build parameters for the current scope
    pub fn scope_params(&self) -> serde_json::Value {
        serde_json::json!({
            "repository_path": self.scope.repository_path,
            "branch": self.scope.branch,
        })
    }

    /// Build parameters with an additional value
    pub fn scope_params_with(&self, key: &str, value: serde_json::Value) -> serde_json::Value {
        let mut params = self.scope_params();
        if let serde_json::Value::Object(ref mut map) = params {
            map.insert(key.to_string(), value);
        }
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_scope_creation() {
        let scope = QueryScope::new("/path/to/repo", "main");
        assert_eq!(scope.repository_path, "/path/to/repo");
        assert_eq!(scope.branch, "main");
        assert!(scope.commit.is_none());
    }

    #[test]
    fn query_scope_with_commit() {
        let scope = QueryScope::with_commit("/path/to/repo", "feature", "abc123");
        assert_eq!(scope.commit, Some("abc123".to_string()));
    }

    #[test]
    fn query_scope_matches() {
        let scope = QueryScope::new("/repo", "main");
        assert!(scope.matches("/repo", "main"));
        assert!(!scope.matches("/repo", "dev"));
        assert!(!scope.matches("/other", "main"));
    }

    #[test]
    fn query_scope_default() {
        let scope = QueryScope::default();
        assert!(scope.repository_path.is_empty());
        assert_eq!(scope.branch, "main");
    }

    #[test]
    fn scoped_result_creation() {
        let scope = QueryScope::new("/repo", "main");
        let result: ScopedResult<Vec<String>> = ScopedResult::new(
            vec!["item1".to_string()],
            scope.clone(),
            10,
            1,
        );
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.total_unscoped, 10);
        assert_eq!(result.total_scoped, 1);
    }

    #[test]
    fn scoped_query_builder_scope_where() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let where_clause = builder.build_scope_where("n");
        assert!(where_clause.contains("n.repository_path = $repository_path"));
        assert!(where_clause.contains("n.branch = $branch"));
    }

    #[test]
    fn scoped_query_builder_find_by_name() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_by_name("UserRepository", Some(EntityKind::Function));
        assert!(query.contains("n.name = $name"));
        assert!(query.contains("n.repository_path = $repository_path"));
        assert!(query.contains("n.branch = $branch"));
        assert!(query.contains(":Function"));
    }

    #[test]
    fn scoped_query_builder_find_by_pattern() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_by_pattern("Handler.*", Some(EntityKind::Class));
        assert!(query.contains("n.name =~ $pattern"));
        assert!(query.contains(":Class"));
    }

    #[test]
    fn scoped_query_builder_find_by_path_prefix() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_by_path_prefix("/repo/src");
        assert!(query.contains("n.path STARTS WITH $path_prefix"));
    }

    #[test]
    fn scoped_query_builder_find_callers() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_callers("func:main");
        assert!(query.contains("-[:CALLS]->"));
        assert!(query.contains("$target_id"));
    }

    #[test]
    fn scoped_query_builder_find_callees() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_callees("func:main");
        assert!(query.contains("-[:CALLS]->"));
        assert!(query.contains("$source_id"));
    }

    #[test]
    fn scoped_query_builder_find_class_hierarchy() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_class_hierarchy("class:BaseHandler");
        assert!(query.contains("-[:EXTENDS*]->"));
    }

    #[test]
    fn scoped_query_builder_find_tests() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_tests("UserService");
        assert!(query.contains("test_"));
        assert!(query.contains("$symbol_name"));
    }

    #[test]
    fn scoped_query_builder_find_impact_graph() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_impact_graph("func:main", 3);
        assert!(query.contains("[:CALLS|EXTENDS|IMPLEMENTS|IMPORTS*1..3]"));
    }

    #[test]
    fn scoped_query_builder_find_dead_code() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_dead_code();
        assert!(query.contains("NOT ()-[:CALLS]->(n)"));
        assert!(query.contains("NOT (n.name STARTS WITH 'main')"));
    }

    #[test]
    fn scoped_query_builder_get_branch_stats() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.get_branch_stats();
        assert!(query.contains("count(n) as total_nodes"));
        assert!(query.contains("function_count"));
        assert!(query.contains("class_count"));
    }

    #[test]
    fn scoped_query_builder_count_by_kind() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.count_by_kind();
        assert!(query.contains("n.kind as kind"));
        assert!(query.contains("count(n) as count"));
    }

    #[test]
    fn scoped_query_builder_find_most_complex() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let query = builder.find_most_complex(20);
        assert!(query.contains("n.cyclomatic_complexity > 1"));
        assert!(query.contains("LIMIT 20"));
    }

    #[test]
    fn scoped_query_builder_scope_params() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let params = builder.scope_params();
        assert_eq!(params["repository_path"], "/repo");
        assert_eq!(params["branch"], "main");
    }

    #[test]
    fn scoped_query_builder_scope_params_with() {
        let scope = QueryScope::new("/repo", "main");
        let builder = ScopedQueryBuilder::new(scope);
        let params = builder.scope_params_with("name", serde_json::json!("UserService"));
        assert_eq!(params["repository_path"], "/repo");
        assert_eq!(params["branch"], "main");
        assert_eq!(params["name"], "UserService");
    }

    #[test]
    fn scoped_query_builder_with_scope() {
        let scope1 = QueryScope::new("/repo1", "main");
        let scope2 = QueryScope::new("/repo2", "dev");
        let builder = ScopedQueryBuilder::new(scope1).with_scope(scope2);
        assert_eq!(builder.scope().repository_path, "/repo2");
        assert_eq!(builder.scope().branch, "dev");
    }
}
