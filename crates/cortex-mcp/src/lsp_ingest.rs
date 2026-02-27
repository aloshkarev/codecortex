//! LSP Edge Ingestion for Code Relationships
//!
//! Provides ingestion of LSP-derived call edges with:
//! - Deduplication (24h window)
//! - Symbol validation
//! - Provenance tracking
//! - Merge modes (Upsert, ReplaceWindow)

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Deduplication window duration (24 hours)
const DEDUP_WINDOW_SECS: u64 = 24 * 60 * 60;

/// LSP edge input from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspEdgeInput {
    /// Fully qualified name of the caller
    pub caller_fqn: String,
    /// Fully qualified name of the callee
    pub callee_fqn: String,
    /// File containing the call
    pub file: String,
    /// Line number of the call
    pub line: u64,
    /// Confidence score (0.0 - 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    /// Source of this edge (e.g., "rust-analyzer", "pyright")
    #[serde(default)]
    pub source: Option<String>,
}

fn default_confidence() -> f64 {
    0.9
}

/// Merge mode for edge ingestion
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    /// Upsert: Add new edges, update existing ones
    #[default]
    Upsert,
    /// ReplaceWindow: Replace all edges in the dedup window
    ReplaceWindow,
}

impl std::str::FromStr for MergeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "upsert" => Ok(Self::Upsert),
            "replace_window" | "replacewindow" => Ok(Self::ReplaceWindow),
            _ => Err(format!("Unknown merge mode: {}", s)),
        }
    }
}

/// Ingested edge with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestedEdge {
    /// Edge ID (hash-based)
    pub id: String,
    /// Caller FQN
    pub caller_fqn: String,
    /// Callee FQN
    pub callee_fqn: String,
    /// File path
    pub file: String,
    /// Line number
    pub line: u64,
    /// Confidence score
    pub confidence: f64,
    /// Provenance (source)
    pub provenance: Provenance,
    /// When this edge was first seen
    pub first_seen_ms: i64,
    /// When this edge was last updated
    pub last_updated_ms: i64,
    /// Merge mode used
    pub merge_mode: MergeMode,
}

/// Provenance tracking for edge sources
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// From static analysis (tree-sitter)
    Static,
    /// From LSP server
    #[default]
    Lsp,
    /// From runtime profiling
    Runtime,
    /// Manually added
    Manual,
    /// Inferred from patterns
    Inferred,
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provenance::Static => write!(f, "static"),
            Provenance::Lsp => write!(f, "lsp"),
            Provenance::Runtime => write!(f, "runtime"),
            Provenance::Manual => write!(f, "manual"),
            Provenance::Inferred => write!(f, "inferred"),
        }
    }
}

/// Ingestion result statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestionResult {
    /// Number of edges successfully ingested
    pub ingested: usize,
    /// Number of edges deduplicated
    pub deduped: usize,
    /// Number of edges rejected
    pub rejected: usize,
    /// Reasons for rejections
    pub rejection_reasons: HashMap<String, usize>,
}

/// LSP Edge Ingester
pub struct LspEdgeIngester {
    /// Deduplication window
    dedup_window: Duration,
    /// Known edges for deduplication (edge_id -> first_seen_ms)
    known_edges: HashMap<String, i64>,
    /// Window start time for replace mode
    window_start_ms: i64,
}

impl LspEdgeIngester {
    /// Create a new ingester with default dedup window (24h)
    pub fn new() -> Self {
        Self {
            dedup_window: Duration::from_secs(DEDUP_WINDOW_SECS),
            known_edges: HashMap::new(),
            window_start_ms: current_time_ms(),
        }
    }

    /// Create an ingester with a custom dedup window
    pub fn with_dedup_window(window: Duration) -> Self {
        Self {
            dedup_window: window,
            known_edges: HashMap::new(),
            window_start_ms: current_time_ms(),
        }
    }

    /// Generate a unique edge ID
    fn make_edge_id(caller: &str, callee: &str, file: &str, line: u64) -> String {
        // Simple hash-based ID
        let combined = format!("{}:{}:{}:{}", caller, callee, file, line);
        format!("edge:{:x}", md5_hash(&combined))
    }

    /// Ingest a batch of edges
    pub fn ingest(
        &mut self,
        repo_path: &str,
        edges: Vec<LspEdgeInput>,
        merge_mode: MergeMode,
    ) -> IngestionResult {
        let mut result = IngestionResult::default();
        let now = current_time_ms();
        let window_cutoff = now - self.dedup_window.as_millis() as i64;

        // Clean old entries from known_edges
        self.known_edges
            .retain(|_, &mut first_seen| first_seen > window_cutoff);

        // Track unique edges within this batch
        let mut batch_seen: HashSet<String> = HashSet::new();

        for edge in edges {
            // Validate edge
            if let Err(reason) = self.validate_edge(&edge, repo_path) {
                result.rejected += 1;
                *result.rejection_reasons.entry(reason).or_insert(0) += 1;
                continue;
            }

            let edge_id =
                Self::make_edge_id(&edge.caller_fqn, &edge.callee_fqn, &edge.file, edge.line);

            // Check for duplicates within batch
            if !batch_seen.insert(edge_id.clone()) {
                result.deduped += 1;
                continue;
            }

            // Check for duplicates within window
            if merge_mode == MergeMode::Upsert
                && let Some(&first_seen) = self.known_edges.get(&edge_id)
                && first_seen > window_cutoff
            {
                result.deduped += 1;
                continue;
            }

            // Mark as known
            self.known_edges.insert(edge_id.clone(), now);
            result.ingested += 1;
        }

        result
    }

    /// Validate an edge
    fn validate_edge(&self, edge: &LspEdgeInput, repo_path: &str) -> Result<(), String> {
        // Check for empty fields
        if edge.caller_fqn.is_empty() {
            return Err("empty_caller_fqn".to_string());
        }
        if edge.callee_fqn.is_empty() {
            return Err("empty_callee_fqn".to_string());
        }
        if edge.file.is_empty() {
            return Err("empty_file".to_string());
        }

        // Validate confidence range
        if edge.confidence < 0.0 || edge.confidence > 1.0 {
            return Err("invalid_confidence_range".to_string());
        }

        // Validate file path is within repo
        // Accept if file starts with repo_path OR is a relative path (doesn't start with /)
        if !edge.file.starts_with(repo_path) && edge.file.starts_with('/') {
            return Err("file_outside_repo".to_string());
        }

        // Validate line number
        if edge.line == 0 {
            return Err("invalid_line_number".to_string());
        }

        Ok(())
    }

    /// Convert edge input to ingested edge
    pub fn to_ingested_edge(&self, edge: LspEdgeInput, merge_mode: MergeMode) -> IngestedEdge {
        let now = current_time_ms();
        let id = Self::make_edge_id(&edge.caller_fqn, &edge.callee_fqn, &edge.file, edge.line);

        IngestedEdge {
            id,
            caller_fqn: edge.caller_fqn,
            callee_fqn: edge.callee_fqn,
            file: edge.file,
            line: edge.line,
            confidence: edge.confidence,
            provenance: edge.source.map(|_| Provenance::Lsp).unwrap_or_default(),
            first_seen_ms: now,
            last_updated_ms: now,
            merge_mode,
        }
    }

    /// Reset the deduplication window
    pub fn reset_window(&mut self) {
        self.known_edges.clear();
        self.window_start_ms = current_time_ms();
    }

    /// Get the number of known edges
    pub fn known_edge_count(&self) -> usize {
        self.known_edges.len()
    }
}

impl Default for LspEdgeIngester {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple MD5 hash for edge IDs
fn md5_hash(input: &str) -> u128 {
    // Simplified hash - not cryptographically secure
    let mut hash: u128 = 0;
    for (i, byte) in input.bytes().enumerate() {
        let shift = (i % 16) * 8;
        hash ^= (byte as u128) << shift;
    }
    // Mixing using native wrapping multiplication
    hash.wrapping_mul(0x5851F42D4C957F2D)
}

/// Get current time in milliseconds
fn current_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn make_edge(caller: &str, callee: &str, file: &str, line: u64) -> LspEdgeInput {
        LspEdgeInput {
            caller_fqn: caller.to_string(),
            callee_fqn: callee.to_string(),
            file: file.to_string(),
            line,
            confidence: 0.9,
            source: Some("rust-analyzer".to_string()),
        }
    }

    #[test]
    fn merge_mode_parsing() {
        assert_eq!(MergeMode::from_str("upsert").unwrap(), MergeMode::Upsert);
        assert_eq!(
            MergeMode::from_str("replace_window").unwrap(),
            MergeMode::ReplaceWindow
        );
        assert!(MergeMode::from_str("unknown").is_err());
    }

    #[test]
    fn edge_id_is_deterministic() {
        let id1 = LspEdgeIngester::make_edge_id("a", "b", "file.rs", 10);
        let id2 = LspEdgeIngester::make_edge_id("a", "b", "file.rs", 10);

        assert_eq!(id1, id2);
    }

    #[test]
    fn edge_id_differs_for_different_edges() {
        let id1 = LspEdgeIngester::make_edge_id("a", "b", "file.rs", 10);
        let id2 = LspEdgeIngester::make_edge_id("a", "b", "file.rs", 11);

        assert_ne!(id1, id2);
    }

    #[test]
    fn ingester_ingests_valid_edges() {
        let mut ingester = LspEdgeIngester::new();

        let edges = vec![
            make_edge("func::a", "func::b", "/repo/src/a.rs", 10),
            make_edge("func::b", "func::c", "/repo/src/b.rs", 20),
        ];

        let result = ingester.ingest("/repo", edges, MergeMode::Upsert);

        assert_eq!(result.ingested, 2);
        assert_eq!(result.deduped, 0);
        assert_eq!(result.rejected, 0);
    }

    #[test]
    fn ingester_deduplicates_within_batch() {
        let mut ingester = LspEdgeIngester::new();

        let edges = vec![
            make_edge("func::a", "func::b", "/repo/src/a.rs", 10),
            make_edge("func::a", "func::b", "/repo/src/a.rs", 10), // Duplicate
        ];

        let result = ingester.ingest("/repo", edges, MergeMode::Upsert);

        assert_eq!(result.ingested, 1);
        assert_eq!(result.deduped, 1);
    }

    #[test]
    fn ingester_deduplicates_across_batches() {
        let mut ingester = LspEdgeIngester::new();

        let batch1 = vec![make_edge("func::a", "func::b", "/repo/src/a.rs", 10)];
        let result1 = ingester.ingest("/repo", batch1, MergeMode::Upsert);
        assert_eq!(result1.ingested, 1);

        // Same edge in second batch
        let batch2 = vec![make_edge("func::a", "func::b", "/repo/src/a.rs", 10)];
        let result2 = ingester.ingest("/repo", batch2, MergeMode::Upsert);
        assert_eq!(result2.ingested, 0);
        assert_eq!(result2.deduped, 1);
    }

    #[test]
    fn ingester_rejects_invalid_edges() {
        let mut ingester = LspEdgeIngester::new();

        let edges = vec![
            LspEdgeInput {
                caller_fqn: "".to_string(), // Empty caller
                callee_fqn: "func::b".to_string(),
                file: "/repo/src/a.rs".to_string(),
                line: 10,
                confidence: 0.9,
                source: None,
            },
            LspEdgeInput {
                caller_fqn: "func::a".to_string(),
                callee_fqn: "func::b".to_string(),
                file: "/repo/src/a.rs".to_string(),
                line: 0, // Invalid line
                confidence: 0.9,
                source: None,
            },
            LspEdgeInput {
                caller_fqn: "func::a".to_string(),
                callee_fqn: "func::b".to_string(),
                file: "/repo/src/a.rs".to_string(),
                line: 10,
                confidence: 1.5, // Invalid confidence
                source: None,
            },
        ];

        let result = ingester.ingest("/repo", edges, MergeMode::Upsert);

        assert_eq!(result.ingested, 0);
        assert_eq!(result.rejected, 3);
        assert!(result.rejection_reasons.contains_key("empty_caller_fqn"));
        assert!(result.rejection_reasons.contains_key("invalid_line_number"));
        assert!(
            result
                .rejection_reasons
                .contains_key("invalid_confidence_range")
        );
    }

    #[test]
    fn ingester_validates_repo_path() {
        let mut ingester = LspEdgeIngester::new();

        let edges = vec![make_edge("func::a", "func::b", "/other/src/a.rs", 10)];

        let result = ingester.ingest("/repo", edges, MergeMode::Upsert);

        assert_eq!(result.rejected, 1);
        assert!(result.rejection_reasons.contains_key("file_outside_repo"));
    }

    #[test]
    fn to_ingested_edge() {
        let ingester = LspEdgeIngester::new();

        let input = make_edge("func::a", "func::b", "/repo/src/a.rs", 10);
        let ingested = ingester.to_ingested_edge(input, MergeMode::Upsert);

        assert!(!ingested.id.is_empty());
        assert_eq!(ingested.caller_fqn, "func::a");
        assert_eq!(ingested.callee_fqn, "func::b");
        assert_eq!(ingested.file, "/repo/src/a.rs");
        assert_eq!(ingested.line, 10);
        assert_eq!(ingested.confidence, 0.9);
        assert_eq!(ingested.provenance, Provenance::Lsp);
    }

    #[test]
    fn reset_window_clears_known_edges() {
        let mut ingester = LspEdgeIngester::new();

        let edges = vec![make_edge("func::a", "func::b", "/repo/src/a.rs", 10)];
        ingester.ingest("/repo", edges, MergeMode::Upsert);

        assert_eq!(ingester.known_edge_count(), 1);

        ingester.reset_window();

        assert_eq!(ingester.known_edge_count(), 0);
    }
}
