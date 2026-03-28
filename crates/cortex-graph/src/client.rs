//! Graph Database Client
//!
//! This module provides a unified client for graph databases, supporting both
//! Memgraph (via rsmgclient) and Neo4j (via neo4rs).

use crate::backend::BackendKind;
use crate::memgraph::MemgraphClient;
use crate::schema;
use anyhow::Context;
use cortex_core::{CodeEdge, CodeNode, CortexConfig, CortexError, Repository, Result};
use neo4rs::{ConfigBuilder, Graph, query};
use rsmgclient::QueryParam;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Internal driver enum to hold either Memgraph or Neo4j connection
enum GraphDriver {
    /// Memgraph connection using rsmgclient
    Memgraph(MemgraphClient),
    /// Neo4j connection using neo4rs
    Neo4j(Arc<Graph>),
}

/// Unified graph database client supporting multiple backends
#[derive(Clone)]
pub struct GraphClient {
    driver: Arc<GraphDriver>,
    backend: BackendKind,
}

impl GraphClient {
    /// Detect backend type from configuration
    fn detect_backend(config: &CortexConfig) -> BackendKind {
        // Check environment variable override first
        if let Ok(backend_type) = std::env::var("CORTEX_BACKEND_TYPE") {
            let backend_lower = backend_type.to_lowercase();
            if backend_lower == "neo4j" {
                debug!("Backend type explicitly set to Neo4j via CORTEX_BACKEND_TYPE");
                return BackendKind::Neo4j;
            } else if backend_lower == "memgraph" {
                debug!("Backend type explicitly set to Memgraph via CORTEX_BACKEND_TYPE");
                return BackendKind::Memgraph;
            }
        }

        // Config override is the next priority (used by quickstart/install defaults).
        let configured = config.backend_type.trim().to_lowercase();
        if configured == "neo4j" {
            debug!("Backend type set to Neo4j via config.backend_type");
            return BackendKind::Neo4j;
        }
        if configured == "memgraph" {
            debug!("Backend type set to Memgraph via config.backend_type");
            return BackendKind::Memgraph;
        }

        // Detect from URI
        BackendKind::from_uri(&config.memgraph_uri)
    }

    /// Connect to a graph database
    #[instrument(skip(config), fields(uri = %config.memgraph_uri))]
    pub async fn connect(config: &CortexConfig) -> Result<Self> {
        let backend = Self::detect_backend(config);
        info!(uri = %config.memgraph_uri, backend = ?backend, "Connecting to graph database");

        let driver = match backend {
            BackendKind::Memgraph => {
                debug!("Using Memgraph driver (rsmgclient)");
                let client = MemgraphClient::connect(config).await?;
                Arc::new(GraphDriver::Memgraph(client))
            }
            BackendKind::Neo4j | BackendKind::Neptune | BackendKind::Other => {
                debug!("Using Neo4j driver (neo4rs)");
                let graph = ConfigBuilder::default()
                    .uri(config.memgraph_uri.as_str())
                    .user(config.memgraph_user.as_str())
                    .password(config.memgraph_password.as_str())
                    .build()
                    .map_err(|e| {
                        warn!(error = %e.to_string(), "Failed to build database config");
                        CortexError::Database(e.to_string())
                    })?;

                let graph =
                    tokio::time::timeout(std::time::Duration::from_secs(10), Graph::connect(graph))
                        .await
                        .map_err(|e| CortexError::Database(format!("Connection timeout: {}", e)))?
                        .map_err(|e| {
                            warn!(error = %e.to_string(), "Failed to connect to graph database");
                            CortexError::Database(e.to_string())
                        })?;

                Arc::new(GraphDriver::Neo4j(Arc::new(graph)))
            }
        };

        let client = Self { driver, backend };
        schema::ensure_constraints(&client).await?;
        info!(backend = ?backend, "Successfully connected to graph database");
        Ok(client)
    }

    /// Get the backend type
    pub fn backend(&self) -> BackendKind {
        self.backend
    }

    /// Get the inner Neo4j graph if using Neo4j backend
    pub fn inner_neo4j(&self) -> Option<Arc<Graph>> {
        match self.driver.as_ref() {
            GraphDriver::Neo4j(graph) => Some(Arc::clone(graph)),
            GraphDriver::Memgraph(_) => None,
        }
    }

    /// Get the inner Memgraph client if using Memgraph backend
    pub fn inner_memgraph(&self) -> Option<MemgraphClient> {
        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => Some(client.clone()),
            GraphDriver::Neo4j(_) => None,
        }
    }

    /// Execute a Cypher query without returning results
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn run(&self, cypher: &str) -> Result<()> {
        debug!(cypher = %cypher, "Executing cypher query");
        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => client.run(cypher).await,
            GraphDriver::Neo4j(graph) => graph.run(query(cypher)).await.map_err(|e| {
                warn!(error = %e.to_string(), "Cypher query failed");
                CortexError::Database(e.to_string())
            }),
        }
    }

    /// Execute a Cypher query and return results as JSON
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn raw_query(&self, cypher: &str) -> Result<Vec<Value>> {
        debug!(cypher = %cypher, "Executing raw query");
        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => client.raw_query(cypher).await,
            GraphDriver::Neo4j(graph) => {
                let mut result = graph.execute(query(cypher)).await.map_err(|e| {
                    warn!(error = %e.to_string(), "Raw query failed");
                    CortexError::Database(e.to_string())
                })?;
                let mut rows = Vec::with_capacity(64);
                loop {
                    match result.next().await {
                        Ok(Some(row)) => match row.to::<Value>() {
                            Ok(v) => rows.push(v),
                            Err(_) => rows.push(serde_json::json!({ "row": format!("{row:?}") })),
                        },
                        Ok(None) => break,
                        Err(e) => return Err(CortexError::Database(e.to_string())),
                    }
                }
                debug!(row_count = rows.len(), "Raw query completed");
                Ok(rows)
            }
        }
    }

    /// Execute a parameterized query with a single string parameter
    pub async fn query_with_param(
        &self,
        cypher: &str,
        param_name: &str,
        param_value: &str,
    ) -> Result<Vec<Value>> {
        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => {
                client
                    .query_with_param(cypher, param_name, param_value)
                    .await
            }
            GraphDriver::Neo4j(graph) => {
                let mut result = graph
                    .execute(query(cypher).param(param_name, param_value.to_string()))
                    .await
                    .map_err(|e| CortexError::Database(e.to_string()))?;
                let mut rows = Vec::with_capacity(64);
                loop {
                    match result.next().await {
                        Ok(Some(row)) => match row.to::<Value>() {
                            Ok(v) => rows.push(v),
                            Err(_) => rows.push(serde_json::json!({ "row": format!("{row:?}") })),
                        },
                        Ok(None) => break,
                        Err(e) => return Err(CortexError::Database(e.to_string())),
                    }
                }
                Ok(rows)
            }
        }
    }

    /// Execute a parameterized query with multiple string parameters
    pub async fn query_with_params(
        &self,
        cypher: &str,
        params: Vec<(&str, String)>,
    ) -> Result<Vec<Value>> {
        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => client.query_with_params(cypher, params).await,
            GraphDriver::Neo4j(graph) => {
                let mut q = query(cypher);
                for (name, value) in params {
                    q = q.param(name, value);
                }
                let mut result = graph
                    .execute(q)
                    .await
                    .map_err(|e| CortexError::Database(e.to_string()))?;
                let mut rows = Vec::with_capacity(64);
                loop {
                    match result.next().await {
                        Ok(Some(row)) => match row.to::<Value>() {
                            Ok(v) => rows.push(v),
                            Err(_) => rows.push(serde_json::json!({ "row": format!("{row:?}") })),
                        },
                        Ok(None) => break,
                        Err(e) => return Err(CortexError::Database(e.to_string())),
                    }
                }
                Ok(rows)
            }
        }
    }

    /// Upsert a repository node
    pub async fn upsert_repository(&self, repository: &Repository) -> Result<()> {
        let repo_id = format!("repo:{}", repository.path);
        let cypher = "MERGE (r:Repository {path: $path})
             SET r:CodeNode,
                 r.id = $id,
                 r.kind = 'Repository',
                 r.name = $name,
                 r.path = $path,
                 r.watched = toBoolean($watched)";
        let params = vec![
            ("id", repo_id.clone()),
            ("path", repository.path.clone()),
            ("name", repository.name.clone()),
            ("watched", repository.watched.to_string()),
        ];
        self.query_with_params(cypher, params).await?;
        Ok(())
    }

    /// Upsert a call target node
    pub async fn upsert_call_target(&self, id: &str, name: &str) -> Result<()> {
        let cypher = "MERGE (n:CallTarget {id: $id})
             SET n:CodeNode, n.kind = 'CallTarget', n.name = $name";
        let params = vec![("id", id.to_string()), ("name", name.to_string())];
        self.query_with_params(cypher, params).await?;
        Ok(())
    }

    /// List all repositories
    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let rows = self
            .raw_query(
                "MATCH (r:Repository)
                 RETURN r.path AS path, r.name AS name, coalesce(r.watched, false) AS watched
                 ORDER BY r.path",
            )
            .await?;

        let mut repos = Vec::with_capacity(rows.len());
        for row in rows {
            let path: String = row
                .get("path")
                .and_then(|v| v.as_str())
                .map(String::from)
                .context("missing path")
                .map_err(|e| {
                    CortexError::Database(format!("failed to decode repository path: {e}"))
                })?;
            let name: String = row
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let watched: bool = row
                .get("watched")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            repos.push(Repository {
                path,
                name,
                watched,
            });
        }
        Ok(repos)
    }

    /// Delete a repository and all its nodes
    pub async fn delete_repository(&self, repository_path: &str) -> Result<()> {
        let cypher = "MATCH (r:Repository {path: $path})
             OPTIONAL MATCH (r)-[:CONTAINS*]->(n)
             DETACH DELETE n, r";
        self.query_with_param(cypher, "path", repository_path)
            .await?;
        Ok(())
    }

    /// Upsert a code node
    ///
    /// Branch and repository_path are promoted from `node.properties` to
    /// top-level graph properties so that scoped queries, indexes, and branch
    /// cleanup can match them directly.
    pub async fn upsert_node(&self, node: &CodeNode) -> Result<()> {
        let label = node.kind.cypher_label();
        let cyclomatic = node
            .properties
            .get("cyclomatic_complexity")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0);

        let branch = node.properties.get("branch").cloned().unwrap_or_default();
        let repository_path = node
            .properties
            .get("repository_path")
            .cloned()
            .unwrap_or_default();
        let qualified_name = node
            .properties
            .get("qualified_name")
            .cloned()
            .unwrap_or_default();
        let visibility = node
            .properties
            .get("visibility")
            .cloned()
            .unwrap_or_default();

        let cypher = format!(
            "MERGE (n:{label} {{id: $id}})
             SET n:CodeNode,
                 n.kind = $kind, n.name = $name, n.path = $path,
                 n.line_number = toInteger($line_number), n.lang = $lang,
                 n.source = $source, n.docstring = $docstring,
                 n.cyclomatic_complexity = toInteger($cyclomatic_complexity),
                 n.qualified_name = $qualified_name,
                 n.visibility = $visibility,
                 n.properties = $properties,
                 n.branch = $branch,
                 n.repository_path = $repository_path"
        );

        let params = vec![
            ("id", node.id.clone()),
            ("kind", format!("{:?}", node.kind)),
            ("name", node.name.clone()),
            ("path", node.path.clone().unwrap_or_default()),
            (
                "line_number",
                node.line_number.unwrap_or_default().to_string(),
            ),
            (
                "lang",
                node.lang
                    .map(|l| l.as_str().to_string())
                    .unwrap_or_default(),
            ),
            ("source", node.source.clone().unwrap_or_default()),
            ("docstring", node.docstring.clone().unwrap_or_default()),
            ("cyclomatic_complexity", cyclomatic.to_string()),
            (
                "properties",
                serde_json::to_string(&node.properties).unwrap_or_default(),
            ),
            ("qualified_name", qualified_name),
            ("visibility", visibility),
            ("branch", branch),
            ("repository_path", repository_path),
        ];

        self.query_with_params(&cypher, params).await?;
        Ok(())
    }

    /// Upsert an edge
    pub async fn upsert_edge(&self, edge: &CodeEdge) -> Result<()> {
        let rel_type = edge.kind.cypher_rel_type();
        let cypher = format!(
            "MATCH (from {{id: $from}}), (to {{id: $to}})
             MERGE (from)-[r:{rel_type}]->(to)
             SET r.kind = $kind, r.properties = $properties"
        );

        let params = vec![
            ("from", edge.from.clone()),
            ("to", edge.to.clone()),
            ("kind", format!("{:?}", edge.kind)),
            (
                "properties",
                serde_json::to_string(&edge.properties).unwrap_or_default(),
            ),
        ];

        self.query_with_params(&cypher, params).await?;
        Ok(())
    }

    /// Bulk-upsert multiple nodes in a single Cypher round-trip (Memgraph) or via
    /// per-node fallback (Neo4j).
    ///
    /// Nodes are grouped by their Cypher label so the `UNWIND … MERGE (n:Label …)`
    /// query stays static.  Each label group is sent as one parameterised query
    /// containing `QueryParam::List(QueryParam::Map)`, which Memgraph handles
    /// efficiently without N separate round-trips.
    pub async fn bulk_upsert_nodes(&self, nodes: &[CodeNode]) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }

        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => {
                // Group nodes by label to keep the MERGE clause label-safe.
                let mut by_label: HashMap<&'static str, Vec<&CodeNode>> = HashMap::new();
                for node in nodes {
                    by_label
                        .entry(node.kind.cypher_label())
                        .or_default()
                        .push(node);
                }

                for (label, group) in &by_label {
                    let batch = build_node_batch_param(group);
                    let cypher = format!(
                        "UNWIND $batch AS item \
                         MERGE (n:{label} {{id: item.id}}) \
                         SET n:CodeNode, \
                             n.kind = item.kind, n.name = item.name, \
                             n.path = item.path, \
                             n.line_number = item.line_number, \
                             n.lang = item.lang, \
                             n.source = item.source, n.docstring = item.docstring, \
                             n.cyclomatic_complexity = item.cyclomatic_complexity, \
                             n.qualified_name = item.qualified_name, \
                             n.visibility = item.visibility, \
                             n.properties = item.properties, \
                             n.branch = item.branch, \
                             n.repository_path = item.repository_path"
                    );
                    let mut params = HashMap::new();
                    params.insert("batch".to_string(), batch);
                    client.execute_with_raw_params(&cypher, params).await?;
                }
            }
            // Neo4j: fall back to per-node upserts (neo4rs has its own connection pool).
            GraphDriver::Neo4j(_) => {
                for node in nodes {
                    self.upsert_node(node).await?;
                }
            }
        }
        Ok(())
    }

    /// Bulk-upsert multiple edges in a single Cypher round-trip per relationship
    /// type (Memgraph) or via per-edge fallback (Neo4j).
    pub async fn bulk_upsert_edges(&self, edges: &[CodeEdge]) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => {
                // Group edges by relationship type.
                let mut by_rel: HashMap<&'static str, Vec<&CodeEdge>> = HashMap::new();
                for edge in edges {
                    by_rel
                        .entry(edge.kind.cypher_rel_type())
                        .or_default()
                        .push(edge);
                }

                for (rel_type, group) in &by_rel {
                    let batch = build_edge_batch_param(group);
                    let cypher = format!(
                        "UNWIND $batch AS item \
                         MATCH (from {{id: item.from}}), (to {{id: item.to}}) \
                         MERGE (from)-[r:{rel_type}]->(to) \
                         SET r.kind = item.kind, r.properties = item.properties"
                    );
                    let mut params = HashMap::new();
                    params.insert("batch".to_string(), batch);
                    client.execute_with_raw_params(&cypher, params).await?;
                }
            }
            GraphDriver::Neo4j(_) => {
                for edge in edges {
                    self.upsert_edge(edge).await?;
                }
            }
        }
        Ok(())
    }

    /// Bulk-upsert call-target placeholder nodes in a single round-trip (Memgraph)
    /// or per-target fallback (Neo4j).
    pub async fn bulk_upsert_call_targets(&self, targets: &[(String, String)]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }

        match self.driver.as_ref() {
            GraphDriver::Memgraph(client) => {
                let batch: Vec<QueryParam> = targets
                    .iter()
                    .map(|(id, name)| {
                        let mut m = HashMap::new();
                        m.insert("id".to_string(), QueryParam::String(id.clone()));
                        m.insert("name".to_string(), QueryParam::String(name.clone()));
                        QueryParam::Map(m)
                    })
                    .collect();

                let cypher = "UNWIND $batch AS item \
                              MERGE (n:CallTarget {id: item.id}) \
                              SET n:CodeNode, n.kind = 'CallTarget', n.name = item.name";
                let mut params = HashMap::new();
                params.insert("batch".to_string(), QueryParam::List(batch));
                client.execute_with_raw_params(cypher, params).await?;
            }
            GraphDriver::Neo4j(_) => {
                for (id, name) in targets {
                    self.upsert_call_target(id, name).await?;
                }
            }
        }
        Ok(())
    }

    /// Resolve call targets to concrete functions.
    ///
    /// When `branch` is provided, resolution is scoped to nodes on that branch,
    /// preventing cross-branch mis-linking.
    pub async fn resolve_call_targets(
        &self,
        repository_path: &str,
        branch: Option<&str>,
    ) -> Result<usize> {
        let branch_filter = if branch.is_some() {
            " AND caller.branch = $branch AND callee.branch = $branch"
        } else {
            ""
        };

        let cypher = format!(
            "MATCH (caller)-[old:CALLS]->(ct:CallTarget)
             WHERE caller.repository_path = $repo
             WITH caller, old, ct, coalesce(old.callee_name, ct.name) AS callee_name
             MATCH (callee:Function {{name: callee_name}})
             WHERE callee.repository_path = $repo{branch_filter}
             MERGE (caller)-[r:CALLS]->(callee)
             SET r.kind = 'Calls', r.properties = old.properties
             DELETE old
             RETURN count(r) AS resolved"
        );

        let mut params = vec![("repo", repository_path.to_string())];
        if let Some(br) = branch {
            params.push(("branch", br.to_string()));
        }

        let rows = self.query_with_params(&cypher, params).await?;

        let resolved = rows
            .iter()
            .filter_map(|row| row.get("resolved").and_then(|v| v.as_u64()))
            .sum::<u64>() as usize;

        // Cleanup orphaned call targets
        self.run(
            "MATCH (ct:CallTarget)
             WHERE NOT ()-->(ct)
             DETACH DELETE ct",
        )
        .await?;

        Ok(resolved)
    }

    /// Resolve TYPE_REFERENCE placeholders to concrete type/code nodes.
    pub async fn resolve_type_references(
        &self,
        repository_path: &str,
        branch: Option<&str>,
    ) -> Result<usize> {
        let branch_filter = if branch.is_some() {
            " AND source.branch = $branch AND target.branch = $branch"
        } else {
            ""
        };

        let cypher = format!(
            "MATCH (source)-[old:TYPE_REFERENCE]->(ct:CallTarget)
             WHERE source.repository_path = $repo
             WITH source, old, ct
             MATCH (target:CodeNode {{name: ct.name}})
             WHERE target.repository_path = $repo
               AND target.kind IN ['CLASS', 'STRUCT', 'TRAIT', 'INTERFACE', 'ENUM', 'TYPE_ALIAS']
               {branch_filter}
             MERGE (source)-[r:TYPE_REFERENCE]->(target)
             SET r.kind = 'TypeReference', r.properties = old.properties
             DELETE old
             RETURN count(r) AS resolved"
        );

        let mut params = vec![("repo", repository_path.to_string())];
        if let Some(br) = branch {
            params.push(("branch", br.to_string()));
        }
        let rows = self.query_with_params(&cypher, params).await?;

        let resolved = rows
            .iter()
            .filter_map(|row| row.get("resolved").and_then(|v| v.as_u64()))
            .sum::<u64>() as usize;
        Ok(resolved)
    }

    /// Resolve FIELD_ACCESS placeholders to concrete property/field-like nodes.
    pub async fn resolve_field_accesses(
        &self,
        repository_path: &str,
        branch: Option<&str>,
    ) -> Result<usize> {
        let branch_filter = if branch.is_some() {
            " AND source.branch = $branch AND target.branch = $branch"
        } else {
            ""
        };

        let cypher = format!(
            "MATCH (source)-[old:FIELD_ACCESS]->(ct:CallTarget)
             WHERE source.repository_path = $repo
             WITH source, old, ct
             MATCH (target:CodeNode {{name: ct.name}})
             WHERE target.repository_path = $repo
               AND target.kind IN ['FIELD', 'PROPERTY', 'VARIABLE', 'CONSTANT', 'PARAMETER', 'ENUM_VARIANT']
               {branch_filter}
             MERGE (source)-[r:FIELD_ACCESS]->(target)
             SET r.kind = 'FieldAccess', r.properties = old.properties
             DELETE old
             RETURN count(r) AS resolved"
        );

        let mut params = vec![("repo", repository_path.to_string())];
        if let Some(br) = branch {
            params.push(("branch", br.to_string()));
        }
        let rows = self.query_with_params(&cypher, params).await?;

        let resolved = rows
            .iter()
            .filter_map(|row| row.get("resolved").and_then(|v| v.as_u64()))
            .sum::<u64>() as usize;
        Ok(resolved)
    }
}

/// Build a `QueryParam::List(QueryParam::Map)` batch for a slice of `CodeNode`s.
///
/// Each map entry uses `QueryParam::Int` for numeric fields so Memgraph stores
/// them as integers without needing a `toInteger()` cast in the Cypher query.
fn build_node_batch_param(nodes: &[&CodeNode]) -> QueryParam {
    let items: Vec<QueryParam> = nodes
        .iter()
        .map(|node| {
            let cyclomatic = node
                .properties
                .get("cyclomatic_complexity")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);
            let branch = node.properties.get("branch").cloned().unwrap_or_default();
            let repository_path = node
                .properties
                .get("repository_path")
                .cloned()
                .unwrap_or_default();
            let qualified_name = node
                .properties
                .get("qualified_name")
                .cloned()
                .unwrap_or_default();
            let visibility = node
                .properties
                .get("visibility")
                .cloned()
                .unwrap_or_default();

            let mut m = HashMap::new();
            m.insert("id".to_string(), QueryParam::String(node.id.clone()));
            m.insert(
                "kind".to_string(),
                QueryParam::String(format!("{:?}", node.kind)),
            );
            m.insert("name".to_string(), QueryParam::String(node.name.clone()));
            m.insert(
                "path".to_string(),
                QueryParam::String(node.path.clone().unwrap_or_default()),
            );
            m.insert(
                "line_number".to_string(),
                QueryParam::Int(node.line_number.unwrap_or_default() as i64),
            );
            m.insert(
                "lang".to_string(),
                QueryParam::String(
                    node.lang
                        .map(|l| l.as_str().to_string())
                        .unwrap_or_default(),
                ),
            );
            m.insert(
                "source".to_string(),
                QueryParam::String(node.source.clone().unwrap_or_default()),
            );
            m.insert(
                "docstring".to_string(),
                QueryParam::String(node.docstring.clone().unwrap_or_default()),
            );
            m.insert(
                "cyclomatic_complexity".to_string(),
                QueryParam::Int(cyclomatic),
            );
            m.insert(
                "properties".to_string(),
                QueryParam::String(serde_json::to_string(&node.properties).unwrap_or_default()),
            );
            m.insert(
                "qualified_name".to_string(),
                QueryParam::String(qualified_name),
            );
            m.insert("visibility".to_string(), QueryParam::String(visibility));
            m.insert("branch".to_string(), QueryParam::String(branch));
            m.insert(
                "repository_path".to_string(),
                QueryParam::String(repository_path),
            );
            QueryParam::Map(m)
        })
        .collect();

    QueryParam::List(items)
}

/// Build a `QueryParam::List(QueryParam::Map)` batch for a slice of `CodeEdge`s.
fn build_edge_batch_param(edges: &[&CodeEdge]) -> QueryParam {
    let items: Vec<QueryParam> = edges
        .iter()
        .map(|edge| {
            let mut m = HashMap::new();
            m.insert("from".to_string(), QueryParam::String(edge.from.clone()));
            m.insert("to".to_string(), QueryParam::String(edge.to.clone()));
            m.insert(
                "kind".to_string(),
                QueryParam::String(format!("{:?}", edge.kind)),
            );
            m.insert(
                "properties".to_string(),
                QueryParam::String(serde_json::to_string(&edge.properties).unwrap_or_default()),
            );
            QueryParam::Map(m)
        })
        .collect();

    QueryParam::List(items)
}

/// Extract properties from a Neo4j node (for Neo4j backend compatibility)
#[allow(dead_code)]
pub(crate) fn extract_node_properties(node: &neo4rs::Node) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
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

/// Extract properties from a Neo4j relationship (for Neo4j backend compatibility)
#[allow(dead_code)]
pub(crate) fn extract_relation_properties(
    rel: &neo4rs::Relation,
) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
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
