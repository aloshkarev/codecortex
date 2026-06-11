//! Graph Database Client (FalkorDB-only).

use crate::backend::{BackendKind, detect_backend_from_config};
use crate::edge_profile::EdgeWriteProfile;
use crate::falkordb::FalkorDbClient;
use crate::falkordb_params::GraphParam;
use crate::schema;
use anyhow::Context;
use cortex_core::{CodeEdge, CodeNode, CortexConfig, CortexError, Repository, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;
use tracing::{debug, info, instrument};

/// Internal FalkorDB driver (single client or write pool).
enum GraphDriver {
    FalkorDB(FalkorDbClient),
    FalkorDBPool(Arc<Vec<FalkorDbClient>>),
}

fn falkordb_primary(d: &GraphDriver) -> Option<&FalkorDbClient> {
    match d {
        GraphDriver::FalkorDB(c) => Some(c),
        GraphDriver::FalkorDBPool(v) => v.first(),
    }
}

fn falkordb_clients(d: &GraphDriver) -> Option<&[FalkorDbClient]> {
    match d {
        GraphDriver::FalkorDB(c) => Some(std::slice::from_ref(c)),
        GraphDriver::FalkorDBPool(v) => Some(v.as_ref()),
    }
}

fn write_pool_len(d: &GraphDriver) -> usize {
    match d {
        GraphDriver::FalkorDBPool(v) => v.len().max(1),
        GraphDriver::FalkorDB(_) => 1,
    }
}

fn shard_index(id: &str, shards: usize) -> usize {
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    (h.finish() as usize) % shards.max(1)
}

const FALKORDB_NODE_UNWIND_FULL: &str = "UNWIND $batch AS item
 MERGE (n:CodeNode {id: item.id})
 SET n:{label},
     n.kind = item.kind,
     n.name = item.name,
     n.path = item.path,
     n.line_number = item.line_number,
     n.lang = item.lang,
     n.source = item.source,
     n.docstring = item.docstring,
     n.cyclomatic_complexity = item.cyclomatic_complexity,
     n.qualified_name = item.qualified_name,
     n.visibility = item.visibility,
     n.properties = item.properties,
     n.branch = item.branch,
     n.repository_path = item.repository_path";

const FALKORDB_NODE_UNWIND_SLIM: &str = "UNWIND $batch AS item
 MERGE (n:CodeNode {id: item.id})
 SET n:{label},
     n.kind = item.kind,
     n.name = item.name,
     n.path = item.path,
     n.line_number = item.line_number,
     n.lang = item.lang,
     n.cyclomatic_complexity = item.cyclomatic_complexity,
     n.qualified_name = item.qualified_name,
     n.visibility = item.visibility,
     n.branch = item.branch,
     n.repository_path = item.repository_path";

async fn falkordb_execute_node_unwind_batch(
    client: &FalkorDbClient,
    label: &'static str,
    group: &[&CodeNode],
    include_source: bool,
) -> Result<()> {
    let template = if include_source {
        FALKORDB_NODE_UNWIND_FULL
    } else {
        FALKORDB_NODE_UNWIND_SLIM
    };
    let cypher = template.replace("{label}", label);
    let batch = if include_source {
        build_node_batch_param(group)
    } else {
        build_falkordb_node_batch_param_slim(group)
    };
    let mut params = HashMap::new();
    params.insert("batch".to_string(), batch);
    client.execute_with_raw_params(&cypher, params).await
}

async fn falkordb_bulk_upsert_nodes(
    clients: &[FalkorDbClient],
    nodes: &[CodeNode],
    include_source: bool,
) -> Result<()> {
    let k = clients.len().max(1);
    let mut by_label: HashMap<&'static str, Vec<&CodeNode>> = HashMap::new();
    for node in nodes {
        by_label
            .entry(node.kind.cypher_label())
            .or_default()
            .push(node);
    }

    for (label, group) in by_label {
        if k == 1 {
            falkordb_execute_node_unwind_batch(&clients[0], label, &group, include_source).await?;
            continue;
        }

        let mut buckets: Vec<Vec<CodeNode>> = (0..k).map(|_| Vec::new()).collect();
        for node in group {
            let idx = shard_index(&node.id, k);
            buckets[idx].push((*node).clone());
        }

        let mut set = JoinSet::new();
        for (i, bucket) in buckets.into_iter().enumerate() {
            if bucket.is_empty() {
                continue;
            }
            let client = clients[i].clone();
            set.spawn(async move {
                let refs: Vec<&CodeNode> = bucket.iter().collect();
                falkordb_execute_node_unwind_batch(&client, label, &refs, include_source).await
            });
        }
        while let Some(joined) = set.join_next().await {
            joined
                .map_err(|e| CortexError::Database(format!("falkordb node upsert join: {e}")))??;
        }
    }
    Ok(())
}

/// All Cypher relationship types used in bulk edge upserts (one FOREACH branch each).
const FALKOR_EDGE_REL_TYPES: &[&str] = &[
    "CONTAINS",
    "CALLS",
    "IMPORTS",
    "INHERITS",
    "IMPLEMENTS",
    "HAS_PARAMETER",
    "DEFINED_IN",
    "REFERENCES",
    "USES",
    "THROWS",
    "RETURNS",
    "HAS_FIELD",
    "HAS_METHOD",
    "HAS_PROPERTY",
    "DOCUMENTS",
    "ANNOTATES",
    "MEMBER_OF",
    "TYPE_REFERENCE",
    "FIELD_ACCESS",
    "SIMILAR_TO",
];

fn falkordb_mixed_rel_edge_cypher() -> &'static str {
    use std::sync::OnceLock;
    static CY: OnceLock<String> = OnceLock::new();
    CY.get_or_init(|| {
        let mut s = String::from(
            "UNWIND $batch AS item\n\
             MATCH (from:CodeNode {id: item.from}), (to:CodeNode {id: item.to})\n",
        );
        for rt in FALKOR_EDGE_REL_TYPES {
            s.push_str(&format!(
                "FOREACH (_ IN CASE item.rel WHEN '{rt}' THEN [1] ELSE [] END |\n\
                   MERGE (from)-[r:{rt}]->(to) SET r.kind = item.kind, r.properties = item.properties)\n"
            ));
        }
        s
    })
}

async fn falkordb_execute_mixed_rel_edge_batch(
    client: &FalkorDbClient,
    group: &[&CodeEdge],
) -> Result<()> {
    if group.is_empty() {
        return Ok(());
    }
    let mut params = HashMap::new();
    params.insert("batch".to_string(), build_edge_batch_param(group));
    client
        .execute_with_raw_params(falkordb_mixed_rel_edge_cypher(), params)
        .await
}

async fn falkordb_bulk_upsert_edges(
    clients: &[FalkorDbClient],
    edges: &[CodeEdge],
    mut profile: Option<&mut EdgeWriteProfile>,
) -> Result<u64> {
    if edges.is_empty() {
        return Ok(0);
    }
    let k = clients.len().max(1);
    let t_rel = Instant::now();
    let bolt_executions = if k == 1 {
        let refs: Vec<&CodeEdge> = edges.iter().collect();
        falkordb_execute_mixed_rel_edge_batch(&clients[0], &refs).await?;
        1
    } else {
        let mut buckets: Vec<Vec<CodeEdge>> = (0..k).map(|_| Vec::new()).collect();
        for e in edges {
            let idx = shard_index(&e.from, k);
            buckets[idx].push(e.clone());
        }
        let mut set = JoinSet::new();
        let mut parallel_queries: u64 = 0;
        for (i, bucket) in buckets.into_iter().enumerate() {
            if bucket.is_empty() {
                continue;
            }
            parallel_queries += 1;
            let client = clients[i].clone();
            set.spawn(async move {
                let refs: Vec<&CodeEdge> = bucket.iter().collect();
                falkordb_execute_mixed_rel_edge_batch(&client, &refs).await
            });
        }
        while let Some(joined) = set.join_next().await {
            joined
                .map_err(|e| CortexError::Database(format!("falkordb edge upsert join: {e}")))??;
        }
        parallel_queries
    };
    if let Some(p) = profile.as_mut() {
        p.record("mixed_rel", bolt_executions, t_rel.elapsed());
    }
    Ok(bolt_executions)
}

/// FalkorDB graph database client.
#[derive(Clone)]
pub struct GraphClient {
    driver: Arc<GraphDriver>,
    /// FalkorDB bulk node upserts include `source` / `docstring` / JSON `properties` when true.
    falkordb_bulk_include_source: bool,
}

impl GraphClient {
    /// Backend selected from [`CortexConfig`] (always FalkorDB in Phase 1).
    pub fn configured_backend(config: &CortexConfig) -> BackendKind {
        detect_backend_from_config(config)
    }

    /// Connect to FalkorDB.
    #[instrument(skip(config), fields(uri = %config.falkordb_uri))]
    pub async fn connect(config: &CortexConfig) -> Result<Self> {
        info!(uri = %config.falkordb_uri, "Connecting to FalkorDB");

        let pool_n = config.falkordb_write_pool_size.max(1);
        let driver = if pool_n == 1 {
            let client = FalkorDbClient::connect(config).await?;
            Arc::new(GraphDriver::FalkorDB(client))
        } else {
            info!(pool_size = pool_n, "Opening FalkorDB writer pool");
            let mut clients = Vec::with_capacity(pool_n);
            for _ in 0..pool_n {
                clients.push(FalkorDbClient::connect(config).await?);
            }
            Arc::new(GraphDriver::FalkorDBPool(Arc::new(clients)))
        };

        let client = Self {
            driver,
            falkordb_bulk_include_source: config.falkordb_bulk_index_include_source,
        };
        schema::ensure_constraints(&client).await?;
        schema::warn_if_falkordb_codenode_id_index_missing(&client).await?;
        info!("Successfully connected to FalkorDB");
        Ok(client)
    }

    /// Get the backend type (always FalkorDB).
    pub fn backend(&self) -> BackendKind {
        BackendKind::FalkorDB
    }

    /// Reset FalkorDB `GRAPH.QUERY` micro-profile counters.
    pub fn reset_falkordb_profile(&self) {
        FalkorDbClient::reset_profile();
    }

    /// FalkorDB write-path micro-metrics for the current run (`CORTEX_FALKORDB_PROFILE=1`).
    pub fn falkordb_profile_snapshot(
        &self,
        reset_after: bool,
    ) -> Option<crate::falkordb_profile::FalkorDbProfileSnapshot> {
        Some(FalkorDbClient::profile_snapshot(reset_after))
    }

    /// Get the inner FalkorDB client.
    pub fn inner_falkordb(&self) -> Option<&FalkorDbClient> {
        falkordb_primary(self.driver.as_ref())
    }

    /// Execute a Cypher query without returning results.
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn run(&self, cypher: &str) -> Result<()> {
        debug!(cypher = %cypher, "Executing cypher query");
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        client.run(cypher).await
    }

    /// Execute a Cypher query and return results as JSON.
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn raw_query(&self, cypher: &str) -> Result<Vec<Value>> {
        debug!(cypher = %cypher, "Executing raw query");
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        let rows = client.raw_query(cypher).await?;
        debug!(row_count = rows.len(), "Raw query completed");
        Ok(rows)
    }

    /// Execute a parameterized query with a single string parameter.
    pub async fn query_with_param(
        &self,
        cypher: &str,
        param_name: &str,
        param_value: &str,
    ) -> Result<Vec<Value>> {
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        client
            .query_with_param(cypher, param_name, param_value)
            .await
    }

    /// Execute a parameterized query with multiple string parameters.
    pub async fn query_with_params(
        &self,
        cypher: &str,
        params: Vec<(&str, String)>,
    ) -> Result<Vec<Value>> {
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        client.query_with_params(cypher, params).await
    }

    /// Parameterized query with arbitrary [`GraphParam`] values (lists, maps).
    pub async fn raw_query_with_param_map(
        &self,
        cypher: &str,
        params: HashMap<String, GraphParam>,
    ) -> Result<Vec<Value>> {
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        client.raw_query_with_params(cypher, Some(params)).await
    }

    /// Execute write with arbitrary parameters.
    pub async fn execute_with_raw_param_map(
        &self,
        cypher: &str,
        params: HashMap<String, GraphParam>,
    ) -> Result<()> {
        let client = falkordb_primary(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without client".to_string())
        })?;
        client.execute_with_raw_params(cypher, params).await
    }

    /// Delete all code nodes for many file paths in one or few round trips.
    pub async fn delete_file_nodes_batch(
        &self,
        repository_path: &str,
        branch: Option<&str>,
        paths: &[String],
    ) -> Result<usize> {
        if paths.is_empty() {
            return Ok(0);
        }
        const CHUNK: usize = 256;
        let mut attempted = 0usize;
        for chunk in paths.chunks(CHUNK) {
            let list: Vec<GraphParam> = chunk
                .iter()
                .map(|p| GraphParam::String(p.clone()))
                .collect();
            let mut params = HashMap::new();
            params.insert(
                "repository_path".to_string(),
                GraphParam::String(repository_path.to_string()),
            );
            params.insert("paths".to_string(), GraphParam::List(list));
            let cypher = if let Some(br) = branch {
                params.insert("branch".to_string(), GraphParam::String(br.to_string()));
                "UNWIND $paths AS path\n             MATCH (n:CodeNode {repository_path: $repository_path, branch: $branch, path: path})\n             DETACH DELETE n"
            } else {
                "UNWIND $paths AS path\n             MATCH (n:CodeNode {repository_path: $repository_path, path: path})\n             WHERE n.branch IS NULL\n             DETACH DELETE n"
            };
            self.execute_with_raw_param_map(cypher, params).await?;
            attempted += chunk.len();
        }
        Ok(attempted)
    }

    /// Remove file tombstones for many paths.
    pub async fn clear_file_tombstones_batch(
        &self,
        repository_path: &str,
        branch: Option<&str>,
        paths: &[String],
    ) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        const CHUNK: usize = 512;
        for chunk in paths.chunks(CHUNK) {
            let list: Vec<GraphParam> = chunk
                .iter()
                .map(|p| GraphParam::String(p.clone()))
                .collect();
            let mut params = HashMap::new();
            params.insert(
                "repository_path".to_string(),
                GraphParam::String(repository_path.to_string()),
            );
            params.insert("paths".to_string(), GraphParam::List(list));
            let cypher = if let Some(br) = branch {
                params.insert("branch".to_string(), GraphParam::String(br.to_string()));
                "UNWIND $paths AS path\n             MATCH (t:FileTombstone {repository_path: $repository_path, branch: $branch, path: path})\n             DELETE t"
            } else {
                "UNWIND $paths AS path\n             MATCH (t:FileTombstone {repository_path: $repository_path, branch: '', path: path})\n             DELETE t"
            };
            self.execute_with_raw_param_map(cypher, params).await?;
        }
        Ok(())
    }

    /// Upsert a repository node.
    pub async fn upsert_repository(&self, repository: &Repository) -> Result<()> {
        let repo_id = format!("repo:{}", repository.path);
        let cypher = format!(
            "MERGE (r:Repository {{path: $path}})
             SET r:CodeNode,
                 r.id = $id,
                 r.kind = 'Repository',
                 r.name = $name,
                 r.path = $path,
                 r.watched = toBoolean($watched)"
        );
        let params = vec![
            ("id", repo_id.clone()),
            ("path", repository.path.clone()),
            ("name", repository.name.clone()),
            ("watched", repository.watched.to_string()),
        ];
        self.query_with_params(&cypher, params).await?;
        Ok(())
    }

    /// Upsert a call target node.
    pub async fn upsert_call_target(&self, id: &str, name: &str) -> Result<()> {
        let cypher = format!(
            "MERGE (n:CallTarget {{id: $id}})
             SET n:CodeNode, n.kind = 'CallTarget', n.name = $name"
        );
        let params = vec![("id", id.to_string()), ("name", name.to_string())];
        self.query_with_params(&cypher, params).await?;
        Ok(())
    }

    /// List all repositories.
    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let rows = self
            .raw_query(
                "MATCH (r:Repository)
                 RETURN r.path AS path, r.name AS name, coalesce(r.watched, false) AS watched
                 ORDER BY r.path",
            )
            .await?;

        let mut repos = Vec::new();
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

    /// Delete a repository and all its nodes.
    pub async fn delete_repository(&self, repository_path: &str) -> Result<()> {
        let cypher = format!(
            "MATCH (r:Repository {{path: $path}})
             OPTIONAL MATCH (r)-[:CONTAINS*]->(n)
             DETACH DELETE n, r"
        );
        self.query_with_param(&cypher, "path", repository_path)
            .await?;
        Ok(())
    }

    /// Upsert a code node.
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
            "MERGE (n:CodeNode {{id: $id}})
             SET n:{label},
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

    /// Upsert an edge.
    pub async fn upsert_edge(&self, edge: &CodeEdge) -> Result<()> {
        let rel_type = edge.kind.cypher_rel_type();
        let cypher = format!(
            "MATCH (from:CodeNode {{id: $from}}), (to:CodeNode {{id: $to}})
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

    /// Bulk-upsert code nodes in fewer database round trips.
    pub async fn bulk_upsert_nodes(&self, nodes: &[CodeNode]) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        let clients = falkordb_clients(self.driver.as_ref()).expect("falkordb driver");
        falkordb_bulk_upsert_nodes(clients, nodes, self.falkordb_bulk_include_source).await
    }

    /// Bulk-upsert edges in fewer database round trips.
    pub async fn bulk_upsert_edges(&self, edges: &[CodeEdge]) -> Result<u64> {
        self.bulk_upsert_edges_profiled(edges, None).await
    }

    /// Like [`Self::bulk_upsert_edges`] but records per-relationship-type timing when `profile` is set.
    pub async fn bulk_upsert_edges_profiled(
        &self,
        edges: &[CodeEdge],
        profile: Option<&mut EdgeWriteProfile>,
    ) -> Result<u64> {
        if edges.is_empty() {
            return Ok(0);
        }
        let clients = falkordb_clients(self.driver.as_ref()).expect("falkordb driver");
        falkordb_bulk_upsert_edges(clients, edges, profile).await
    }

    /// Bulk-upsert call-target placeholders before edge resolution.
    pub async fn bulk_upsert_call_targets(&self, targets: &[(String, String)]) -> Result<()> {
        if targets.is_empty() {
            return Ok(());
        }
        let client = falkordb_primary(self.driver.as_ref()).expect("falkordb driver");
        bulk_upsert_call_targets_unwind(client, targets).await
    }

    /// Resolve call targets to concrete functions.
    pub async fn resolve_call_targets(
        &self,
        repository_path: &str,
        branch: Option<&str>,
        touched_call_target_ids: &[String],
    ) -> Result<usize> {
        let resolved = self
            .resolve_call_targets_legacy(repository_path, branch)
            .await?;

        if !touched_call_target_ids.is_empty() {
            self.cleanup_orphan_call_targets(touched_call_target_ids)
                .await?;
        }

        Ok(resolved)
    }

    /// Remove touched [`CallTarget`] nodes with no incoming edges (bounded parallel on write pool).
    async fn cleanup_orphan_call_targets(&self, touched_call_target_ids: &[String]) -> Result<()> {
        let pool_n = write_pool_len(self.driver.as_ref()).max(1);
        let chunks: Vec<(usize, Vec<String>)> = touched_call_target_ids
            .chunks(Self::RESOLVE_CALL_TARGETS_CHUNK)
            .enumerate()
            .map(|(i, c)| (i, c.to_vec()))
            .collect();
        for wave in chunks.chunks(pool_n) {
            let mut join = JoinSet::new();
            for (chunk_idx, chunk_ids) in wave {
                let client = self.clone();
                let shard = chunk_idx % pool_n;
                let list: Vec<GraphParam> = chunk_ids
                    .iter()
                    .map(|id| GraphParam::String(id.clone()))
                    .collect();
                let mut params = HashMap::new();
                params.insert("ids".to_string(), GraphParam::List(list));
                let cypher = "UNWIND $ids AS id
                             MATCH (ct:CallTarget {id: id})
                             WHERE NOT ()-->(ct)
                             DETACH DELETE ct";
                join.spawn(async move {
                    client
                        .query_with_param_map_on_shard(cypher, params, shard)
                        .await
                        .map(|_| ())
                });
            }
            while let Some(res) = join.join_next().await {
                res.map_err(|e| CortexError::Database(format!("orphan cleanup join: {e}")))??;
            }
        }
        Ok(())
    }

    async fn resolve_call_targets_legacy(
        &self,
        repository_path: &str,
        branch: Option<&str>,
    ) -> Result<usize> {
        let callee_pattern = if branch.is_some() {
            "MATCH (callee:Function {name: callee_name, repository_path: $repo, branch: $branch})"
        } else {
            "MATCH (callee:Function {name: callee_name, repository_path: $repo})"
        };
        let cypher = format!(
            "MATCH (caller:CodeNode)-[old:CALLS]->(ct:CallTarget)
             WHERE caller.repository_path = $repo
             WITH caller, old, ct, coalesce(old.callee_name, ct.name) AS callee_name
             {callee_pattern}
             MERGE (caller)-[r:CALLS]->(callee)
             SET r.kind = 'Calls', r.properties = old.properties
             DELETE old
             RETURN count(r) AS resolved"
        );

        let mut params = vec![("repo", repository_path.to_string())];
        if let Some(br) = branch {
            params.push(("branch", br.to_string()));
        }

        self.sum_resolved_rows(self.query_with_params(&cypher, params).await?)
    }

    fn sum_resolved_rows(&self, rows: Vec<Value>) -> Result<usize> {
        let mut resolved = 0usize;
        for row in rows {
            if let Some(count) = row.get("resolved").and_then(|v| v.as_u64()) {
                resolved += count as usize;
            }
        }
        Ok(resolved)
    }

    /// Chunk size for parallel orphan CallTarget cleanup.
    pub const RESOLVE_CALL_TARGETS_CHUNK: usize = 256;

    async fn query_with_param_map_on_shard(
        &self,
        cypher: &str,
        params: HashMap<String, GraphParam>,
        pool_shard: usize,
    ) -> Result<Vec<Value>> {
        let clients = falkordb_clients(self.driver.as_ref()).ok_or_else(|| {
            CortexError::Database("internal: falkordb driver without clients".to_string())
        })?;
        let client = &clients[pool_shard % clients.len().max(1)];
        client.raw_query_with_params(cypher, Some(params)).await
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
            "MATCH (source:CodeNode)-[old:TYPE_REFERENCE]->(ct:CallTarget)
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

        let mut resolved = 0usize;
        for row in rows {
            if let Some(count) = row.get("resolved").and_then(|v| v.as_u64()) {
                resolved += count as usize;
            }
        }
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
            "MATCH (source:CodeNode)-[old:FIELD_ACCESS]->(ct:CallTarget)
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

        let mut resolved = 0usize;
        for row in rows {
            if let Some(count) = row.get("resolved").and_then(|v| v.as_u64()) {
                resolved += count as usize;
            }
        }
        Ok(resolved)
    }
}

async fn bulk_upsert_call_targets_unwind(
    client: &FalkorDbClient,
    targets: &[(String, String)],
) -> Result<()> {
    let batch = targets
        .iter()
        .map(|(id, name)| {
            let mut item = HashMap::new();
            item.insert("id".to_string(), GraphParam::String(id.clone()));
            item.insert("name".to_string(), GraphParam::String(name.clone()));
            GraphParam::Map(item)
        })
        .collect::<Vec<_>>();
    let mut params = HashMap::new();
    params.insert("batch".to_string(), GraphParam::List(batch));
    client
        .execute_with_raw_params(
            "UNWIND $batch AS item
             MERGE (n:CodeNode:CallTarget {id: item.id})
             SET n.kind = 'CallTarget', n.name = item.name",
            params,
        )
        .await
}

fn build_node_batch_param(nodes: &[&CodeNode]) -> GraphParam {
    let items = nodes
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

            let mut item = HashMap::new();
            item.insert("id".to_string(), GraphParam::String(node.id.clone()));
            item.insert(
                "kind".to_string(),
                GraphParam::String(format!("{:?}", node.kind)),
            );
            item.insert("name".to_string(), GraphParam::String(node.name.clone()));
            item.insert(
                "path".to_string(),
                GraphParam::String(node.path.clone().unwrap_or_default()),
            );
            item.insert(
                "line_number".to_string(),
                GraphParam::Int(node.line_number.unwrap_or_default() as i64),
            );
            item.insert(
                "lang".to_string(),
                GraphParam::String(
                    node.lang
                        .map(|l| l.as_str().to_string())
                        .unwrap_or_default(),
                ),
            );
            item.insert(
                "source".to_string(),
                GraphParam::String(node.source.clone().unwrap_or_default()),
            );
            item.insert(
                "docstring".to_string(),
                GraphParam::String(node.docstring.clone().unwrap_or_default()),
            );
            item.insert(
                "cyclomatic_complexity".to_string(),
                GraphParam::Int(cyclomatic),
            );
            item.insert(
                "properties".to_string(),
                GraphParam::String(serde_json::to_string(&node.properties).unwrap_or_default()),
            );
            item.insert(
                "qualified_name".to_string(),
                GraphParam::String(qualified_name),
            );
            item.insert("visibility".to_string(), GraphParam::String(visibility));
            item.insert("branch".to_string(), GraphParam::String(branch));
            item.insert(
                "repository_path".to_string(),
                GraphParam::String(repository_path),
            );
            GraphParam::Map(item)
        })
        .collect();
    GraphParam::List(items)
}

fn build_falkordb_node_batch_param_slim(nodes: &[&CodeNode]) -> GraphParam {
    let items = nodes
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

            let mut item = HashMap::new();
            item.insert("id".to_string(), GraphParam::String(node.id.clone()));
            item.insert(
                "kind".to_string(),
                GraphParam::String(format!("{:?}", node.kind)),
            );
            item.insert("name".to_string(), GraphParam::String(node.name.clone()));
            item.insert(
                "path".to_string(),
                GraphParam::String(node.path.clone().unwrap_or_default()),
            );
            item.insert(
                "line_number".to_string(),
                GraphParam::Int(node.line_number.unwrap_or_default() as i64),
            );
            item.insert(
                "lang".to_string(),
                GraphParam::String(
                    node.lang
                        .map(|l| l.as_str().to_string())
                        .unwrap_or_default(),
                ),
            );
            item.insert(
                "cyclomatic_complexity".to_string(),
                GraphParam::Int(cyclomatic),
            );
            item.insert(
                "qualified_name".to_string(),
                GraphParam::String(qualified_name),
            );
            item.insert("visibility".to_string(), GraphParam::String(visibility));
            item.insert("branch".to_string(), GraphParam::String(branch));
            item.insert(
                "repository_path".to_string(),
                GraphParam::String(repository_path),
            );
            GraphParam::Map(item)
        })
        .collect();
    GraphParam::List(items)
}

fn build_edge_batch_param(edges: &[&CodeEdge]) -> GraphParam {
    let items = edges
        .iter()
        .map(|edge| {
            let mut item = HashMap::new();
            item.insert("from".to_string(), GraphParam::String(edge.from.clone()));
            item.insert("to".to_string(), GraphParam::String(edge.to.clone()));
            item.insert(
                "rel".to_string(),
                GraphParam::String(edge.kind.cypher_rel_type().to_string()),
            );
            item.insert(
                "kind".to_string(),
                GraphParam::String(format!("{:?}", edge.kind)),
            );
            item.insert(
                "properties".to_string(),
                GraphParam::String(serde_json::to_string(&edge.properties).unwrap_or_default()),
            );
            GraphParam::Map(item)
        })
        .collect();
    GraphParam::List(items)
}

#[cfg(test)]
mod falkordb_batch_tests {
    use super::*;
    use cortex_core::{CodeNode, EntityKind};

    #[test]
    fn slim_node_batch_smaller_than_full_with_large_source() {
        let node = CodeNode {
            id: "n1".to_string(),
            kind: EntityKind::Function,
            name: "foo".to_string(),
            path: Some("a.rs".to_string()),
            line_number: Some(1),
            lang: None,
            source: Some("x".repeat(4096)),
            docstring: Some("d".repeat(1024)),
            properties: HashMap::from([
                ("branch".to_string(), "main".to_string()),
                ("repository_path".to_string(), "/repo".to_string()),
            ]),
        };
        let refs = [&node];
        let full =
            crate::falkordb_params::query_param_to_cypher_literal(&build_node_batch_param(&refs));
        let slim = crate::falkordb_params::query_param_to_cypher_literal(
            &build_falkordb_node_batch_param_slim(&refs),
        );
        assert!(
            slim.len() < full.len() / 2,
            "slim={} full={}",
            slim.len(),
            full.len()
        );
        assert!(!slim.contains("xxxx"));
    }
}

#[cfg(test)]
mod resolve_chunk_tests {
    use super::GraphClient;

    #[test]
    fn resolve_chunk_splits_at_boundary() {
        let chunk = GraphClient::RESOLVE_CALL_TARGETS_CHUNK;
        let ids: Vec<String> = (0..chunk + 44).map(|i| format!("ct_{i}")).collect();
        let chunks: Vec<_> = ids.chunks(chunk).collect();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), chunk);
        assert_eq!(chunks[1].len(), 44);
    }
}
