use crate::schema;
use anyhow::Context;
use cortex_core::{CodeEdge, CodeNode, CortexConfig, CortexError, Repository, Result};
use neo4rs::{Graph, Node, Relation, query};
use serde_json::{Map, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct GraphClient {
    graph: Arc<Graph>,
}

impl GraphClient {
    pub async fn connect(config: &CortexConfig) -> Result<Self> {
        let graph = Graph::new(
            config.memgraph_uri.as_str(),
            config.memgraph_user.as_str(),
            config.memgraph_password.as_str(),
        )
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

        let client = Self {
            graph: Arc::new(graph),
        };
        schema::ensure_constraints(&client).await?;
        Ok(client)
    }

    pub fn inner(&self) -> Arc<Graph> {
        Arc::clone(&self.graph)
    }

    pub async fn run(&self, cypher: &str) -> Result<()> {
        self.graph
            .run(query(cypher))
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn raw_query(&self, cypher: &str) -> Result<Vec<serde_json::Value>> {
        let mut result = self
            .graph
            .execute(query(cypher))
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;
        let mut rows = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            match row.to::<Value>() {
                Ok(v) => rows.push(v),
                Err(_) => rows.push(serde_json::json!({ "row": format!("{row:?}") })),
            }
        }
        Ok(rows)
    }

    pub async fn upsert_repository(&self, repository: &Repository) -> Result<()> {
        let repo_id = format!("repo:{}", repository.path);
        let q = query(
            "MERGE (r:Repository {path: $path})
             SET r:CodeNode,
                 r.id = $id,
                 r.kind = 'Repository',
                 r.name = $name,
                 r.path = $path,
                 r.watched = $watched",
        )
        .param("id", repo_id)
        .param("path", repository.path.clone())
        .param("name", repository.name.clone())
        .param("watched", repository.watched);
        self.graph
            .run(q)
            .await
            .map_err(|e| CortexError::Database(e.to_string()))
    }

    pub async fn upsert_call_target(&self, id: &str, name: &str) -> Result<()> {
        let q = query(
            "MERGE (n:CallTarget {id: $id})
             SET n:CodeNode, n.kind = 'CallTarget', n.name = $name",
        )
        .param("id", id.to_string())
        .param("name", name.to_string());
        self.graph
            .run(q)
            .await
            .map_err(|e| CortexError::Database(e.to_string()))
    }

    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let mut result = self
            .graph
            .execute(query(
                "MATCH (r:Repository)
                 RETURN r.path AS path, r.name AS name, coalesce(r.watched, false) AS watched
                 ORDER BY r.path",
            ))
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut repos = Vec::new();
        while let Ok(Some(row)) = result.next().await {
            let path: String = row.get("path").context("missing path").map_err(|e| {
                CortexError::Database(format!("failed to decode repository path: {e}"))
            })?;
            let name: String = row.get("name").unwrap_or_default();
            let watched: bool = row.get("watched").unwrap_or(false);
            repos.push(Repository {
                path,
                name,
                watched,
            });
        }
        Ok(repos)
    }

    pub async fn delete_repository(&self, repository_path: &str) -> Result<()> {
        let q = query(
            "MATCH (r:Repository {path: $path})
             OPTIONAL MATCH (r)-[:CONTAINS*]->(n)
             DETACH DELETE n, r",
        )
        .param("path", repository_path.to_string());
        self.graph
            .run(q)
            .await
            .map_err(|e| CortexError::Database(e.to_string()))
    }

    pub async fn upsert_node(&self, node: &CodeNode) -> Result<()> {
        let label = node.kind.cypher_label();
        let cyclomatic = node
            .properties
            .get("cyclomatic_complexity")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);
        let q = query(&format!(
            "MERGE (n:{label} {{id: $id}})
             SET n:CodeNode,
                 n.kind = $kind, n.name = $name, n.path = $path,
                 n.line_number = $line_number, n.lang = $lang,
                 n.source = $source, n.docstring = $docstring,
                 n.cyclomatic_complexity = $cyclomatic_complexity,
                 n.properties = $properties"
        ))
        .param("id", node.id.clone())
        .param("kind", format!("{:?}", node.kind))
        .param("name", node.name.clone())
        .param("path", node.path.clone().unwrap_or_default())
        .param("line_number", node.line_number.unwrap_or_default() as i64)
        .param(
            "lang",
            node.lang
                .map(|l| l.as_str().to_string())
                .unwrap_or_default(),
        )
        .param("source", node.source.clone().unwrap_or_default())
        .param("docstring", node.docstring.clone().unwrap_or_default())
        .param("cyclomatic_complexity", cyclomatic)
        .param(
            "properties",
            serde_json::to_string(&node.properties).unwrap_or_default(),
        );
        self.graph
            .run(q)
            .await
            .map_err(|e| CortexError::Database(e.to_string()))
    }

    pub async fn upsert_edge(&self, edge: &CodeEdge) -> Result<()> {
        let rel_type = edge.kind.cypher_rel_type();
        let q = query(&format!(
            "MATCH (from {{id: $from}}), (to {{id: $to}})
             MERGE (from)-[r:{rel_type}]->(to)
             SET r.kind = $kind, r.properties = $properties"
        ))
        .param("from", edge.from.clone())
        .param("to", edge.to.clone())
        .param("kind", format!("{:?}", edge.kind))
        .param(
            "properties",
            serde_json::to_string(&edge.properties).unwrap_or_default(),
        );
        self.graph
            .run(q)
            .await
            .map_err(|e| CortexError::Database(e.to_string()))
    }

    /// Resolves symbolic CALLS edges (to `:CallTarget` placeholders) into concrete
    /// `(:Function)-[:CALLS]->(:Function)` edges for a repository.
    pub async fn resolve_call_targets(&self, repository_path: &str) -> Result<usize> {
        let mut result = self
            .graph
            .execute(
                query(
                    "MATCH (caller)-[old:CALLS]->(ct:CallTarget)
                     WHERE caller.path STARTS WITH $repo
                     WITH caller, old, ct, coalesce(old.callee_name, ct.name) AS callee_name
                     MATCH (callee:Function {name: callee_name})
                     WHERE callee.path STARTS WITH $repo
                     MERGE (caller)-[r:CALLS]->(callee)
                     SET r.kind = 'Calls', r.properties = old.properties
                     DELETE old
                     RETURN count(r) AS resolved",
                )
                .param("repo", repository_path.to_string()),
            )
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut resolved = 0usize;
        while let Ok(Some(row)) = result.next().await {
            if let Ok(count) = row.get::<i64>("resolved") {
                resolved += count.max(0) as usize;
            }
        }

        // Cleanup only placeholders that became orphaned after resolution.
        self.graph
            .run(query(
                "MATCH (ct:CallTarget)
                 WHERE NOT ()-[:CALLS]->(ct)
                 DETACH DELETE ct",
            ))
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;

        Ok(resolved)
    }
}

#[allow(dead_code)]
pub(crate) fn extract_node_properties(node: &Node) -> Map<String, Value> {
    let mut out = Map::new();
    for key in node.keys() {
        if let Ok(v) = node.get::<String>(key) {
            out.insert(key.to_string(), Value::String(v));
            continue;
        }
        if let Ok(v) = node.get::<i64>(key) {
            out.insert(key.to_string(), Value::Number(v.into()));
            continue;
        }
        if let Ok(v) = node.get::<f64>(key)
            && let Some(n) = serde_json::Number::from_f64(v)
        {
            out.insert(key.to_string(), Value::Number(n));
            continue;
        }
        if let Ok(v) = node.get::<bool>(key) {
            out.insert(key.to_string(), Value::Bool(v));
            continue;
        }
        if let Ok(v) = node.get::<Value>(key) {
            out.insert(key.to_string(), v);
        }
    }
    out
}

#[allow(dead_code)]
pub(crate) fn extract_relation_properties(rel: &Relation) -> Map<String, Value> {
    let mut out = Map::new();
    for key in rel.keys() {
        if let Ok(v) = rel.get::<String>(key) {
            out.insert(key.to_string(), Value::String(v));
            continue;
        }
        if let Ok(v) = rel.get::<i64>(key) {
            out.insert(key.to_string(), Value::Number(v.into()));
            continue;
        }
        if let Ok(v) = rel.get::<bool>(key) {
            out.insert(key.to_string(), Value::Bool(v));
            continue;
        }
        if let Ok(v) = rel.get::<Value>(key) {
            out.insert(key.to_string(), v);
        }
    }
    out
}
