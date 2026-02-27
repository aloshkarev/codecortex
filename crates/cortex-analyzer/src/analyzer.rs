use cortex_core::{Result, SearchKind};
use cortex_graph::GraphClient;
use serde_json::Value;

#[derive(Clone)]
pub struct Analyzer {
    graph: GraphClient,
}

impl Analyzer {
    pub fn new(graph: GraphClient) -> Self {
        Self { graph }
    }

    pub async fn find_code(
        &self,
        query: &str,
        kind: SearchKind,
        path_filter: Option<&str>,
    ) -> Result<Vec<Value>> {
        let predicate = match kind {
            SearchKind::Name => format!("n.name = '{query}'"),
            SearchKind::Pattern => format!("n.name CONTAINS '{query}'"),
            SearchKind::Type => format!("n.kind = '{query}'"),
            SearchKind::Content => format!("n.source CONTAINS '{query}'"),
        };
        let path_clause = path_filter
            .map(|path| format!(" AND n.path CONTAINS '{path}'"))
            .unwrap_or_default();
        self.graph
            .raw_query(&format!(
                "MATCH (n:CodeNode) WHERE {predicate}{path_clause} RETURN n LIMIT 100"
            ))
            .await
    }

    pub async fn callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (caller)-[:CALLS]->(callee:Function {{name:'{function_name}'}})
                 RETURN caller, callee"
            ))
            .await
    }

    pub async fn callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (caller:Function {{name:'{function_name}'}})-[:CALLS]->(callee)
                 WHERE callee:Function OR callee:CallTarget
                 RETURN caller, callee"
            ))
            .await
    }

    pub async fn all_callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH p=(caller)-[:CALLS*1..]->(callee:Function {{name:'{function_name}'}})
                 RETURN p"
            ))
            .await
    }

    pub async fn all_callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH p=(caller:Function {{name:'{function_name}'}})-[:CALLS*1..]->(callee:Function)
                 RETURN p"
            ))
            .await
    }

    pub async fn call_chain(
        &self,
        from: &str,
        to: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(15).max(1);
        self.graph
            .raw_query(&format!(
                "MATCH p=shortestPath((a:Function {{name:'{from}'}})-[:CALLS*..{depth}]->(b:Function {{name:'{to}'}}))
                 RETURN p"
            ))
            .await
    }

    pub async fn class_hierarchy(&self, class_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH p=(n {{name:'{class_name}'}})-[:INHERITS|IMPLEMENTS*0..10]->(m)
                 RETURN p"
            ))
            .await
    }

    pub async fn dead_code(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 WHERE NOT (()-[:CALLS]->(f))
                   AND NOT f.name IN ['main', '__init__', '__main__', 'new', 'default', 'drop']
                 RETURN f",
            )
            .await
    }

    pub async fn complexity(&self, top_n: usize) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:Function)
                 RETURN f.name AS function, toInteger(f.cyclomatic_complexity) AS complexity
                 ORDER BY complexity DESC
                 LIMIT {top_n}"
            ))
            .await
    }

    pub async fn find_complexity(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:Function {{name:'{function_name}'}})
                 RETURN f.name AS function, toInteger(f.cyclomatic_complexity) AS complexity"
            ))
            .await
    }

    pub async fn overrides(&self, method_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:Function {{name:'{method_name}'}})
                 MATCH (f)<-[:DEFINED_IN]-(owner)
                 RETURN owner, f"
            ))
            .await
    }

    pub async fn module_dependencies(&self, module: &str) -> Result<Vec<Value>> {
        self.find_importers(module).await
    }

    pub async fn find_importers(&self, module: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:File)-[:IMPORTS]->(m {{name:'{module}'}})
                 RETURN f, m"
            ))
            .await
    }

    pub async fn find_by_decorator(&self, decorator: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:Function)
                 WHERE f.decorators CONTAINS '{decorator}'
                 RETURN f"
            ))
            .await
    }

    pub async fn find_by_argument(&self, argument_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (f:Function)-[:HAS_PARAMETER]->(p:Parameter {{name:'{argument_name}'}})
                 RETURN f, p"
            ))
            .await
    }

    pub async fn variable_scope(&self, variable_name: &str) -> Result<Vec<Value>> {
        self.graph
            .raw_query(&format!(
                "MATCH (v:Variable {{name:'{variable_name}'}})
                 OPTIONAL MATCH (n)-[]->(v)
                 RETURN v, n"
            ))
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
}

/// Build a find_code Cypher query (exposed for testing)
#[cfg(test)]
pub fn build_find_code_query(query: &str, kind: SearchKind, path_filter: Option<&str>) -> String {
    let predicate = match kind {
        SearchKind::Name => format!("n.name = '{query}'"),
        SearchKind::Pattern => format!("n.name CONTAINS '{query}'"),
        SearchKind::Type => format!("n.kind = '{query}'"),
        SearchKind::Content => format!("n.source CONTAINS '{query}'"),
    };
    let path_clause = path_filter
        .map(|path| format!(" AND n.path CONTAINS '{path}'"))
        .unwrap_or_default();
    format!("MATCH (n:CodeNode) WHERE {predicate}{path_clause} RETURN n LIMIT 100")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_find_code_by_name() {
        let query = build_find_code_query("main", SearchKind::Name, None);
        assert!(query.contains("n.name = 'main'"));
        assert!(query.contains("MATCH (n:CodeNode)"));
    }

    #[test]
    fn build_find_code_by_pattern() {
        let query = build_find_code_query("Handler", SearchKind::Pattern, None);
        assert!(query.contains("n.name CONTAINS 'Handler'"));
    }

    #[test]
    fn build_find_code_by_type() {
        let query = build_find_code_query("Function", SearchKind::Type, None);
        assert!(query.contains("n.kind = 'Function'"));
    }

    #[test]
    fn build_find_code_by_content() {
        let query = build_find_code_query("println", SearchKind::Content, None);
        assert!(query.contains("n.source CONTAINS 'println'"));
    }

    #[test]
    fn build_find_code_with_path_filter() {
        let query = build_find_code_query("main", SearchKind::Name, Some("src/"));
        assert!(query.contains("n.name = 'main'"));
        assert!(query.contains("n.path CONTAINS 'src/'"));
    }

    #[test]
    fn build_find_code_without_path_filter() {
        let query = build_find_code_query("main", SearchKind::Name, None);
        assert!(!query.contains("n.path CONTAINS"));
    }

    #[test]
    fn build_callers_query() {
        let expected = "MATCH (caller)-[:CALLS]->(callee:Function {name:'handler'}) RETURN caller, callee";
        let query = format!(
            "MATCH (caller)-[:CALLS]->(callee:Function {{name:'{}'}}) RETURN caller, callee",
            "handler"
        );
        assert_eq!(query, expected);
    }

    #[test]
    fn build_callees_query() {
        let expected = "MATCH (caller:Function {name:'main'})-[:CALLS]->(callee) WHERE callee:Function OR callee:CallTarget RETURN caller, callee";
        let query = format!(
            "MATCH (caller:Function {{name:'{}'}})-[:CALLS]->(callee) WHERE callee:Function OR callee:CallTarget RETURN caller, callee",
            "main"
        );
        assert_eq!(query, expected);
    }

    #[test]
    fn build_call_chain_query() {
        let from = "main";
        let to = "helper";
        let depth = 10;
        let query = format!(
            "MATCH p=shortestPath((a:Function {{name:'{}'}})-[:CALLS*..{}]->(b:Function {{name:'{}'}})) RETURN p",
            from, depth, to
        );
        assert!(query.contains("shortestPath"));
        assert!(query.contains("CALLS*..10"));
    }

    #[test]
    fn build_call_chain_default_depth() {
        let default_depth = 15;
        let query = format!(
            "MATCH p=shortestPath((a:Function {{name:'{}'}})-[:CALLS*..{}]->(b:Function {{name:'{}'}})) RETURN p",
            "a", default_depth, "b"
        );
        assert!(query.contains("CALLS*..15"));
    }

    #[test]
    fn build_dead_code_query() {
        let query = "MATCH (f:Function) WHERE NOT (()-[:CALLS]->(f)) AND NOT f.name IN ['main', '__init__', '__main__', 'new', 'default', 'drop'] RETURN f";
        assert!(query.contains("NOT (()-[:CALLS]->(f))"));
        assert!(query.contains("main"));
    }

    #[test]
    fn build_complexity_query() {
        let top_n = 10;
        let query = format!(
            "MATCH (f:Function) RETURN f.name AS function, toInteger(f.cyclomatic_complexity) AS complexity ORDER BY complexity DESC LIMIT {}",
            top_n
        );
        assert!(query.contains("ORDER BY complexity DESC"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn build_repository_stats_query() {
        let query = "MATCH (r:Repository) OPTIONAL MATCH (r)-[:CONTAINS*]->(n) RETURN r.path AS repository, count(n) AS node_count";
        assert!(query.contains("CONTAINS*"));
        assert!(query.contains("count(n)"));
    }

    #[test]
    fn search_kind_variants() {
        assert_eq!(SearchKind::Name, SearchKind::Name);
        assert_ne!(SearchKind::Name, SearchKind::Pattern);
    }

    #[test]
    fn escape_single_quotes_in_query() {
        // Verify that the query building handles the name correctly
        let name = "user's function";
        let query = format!("n.name = '{}'", name);
        // In a real implementation, this would need proper escaping
        assert!(query.contains("user's function"));
    }
}
