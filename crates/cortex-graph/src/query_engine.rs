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

    pub async fn callers(&self, function_name: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .raw_query(&format!(
                "MATCH (caller:CodeNode)-[r:EDGE {{kind: 'Calls'}}]->(callee:CodeNode {{name: '{function_name}'}})
                 RETURN caller.name AS caller, callee.name AS callee, r.properties AS relationship"
            ))
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
}
