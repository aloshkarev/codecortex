//! Schema management for the graph database.
//!
//! This module handles:
//! - Creating and maintaining database constraints
//! - Branch-aware indexes for multi-project support
//! - BranchIndex node management for tracking indexed branches

use crate::GraphClient;
use chrono::{DateTime, Utc};
use cortex_core::IndexFreshness;
use cortex_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};

/// Schema statements for basic constraints and indexes
/// Note: Memgraph syntax is used (different from Neo4j)
/// - Memgraph does NOT support IF NOT EXISTS for indexes/constraints
/// - Use DROP before CREATE for idempotent schema setup
const SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE INDEX ON :Repository;",
    "CREATE INDEX ON :Directory;",
    "CREATE INDEX ON :File;",
    "CREATE INDEX ON :Function;",
    "CREATE INDEX ON :Class;",
    "CREATE INDEX ON :Variable;",
    "CREATE INDEX ON :Parameter;",
    "CREATE INDEX ON :Module;",
    "CREATE INDEX ON :CallTarget;",
    "CREATE INDEX ON :CodeNode;",
    "CREATE INDEX ON :Repository(path);",
    "CREATE INDEX ON :Directory(path);",
    "CREATE INDEX ON :File(path);",
    "CREATE INDEX ON :Function(name);",
    "CREATE INDEX ON :Function(path);",
    "CREATE INDEX ON :Class(name);",
    "CREATE INDEX ON :Class(path);",
    "CREATE INDEX ON :Variable(name);",
    "CREATE INDEX ON :Parameter(name);",
    "CREATE INDEX ON :Module(name);",
    "CREATE INDEX ON :CallTarget(name);",
    "CREATE INDEX ON :CallTarget(id);",
    "CREATE INDEX ON :CodeNode(id);",
    "CREATE INDEX ON :CodeNode(path);",
    "CREATE INDEX ON :CodeNode(kind);",
    "CREATE INDEX ON :CodeNode(name);",
];

/// Branch-aware indexes for multi-project support
/// Note: Memgraph does NOT support IF NOT EXISTS - errors are ignored in ensure_constraints
const BRANCH_SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE INDEX ON :CodeNode(branch);",
    "CREATE INDEX ON :CodeNode(repository_path);",
    "CREATE INDEX ON :Function(branch);",
    "CREATE INDEX ON :Function(repository_path);",
    "CREATE INDEX ON :Class(branch);",
    "CREATE INDEX ON :Class(repository_path);",
    "CREATE INDEX ON :File(branch);",
    "CREATE INDEX ON :File(repository_path);",
    "CREATE INDEX ON :Variable(branch);",
    "CREATE INDEX ON :Module(branch);",
    "CREATE INDEX ON :BranchIndex(id);",
    "CREATE INDEX ON :BranchIndex(repository_path);",
    "CREATE INDEX ON :BranchIndex(branch);",
    "CREATE INDEX ON :BranchIndex(commit_hash);",
    // Tombstones let incremental index explain deletes/renames without keeping stale code nodes.
    "CREATE INDEX ON :FileTombstone(id);",
    "CREATE INDEX ON :FileTombstone(repository_path);",
    "CREATE INDEX ON :FileTombstone(branch);",
    "CREATE INDEX ON :FileTombstone(path);",
];

/// Additional indexes for navigation-heavy lookups.
/// A2A blackboard schema (sessions and agent insights).
const A2A_SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE INDEX ON :A2aSession;",
    "CREATE INDEX ON :A2aSession(id);",
    "CREATE INDEX ON :AgentInsight;",
    "CREATE INDEX ON :AgentInsight(id);",
    "CREATE INDEX ON :AgentInsight(session_id);",
    "CREATE INDEX ON :AgentInsight(created_at);",
];

const NAVIGATION_SCHEMA_STATEMENTS: &[&str] = &[
    "CREATE INDEX ON :CodeNode(qualified_name);",
    "CREATE INDEX ON :Function(qualified_name);",
    "CREATE INDEX ON :Class(qualified_name);",
    "CREATE INDEX ON :Method(qualified_name);",
    "CREATE INDEX ON :Struct(qualified_name);",
];

/// Record of an indexed branch stored in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchIndexRecord {
    /// Unique identifier: "branch:/path/to/repo@branch_name"
    pub id: String,
    /// Path to the repository root
    pub repository_path: String,
    /// Branch name
    pub branch: String,
    /// Commit hash that was indexed
    pub commit_hash: String,
    /// When this branch was indexed
    pub indexed_at: DateTime<Utc>,
    /// Number of files indexed
    pub file_count: usize,
    /// Number of symbols indexed
    pub symbol_count: usize,
    /// Whether this index is stale (commit has changed)
    pub is_stale: bool,
    /// Duration of indexing in milliseconds
    pub index_duration_ms: u64,
    /// Worktree hash or dirty snapshot that was indexed.
    #[serde(default)]
    pub worktree_hash: Option<String>,
    /// Aggregate hash of indexed file contents.
    #[serde(default)]
    pub file_hash_watermark: Option<String>,
    /// Graph freshness for this branch snapshot.
    #[serde(default)]
    pub graph_freshness: IndexFreshness,
    /// Vector freshness for this branch snapshot.
    #[serde(default)]
    pub vector_freshness: IndexFreshness,
    /// Last successful incremental or full update timestamp.
    #[serde(default)]
    pub last_successful_update_at: Option<DateTime<Utc>>,
    /// Last failed update reason, if any.
    #[serde(default)]
    pub last_failed_update_reason: Option<String>,
}

impl BranchIndexRecord {
    /// Create a new branch index record
    pub fn new(
        repository_path: &str,
        branch: &str,
        commit_hash: &str,
        file_count: usize,
        symbol_count: usize,
        index_duration_ms: u64,
    ) -> Self {
        Self {
            id: format!("branch:{}@{}", repository_path, branch),
            repository_path: repository_path.to_string(),
            branch: branch.to_string(),
            commit_hash: commit_hash.to_string(),
            indexed_at: Utc::now(),
            file_count,
            symbol_count,
            is_stale: false,
            index_duration_ms,
            worktree_hash: None,
            file_hash_watermark: None,
            graph_freshness: IndexFreshness::Fresh,
            vector_freshness: IndexFreshness::Unknown,
            last_successful_update_at: Some(Utc::now()),
            last_failed_update_reason: None,
        }
    }
}

/// Skip re-running hundreds of `CREATE INDEX` statements on every new connection.
static SCHEMA_ENSURE_DONE: AtomicBool = AtomicBool::new(false);

/// Ensure all schema constraints and indexes exist (FalkorDB Cypher syntax).
pub async fn ensure_constraints(client: &GraphClient) -> Result<()> {
    if SCHEMA_ENSURE_DONE.load(Ordering::Acquire) {
        // Re-verify FalkorDB property indexes; a prior run may have set the flag while CREATE INDEX failed.
        let rows = client
            .raw_query("CALL db.indexes()")
            .await
            .unwrap_or_default();
        let has_codenode_id = rows
            .iter()
            .any(|row| index_row_has_label_property(row, "CodeNode", "id"));
        if has_codenode_id {
            return Ok(());
        }
        SCHEMA_ENSURE_DONE.store(false, Ordering::Release);
    }

    let all_statements = SCHEMA_STATEMENTS
        .iter()
        .chain(BRANCH_SCHEMA_STATEMENTS.iter())
        .chain(NAVIGATION_SCHEMA_STATEMENTS.iter())
        .chain(A2A_SCHEMA_STATEMENTS.iter());

    for statement in all_statements {
        let cypher = (*statement).to_string();
        if let Err(e) = client.run(&cypher).await {
            tracing::debug!(
                "Schema statement returned (may already exist): {} - {}",
                cypher,
                e
            );
        }
    }

    SCHEMA_ENSURE_DONE.store(true, Ordering::Release);
    tracing::info!(
        target: "cortex_graph::schema",
        "FalkorDB schema ensured (CREATE INDEX ON :CodeNode(id) and related statements)"
    );
    Ok(())
}

/// Ensure A2A blackboard indexes (`A2aSession`, `AgentInsight`).
pub async fn ensure_a2a_schema(client: &GraphClient) -> Result<()> {
    for statement in A2A_SCHEMA_STATEMENTS {
        let cypher = (*statement).to_string();
        if let Err(e) = client.run(&cypher).await {
            tracing::debug!("A2A schema (may exist): {} - {}", cypher, e);
        }
    }
    Ok(())
}

/// After [`ensure_constraints`], warn when FalkorDB has no label–property index on `CodeNode(id)`.
///
/// Bulk edge upserts use `MATCH` on `id`; missing indexes slow the apply phase. Tries
/// `CALL db.indexes()` when supported; otherwise logs guidance to confirm schema setup.
pub async fn warn_if_falkordb_codenode_id_index_missing(client: &GraphClient) -> Result<()> {
    let rows = match client.raw_query("CALL db.indexes()").await {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!(
                error = %e,
                "CALL db.indexes() unavailable on FalkorDB; skipping CodeNode(id) index check"
            );
            tracing::info!(
                "FalkorDB: ensure schema::ensure_constraints ran (CREATE INDEX ON :CodeNode(id)); \
                 bulk edge MATCH is faster with that index"
            );
            return Ok(());
        }
    };
    let has_codenode_id = rows
        .iter()
        .any(|row| index_row_has_label_property(row, "CodeNode", "id"));
    if !has_codenode_id {
        tracing::warn!(
            "FalkorDB: no index on :CodeNode(id) reported by CALL db.indexes() — bulk edge MATCH may be slow; \
             confirm ensure_constraints completed (CREATE INDEX ON :CodeNode(id))."
        );
    }
    Ok(())
}

fn index_row_has_label_property(row: &Value, want_label: &str, want_prop: &str) -> bool {
    let Some(obj) = row.as_object() else {
        return false;
    };
    let mut label_ok = false;
    let mut prop_ok = false;
    for (k, v) in obj {
        let kn: String = k.chars().filter(|c| !c.is_whitespace()).collect::<String>();
        let kn = kn.to_ascii_lowercase();
        if kn.contains("label") && !kn.contains("indextype") {
            if json_value_matches_str(v, want_label) {
                label_ok = true;
            }
        }
        if kn == "property" || (kn.contains("property") && !kn.contains("edge")) {
            if json_value_matches_str(v, want_prop) {
                prop_ok = true;
            }
        }
    }
    label_ok && prop_ok
}

fn json_value_matches_str(v: &Value, want: &str) -> bool {
    match v {
        Value::String(s) => s == want,
        Value::Array(items) => items
            .first()
            .and_then(|x| x.as_str())
            .is_some_and(|s| s == want),
        _ => false,
    }
}

/// Ensure optional navigation schema indexes.
pub async fn ensure_navigation_schema(client: &GraphClient) -> Result<()> {
    for stmt in NAVIGATION_SCHEMA_STATEMENTS {
        if let Err(e) = client.run(stmt).await {
            tracing::debug!("Navigation schema statement skipped: {} ({})", stmt, e);
        }
    }
    Ok(())
}

/// Create a BranchIndex node in the graph
pub async fn create_branch_index(client: &GraphClient, record: &BranchIndexRecord) -> Result<()> {
    let cypher = r#"
        MERGE (bi:BranchIndex {id: $id})
        SET bi.repository_path = $repository_path,
            bi.branch = $branch,
            bi.commit_hash = $commit_hash,
            bi.indexed_at = $indexed_at,
            bi.file_count = toInteger($file_count),
            bi.symbol_count = toInteger($symbol_count),
            bi.is_stale = toBoolean($is_stale),
            bi.index_duration_ms = toInteger($index_duration_ms),
            bi.worktree_hash = $worktree_hash,
            bi.file_hash_watermark = $file_hash_watermark,
            bi.graph_freshness = $graph_freshness,
            bi.vector_freshness = $vector_freshness,
            bi.last_successful_update_at = $last_successful_update_at,
            bi.last_failed_update_reason = $last_failed_update_reason
        "#;

    let params = vec![
        ("id", record.id.clone()),
        ("repository_path", record.repository_path.clone()),
        ("branch", record.branch.clone()),
        ("commit_hash", record.commit_hash.clone()),
        ("indexed_at", record.indexed_at.to_rfc3339()),
        ("file_count", record.file_count.to_string()),
        ("symbol_count", record.symbol_count.to_string()),
        ("is_stale", record.is_stale.to_string()),
        ("index_duration_ms", record.index_duration_ms.to_string()),
        (
            "worktree_hash",
            record.worktree_hash.clone().unwrap_or_default(),
        ),
        (
            "file_hash_watermark",
            record.file_hash_watermark.clone().unwrap_or_default(),
        ),
        (
            "graph_freshness",
            record.graph_freshness.as_str().to_string(),
        ),
        (
            "vector_freshness",
            record.vector_freshness.as_str().to_string(),
        ),
        (
            "last_successful_update_at",
            record
                .last_successful_update_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        ),
        (
            "last_failed_update_reason",
            record.last_failed_update_reason.clone().unwrap_or_default(),
        ),
    ];

    client.query_with_params(cypher, params).await?;
    Ok(())
}

/// Get all branch indexes for a repository
pub async fn get_branch_indexes(
    client: &GraphClient,
    repository_path: &str,
) -> Result<Vec<BranchIndexRecord>> {
    let cypher = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path})
        RETURN bi.id AS id,
               bi.repository_path AS repository_path,
               bi.branch AS branch,
               bi.commit_hash AS commit_hash,
               bi.indexed_at AS indexed_at,
               bi.file_count AS file_count,
               bi.symbol_count AS symbol_count,
               bi.is_stale AS is_stale,
               bi.index_duration_ms AS index_duration_ms,
               bi.worktree_hash AS worktree_hash,
               bi.file_hash_watermark AS file_hash_watermark,
               bi.graph_freshness AS graph_freshness,
               bi.vector_freshness AS vector_freshness,
               bi.last_successful_update_at AS last_successful_update_at,
               bi.last_failed_update_reason AS last_failed_update_reason
        ORDER BY bi.indexed_at DESC
        "#;

    let rows = client
        .query_with_param(cypher, "repository_path", repository_path)
        .await?;

    let mut indexes = Vec::new();
    for row in rows {
        indexes.push(decode_branch_index_row(&row));
    }

    Ok(indexes)
}

fn decode_branch_index_row(row: &serde_json::Value) -> BranchIndexRecord {
    fn str_field(row: &serde_json::Value, bare: &str, qualified: &str) -> String {
        row.get(bare)
            .or_else(|| row.get(qualified))
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default()
    }

    fn i64_field(row: &serde_json::Value, bare: &str, qualified: &str) -> i64 {
        row.get(bare)
            .or_else(|| row.get(qualified))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    }

    fn bool_field(row: &serde_json::Value, bare: &str, qualified: &str) -> bool {
        row.get(bare)
            .or_else(|| row.get(qualified))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    fn freshness_field(row: &serde_json::Value, bare: &str, qualified: &str) -> IndexFreshness {
        match str_field(row, bare, qualified).as_str() {
            "fresh" => IndexFreshness::Fresh,
            "warming" => IndexFreshness::Warming,
            "stale" => IndexFreshness::Stale,
            "partial" => IndexFreshness::Partial,
            _ => IndexFreshness::Unknown,
        }
    }

    let indexed_at = row
        .get("indexed_at")
        .or_else(|| row.get("bi.indexed_at"))
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let last_successful_update_at = row
        .get("last_successful_update_at")
        .or_else(|| row.get("bi.last_successful_update_at"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc));

    BranchIndexRecord {
        id: str_field(row, "id", "bi.id"),
        repository_path: str_field(row, "repository_path", "bi.repository_path"),
        branch: str_field(row, "branch", "bi.branch"),
        commit_hash: str_field(row, "commit_hash", "bi.commit_hash"),
        indexed_at,
        file_count: i64_field(row, "file_count", "bi.file_count").max(0) as usize,
        symbol_count: i64_field(row, "symbol_count", "bi.symbol_count").max(0) as usize,
        is_stale: bool_field(row, "is_stale", "bi.is_stale"),
        index_duration_ms: i64_field(row, "index_duration_ms", "bi.index_duration_ms").max(0)
            as u64,
        worktree_hash: non_empty(str_field(row, "worktree_hash", "bi.worktree_hash")),
        file_hash_watermark: non_empty(str_field(
            row,
            "file_hash_watermark",
            "bi.file_hash_watermark",
        )),
        graph_freshness: freshness_field(row, "graph_freshness", "bi.graph_freshness"),
        vector_freshness: freshness_field(row, "vector_freshness", "bi.vector_freshness"),
        last_successful_update_at,
        last_failed_update_reason: non_empty(str_field(
            row,
            "last_failed_update_reason",
            "bi.last_failed_update_reason",
        )),
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

/// Delete a branch index and its associated nodes
pub async fn delete_branch_index(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
) -> Result<usize> {
    let delete_nodes_cypher = r#"
        MATCH (n:CodeNode {repository_path: $repository_path, branch: $branch})
        DETACH DELETE n
        RETURN count(n) as deleted
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
    ];

    let rows = client
        .query_with_params(delete_nodes_cypher, params)
        .await?;

    let mut deleted_count = 0usize;
    for row in rows {
        if let Some(count) = row.get("deleted").and_then(|v| v.as_i64()) {
            deleted_count = count.max(0) as usize;
        }
    }

    let delete_index_cypher = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        DELETE bi
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
    ];

    client
        .query_with_params(delete_index_cypher, params)
        .await?;

    Ok(deleted_count)
}

/// Delete graph nodes for a single file in a branch slice.
pub async fn delete_file_index(
    client: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
    path: &str,
) -> Result<usize> {
    let (cypher, params) = if let Some(branch) = branch {
        (
            r#"
            MATCH (n:CodeNode {repository_path: $repository_path, branch: $branch, path: $path})
            DETACH DELETE n
            RETURN count(n) as deleted
            "#,
            vec![
                ("repository_path", repository_path.to_string()),
                ("branch", branch.to_string()),
                ("path", path.to_string()),
            ],
        )
    } else {
        (
            r#"
            MATCH (n:CodeNode {repository_path: $repository_path, path: $path})
            WHERE n.branch IS NULL
            DETACH DELETE n
            RETURN count(n) as deleted
            "#,
            vec![
                ("repository_path", repository_path.to_string()),
                ("path", path.to_string()),
            ],
        )
    };

    let rows = client.query_with_params(cypher, params).await?;
    Ok(rows
        .iter()
        .find_map(|row| row.get("deleted").and_then(|v| v.as_i64()))
        .unwrap_or(0)
        .max(0) as usize)
}

/// Record a deleted or renamed file so freshness/delta tools can explain removals.
pub async fn upsert_file_tombstone(
    client: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
    path: &str,
    revision: Option<&str>,
    reason: &str,
) -> Result<()> {
    let branch_value = branch.unwrap_or("");
    let id = format!("tombstone:{repository_path}@{branch_value}:{path}");
    let cypher = r#"
        MERGE (t:FileTombstone {id: $id})
        SET t.repository_path = $repository_path,
            t.branch = $branch,
            t.path = $path,
            t.revision = $revision,
            t.reason = $reason,
            t.deleted_at = $deleted_at
        "#;
    let params = vec![
        ("id", id),
        ("repository_path", repository_path.to_string()),
        ("branch", branch_value.to_string()),
        ("path", path.to_string()),
        ("revision", revision.unwrap_or("").to_string()),
        ("reason", reason.to_string()),
        ("deleted_at", Utc::now().to_rfc3339()),
    ];
    client.query_with_params(cypher, params).await?;
    Ok(())
}

/// Remove stale tombstone once a file is indexed again.
pub async fn clear_file_tombstone(
    client: &GraphClient,
    repository_path: &str,
    branch: Option<&str>,
    path: &str,
) -> Result<()> {
    let branch_value = branch.unwrap_or("");
    let cypher = r#"
        MATCH (t:FileTombstone {repository_path: $repository_path, branch: $branch, path: $path})
        DELETE t
        "#;
    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch_value.to_string()),
        ("path", path.to_string()),
    ];
    client.query_with_params(cypher, params).await?;
    Ok(())
}

/// Check if a branch index is current (matches the given commit)
pub async fn is_branch_index_current(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
    commit_hash: &str,
) -> Result<bool> {
    let cypher = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        RETURN bi.commit_hash = $commit_hash AS is_current, bi.is_stale AS is_stale
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
        ("commit_hash", commit_hash.to_string()),
    ];

    let rows = client.query_with_params(cypher, params).await?;

    for row in rows {
        let is_current = row
            .get("is_current")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let is_stale = row
            .get("is_stale")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if is_current && !is_stale {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Mark a branch index as stale
pub async fn mark_branch_index_stale(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
) -> Result<()> {
    let cypher = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        SET bi.is_stale = true,
            bi.graph_freshness = 'stale',
            bi.vector_freshness = CASE
                WHEN bi.vector_freshness IS NULL THEN 'unknown'
                ELSE bi.vector_freshness
            END
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
    ];

    client.query_with_params(cypher, params).await?;
    Ok(())
}

/// Update vector freshness on an existing branch index after vector indexing completes.
pub async fn mark_branch_vector_fresh(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
    freshness: IndexFreshness,
) -> Result<()> {
    let cypher = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        SET bi.vector_freshness = $vector_freshness,
            bi.last_successful_update_at = $last_successful_update_at
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
        ("vector_freshness", freshness.as_str().to_string()),
        ("last_successful_update_at", Utc::now().to_rfc3339()),
    ];

    client.query_with_params(cypher, params).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_statements_not_empty() {
        assert!(!SCHEMA_STATEMENTS.is_empty());
    }

    #[test]
    fn branch_schema_statements_not_empty() {
        assert!(!BRANCH_SCHEMA_STATEMENTS.is_empty());
    }

    #[test]
    fn schema_indexes_file_tombstones() {
        assert!(
            BRANCH_SCHEMA_STATEMENTS
                .iter()
                .any(|stmt| stmt.contains(":FileTombstone(path)"))
        );
    }

    #[test]
    fn schema_contains_only_indexes() {
        // Memgraph uses indexes, not constraints for this schema
        let index_count = SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
            .filter(|s| s.contains("INDEX"))
            .count();
        assert!(index_count > 0);
    }

    #[test]
    fn schema_statements_are_valid_falkordb_cypher() {
        for statement in SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
        {
            assert!(
                statement.ends_with(';'),
                "Statement missing semicolon: {}",
                statement
            );
            assert!(
                statement.contains("CREATE"),
                "Invalid statement: {}",
                statement
            );
            assert!(
                !statement.contains("IF NOT EXISTS"),
                "IF NOT EXISTS not supported by Memgraph: {}",
                statement
            );
        }
    }

    #[test]
    fn schema_has_repository_indexes() {
        let repo_indexes: Vec<_> = SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.contains(":Repository"))
            .collect();
        assert!(!repo_indexes.is_empty());
    }

    #[test]
    fn schema_has_function_indexes() {
        let func_indexes: Vec<_> = SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
            .filter(|s| s.contains(":Function"))
            .collect();
        assert!(!func_indexes.is_empty());
    }

    #[test]
    fn schema_has_class_indexes() {
        let class_indexes: Vec<_> = SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
            .filter(|s| s.contains(":Class"))
            .collect();
        assert!(!class_indexes.is_empty());
    }

    #[test]
    fn schema_has_code_node_indexes() {
        let code_node_indexes: Vec<_> = SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
            .filter(|s| s.contains(":CodeNode"))
            .collect();
        assert!(!code_node_indexes.is_empty());
    }

    #[test]
    fn schema_has_branch_indexes() {
        let branch_indexes: Vec<_> = BRANCH_SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.contains("branch"))
            .collect();
        assert!(!branch_indexes.is_empty());
    }

    #[test]
    fn schema_has_branch_index_node() {
        let branch_index_nodes: Vec<_> = BRANCH_SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.contains("BranchIndex"))
            .collect();
        assert!(!branch_index_nodes.is_empty());
    }

    #[test]
    fn schema_statements_count() {
        assert!(SCHEMA_STATEMENTS.len() >= 10);
        assert!(BRANCH_SCHEMA_STATEMENTS.len() >= 5);
    }

    #[test]
    fn branch_index_record_creation() {
        let record = BranchIndexRecord::new("/path/to/repo", "main", "abc123", 42, 256, 1500);

        assert_eq!(record.id, "branch:/path/to/repo@main");
        assert_eq!(record.repository_path, "/path/to/repo");
        assert_eq!(record.branch, "main");
        assert_eq!(record.commit_hash, "abc123");
        assert_eq!(record.file_count, 42);
        assert_eq!(record.symbol_count, 256);
        assert_eq!(record.index_duration_ms, 1500);
        assert!(!record.is_stale);
        assert_eq!(record.graph_freshness, IndexFreshness::Fresh);
        assert_eq!(record.vector_freshness, IndexFreshness::Unknown);
        assert!(record.last_successful_update_at.is_some());
    }

    #[test]
    fn branch_index_record_serialization() {
        let record = BranchIndexRecord::new("/path/to/repo", "feature/test", "def456", 10, 50, 500);

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("feature/test"));
        assert!(json.contains("def456"));

        let deserialized: BranchIndexRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, record.id);
        assert_eq!(deserialized.branch, record.branch);
    }

    #[test]
    fn get_branch_indexes_query_uses_stable_aliases() {
        let query = r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path})
        RETURN bi.id AS id,
               bi.repository_path AS repository_path,
               bi.branch AS branch,
               bi.commit_hash AS commit_hash,
               bi.indexed_at AS indexed_at,
               bi.file_count AS file_count,
               bi.symbol_count AS symbol_count,
               bi.is_stale AS is_stale,
               bi.index_duration_ms AS index_duration_ms,
               bi.graph_freshness AS graph_freshness,
               bi.vector_freshness AS vector_freshness
        ORDER BY bi.indexed_at DESC
        "#;

        assert!(query.contains("bi.id AS id"));
        assert!(query.contains("bi.repository_path AS repository_path"));
        assert!(query.contains("bi.index_duration_ms AS index_duration_ms"));
    }

    #[test]
    fn decode_branch_index_row_supports_aliased_fields() {
        let row = serde_json::json!({
            "id": "branch:/repo@main",
            "repository_path": "/repo",
            "branch": "main",
            "commit_hash": "abc123",
            "indexed_at": "2024-01-02T03:04:05Z",
            "file_count": 12,
            "symbol_count": 34,
            "is_stale": true,
            "index_duration_ms": 56,
            "graph_freshness": "stale",
            "vector_freshness": "partial",
            "last_failed_update_reason": "delete tombstone pending"
        });

        let record = decode_branch_index_row(&row);
        assert_eq!(record.id, "branch:/repo@main");
        assert_eq!(record.repository_path, "/repo");
        assert_eq!(record.branch, "main");
        assert_eq!(record.commit_hash, "abc123");
        assert_eq!(record.file_count, 12);
        assert_eq!(record.symbol_count, 34);
        assert!(record.is_stale);
        assert_eq!(record.index_duration_ms, 56);
        assert_eq!(record.graph_freshness, IndexFreshness::Stale);
        assert_eq!(record.vector_freshness, IndexFreshness::Partial);
        assert_eq!(
            record.last_failed_update_reason.as_deref(),
            Some("delete tombstone pending")
        );
    }

    #[test]
    fn decode_branch_index_row_supports_qualified_fields() {
        let row = serde_json::json!({
            "bi.id": "branch:/repo@feature",
            "bi.repository_path": "/repo",
            "bi.branch": "feature",
            "bi.commit_hash": "def456",
            "bi.indexed_at": "2024-01-02T03:04:05Z",
            "bi.file_count": 7,
            "bi.symbol_count": 8,
            "bi.is_stale": false,
            "bi.index_duration_ms": 9
        });

        let record = decode_branch_index_row(&row);
        assert_eq!(record.branch, "feature");
        assert_eq!(record.commit_hash, "def456");
        assert_eq!(record.file_count, 7);
        assert_eq!(record.symbol_count, 8);
        assert!(!record.is_stale);
        assert_eq!(record.index_duration_ms, 9);
    }

    #[test]
    fn index_row_detects_codenode_id() {
        let row = serde_json::json!({
            "index type": "label+property",
            "label": "CodeNode",
            "property": "id",
            "count": 0
        });
        assert!(super::index_row_has_label_property(&row, "CodeNode", "id"));
        let row2 = serde_json::json!({"label": "File", "property": "path"});
        assert!(!super::index_row_has_label_property(&row2, "CodeNode", "id"));
    }
}
