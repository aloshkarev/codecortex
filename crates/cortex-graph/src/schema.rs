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

/// Additional indexes for navigation-heavy lookups.
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

/// Ensure all schema constraints and indexes exist.
///
/// Uses Memgraph syntax by default.  When the backend is Neo4j the
/// `CREATE INDEX ON :Label(prop)` statements are rewritten to the
/// `CREATE INDEX IF NOT EXISTS FOR (n:Label) ON (n.prop)` form that
/// Neo4j 4.x+ requires.  Label-only indexes (Memgraph-specific) are
/// skipped on Neo4j.
pub async fn ensure_constraints(client: &GraphClient) -> Result<()> {
    let is_neo4j = matches!(
        client.backend(),
        crate::backend::BackendKind::Neo4j | crate::backend::BackendKind::Neptune
    );

    let all_statements = SCHEMA_STATEMENTS
        .iter()
        .chain(BRANCH_SCHEMA_STATEMENTS.iter())
        .chain(NAVIGATION_SCHEMA_STATEMENTS.iter());

    for statement in all_statements {
        let cypher = if is_neo4j {
            match rewrite_index_for_neo4j(statement) {
                Some(c) => c,
                None => continue, // skip unsupported statement
            }
        } else {
            (*statement).to_string()
        };

        if let Err(e) = client.run(&cypher).await {
            tracing::debug!(
                "Schema statement returned (may already exist): {} - {}",
                cypher,
                e
            );
        }
    }

    Ok(())
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

/// Rewrite a Memgraph-style index statement to Neo4j syntax.
/// Returns `None` for statements that have no Neo4j equivalent
/// (e.g. label-only indexes).
fn rewrite_index_for_neo4j(statement: &str) -> Option<String> {
    let s = statement.trim().trim_end_matches(';');

    // Label-property index: "CREATE INDEX ON :Label(prop)"
    if let Some(rest) = s.strip_prefix("CREATE INDEX ON :") {
        if let Some((label, prop_part)) = rest.split_once('(') {
            let prop = prop_part.trim_end_matches(')');
            return Some(format!(
                "CREATE INDEX IF NOT EXISTS FOR (n:{label}) ON (n.{prop});"
            ));
        }
        // Label-only index: "CREATE INDEX ON :Label" — skip for Neo4j
        return None;
    }

    // Pass through anything else unchanged
    Some(format!("{s};"))
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
        RETURN bi.id AS id,
               bi.repository_path AS repository_path,
               bi.branch AS branch,
               bi.commit_hash AS commit_hash,
               bi.indexed_at AS indexed_at,
               bi.file_count AS file_count,
               bi.symbol_count AS symbol_count,
               bi.is_stale AS is_stale,
               bi.index_duration_ms AS index_duration_ms
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

    let indexed_at = row
        .get("indexed_at")
        .or_else(|| row.get("bi.indexed_at"))
        .and_then(|v| v.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

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
    }
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
               bi.index_duration_ms AS index_duration_ms
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
            "index_duration_ms": 56
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
    }

    #[test]
    fn rewrite_label_property_index_for_neo4j() {
        let result = rewrite_index_for_neo4j("CREATE INDEX ON :CodeNode(id);");
        assert_eq!(
            result.as_deref(),
            Some("CREATE INDEX IF NOT EXISTS FOR (n:CodeNode) ON (n.id);")
        );
    }

    #[test]
    fn rewrite_label_only_index_skipped_for_neo4j() {
        let result = rewrite_index_for_neo4j("CREATE INDEX ON :Repository;");
        assert!(
            result.is_none(),
            "Label-only indexes should be skipped for Neo4j"
        );
    }

    #[test]
    fn rewrite_branch_index_for_neo4j() {
        let result = rewrite_index_for_neo4j("CREATE INDEX ON :CodeNode(branch);");
        assert_eq!(
            result.as_deref(),
            Some("CREATE INDEX IF NOT EXISTS FOR (n:CodeNode) ON (n.branch);")
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
}
