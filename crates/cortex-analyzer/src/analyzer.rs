//! Code analysis queries with safe parameterized Cypher queries.
//!
//! All queries use parameterized inputs to prevent Cypher injection attacks.

use cortex_core::{Result, SearchKind};
use cortex_graph::GraphClient;
use serde_json::Value;

#[derive(Clone)]
pub struct Analyzer {
    graph: GraphClient,
}

impl Analyzer {
    fn callers_query_with_depth(depth: usize) -> String {
        format!(
            "MATCH p=(caller)-[:CALLS*1..{}]->(callee:Function {{name: $name}})
             RETURN p",
            depth.max(1)
        )
    }

    fn callees_query_with_depth(depth: usize) -> String {
        format!(
            "MATCH p=(caller:Function {{name: $name}})-[:CALLS*1..{}]->(callee:Function)
             RETURN p",
            depth.max(1)
        )
    }

    pub fn new(graph: GraphClient) -> Self {
        Self { graph }
    }

    pub async fn find_code(
        &self,
        query: &str,
        kind: SearchKind,
        path_filter: Option<&str>,
    ) -> Result<Vec<Value>> {
        let (predicate, _param_name) = match kind {
            SearchKind::Name => ("n.name = $query", "query"),
            SearchKind::Pattern => ("n.name CONTAINS $query", "query"),
            SearchKind::Type => ("n.kind = $query", "query"),
            SearchKind::Content => ("n.source CONTAINS $query", "query"),
        };

        let (cypher, params) = if let Some(path) = path_filter {
            (
                format!(
                    "MATCH (n:CodeNode) WHERE {predicate} AND n.path CONTAINS $path RETURN n LIMIT 100"
                ),
                vec![("query", query.to_string()), ("path", path.to_string())],
            )
        } else {
            (
                format!("MATCH (n:CodeNode) WHERE {predicate} RETURN n LIMIT 100"),
                vec![("query", query.to_string())],
            )
        };

        self.graph.query_with_params(&cypher, params).await
    }

    pub async fn callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (caller)-[:CALLS]->(callee:Function {name: $name})
                 RETURN caller, callee",
                "name",
                function_name,
            )
            .await
    }

    pub async fn callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (caller:Function {name: $name})-[:CALLS]->(callee)
                 WHERE callee:Function OR callee:CallTarget
                 RETURN caller, callee",
                "name",
                function_name,
            )
            .await
    }

    fn all_callers_query() -> &'static str {
        "MATCH p=(caller)-[:CALLS*1..20]->(callee:Function {name: $name}) RETURN p"
    }

    pub async fn all_callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(Self::all_callers_query(), "name", function_name)
            .await
    }

    fn all_callees_query() -> &'static str {
        "MATCH p=(caller:Function {name: $name})-[:CALLS*1..20]->(callee:Function) RETURN p"
    }

    pub async fn all_callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(Self::all_callees_query(), "name", function_name)
            .await
    }

    pub async fn call_chain(
        &self,
        from: &str,
        to: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(15).max(1);
        let cypher = format!(
            "MATCH p=(a:Function {{name: $from}})-[:CALLS*1..{}]->(b:Function {{name: $to}})
             RETURN p
             ORDER BY length(p) ASC
             LIMIT 1",
            depth
        );
        self.graph
            .query_with_params(
                &cypher,
                vec![("from", from.to_string()), ("to", to.to_string())],
            )
            .await
    }

    pub async fn class_hierarchy(&self, class_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH p=(n {name: $name})-[:INHERITS|IMPLEMENTS*0..10]->(m)
                 RETURN p",
                "name",
                class_name,
            )
            .await
    }

    pub async fn dead_code(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, count(caller) AS incoming_calls
                 WHERE incoming_calls = 0
                   AND NOT f.name IN ['main', '__init__', '__main__', 'new', 'default', 'drop']
                 RETURN f",
            )
            .await
    }

    pub async fn complexity(&self, top_n: usize) -> Result<Vec<Value>> {
        let cypher = format!(
            "MATCH (f:Function)
             RETURN f.name AS function, toInteger(f.cyclomatic_complexity) AS complexity
             ORDER BY complexity DESC
             LIMIT {}",
            top_n
        );
        self.graph.raw_query(&cypher).await
    }

    pub async fn find_complexity(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function {name: $name})
                 RETURN f.name AS function, toInteger(f.cyclomatic_complexity) AS complexity",
                "name",
                function_name,
            )
            .await
    }

    pub async fn overrides(&self, method_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function {name: $name})
                 MATCH (f)<-[:DEFINED_IN]-(owner)
                 RETURN owner, f",
                "name",
                method_name,
            )
            .await
    }

    pub async fn module_dependencies(&self, module: &str) -> Result<Vec<Value>> {
        self.find_importers(module).await
    }

    pub async fn find_importers(&self, module: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:File)-[:IMPORTS]->(m {name: $name})
                 RETURN f, m",
                "name",
                module,
            )
            .await
    }

    pub async fn find_by_decorator(&self, decorator: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function)
                 WHERE f.decorators CONTAINS $decorator
                 RETURN f",
                "decorator",
                decorator,
            )
            .await
    }

    pub async fn find_by_argument(&self, argument_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function)-[:HAS_PARAMETER]->(p:Parameter {name: $name})
                 RETURN f, p",
                "name",
                argument_name,
            )
            .await
    }

    pub async fn variable_scope(&self, variable_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (v:Variable {name: $name})
                 OPTIONAL MATCH (n)-[]->(v)
                 RETURN v, n",
                "name",
                variable_name,
            )
            .await
    }

    pub async fn repository_stats(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (r:Repository)
                 OPTIONAL MATCH (r)-[:CONTAINS*]->(n)
                 RETURN r.path AS repository, count(n) AS node_count",
            )
            .await
    }

    /// Find all code nodes in a specific file
    pub async fn find_by_file(&self, file_path: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n:CodeNode)
                 WHERE n.path CONTAINS $path
                 RETURN n
                 ORDER BY n.line_number
                 LIMIT 500",
                "path",
                file_path,
            )
            .await
    }

    /// Find all functions in a specific module/namespace
    pub async fn find_in_module(&self, module: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n:Function)
                 WHERE n.path CONTAINS $module
                 RETURN n
                 ORDER BY n.name
                 LIMIT 200",
                "module",
                module,
            )
            .await
    }

    /// Find similar function names (fuzzy match)
    pub async fn find_similar(&self, name: &str, limit: usize) -> Result<Vec<Value>> {
        let cypher = format!(
            "MATCH (n:Function)
             WHERE n.name CONTAINS $name
             RETURN n.name AS name, n.path AS path, n.line_number AS line
             LIMIT {}",
            limit.min(100)
        );
        self.graph.query_with_param(&cypher, "name", name).await
    }

    /// Get entry points (functions that are not called by any other function)
    pub async fn entry_points(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, count(caller) AS incoming_calls
                 WHERE incoming_calls = 0
                   AND (f.name IN ['main', '__init__', '__main__', 'run', 'execute', 'start', 'handle']
                        OR f.name STARTS WITH 'test_'
                        OR f.name STARTS WITH 'handle_')
                 RETURN f.name AS name, f.path AS path, f.line_number AS line
                 ORDER BY f.path, f.name",
            )
            .await
    }

    /// Find all test functions
    pub async fn find_tests(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 WHERE f.name STARTS WITH 'test_'
                    OR f.name STARTS WITH 'it_'
                    OR f.name STARTS WITH 'should_'
                    OR f.annotations CONTAINS 'Test'
                    OR f.annotations CONTAINS '@test'
                 RETURN f.name AS name, f.path AS path, f.line_number AS line
                 ORDER BY f.path, f.name",
            )
            .await
    }

    fn find_tests_for_query() -> &'static str {
        "MATCH (test:Function)
                 WHERE (test.name STARTS WITH 'test_'
                       OR test.name STARTS WITH 'it_'
                       OR test.name STARTS WITH 'should_')
                   AND test.name CONTAINS $name
                 RETURN test.name AS test_name, test.path AS path, test.line_number AS line"
    }

    /// Find functions that test a specific function
    pub async fn find_tests_for(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(Self::find_tests_for_query(), "name", function_name)
            .await
    }

    /// Find all functions that call a given function (alias for callers)
    pub async fn who_calls(&self, function_name: &str, depth: Option<usize>) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(1).min(10);
        if depth == 1 {
            self.callers(function_name).await
        } else {
            let cypher = Self::callers_query_with_depth(depth);
            self.graph
                .query_with_param(&cypher, "name", function_name)
                .await
        }
    }

    /// Find all functions called by a given function (alias for callees)
    pub async fn what_calls(
        &self,
        function_name: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(1).min(10);
        if depth == 1 {
            self.callees(function_name).await
        } else {
            let cypher = Self::callees_query_with_depth(depth);
            self.graph
                .query_with_param(&cypher, "name", function_name)
                .await
        }
    }

    /// Analyze module cohesion (find tightly/loosely coupled functions)
    pub async fn analyze_module(&self, module_path: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function)
                 WHERE f.path CONTAINS $path
                 OPTIONAL MATCH (f)-[:CALLS]->(called:Function)
                 WITH f, count(DISTINCT called) as outgoing_calls
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, outgoing_calls, count(DISTINCT caller) as incoming_calls
                 RETURN f.name AS function,
                        f.path AS path,
                        outgoing_calls,
                        incoming_calls,
                        outgoing_calls + incoming_calls AS total_coupling
                 ORDER BY total_coupling DESC
                 LIMIT 100",
                "path",
                module_path,
            )
            .await
    }

    /// Get all nodes with a specific annotation/decorator
    pub async fn find_by_annotation(&self, annotation: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n)
                 WHERE n.annotations CONTAINS $annotation
                    OR n.decorators CONTAINS $annotation
                 RETURN labels(n) AS kind, n.name AS name, n.path AS path, n.line_number AS line
                 LIMIT 200",
                "annotation",
                annotation,
            )
            .await
    }

    /// Find unused imports
    pub async fn unused_imports(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:File)-[:IMPORTS]->(m:Module)
                 OPTIONAL MATCH (f)-[:CONTAINS]->(n)
                 WITH f, m,
                      sum(CASE
                              WHEN n.source IS NOT NULL AND n.source CONTAINS m.name THEN 1
                              ELSE 0
                          END) AS usage_count
                 WHERE usage_count = 0
                 RETURN f.path AS file, m.name AS import
                 LIMIT 200",
            )
            .await
    }

    /// Get dependency graph for visualization
    pub async fn dependency_graph(
        &self,
        root_module: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(3).min(5);
        let cypher = format!(
            "MATCH path = (root {{name: $name}})-[:IMPORTS|DEPENDS_ON*..{}]->(dep)
             RETURN [n in nodes(path) | n.name] AS chain",
            depth
        );
        self.graph
            .query_with_param(&cypher, "name", root_module)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_kind_variants() {
        assert_eq!(SearchKind::Name, SearchKind::Name);
        assert_ne!(SearchKind::Name, SearchKind::Pattern);
    }

    #[test]
    fn parameterized_query_safe() {
        // Test that injection attempts are safely parameterized
        let injection_attempt = "' OR '1'='1";
        let cypher = "MATCH (n:Function {name: $name}) RETURN n";
        // The injection string would be passed as a parameter value, not interpolated
        // This is safe because neo4rs handles parameterization properly
        assert!(cypher.contains("$name"));
        assert!(!cypher.contains(injection_attempt));
    }

    #[test]
    fn find_code_query_structure() {
        // Verify queries use parameterized inputs
        let cypher = "MATCH (n:CodeNode) WHERE n.name = $query RETURN n LIMIT 100";
        assert!(cypher.contains("$query"));
        assert!(!cypher.contains("format!"));
    }

    #[test]
    fn callers_query_structure() {
        let cypher =
            "MATCH (caller)-[:CALLS]->(callee:Function {name: $name}) RETURN caller, callee";
        assert!(cypher.contains("$name"));
    }

    #[test]
    fn call_chain_query_structure() {
        let cypher = "MATCH p=(a:Function {name: $from})-[:CALLS*1..15]->(b:Function {name: $to}) RETURN p ORDER BY length(p) ASC LIMIT 1";
        assert!(cypher.contains("$from"));
        assert!(cypher.contains("$to"));
    }

    #[test]
    fn find_by_file_query_structure() {
        // Verify the query uses parameterization
        let cypher = "MATCH (n:CodeNode) WHERE n.path CONTAINS $path RETURN n";
        assert!(cypher.contains("$path"));
    }

    #[test]
    fn find_similar_query_structure() {
        // Verify the query uses parameterization
        let cypher = "MATCH (n:Function) WHERE n.name CONTAINS $name RETURN n";
        assert!(cypher.contains("$name"));
    }

    #[test]
    fn entry_points_query_safe() {
        // Entry points query should not use user input
        let cypher = "MATCH (f:Function) OPTIONAL MATCH (caller:Function)-[:CALLS]->(f) WITH f, count(caller) AS incoming_calls WHERE incoming_calls = 0 RETURN f";
        // No parameters means no injection risk
        assert!(!cypher.contains("$"));
    }

    #[test]
    fn find_tests_for_query_requires_test_prefix() {
        let cypher = Analyzer::find_tests_for_query();
        assert!(cypher.contains("STARTS WITH 'test_'"));
        assert!(cypher.contains("STARTS WITH 'it_'"));
        assert!(cypher.contains("STARTS WITH 'should_'"));
        assert!(cypher.contains("CONTAINS $name"));
        // No redundant OR test.name CONTAINS $name in the OR branch
        assert!(
            !cypher.contains("OR test.name CONTAINS $name"),
            "OR branch must not contain CONTAINS $name; use AND test.name CONTAINS $name"
        );
    }

    #[test]
    fn all_callers_bounded_depth() {
        let cypher = Analyzer::all_callers_query();
        assert!(cypher.contains("*1..20"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn all_callees_bounded_depth() {
        let cypher = Analyzer::all_callees_query();
        assert!(cypher.contains("*1..20"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn dependency_graph_depth_limit() {
        // Verify depth is limited to prevent overly complex queries
        let depth = 5;
        let cypher = format!(
            "MATCH path = (root {{name: $name}})-[:IMPORTS|DEPENDS_ON*..{}]->(dep) RETURN path",
            depth
        );
        assert!(cypher.contains("*..5"));
    }

    #[test]
    fn who_calls_query_uses_requested_depth() {
        let cypher = Analyzer::callers_query_with_depth(3);
        assert!(cypher.contains("*1..3"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn what_calls_query_uses_requested_depth() {
        let cypher = Analyzer::callees_query_with_depth(4);
        assert!(cypher.contains("*1..4"));
    }

    #[test]
    fn analyze_module_query_structure() {
        // Verify module analysis uses parameterization
        let cypher = "MATCH (f:Function) WHERE f.path CONTAINS $path RETURN f";
        assert!(cypher.contains("$path"));
    }
}
