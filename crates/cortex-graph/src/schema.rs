//! Schema management for the graph database.
//!
//! This module handles:
//! - Creating and maintaining database constraints
//! - Branch-aware indexes for multi-project support
//! - BranchIndex node management for tracking indexed branches

use crate::GraphClient;
use chrono::{DateTime, Utc};
use cortex_core::Result;
use serde::{Deserialize, Serialize};

/// Schema statements for basic constraints and indexes
/// Note: Memgraph syntax is used (different from Neo4j)
/// - Memgraph does NOT support IF NOT EXISTS for indexes/constraints
/// - Use DROP before CREATE for idempotent schema setup
const SCHEMA_STATEMENTS: &[&str] = &[
    // Label indexes (Memgraph supports label-only indexes)
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
    // Label-property indexes for faster lookups
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
    "CREATE INDEX ON :CodeNode(id);",
    "CREATE INDEX ON :CodeNode(path);",
    "CREATE INDEX ON :CodeNode(kind);",
    "CREATE INDEX ON :CodeNode(name);",
];

/// Branch-aware indexes for multi-project support
/// Note: Memgraph does NOT support IF NOT EXISTS - errors are ignored in ensure_constraints
const BRANCH_SCHEMA_STATEMENTS: &[&str] = &[
    // Branch property indexes on code nodes
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
    // BranchIndex node indexes (constraints created separately for idempotency)
    "CREATE INDEX ON :BranchIndex(id);",
    "CREATE INDEX ON :BranchIndex(repository_path);",
    "CREATE INDEX ON :BranchIndex(branch);",
    "CREATE INDEX ON :BranchIndex(commit_hash);",
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
        }
    }
}

/// Ensure all schema constraints and indexes exist
/// Errors are logged but not propagated since indexes/constraints may already exist
pub async fn ensure_constraints(client: &GraphClient) -> Result<()> {
    // Apply basic schema (ignore errors - indexes may already exist)
    for statement in SCHEMA_STATEMENTS {
        if let Err(e) = client.run(statement).await {
            // Log but continue - index/constraint may already exist
            tracing::debug!(
                "Schema statement returned (may already exist): {} - {}",
                statement,
                e
            );
        }
    }

    // Apply branch-aware schema (ignore errors - indexes may already exist)
    for statement in BRANCH_SCHEMA_STATEMENTS {
        if let Err(e) = client.run(statement).await {
            tracing::debug!(
                "Branch schema statement returned (may already exist): {} - {}",
                statement,
                e
            );
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
            bi.index_duration_ms = toInteger($index_duration_ms)
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
        RETURN bi.id, bi.repository_path, bi.branch, bi.commit_hash,
               bi.indexed_at, bi.file_count, bi.symbol_count,
               bi.is_stale, bi.index_duration_ms
        ORDER BY bi.indexed_at DESC
        "#;

    let rows = client
        .query_with_param(cypher, "repository_path", repository_path)
        .await?;

    let mut indexes = Vec::new();
    for row in rows {
        let id = row
            .get("bi.id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let repo_path = row
            .get("bi.repository_path")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let branch = row
            .get("bi.branch")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let commit_hash = row
            .get("bi.commit_hash")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let file_count = row
            .get("bi.file_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;
        let symbol_count = row
            .get("bi.symbol_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;
        let is_stale = row
            .get("bi.is_stale")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let index_duration_ms = row
            .get("bi.index_duration_ms")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as u64;

        // Parse indexed_at or use current time as fallback
        let indexed_at = row
            .get("bi.indexed_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        indexes.push(BranchIndexRecord {
            id,
            repository_path: repo_path,
            branch,
            commit_hash,
            indexed_at,
            file_count,
            symbol_count,
            is_stale,
            index_duration_ms,
        });
    }

    Ok(indexes)
}

/// Delete a branch index and its associated nodes
pub async fn delete_branch_index(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
) -> Result<usize> {
    // First, delete all code nodes for this branch
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

    // Then delete the BranchIndex node
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
        SET bi.is_stale = true
        "#;

    let params = vec![
        ("repository_path", repository_path.to_string()),
        ("branch", branch.to_string()),
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
    fn schema_statements_are_valid_memgraph_cypher() {
        for statement in SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
        {
            // Must end with semicolon
            assert!(
                statement.ends_with(';'),
                "Statement missing semicolon: {}",
                statement
            );
            // Must contain CREATE
            assert!(
                statement.contains("CREATE"),
                "Invalid statement: {}",
                statement
            );
            // Must NOT contain IF NOT EXISTS (Memgraph doesn't support this)
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
        // Verify we have a reasonable number of schema statements
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
}
