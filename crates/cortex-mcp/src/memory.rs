//! SQLite Memory Store for Session Observations
//!
//! Provides persistent storage for session observations with:
//! - SQLite backend for durability
//! - Staleness tracking via symbol hash changes
//! - Retention TTL (90 days default)
//! - Audit append-only log
//! - Security controls (secret detection, size limits, rate limiting)

#![allow(dead_code)]

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default retention period in days
const DEFAULT_RETENTION_DAYS: u64 = 90;

/// Maximum observation size in bytes (8 KB)
const MAX_OBSERVATION_SIZE: usize = 8 * 1024;

/// Rate limit: maximum observations per session per minute
const RATE_LIMIT_PER_MINUTE: usize = 30;

/// Observation classification
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    #[default]
    Internal,
    External,
    Hypothesis,
    Decision,
    Blocker,
    Note,
}

impl std::fmt::Display for Classification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Classification::Internal => write!(f, "internal"),
            Classification::External => write!(f, "external"),
            Classification::Hypothesis => write!(f, "hypothesis"),
            Classification::Decision => write!(f, "decision"),
            Classification::Blocker => write!(f, "blocker"),
            Classification::Note => write!(f, "note"),
        }
    }
}

impl std::str::FromStr for Classification {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "internal" => Ok(Self::Internal),
            "external" => Ok(Self::External),
            "hypothesis" => Ok(Self::Hypothesis),
            "decision" => Ok(Self::Decision),
            "blocker" => Ok(Self::Blocker),
            "note" => Ok(Self::Note),
            _ => Err(format!("Unknown classification: {}", s)),
        }
    }
}

/// Observation severity
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    #[default]
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("Unknown severity: {}", s)),
        }
    }
}

/// An observation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Unique observation ID
    pub observation_id: String,
    /// Repository ID
    pub repo_id: String,
    /// Session ID
    pub session_id: String,
    /// Creation timestamp (milliseconds since epoch)
    pub created_at: i64,
    /// Creator (e.g., "mcp", "cli")
    pub created_by: String,
    /// Observation text content
    pub text: String,
    /// Symbol references
    pub symbol_refs: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Whether this observation is stale
    pub stale: bool,
    /// Classification
    pub classification: Classification,
    /// Severity
    pub severity: Severity,
    /// Tags
    pub tags: Vec<String>,
    /// Source revision (git commit hash or similar)
    pub source_revision: String,
}

/// Staleness checker for observations
pub struct StalenessChecker {
    /// Symbol hashes from the last index
    symbol_hashes: std::collections::HashMap<String, String>,
}

impl StalenessChecker {
    /// Create a new staleness checker
    pub fn new() -> Self {
        Self {
            symbol_hashes: std::collections::HashMap::new(),
        }
    }

    /// Update symbol hashes from current index
    pub fn update_hashes(&mut self, hashes: std::collections::HashMap<String, String>) {
        self.symbol_hashes = hashes;
    }

    /// Check if an observation is stale based on symbol hash changes
    pub fn is_stale(
        &self,
        symbol_refs: &[String],
        stored_hashes: &std::collections::HashMap<String, String>,
    ) -> bool {
        for symbol in symbol_refs {
            let current = self.symbol_hashes.get(symbol);
            let stored = stored_hashes.get(symbol);

            match (current, stored) {
                (Some(c), Some(s)) if c != s => return true,
                (None, Some(_)) => return true, // Symbol no longer exists
                _ => continue,
            }
        }
        false
    }
}

impl Default for StalenessChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// SQLite-based memory store
pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
    staleness_checker: StalenessChecker,
    retention_days: u64,
}

impl MemoryStore {
    /// Open or create the memory store at the default location
    pub fn open() -> Result<Self, MemoryStoreError> {
        let path = Self::default_path();
        Self::open_at(&path)
    }

    /// Open or create the memory store at a custom location
    pub fn open_at(path: &Path) -> Result<Self, MemoryStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            staleness_checker: StalenessChecker::new(),
            retention_days: DEFAULT_RETENTION_DAYS,
        };

        store.initialize()?;
        Ok(store)
    }

    /// Get the default storage path
    pub fn default_path() -> PathBuf {
        if let Ok(path) = std::env::var("CORTEX_MEMORY_DB_PATH") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/memory.db")
    }

    /// Initialize the database schema
    fn initialize(&self) -> Result<(), MemoryStoreError> {
        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS observations (
                observation_id TEXT PRIMARY KEY,
                repo_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL,
                text TEXT NOT NULL,
                symbol_refs TEXT NOT NULL DEFAULT '[]',
                confidence REAL NOT NULL DEFAULT 0.8,
                stale INTEGER NOT NULL DEFAULT 0,
                classification TEXT NOT NULL DEFAULT 'internal',
                severity TEXT NOT NULL DEFAULT 'info',
                tags TEXT NOT NULL DEFAULT '[]',
                source_revision TEXT NOT NULL DEFAULT ''
            );

            CREATE INDEX IF NOT EXISTS idx_observations_repo ON observations(repo_id);
            CREATE INDEX IF NOT EXISTS idx_observations_session ON observations(session_id);
            CREATE INDEX IF NOT EXISTS idx_observations_created ON observations(created_at);
            CREATE INDEX IF NOT EXISTS idx_observations_stale ON observations(stale);

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ms INTEGER NOT NULL,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                target_id TEXT NOT NULL,
                details TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp_ms);
            "#,
        )?;

        Ok(())
    }

    /// Save an observation
    pub fn save(&self, obs: &Observation) -> Result<(), MemoryStoreError> {
        // Validate
        if obs.text.is_empty() {
            return Err(MemoryStoreError::ValidationError(
                "text must not be empty".into(),
            ));
        }

        if obs.text.len() > MAX_OBSERVATION_SIZE {
            return Err(MemoryStoreError::ValidationError(format!(
                "text too large; max {} bytes",
                MAX_OBSERVATION_SIZE
            )));
        }

        if looks_sensitive(&obs.text) {
            return Err(MemoryStoreError::SensitiveContent);
        }

        // Check rate limit
        if self.is_rate_limited(&obs.session_id)? {
            return Err(MemoryStoreError::RateLimited);
        }

        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO observations (
                observation_id, repo_id, session_id, created_at, created_by,
                text, symbol_refs, confidence, stale, classification,
                severity, tags, source_revision
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                obs.observation_id,
                obs.repo_id,
                obs.session_id,
                obs.created_at,
                obs.created_by,
                obs.text,
                serde_json::to_string(&obs.symbol_refs)?,
                obs.confidence,
                if obs.stale { 1 } else { 0 },
                obs.classification.to_string(),
                obs.severity.to_string(),
                serde_json::to_string(&obs.tags)?,
                obs.source_revision,
            ],
        )?;

        // Append to audit log
        self.append_audit(&conn, "save", &obs.observation_id, None)?;

        Ok(())
    }

    /// Get an observation by ID
    pub fn get(&self, observation_id: &str) -> Result<Option<Observation>, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let mut stmt = conn.prepare(
            r#"
            SELECT observation_id, repo_id, session_id, created_at, created_by,
                   text, symbol_refs, confidence, stale, classification,
                   severity, tags, source_revision
            FROM observations WHERE observation_id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![observation_id], |row| {
            Ok(Observation {
                observation_id: row.get(0)?,
                repo_id: row.get(1)?,
                session_id: row.get(2)?,
                created_at: row.get(3)?,
                created_by: row.get(4)?,
                text: row.get(5)?,
                symbol_refs: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                confidence: row.get(7)?,
                stale: row.get::<_, i32>(8)? != 0,
                classification: row.get::<_, String>(9)?.parse().unwrap_or_default(),
                severity: row.get::<_, String>(10)?.parse().unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                source_revision: row.get(12)?,
            })
        });

        match result {
            Ok(obs) => Ok(Some(obs)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Search observations
    pub fn search(
        &self,
        repo_id: &str,
        query: Option<&str>,
        session_id: Option<&str>,
        include_stale: bool,
        max_items: usize,
    ) -> Result<Vec<Observation>, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let sql = if include_stale {
            r#"
            SELECT observation_id, repo_id, session_id, created_at, created_by,
                   text, symbol_refs, confidence, stale, classification,
                   severity, tags, source_revision
            FROM observations
            WHERE repo_id = ?1
              AND (?2 IS NULL OR session_id = ?2)
              AND (?3 IS NULL OR text LIKE '%' || ?3 || '%')
            ORDER BY created_at DESC
            LIMIT ?4
            "#
        } else {
            r#"
            SELECT observation_id, repo_id, session_id, created_at, created_by,
                   text, symbol_refs, confidence, stale, classification,
                   severity, tags, source_revision
            FROM observations
            WHERE repo_id = ?1 AND stale = 0
              AND (?2 IS NULL OR session_id = ?2)
              AND (?3 IS NULL OR text LIKE '%' || ?3 || '%')
            ORDER BY created_at DESC
            LIMIT ?4
            "#
        };

        let mut stmt = conn.prepare(sql)?;

        let rows = stmt.query_map(
            params![repo_id, session_id, query, max_items as i32],
            |row| {
                Ok(Observation {
                    observation_id: row.get(0)?,
                    repo_id: row.get(1)?,
                    session_id: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                    text: row.get(5)?,
                    symbol_refs: serde_json::from_str(&row.get::<_, String>(6)?)
                        .unwrap_or_default(),
                    confidence: row.get(7)?,
                    stale: row.get::<_, i32>(8)? != 0,
                    classification: row.get::<_, String>(9)?.parse().unwrap_or_default(),
                    severity: row.get::<_, String>(10)?.parse().unwrap_or_default(),
                    tags: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
                    source_revision: row.get(12)?,
                })
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Get session context (observations for a session)
    pub fn get_session_context(
        &self,
        repo_id: &str,
        session_id: &str,
        include_stale: bool,
        max_items: usize,
    ) -> Result<Vec<Observation>, MemoryStoreError> {
        self.search(repo_id, None, Some(session_id), include_stale, max_items)
    }

    /// Mark observations as stale based on symbol changes
    pub fn update_staleness(
        &self,
        repo_id: &str,
        changed_symbols: &[String],
    ) -> Result<usize, MemoryStoreError> {
        if changed_symbols.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let mut count = 0;
        for symbol in changed_symbols {
            let affected = conn.execute(
                r#"
                UPDATE observations SET stale = 1
                WHERE repo_id = ?1 AND stale = 0
                  AND symbol_refs LIKE '%' || ?2 || '%'
                "#,
                params![repo_id, symbol],
            )?;
            count += affected;
        }

        Ok(count)
    }

    /// Delete old observations based on retention policy
    pub fn prune(&self) -> Result<usize, MemoryStoreError> {
        let cutoff = current_time_ms() - (self.retention_days * 24 * 60 * 60 * 1000) as i64;

        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let count = conn.execute(
            "DELETE FROM observations WHERE created_at < ?1",
            params![cutoff],
        )?;

        Ok(count)
    }

    /// Get observation count
    pub fn count(&self, repo_id: Option<&str>) -> Result<usize, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let count: i32 = match repo_id {
            Some(repo) => conn.query_row(
                "SELECT COUNT(*) FROM observations WHERE repo_id = ?1",
                params![repo],
                |row| row.get(0),
            )?,
            None => conn.query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))?,
        };

        Ok(count as usize)
    }

    /// Check if rate limited
    fn is_rate_limited(&self, session_id: &str) -> Result<bool, MemoryStoreError> {
        let one_minute_ago = current_time_ms() - 60_000;

        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM observations WHERE session_id = ?1 AND created_at > ?2",
            params![session_id, one_minute_ago],
            |row| row.get(0),
        )?;

        Ok(count >= RATE_LIMIT_PER_MINUTE as i32)
    }

    /// Append to audit log
    fn append_audit(
        &self,
        conn: &Connection,
        action: &str,
        target_id: &str,
        details: Option<&str>,
    ) -> Result<(), MemoryStoreError> {
        conn.execute(
            "INSERT INTO audit_log (timestamp_ms, actor, action, target_id, details) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![current_time_ms(), "mcp", action, target_id, details],
        )?;

        Ok(())
    }

    /// Get audit log entries
    pub fn get_audit_log(
        &self,
        since_ms: i64,
        limit: usize,
    ) -> Result<Vec<AuditEntry>, MemoryStoreError> {
        let conn = self.conn.lock().map_err(|_| MemoryStoreError::LockError)?;

        let mut stmt = conn.prepare(
            "SELECT timestamp_ms, actor, action, target_id, details FROM audit_log WHERE timestamp_ms > ?1 ORDER BY timestamp_ms DESC LIMIT ?2"
        )?;

        let rows = stmt.query_map(params![since_ms, limit as i32], |row| {
            Ok(AuditEntry {
                timestamp_ms: row.get(0)?,
                actor: row.get(1)?,
                action: row.get(2)?,
                target_id: row.get(3)?,
                details: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_ms: i64,
    pub actor: String,
    pub action: String,
    pub target_id: String,
    pub details: Option<String>,
}

/// Memory store errors
#[derive(Debug, thiserror::Error)]
pub enum MemoryStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Sensitive content detected")]
    SensitiveContent,

    #[error("Rate limited")]
    RateLimited,

    #[error("Lock error")]
    LockError,
}

/// Check if text looks like it contains sensitive content
pub fn looks_sensitive(text: &str) -> bool {
    let lowered = text.to_lowercase();

    // Pattern matching for common secrets
    let patterns = [
        "password=",
        "secret=",
        "api_key",
        "apikey",
        "token=",
        "bearer ",
        "begin private key",
        "begin rsa private key",
        "aws_access_key_id",
        "aws_secret_access_key",
        "x-api-key",
    ];

    for pattern in patterns {
        if lowered.contains(pattern) {
            return true;
        }
    }

    // High entropy detection for potential secrets
    // Look for strings that look like base64 encoded secrets
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in words {
        if word.len() >= 32 && is_high_entropy(word) {
            return true;
        }
    }

    false
}

/// Check if a string has high entropy (potential secret)
fn is_high_entropy(s: &str) -> bool {
    let mut char_counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();

    for c in s.chars() {
        *char_counts.entry(c).or_insert(0) += 1;
    }

    if char_counts.len() < 8 {
        return false; // Not enough character variety
    }

    // Calculate entropy
    let len = s.len() as f64;
    let mut entropy = 0.0;

    for &count in char_counts.values() {
        let p = count as f64 / len;
        entropy -= p * p.log2();
    }

    entropy > 4.0 // High entropy threshold
}

/// Get current time in milliseconds
fn current_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Generate a unique observation ID
pub fn generate_observation_id() -> String {
    format!("obs-{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn classification_parsing() {
        assert_eq!(
            Classification::from_str("internal").unwrap(),
            Classification::Internal
        );
        assert_eq!(
            Classification::from_str("HYPOTHESIS").unwrap(),
            Classification::Hypothesis
        );
        assert!(Classification::from_str("unknown").is_err());
    }

    #[test]
    fn severity_parsing() {
        assert_eq!(Severity::from_str("info").unwrap(), Severity::Info);
        assert_eq!(Severity::from_str("WARNING").unwrap(), Severity::Warning);
        assert!(Severity::from_str("unknown").is_err());
    }

    #[test]
    fn looks_sensitive_detects_secrets() {
        assert!(looks_sensitive("my API_KEY=12345"));
        assert!(looks_sensitive("password=secret123"));
        assert!(looks_sensitive("token=abc123def456"));
        assert!(looks_sensitive("-----BEGIN PRIVATE KEY-----"));
        assert!(!looks_sensitive("regular engineering note"));
        assert!(!looks_sensitive("function calculate(x, y)"));
    }

    #[test]
    fn test_is_high_entropy() {
        assert!(super::is_high_entropy("aB1cD2eF3gH4iJ5kL6mN7oP8"));
        assert!(!super::is_high_entropy("aaaaaaaaaaaa"));
        assert!(!super::is_high_entropy("12345678"));
    }

    #[test]
    fn memory_store_crud() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = MemoryStore::open_at(&path).unwrap();

        let obs = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test-repo".to_string(),
            session_id: "test-session".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "This is a test observation".to_string(),
            symbol_refs: vec!["func:test".to_string()],
            confidence: 0.9,
            stale: false,
            classification: Classification::Internal,
            severity: Severity::Info,
            tags: vec!["test".to_string()],
            source_revision: "abc123".to_string(),
        };

        store.save(&obs).unwrap();

        let retrieved = store.get(&obs.observation_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().text, "This is a test observation");
    }

    #[test]
    fn memory_store_search() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = MemoryStore::open_at(&path).unwrap();

        let obs1 = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test-repo".to_string(),
            session_id: "session-1".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "Authentication bug found".to_string(),
            symbol_refs: vec![],
            confidence: 0.9,
            stale: false,
            classification: Classification::Blocker,
            severity: Severity::Error,
            tags: vec![],
            source_revision: "".to_string(),
        };

        let obs2 = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test-repo".to_string(),
            session_id: "session-2".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "Refactoring complete".to_string(),
            symbol_refs: vec![],
            confidence: 0.8,
            stale: false,
            classification: Classification::Note,
            severity: Severity::Info,
            tags: vec![],
            source_revision: "".to_string(),
        };

        store.save(&obs1).unwrap();
        store.save(&obs2).unwrap();

        let results = store
            .search("test-repo", Some("bug"), None, false, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("bug"));
    }

    #[test]
    fn memory_store_rate_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = MemoryStore::open_at(&path).unwrap();

        // Create many observations quickly
        for i in 0..35 {
            let obs = Observation {
                observation_id: generate_observation_id(),
                repo_id: "test-repo".to_string(),
                session_id: "same-session".to_string(),
                created_at: current_time_ms(),
                created_by: "test".to_string(),
                text: format!("Observation {}", i),
                symbol_refs: vec![],
                confidence: 0.9,
                stale: false,
                classification: Classification::Internal,
                severity: Severity::Info,
                tags: vec![],
                source_revision: "".to_string(),
            };

            let result = store.save(&obs);
            if i >= RATE_LIMIT_PER_MINUTE {
                assert!(matches!(result, Err(MemoryStoreError::RateLimited)));
            }
        }
    }

    #[test]
    fn memory_store_validation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = MemoryStore::open_at(&path).unwrap();

        // Empty text
        let obs = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test".to_string(),
            session_id: "test".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "".to_string(),
            symbol_refs: vec![],
            confidence: 0.9,
            stale: false,
            classification: Classification::Internal,
            severity: Severity::Info,
            tags: vec![],
            source_revision: "".to_string(),
        };

        let result = store.save(&obs);
        assert!(matches!(result, Err(MemoryStoreError::ValidationError(_))));

        // Sensitive content
        let obs = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test".to_string(),
            session_id: "test".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "password=secret123".to_string(),
            symbol_refs: vec![],
            confidence: 0.9,
            stale: false,
            classification: Classification::Internal,
            severity: Severity::Info,
            tags: vec![],
            source_revision: "".to_string(),
        };

        let result = store.save(&obs);
        assert!(matches!(result, Err(MemoryStoreError::SensitiveContent)));
    }

    #[test]
    fn memory_store_staleness_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = MemoryStore::open_at(&path).unwrap();

        let obs = Observation {
            observation_id: generate_observation_id(),
            repo_id: "test-repo".to_string(),
            session_id: "test".to_string(),
            created_at: current_time_ms(),
            created_by: "test".to_string(),
            text: "Test observation".to_string(),
            symbol_refs: vec!["func:changed_func".to_string()],
            confidence: 0.9,
            stale: false,
            classification: Classification::Internal,
            severity: Severity::Info,
            tags: vec![],
            source_revision: "".to_string(),
        };

        store.save(&obs).unwrap();

        // Update staleness
        let count = store
            .update_staleness("test-repo", &["func:changed_func".to_string()])
            .unwrap();
        assert_eq!(count, 1);

        // Verify it's marked stale
        let retrieved = store.get(&obs.observation_id).unwrap().unwrap();
        assert!(retrieved.stale);
    }
}
