use crate::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Resolve config base directory (HOME or ".") without allocating a String for HOME.
fn config_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Connection pool configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: usize,
    /// Minimum number of idle connections to maintain
    pub min_idle: usize,
    /// Maximum time to wait for a connection
    pub connection_timeout: Duration,
    /// Maximum time a connection can be idle before being closed
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
        }
    }
}

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorConfig {
    /// Vector store type: "lancedb" (embedded, recommended), "json" (simple), or "qdrant" (production)
    pub store_type: String,
    /// Path for local vector storage
    pub store_path: PathBuf,
    /// Qdrant URI (production)
    pub qdrant_uri: String,
    /// Qdrant API key (optional)
    pub qdrant_api_key: Option<String>,
    /// Embedding dimension (default 1536 for OpenAI)
    pub embedding_dim: usize,
}

impl Default for VectorConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            store_type: "lancedb".to_string(),
            store_path: PathBuf::from(home).join(".cortex/vectors"),
            qdrant_uri: "http://127.0.0.1:6333".to_string(),
            qdrant_api_key: None,
            embedding_dim: 1536,
        }
    }
}

/// LLM/Embedding provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Provider: "openai", "ollama", or "none"
    pub provider: String,
    /// OpenAI API key
    pub openai_api_key: Option<String>,
    /// OpenAI embedding model
    pub openai_embedding_model: String,
    /// Ollama base URL
    pub ollama_base_url: String,
    /// Ollama embedding model
    pub ollama_embedding_model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "none".to_string(),
            openai_api_key: None,
            openai_embedding_model: "text-embedding-3-small".to_string(),
            ollama_base_url: "http://127.0.0.1:11434".to_string(),
            ollama_embedding_model: "nomic-embed-text".to_string(),
        }
    }
}

/// Main configuration for CodeCortex
///
/// Supports:
/// - File-based configuration via TOML
/// - Environment variable overrides
/// - Runtime configuration for all components
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CortexConfig {
    // Database settings
    pub memgraph_uri: String,
    pub memgraph_user: String,
    pub memgraph_password: String,
    /// Backend type: "memgraph" (default) or "neo4j"
    /// Can also be set via CORTEX_BACKEND_TYPE environment variable
    pub backend_type: String,

    // Vector store settings
    pub vector: VectorConfig,

    // LLM/Embedding settings
    pub llm: LlmConfig,

    // Indexer settings
    pub max_batch_size: usize,
    pub indexer_timeout_secs: u64,
    pub indexer_max_files: usize,

    // Analyzer settings
    pub analyzer_query_limit: usize,
    pub analyzer_cache_ttl_secs: u64,

    // Watcher settings
    pub watcher_debounce_secs: u64,
    pub watcher_max_events: usize,

    // Connection pool settings
    pub pool_max_connections: usize,
    pub pool_min_idle: usize,
    pub pool_connection_timeout_secs: u64,

    // Watched paths
    pub watched_paths: Vec<PathBuf>,
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            // Database defaults
            memgraph_uri: "bolt://127.0.0.1:7687".to_string(),
            memgraph_user: "memgraph".to_string(),
            memgraph_password: "memgraph".to_string(),
            backend_type: "memgraph".to_string(),

            // Vector store defaults
            vector: VectorConfig::default(),

            // LLM defaults
            llm: LlmConfig::default(),

            // Indexer defaults
            max_batch_size: 500,
            indexer_timeout_secs: 300, // 5 minutes
            indexer_max_files: 0,      // unlimited

            // Analyzer defaults
            analyzer_query_limit: 1000,
            analyzer_cache_ttl_secs: 300, // 5 minutes

            // Watcher defaults
            watcher_debounce_secs: 2,
            watcher_max_events: 128,

            // Pool defaults
            pool_max_connections: 10,
            pool_min_idle: 2,
            pool_connection_timeout_secs: 30,

            watched_paths: Vec::new(),
        }
    }
}

impl CortexConfig {
    /// Get the configuration file path
    pub fn config_path() -> PathBuf {
        config_home().join(".cortex/config.toml")
    }

    /// Ensure the parent directory exists
    pub fn ensure_parent_dir() -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Load configuration from file, with environment variable overrides
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        let mut config = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            toml::from_str(&raw).map_err(|e| CortexError::Config(e.to_string()))?
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides to the configuration
    #[allow(clippy::collapsible_if)]
    pub fn apply_env_overrides(&mut self) {
        // Database settings
        if let Ok(val) = std::env::var("CORTEX_MEMGRAPH_URI") {
            self.memgraph_uri = val;
        }
        if let Ok(val) = std::env::var("CORTEX_MEMGRAPH_USER") {
            self.memgraph_user = val;
        }
        if let Ok(val) = std::env::var("CORTEX_MEMGRAPH_PASSWORD") {
            self.memgraph_password = val;
        }
        if let Ok(val) = std::env::var("CORTEX_BACKEND_TYPE") {
            self.backend_type = val;
        }

        // Indexer settings
        if let Ok(val) = std::env::var("CORTEX_INDEXER_BATCH_SIZE") {
            if let Ok(n) = val.parse() {
                self.max_batch_size = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_INDEXER_TIMEOUT_SECS") {
            if let Ok(n) = val.parse() {
                self.indexer_timeout_secs = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_INDEXER_MAX_FILES") {
            if let Ok(n) = val.parse() {
                self.indexer_max_files = n;
            }
        }

        // Analyzer settings
        if let Ok(val) = std::env::var("CORTEX_ANALYZER_QUERY_LIMIT") {
            if let Ok(n) = val.parse() {
                self.analyzer_query_limit = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_ANALYZER_CACHE_TTL_SECS") {
            if let Ok(n) = val.parse() {
                self.analyzer_cache_ttl_secs = n;
            }
        }

        // Watcher settings
        if let Ok(val) = std::env::var("CORTEX_WATCHER_DEBOUNCE_SECS") {
            if let Ok(n) = val.parse() {
                self.watcher_debounce_secs = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_WATCHER_MAX_EVENTS") {
            if let Ok(n) = val.parse() {
                self.watcher_max_events = n;
            }
        }

        // Pool settings
        if let Ok(val) = std::env::var("CORTEX_POOL_MAX_CONNECTIONS") {
            if let Ok(n) = val.parse() {
                self.pool_max_connections = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_POOL_MIN_IDLE") {
            if let Ok(n) = val.parse() {
                self.pool_min_idle = n;
            }
        }
        if let Ok(val) = std::env::var("CORTEX_POOL_TIMEOUT_SECS") {
            if let Ok(n) = val.parse() {
                self.pool_connection_timeout_secs = n;
            }
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        Self::ensure_parent_dir()?;
        let path = Self::config_path();
        let data = toml::to_string_pretty(self).map_err(|e| CortexError::Config(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Get connection pool configuration
    pub fn pool_config(&self) -> crate::PoolConfig {
        crate::PoolConfig {
            max_connections: self.pool_max_connections,
            min_idle: self.pool_min_idle,
            connection_timeout: Duration::from_secs(self.pool_connection_timeout_secs),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(3600),
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.max_batch_size == 0 {
            return Err(CortexError::Config(
                "max_batch_size cannot be 0".to_string(),
            ));
        }
        if self.pool_max_connections == 0 {
            return Err(CortexError::Config(
                "pool_max_connections cannot be 0".to_string(),
            ));
        }
        if self.pool_min_idle > self.pool_max_connections {
            return Err(CortexError::Config(
                "pool_min_idle cannot exceed pool_max_connections".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn test_config() -> CortexConfig {
        CortexConfig::default()
    }

    #[test]
    fn config_defaults() {
        let config = CortexConfig::default();
        assert_eq!(config.memgraph_uri, "bolt://127.0.0.1:7687");
        assert_eq!(config.memgraph_user, "memgraph");
        assert_eq!(config.max_batch_size, 500);
        assert_eq!(config.indexer_timeout_secs, 300);
        assert_eq!(config.pool_max_connections, 10);
    }

    #[test]
    fn config_roundtrip() {
        let original = CortexConfig {
            memgraph_uri: "bolt://test:7687".to_string(),
            memgraph_user: "user".to_string(),
            memgraph_password: "pwd".to_string(),
            backend_type: "memgraph".to_string(),
            vector: VectorConfig::default(),
            llm: LlmConfig::default(),
            max_batch_size: 100,
            indexer_timeout_secs: 60,
            indexer_max_files: 1000,
            analyzer_query_limit: 500,
            analyzer_cache_ttl_secs: 120,
            watcher_debounce_secs: 5,
            watcher_max_events: 64,
            pool_max_connections: 5,
            pool_min_idle: 1,
            pool_connection_timeout_secs: 15,
            watched_paths: vec![PathBuf::from("/repo1"), PathBuf::from("/repo2")],
        };

        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: CortexConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.memgraph_uri, original.memgraph_uri);
        assert_eq!(parsed.max_batch_size, original.max_batch_size);
        assert_eq!(parsed.indexer_timeout_secs, original.indexer_timeout_secs);
        assert_eq!(parsed.watched_paths, original.watched_paths);
    }

    #[test]
    fn config_path_uses_home() {
        let path = CortexConfig::config_path();
        assert!(path.to_string_lossy().contains(".cortex"));
    }

    #[test]
    fn config_validation() {
        let mut config = CortexConfig::default();
        assert!(config.validate().is_ok());

        config.max_batch_size = 0;
        assert!(config.validate().is_err());

        config.max_batch_size = 100;
        config.pool_max_connections = 0;
        assert!(config.validate().is_err());

        config.pool_max_connections = 5;
        config.pool_min_idle = 10;
        assert!(config.validate().is_err());
    }
}
