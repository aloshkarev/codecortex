//! Schema management for the graph database.
//!
//! This module handles:
//! - Creating and maintaining database constraints
//! - Branch-aware indexes for multi-project support
//! - BranchIndex node management for tracking indexed branches

use crate::GraphClient;
use chrono::{DateTime, Utc};
use cortex_core::Result;
use cortex_core::CortexError;
use neo4rs::query;
use serde::{Deserialize, Serialize};

/// Schema statements for basic constraints and indexes
const SCHEMA_STATEMENTS: &[&str] = &[
    // Basic uniqueness constraints
    "DROP CONSTRAINT ON (r:Repository) ASSERT r.path IS UNIQUE;",
    "DROP CONSTRAINT ON (d:Directory) ASSERT d.path IS UNIQUE;",
    "DROP CONSTRAINT ON (f:File) ASSERT f.path IS UNIQUE;",
    "CREATE CONSTRAINT ON (r:Repository) ASSERT r.path IS UNIQUE;",
    "CREATE CONSTRAINT ON (d:Directory) ASSERT d.path IS UNIQUE;",
    "CREATE CONSTRAINT ON (f:File) ASSERT f.path IS UNIQUE;",
    // Basic name/path indexes
    "CREATE INDEX ON :Function(name);",
    "CREATE INDEX ON :Function(path);",
    "CREATE INDEX ON :Class(name);",
    "CREATE INDEX ON :Class(path);",
    "CREATE INDEX ON :Variable(name);",
    "CREATE INDEX ON :Parameter(name);",
    "CREATE INDEX ON :Module(name);",
    "CREATE INDEX ON :CallTarget(name);",
    "CREATE INDEX ON :CodeNode(path);",
    "CREATE INDEX ON :CodeNode(kind);",
    "CREATE INDEX ON :CodeNode(name);",
    "CREATE INDEX ON :CodeNode(line_number);",
];

/// Branch-aware indexes for multi-project support
const BRANCH_SCHEMA_STATEMENTS: &[&str] = &[
    // Branch property indexes on code nodes
    "CREATE INDEX ON :CodeNode(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :CodeNode(repository_path) IF NOT EXISTS;",
    "CREATE INDEX ON :Function(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :Function(repository_path) IF NOT EXISTS;",
    "CREATE INDEX ON :Class(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :Class(repository_path) IF NOT EXISTS;",
    "CREATE INDEX ON :File(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :File(repository_path) IF NOT EXISTS;",
    "CREATE INDEX ON :Variable(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :Module(branch) IF NOT EXISTS;",
    // BranchIndex node constraints and indexes
    "CREATE CONSTRAINT ON (bi:BranchIndex) ASSERT bi.id IS UNIQUE IF NOT EXISTS;",
    "CREATE INDEX ON :BranchIndex(repository_path) IF NOT EXISTS;",
    "CREATE INDEX ON :BranchIndex(branch) IF NOT EXISTS;",
    "CREATE INDEX ON :BranchIndex(commit_hash) IF NOT EXISTS;",
    "CREATE INDEX ON :BranchIndex(indexed_at) IF NOT EXISTS;",
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
pub async fn ensure_constraints(client: &GraphClient) -> Result<()> {
    // Apply basic schema
    for statement in SCHEMA_STATEMENTS {
        let _ = client.run(statement).await;
    }

    // Apply branch-aware schema
    for statement in BRANCH_SCHEMA_STATEMENTS {
        let _ = client.run(statement).await;
    }

    Ok(())
}

/// Create a BranchIndex node in the graph
pub async fn create_branch_index(client: &GraphClient, record: &BranchIndexRecord) -> Result<()> {
    let q = query(
        r#"
        MERGE (bi:BranchIndex {id: $id})
        SET bi.repository_path = $repository_path,
            bi.branch = $branch,
            bi.commit_hash = $commit_hash,
            bi.indexed_at = datetime($indexed_at),
            bi.file_count = $file_count,
            bi.symbol_count = $symbol_count,
            bi.is_stale = $is_stale,
            bi.index_duration_ms = $index_duration_ms
        "#
    )
    .param("id", record.id.clone())
    .param("repository_path", record.repository_path.clone())
    .param("branch", record.branch.clone())
    .param("commit_hash", record.commit_hash.clone())
    .param("indexed_at", record.indexed_at.to_rfc3339())
    .param("file_count", record.file_count as i64)
    .param("symbol_count", record.symbol_count as i64)
    .param("is_stale", record.is_stale)
    .param("index_duration_ms", record.index_duration_ms as i64);

    client.inner()
        .run(q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

    Ok(())
}

/// Get all branch indexes for a repository
pub async fn get_branch_indexes(
    client: &GraphClient,
    repository_path: &str,
) -> Result<Vec<BranchIndexRecord>> {
    let q = query(
        r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path})
        RETURN bi.id, bi.repository_path, bi.branch, bi.commit_hash,
               bi.indexed_at, bi.file_count, bi.symbol_count,
               bi.is_stale, bi.index_duration_ms
        ORDER BY bi.indexed_at DESC
        "#
    )
    .param("repository_path", repository_path.to_string());

    let mut result = client.inner()
        .execute(q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

    let mut indexes = Vec::new();
    while let Ok(Some(row)) = result.next().await {
        let id: String = row.get("bi.id").unwrap_or_default();
        let repo_path: String = row.get("bi.repository_path").unwrap_or_default();
        let branch: String = row.get("bi.branch").unwrap_or_default();
        let commit_hash: String = row.get("bi.commit_hash").unwrap_or_default();
        let file_count: i64 = row.get("bi.file_count").unwrap_or(0);
        let symbol_count: i64 = row.get("bi.symbol_count").unwrap_or(0);
        let is_stale: bool = row.get("bi.is_stale").unwrap_or(false);
        let index_duration_ms: i64 = row.get("bi.index_duration_ms").unwrap_or(0);

        // Parse indexed_at or use current time as fallback
        let indexed_at_str: Option<String> = row.get("bi.indexed_at").ok();
        let indexed_at = indexed_at_str
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        indexes.push(BranchIndexRecord {
            id,
            repository_path: repo_path,
            branch,
            commit_hash,
            indexed_at,
            file_count: file_count as usize,
            symbol_count: symbol_count as usize,
            is_stale,
            index_duration_ms: index_duration_ms as u64,
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
    let delete_nodes_q = query(
        r#"
        MATCH (n:CodeNode {repository_path: $repository_path, branch: $branch})
        DETACH DELETE n
        RETURN count(n) as deleted
        "#
    )
    .param("repository_path", repository_path.to_string())
    .param("branch", branch.to_string());

    let mut result = client.inner()
        .execute(delete_nodes_q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

    let mut deleted_count = 0usize;
    while let Ok(Some(row)) = result.next().await {
        if let Ok(count) = row.get::<i64>("deleted") {
            deleted_count = count.max(0) as usize;
        }
    }

    // Then delete the BranchIndex node
    let delete_index_q = query(
        r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        DELETE bi
        "#
    )
    .param("repository_path", repository_path.to_string())
    .param("branch", branch.to_string());

    client.inner()
        .run(delete_index_q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

    Ok(deleted_count)
}

/// Check if a branch index is current (matches the given commit)
pub async fn is_branch_index_current(
    client: &GraphClient,
    repository_path: &str,
    branch: &str,
    commit_hash: &str,
) -> Result<bool> {
    let q = query(
        r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        RETURN bi.commit_hash = $commit_hash AS is_current, bi.is_stale AS is_stale
        "#
    )
    .param("repository_path", repository_path.to_string())
    .param("branch", branch.to_string())
    .param("commit_hash", commit_hash.to_string());

    let mut result = client.inner()
        .execute(q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

    while let Ok(Some(row)) = result.next().await {
        let is_current: bool = row.get("is_current").unwrap_or(false);
        let is_stale: bool = row.get("is_stale").unwrap_or(true);
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
    let q = query(
        r#"
        MATCH (bi:BranchIndex {repository_path: $repository_path, branch: $branch})
        SET bi.is_stale = true
        "#
    )
    .param("repository_path", repository_path.to_string())
    .param("branch", branch.to_string());

    client.inner()
        .run(q)
        .await
        .map_err(|e| CortexError::Database(e.to_string()))?;

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
    fn schema_contains_constraints() {
        let constraint_count = SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.contains("CONSTRAINT"))
            .count();
        assert!(constraint_count > 0);
    }

    #[test]
    fn schema_contains_indexes() {
        let index_count = SCHEMA_STATEMENTS
            .iter()
            .chain(BRANCH_SCHEMA_STATEMENTS.iter())
            .filter(|s| s.contains("INDEX"))
            .count();
        assert!(index_count > 0);
    }

    #[test]
    fn schema_statements_are_valid_cypher() {
        for statement in SCHEMA_STATEMENTS.iter().chain(BRANCH_SCHEMA_STATEMENTS.iter()) {
            assert!(statement.ends_with(';'), "Statement missing semicolon: {}", statement);
            assert!(
                statement.contains("CREATE") || statement.contains("DROP"),
                "Invalid statement: {}",
                statement
            );
        }
    }

    #[test]
    fn schema_has_repository_constraints() {
        let repo_constraints: Vec<_> = SCHEMA_STATEMENTS
            .iter()
            .filter(|s| s.contains("Repository"))
            .collect();
        assert!(!repo_constraints.is_empty());
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
        let record = BranchIndexRecord::new(
            "/path/to/repo",
            "main",
            "abc123",
            42,
            256,
            1500,
        );

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
        let record = BranchIndexRecord::new(
            "/path/to/repo",
            "feature/test",
            "def456",
            10,
            50,
            500,
        );

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("feature/test"));
        assert!(json.contains("def456"));

        let deserialized: BranchIndexRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, record.id);
        assert_eq!(deserialized.branch, record.branch);
    }
}
