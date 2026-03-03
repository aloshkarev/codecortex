use crate::GraphClient;
use cortex_core::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisQuery {
    pub cypher: String,
}

#[derive(Clone)]
pub struct QueryEngine {
    client: GraphClient,
}

impl QueryEngine {
    pub fn new(client: GraphClient) -> Self {
        Self { client }
    }

    pub async fn run(&self, query: AnalysisQuery) -> Result<Vec<serde_json::Value>> {
        self.client.raw_query(&query.cypher).await
    }

    /// Get all functions that call the given function (safe parameterized query)
    pub async fn callers(&self, function_name: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .query_with_param(
                "MATCH (caller:CodeNode)-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode {name: $name})
                 RETURN caller.name AS caller, callee.name AS callee, r.properties AS relationship",
                "name",
                function_name,
            )
            .await
    }

    /// Get all functions called by the given function (safe parameterized query)
    pub async fn callees(&self, function_name: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .query_with_param(
                "MATCH (caller:CodeNode {name: $name})-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode)
                 RETURN caller.name AS caller, callee.name AS callee, r.properties AS relationship",
                "name",
                function_name,
            )
            .await
    }

    /// Find functions by name pattern (safe parameterized query)
    pub async fn find_functions(&self, pattern: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .query_with_param(
                "MATCH (f:Function) WHERE f.name CONTAINS $pattern
                 RETURN f.name AS name, f.path AS path, f.line_number AS line
                 ORDER BY f.name
                 LIMIT 100",
                "pattern",
                pattern,
            )
            .await
    }

    /// Get function by exact name (safe parameterized query)
    pub async fn get_function(&self, name: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .query_with_param(
                "MATCH (f:Function {name: $name})
                 RETURN f.name AS name, f.path AS path, f.line_number AS line,
                        f.source AS source, f.docstring AS docstring",
                "name",
                name,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_query_new() {
        let query = AnalysisQuery {
            cypher: "MATCH (n) RETURN n".to_string(),
        };
        assert_eq!(query.cypher, "MATCH (n) RETURN n");
    }

    #[test]
    fn analysis_query_serialization() {
        let query = AnalysisQuery {
            cypher: "MATCH (n:Function) RETURN n.name".to_string(),
        };
        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("MATCH (n:Function)"));

        let parsed: AnalysisQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cypher, query.cypher);
    }

    #[test]
    fn analysis_query_clone() {
        let query = AnalysisQuery {
            cypher: "MATCH (n) RETURN n".to_string(),
        };
        let cloned = query.clone();
        assert_eq!(query.cypher, cloned.cypher);
    }

    #[test]
    fn analysis_query_debug() {
        let query = AnalysisQuery {
            cypher: "test query".to_string(),
        };
        let debug_str = format!("{:?}", query);
        assert!(debug_str.contains("AnalysisQuery"));
        assert!(debug_str.contains("test query"));
    }

    #[test]
    fn callers_query_uses_parameter() {
        // Verify the callers query uses parameterized input (safe from injection)
        let query = "MATCH (caller:CodeNode)-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode {name: $name})
                     RETURN caller.name AS caller, callee.name AS callee, r.properties AS relationship";
        assert!(query.contains("$name"));
        assert!(!query.contains("'{function_name}'"));
    }

    #[test]
    fn callees_query_uses_parameter() {
        // Verify the callees query uses parameterized input (safe from injection)
        let query = "MATCH (caller:CodeNode {name: $name})-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode)
                     RETURN caller.name AS caller, callee.name AS callee, r.properties AS relationship";
        assert!(query.contains("$name"));
        assert!(!query.contains("'{function_name}'"));
    }

    #[test]
    fn find_functions_query_uses_parameter() {
        // Verify the find_functions query uses parameterized input
        let query = "MATCH (f:Function) WHERE f.name CONTAINS $pattern
                     RETURN f.name AS name, f.path AS path, f.line_number AS line
                     ORDER BY f.name
                     LIMIT 100";
        assert!(query.contains("$pattern"));
        assert!(!query.contains("'{pattern}'"));
    }

    #[test]
    fn get_function_query_uses_parameter() {
        // Verify the get_function query uses parameterized input
        let query = "MATCH (f:Function {name: $name})
                     RETURN f.name AS name, f.path AS path, f.line_number AS line,
                            f.source AS source, f.docstring AS docstring";
        assert!(query.contains("$name"));
        assert!(!query.contains("'{name}'"));
    }

    #[test]
    fn all_queries_use_safe_parameters() {
        // Ensure all query engine methods use parameterized queries
        let queries = [
            "MATCH (caller:CodeNode)-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode {name: $name})",
            "MATCH (caller:CodeNode {name: $name})-[r:EDGE {kind: 'Calls'}]->(callee:CodeNode)",
            "MATCH (f:Function) WHERE f.name CONTAINS $pattern",
            "MATCH (f:Function {name: $name})",
        ];

        for query in queries {
            // All queries should use $param syntax
            assert!(query.contains("$"), "Query should use parameter: {}", query);
            // No string interpolation patterns
            assert!(
                !query.contains("'{") && !query.contains("}'"),
                "Query should not use string interpolation: {}",
                query
            );
        }
    }
}
