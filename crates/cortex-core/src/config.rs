use crate::a2a_config::A2aConfig;
use crate::mcp_config::McpConfig;
use crate::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const LEGACY_CONFIG_RENAMES: &[(&str, &str)] = &[
    ("memgraph_uri", "falkordb_uri"),
    ("memgraph_password", "falkordb_password"),
    ("memgraph_unwind_batch_max", "falkordb_unwind_batch_max"),
    ("memgraph_write_pool_size", "falkordb_write_pool_size"),
];

const LEGACY_CONFIG_REMOVED: &[&str] = &["memgraph_user", "backend_type", "grafeo_path"];

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

fn default_rerank_weight_lexical() -> f64 {
    1.0
}

fn default_rerank_weight_vector() -> f64 {
    0.8
}

fn default_rerank_weight_centrality() -> f64 {
    0.6
}

fn default_rerank_weight_path_penalty() -> f64 {
    0.4
}

fn default_rerank_weight_definition_bias() -> f64 {
    0.6
}

fn default_rerank_weight_recency() -> f64 {
    0.3
}

fn default_rerank_weight_token_cost() -> f64 {
    0.25
}

/// Tunable multi-signal rerank weights (`[vector.rerank_weights]` in config.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RerankWeightsConfig {
    #[serde(default = "default_rerank_weight_lexical")]
    pub lexical: f64,
    #[serde(default = "default_rerank_weight_vector")]
    pub vector: f64,
    #[serde(default = "default_rerank_weight_centrality")]
    pub centrality: f64,
    #[serde(default = "default_rerank_weight_path_penalty")]
    pub path_penalty: f64,
    #[serde(default = "default_rerank_weight_definition_bias")]
    pub definition_bias: f64,
    #[serde(default = "default_rerank_weight_recency")]
    pub recency: f64,
    #[serde(default = "default_rerank_weight_token_cost")]
    pub token_cost: f64,
}

impl Default for RerankWeightsConfig {
    fn default() -> Self {
        Self {
            lexical: default_rerank_weight_lexical(),
            vector: default_rerank_weight_vector(),
            centrality: default_rerank_weight_centrality(),
            path_penalty: default_rerank_weight_path_penalty(),
            definition_bias: default_rerank_weight_definition_bias(),
            recency: default_rerank_weight_recency(),
            token_cost: default_rerank_weight_token_cost(),
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
    /// Hybrid fusion: `rrf` (default) or `legacy` weighted sum.
    #[serde(default = "default_hybrid_fusion")]
    pub hybrid_fusion: String,
    /// Fallback embedder when primary provider fails: `static` or `none`.
    #[serde(default = "default_embedding_fallback")]
    pub embedding_fallback: String,
    /// Enable multi-signal reranking in hybrid/capsule paths.
    #[serde(default = "default_true")]
    pub rerank_enabled: bool,
    /// Optional per-signal rerank weights (defaults match built-in `RerankWeights`).
    pub rerank_weights: Option<RerankWeightsConfig>,
}

fn default_hybrid_fusion() -> String {
    "rrf".to_string()
}

fn default_embedding_fallback() -> String {
    "static".to_string()
}

fn default_true() -> bool {
    true
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
            hybrid_fusion: default_hybrid_fusion(),
            embedding_fallback: default_embedding_fallback(),
            rerank_enabled: true,
            rerank_weights: None,
        }
    }
}

/// Indexing throughput profile (`highspeed` default, `conservative` for low-RAM laptops).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexingProfile {
    Highspeed,
    Conservative,
}

impl IndexingProfile {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "conservative" | "low" | "laptop" => Self::Conservative,
            _ => Self::Highspeed,
        }
    }
}

/// Resolved indexer tuning for a profile (see [`indexing_settings`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexingSettings {
    pub max_batch_size: usize,
    pub falkordb_unwind_batch_max: Option<usize>,
    pub graph_node_source_max_bytes: Option<usize>,
    pub indexer_parse_threads: Option<usize>,
    pub indexer_parse_pipeline_depth: usize,
    pub indexer_parse_batch_size: usize,
    pub falkordb_write_pool_size: usize,
}

/// Default files per Rayon parse batch for [`IndexingProfile::Conservative`].
pub const DEFAULT_INDEXER_PARSE_BATCH_SIZE: usize = 160;

/// Default FalkorDB write pool: half of available CPUs, clamped to [2, 8].
pub fn default_write_pool_size() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(2).min(8))
        .unwrap_or(2)
}

pub fn indexing_settings(profile: IndexingProfile) -> IndexingSettings {
    match profile {
        IndexingProfile::Highspeed => IndexingSettings {
            max_batch_size: 4096,
            falkordb_unwind_batch_max: Some(4096),
            graph_node_source_max_bytes: Some(64 * 1024),
            indexer_parse_threads: Some(0),
            indexer_parse_pipeline_depth: 1,
            indexer_parse_batch_size: 256,
            falkordb_write_pool_size: default_write_pool_size(),
        },
        IndexingProfile::Conservative => IndexingSettings {
            max_batch_size: 2048,
            falkordb_unwind_batch_max: Some(2048),
            graph_node_source_max_bytes: Some(256 * 1024),
            indexer_parse_threads: None,
            indexer_parse_pipeline_depth: 0,
            indexer_parse_batch_size: DEFAULT_INDEXER_PARSE_BATCH_SIZE,
            falkordb_write_pool_size: 2,
        },
    }
}

/// Active profile from `CORTEX_INDEX_PROFILE` (overrides TOML when set).
pub fn active_indexing_profile_from_env() -> Option<IndexingProfile> {
    match std::env::var("CORTEX_INDEX_PROFILE").ok().as_deref() {
        Some("conservative") => Some(IndexingProfile::Conservative),
        Some("highspeed") | Some("high") => Some(IndexingProfile::Highspeed),
        _ => None,
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

/// Main configuration for CodeCortex (TOML at [`CortexConfig::config_path`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CortexConfig {
    // FalkorDB settings
    pub falkordb_uri: String,
    pub falkordb_password: String,
    /// FalkorDB graph name.
    pub falkordb_graph: String,
    /// When false (default), FalkorDB bulk node upserts omit `source`, `docstring`, and the
    /// JSON `properties` blob from inlined UNWIND batches for faster indexing. Set true when
    /// MCP or graph queries need full source text on nodes.
    pub falkordb_bulk_index_include_source: bool,

    // Vector store settings
    pub vector: VectorConfig,

    // LLM/Embedding settings
    pub llm: LlmConfig,

    // Indexer settings (defaults tuned for ≥8 GiB RAM: graph batch throughput vs peak RSS)
    /// Primary chunk size for indexer `write_nodes` / `write_edges` (and spill replay).
    /// See [`Self::falkordb_unwind_batch_max`] and [`Self::falkordb_write_pool_size`].
    pub max_batch_size: usize,
    /// Cap UNWIND rows per FalkorDB write (None = use max_batch_size only).
    pub falkordb_unwind_batch_max: Option<usize>,
    /// Truncate graph node `source` text beyond this many UTF-8 bytes (None = keep full source).
    pub graph_node_source_max_bytes: Option<usize>,
    pub indexer_timeout_secs: u64,
    pub indexer_max_files: usize,
    /// Path for the indexer hash cache (`sled`). When unset, defaults to `~/.cortex/hashes.db`.
    pub hash_cache_path: Option<PathBuf>,
    /// When non-empty, file discovery uses only these paths (one entry per line in TOML list).
    pub index_include_files: Vec<PathBuf>,
    /// Extra exclude patterns (substring / `*.ext` / `dir/**`) merged with per-project excludes.
    pub index_exclude_patterns: Vec<String>,
    /// Optional global `.cortexignore` path (defaults to `~/.cortex/cortexignore` when unset).
    pub global_cortexignore_path: Option<PathBuf>,
    /// Rayon thread count for indexer file parsing. `None` uses one fewer than
    /// [`std::thread::available_parallelism`] (minimum 1). `Some(0)` uses the global Rayon pool.
    pub indexer_parse_threads: Option<usize>,
    /// When > 0, overlap parsing of the next file batch with graph writes for the current batch.
    pub indexer_parse_pipeline_depth: usize,
    /// Files per Rayon parse batch during indexing (default 160). Lower if RSS spikes; raise on
    /// large-RAM hosts when parse-bound.
    pub indexer_parse_batch_size: usize,
    /// FalkorDB writer pool size (1 = single connection).
    /// Values greater than 1 shard bulk node and bulk edge upserts across independent Redis
    /// connections: nodes by `id`, edges by source `from` id.
    ///
    /// **Indexer batching:** [`Self::max_batch_size`] is the primary chunk size for node/edge
    /// replay from spills. [`Self::falkordb_unwind_batch_max`], when set, caps each `UNWIND $batch`
    /// row count (`min(max_batch_size, cap)`).
    pub falkordb_write_pool_size: usize,
    /// `highspeed` (default) or `conservative` indexer tuning. Overridden by `CORTEX_INDEX_PROFILE`.
    #[serde(default = "default_indexing_profile_str")]
    pub indexing_profile: String,
    /// Concurrent daemon index workers across different repositories (same repo stays serialized).
    #[serde(default = "default_daemon_index_workers")]
    pub daemon_index_workers: usize,
    /// When true with `force` + git branch: delete branch graph **before** parse and write nodes inline
    /// (skips deferred replay). Timeout mid-run may leave an empty branch until re-index.
    #[serde(default)]
    pub index_force_delete_branch_before_parse: bool,
    /// Precompute incoming-caller reach depth during indexing (`0` = disabled, default `3`).
    pub index_reach_depth: usize,
    /// Cap stored caller ids per symbol in the reach index (default `64`).
    pub index_reach_max_ids: usize,
    /// Index-time MinHash+LSH clone detection and `SIMILAR_TO` edges (default false).
    #[serde(default)]
    pub clone_detection_enabled: bool,

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

    /// MCP server settings (`[mcp]` in config.toml).
    pub mcp: McpConfig,

    /// Agent-to-agent hybrid orchestration (`[a2a]` in config.toml).
    pub a2a: A2aConfig,
}

fn default_indexing_profile_str() -> String {
    "highspeed".to_string()
}

fn default_daemon_index_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1).min(4))
        .unwrap_or(1)
}

impl Default for CortexConfig {
    fn default() -> Self {
        let mut config = Self {
            falkordb_uri: "falkor://127.0.0.1:6379".to_string(),
            falkordb_password: String::new(),
            falkordb_graph: "codecortex".to_string(),
            falkordb_bulk_index_include_source: false,

            // Vector store defaults
            vector: VectorConfig::default(),

            // LLM defaults
            llm: LlmConfig::default(),

            // Indexer fields filled by [`Self::apply_indexing_profile`] below.
            max_batch_size: 4096,
            falkordb_unwind_batch_max: Some(4096),
            graph_node_source_max_bytes: Some(64 * 1024),
            indexer_timeout_secs: 7200, // 2h — large repos / slow disks
            indexer_max_files: 0,       // unlimited
            hash_cache_path: None,
            index_include_files: Vec::new(),
            index_exclude_patterns: Vec::new(),
            global_cortexignore_path: None,
            indexer_parse_threads: Some(0),
            indexer_parse_pipeline_depth: 1,
            indexer_parse_batch_size: 256,
            falkordb_write_pool_size: default_write_pool_size(),
            indexing_profile: default_indexing_profile_str(),
            daemon_index_workers: default_daemon_index_workers(),
            index_force_delete_branch_before_parse: false,
            index_reach_depth: 3,
            index_reach_max_ids: 64,
            clone_detection_enabled: false,

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
            mcp: McpConfig::default(),
            a2a: A2aConfig::default(),
        };
        let profile = active_indexing_profile_from_env()
            .unwrap_or_else(|| IndexingProfile::from_str_lossy(&config.indexing_profile));
        config.apply_indexing_profile(profile);
        config
    }
}

impl CortexConfig {
    /// Apply [`IndexingSettings`] for `profile` to indexer-related fields.
    pub fn apply_indexing_profile(&mut self, profile: IndexingProfile) {
        let s = indexing_settings(profile);
        self.max_batch_size = s.max_batch_size;
        self.falkordb_unwind_batch_max = s.falkordb_unwind_batch_max;
        self.graph_node_source_max_bytes = s.graph_node_source_max_bytes;
        self.indexer_parse_threads = s.indexer_parse_threads;
        self.indexer_parse_pipeline_depth = s.indexer_parse_pipeline_depth;
        self.indexer_parse_batch_size = s.indexer_parse_batch_size;
        self.falkordb_write_pool_size = s.falkordb_write_pool_size;
        self.indexing_profile = match profile {
            IndexingProfile::Highspeed => "highspeed".to_string(),
            IndexingProfile::Conservative => "conservative".to_string(),
        };
        // Highspeed: wipe branch before parse to avoid deferred node replay on force+branch.
        self.index_force_delete_branch_before_parse = profile == IndexingProfile::Highspeed;
    }

    pub fn resolved_indexing_profile(&self) -> IndexingProfile {
        if let Some(p) = active_indexing_profile_from_env() {
            return p;
        }
        IndexingProfile::from_str_lossy(&self.indexing_profile)
    }
    /// Get the configuration file path
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/config.toml")
    }

    /// Ensure the parent directory exists
    pub fn ensure_parent_dir() -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Load configuration from TOML at [`Self::config_path`], or defaults if missing.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        let mut config = if path.exists() {
            let raw = std::fs::read_to_string(&path)?;
            let value: toml::Value =
                toml::from_str(&raw).map_err(|e| CortexError::Config(e.to_string()))?;
            let mut legacy_keys = Vec::new();
            collect_legacy_config_keys(&value, &mut legacy_keys);
            if !legacy_keys.is_empty() {
                legacy_keys.sort();
                legacy_keys.dedup();
                return Err(legacy_config_error(&legacy_keys));
            }
            value
                .try_into()
                .map_err(|e: toml::de::Error| CortexError::Config(e.to_string()))?
        } else {
            Self::default()
        };

        let profile = config.resolved_indexing_profile();
        config.apply_indexing_profile(profile);

        Ok(config)
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
        validate_falkordb_uri(&self.falkordb_uri)?;
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
        if self.a2a.enabled && self.a2a.max_parallel_roles == 0 {
            return Err(CortexError::Config(
                "a2a.max_parallel_roles cannot be 0 when a2a.enabled is true".to_string(),
            ));
        }
        if self.a2a.enabled
            && self.mcp.network.allow_remote == false
            && self.mcp.network.listen.starts_with("0.0.0.0")
        {
            return Err(CortexError::Config(
                "mcp.network.listen binds remotely but allow_remote is false".to_string(),
            ));
        }
        Ok(())
    }

    /// Load bearer token for MCP/A2A HTTP from configured file path, if set.
    pub fn load_mcp_bearer_token(&self) -> Result<Option<String>> {
        let Some(path) = &self.mcp.network.bearer_token_file else {
            return Ok(None);
        };
        let expanded = expand_tilde(path);
        let raw = std::fs::read_to_string(&expanded).map_err(|e| {
            CortexError::Config(format!(
                "failed to read mcp.network.bearer_token_file {}: {e}",
                expanded.display()
            ))
        })?;
        let token = raw.trim().to_string();
        if token.is_empty() {
            Ok(None)
        } else {
            Ok(Some(token))
        }
    }
}

/// Validate FalkorDB connection URI scheme.
pub fn validate_falkordb_uri(uri: &str) -> Result<()> {
    let trimmed = uri.trim();
    if trimmed.starts_with("falkor://")
        || trimmed.starts_with("redis://")
        || trimmed.starts_with("rediss://")
    {
        Ok(())
    } else {
        Err(CortexError::Config(format!(
            "falkordb_uri must use falkor://, redis://, or rediss:// scheme; got {trimmed}"
        )))
    }
}

/// Rewrite legacy Memgraph/Grafeo config keys to FalkorDB names in place.
pub fn migrate_config_file(path: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let mut value: toml::Value =
        toml::from_str(&raw).map_err(|e| CortexError::Config(e.to_string()))?;
    migrate_toml_value(&mut value);
    let data = toml::to_string_pretty(&value).map_err(|e| CortexError::Config(e.to_string()))?;
    std::fs::write(path, data)?;
    Ok(())
}

fn is_legacy_config_key(key: &str) -> bool {
    LEGACY_CONFIG_RENAMES
        .iter()
        .any(|(legacy, _)| *legacy == key)
        || LEGACY_CONFIG_REMOVED.contains(&key)
}

fn collect_legacy_config_keys(value: &toml::Value, found: &mut Vec<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                if is_legacy_config_key(key) {
                    found.push(key.clone());
                }
                collect_legacy_config_keys(val, found);
            }
        }
        toml::Value::Array(arr) => {
            for val in arr {
                collect_legacy_config_keys(val, found);
            }
        }
        _ => {}
    }
}

fn legacy_key_migration_hint(key: &str) -> &'static str {
    match key {
        "memgraph_uri" => "memgraph_uri -> falkordb_uri",
        "memgraph_password" => "memgraph_password -> falkordb_password",
        "memgraph_unwind_batch_max" => "memgraph_unwind_batch_max -> falkordb_unwind_batch_max",
        "memgraph_write_pool_size" => "memgraph_write_pool_size -> falkordb_write_pool_size",
        "memgraph_user" => "memgraph_user -> (removed; FalkorDB uses falkordb_password only)",
        "backend_type" => "backend_type -> (removed; FalkorDB only)",
        "grafeo_path" => "grafeo_path -> (removed)",
        _ => "unknown legacy key",
    }
}

fn legacy_config_error(keys: &[String]) -> CortexError {
    let mut msg =
        String::from("Legacy configuration keys found (FalkorDB-only config in this version):\n");
    for key in keys {
        msg.push_str("  ");
        msg.push_str(legacy_key_migration_hint(key));
        msg.push('\n');
    }
    msg.push_str("\nRun `cortex config migrate` to rewrite config in place.");
    CortexError::Config(msg)
}

fn migrate_toml_value(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for removed in LEGACY_CONFIG_REMOVED {
                table.remove(*removed);
            }
            for (legacy, new_key) in LEGACY_CONFIG_RENAMES {
                if let Some(v) = table.remove(*legacy) {
                    table.insert(new_key.to_string(), v);
                }
            }
            let keys: Vec<String> = table.keys().cloned().collect();
            for key in keys {
                if let Some(val) = table.get_mut(&key) {
                    migrate_toml_value(val);
                }
            }
        }
        toml::Value::Array(arr) => {
            for val in arr {
                migrate_toml_value(val);
            }
        }
        _ => {}
    }
}

fn expand_tilde(path: &PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(rest)
    } else if s == "~" {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
    } else {
        path.clone()
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
        let hs = indexing_settings(IndexingProfile::Highspeed);
        assert_eq!(config.falkordb_uri, "falkor://127.0.0.1:6379");
        assert_eq!(config.max_batch_size, hs.max_batch_size);
        assert_eq!(
            config.falkordb_unwind_batch_max,
            hs.falkordb_unwind_batch_max
        );
        assert_eq!(
            config.graph_node_source_max_bytes,
            hs.graph_node_source_max_bytes
        );
        assert_eq!(config.indexer_timeout_secs, 7200);
        assert_eq!(config.falkordb_write_pool_size, hs.falkordb_write_pool_size);
        assert!(!config.falkordb_bulk_index_include_source);
        assert_eq!(
            config.indexer_parse_pipeline_depth,
            hs.indexer_parse_pipeline_depth
        );
        assert_eq!(config.indexer_parse_batch_size, hs.indexer_parse_batch_size);
        assert_eq!(config.indexer_parse_threads, hs.indexer_parse_threads);
        assert_eq!(config.indexing_profile, "highspeed");
        assert!(config.daemon_index_workers >= 1);
        assert_eq!(config.index_reach_depth, 3);
        assert_eq!(config.index_reach_max_ids, 64);
        assert_eq!(config.pool_max_connections, 10);
    }

    #[test]
    fn conservative_profile_overrides_indexer_fields() {
        let mut config = CortexConfig::default();
        config.apply_indexing_profile(IndexingProfile::Conservative);
        let c = indexing_settings(IndexingProfile::Conservative);
        assert_eq!(config.max_batch_size, c.max_batch_size);
        assert_eq!(config.indexer_parse_pipeline_depth, 0);
        assert_eq!(config.indexer_parse_batch_size, 160);
        assert_eq!(config.falkordb_write_pool_size, 2);
    }

    #[test]
    fn rerank_weights_toml_parses_and_applies_defaults() {
        let toml_str = r#"
[vector]
rerank_enabled = true

[vector.rerank_weights]
lexical = 2.0
"#;
        let config: CortexConfig = toml::from_str(toml_str).unwrap();
        let weights = config
            .vector
            .rerank_weights
            .as_ref()
            .expect("rerank_weights table");
        assert_eq!(weights.lexical, 2.0);
        assert_eq!(weights.vector, default_rerank_weight_vector());
        assert_eq!(weights.centrality, default_rerank_weight_centrality());
        assert_eq!(weights.path_penalty, default_rerank_weight_path_penalty());
        assert_eq!(weights.definition_bias, default_rerank_weight_definition_bias());
        assert_eq!(weights.recency, default_rerank_weight_recency());
        assert_eq!(weights.token_cost, default_rerank_weight_token_cost());
    }

    #[test]
    fn config_roundtrip() {
        let original = CortexConfig {
            falkordb_uri: "falkor://test:6379".to_string(),
            falkordb_password: "pwd".to_string(),
            falkordb_graph: "codecortex".to_string(),
            falkordb_bulk_index_include_source: false,
            vector: VectorConfig::default(),
            llm: LlmConfig::default(),
            max_batch_size: 100,
            falkordb_unwind_batch_max: None,
            graph_node_source_max_bytes: None,
            indexer_timeout_secs: 60,
            indexer_max_files: 1000,
            hash_cache_path: None,
            index_include_files: vec![PathBuf::from("src/main.rs")],
            index_exclude_patterns: vec!["target/**".to_string()],
            global_cortexignore_path: Some(PathBuf::from("/home/test/.cortex/cortexignore")),
            indexer_parse_threads: Some(4),
            indexer_parse_pipeline_depth: 1,
            indexer_parse_batch_size: 200,
            falkordb_write_pool_size: 2,
            indexing_profile: "highspeed".to_string(),
            daemon_index_workers: 1,
            index_force_delete_branch_before_parse: false,
            index_reach_depth: 3,
            index_reach_max_ids: 64,
            clone_detection_enabled: false,
            analyzer_query_limit: 500,
            analyzer_cache_ttl_secs: 120,
            watcher_debounce_secs: 5,
            watcher_max_events: 64,
            pool_max_connections: 5,
            pool_min_idle: 1,
            pool_connection_timeout_secs: 15,
            watched_paths: vec![PathBuf::from("/repo1"), PathBuf::from("/repo2")],
            mcp: McpConfig::default(),
            a2a: A2aConfig::default(),
        };

        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: CortexConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.falkordb_uri, original.falkordb_uri);
        assert_eq!(parsed.max_batch_size, original.max_batch_size);
        assert_eq!(parsed.indexer_timeout_secs, original.indexer_timeout_secs);
        assert_eq!(parsed.index_include_files, original.index_include_files);
        assert_eq!(
            parsed.index_exclude_patterns,
            original.index_exclude_patterns
        );
        assert_eq!(
            parsed.global_cortexignore_path,
            original.global_cortexignore_path
        );
        assert_eq!(parsed.hash_cache_path, original.hash_cache_path);
        assert_eq!(parsed.indexer_parse_threads, original.indexer_parse_threads);
        assert_eq!(
            parsed.indexer_parse_pipeline_depth,
            original.indexer_parse_pipeline_depth
        );
        assert_eq!(
            parsed.falkordb_write_pool_size,
            original.falkordb_write_pool_size
        );
    }

    #[test]
    fn load_rejects_legacy_keys() {
        let dir = std::env::temp_dir().join(format!(
            "cortex-legacy-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "memgraph_uri = \"falkor://127.0.0.1:6379\"\nbackend_type = \"falkordb\"\n",
        )
        .unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let value: toml::Value = toml::from_str(&raw).unwrap();
        let mut legacy = Vec::new();
        collect_legacy_config_keys(&value, &mut legacy);
        assert!(legacy.contains(&"memgraph_uri".to_string()));
        assert!(legacy.contains(&"backend_type".to_string()));

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn migrate_config_file_rewrites_keys() {
        let dir = std::env::temp_dir().join(format!(
            "cortex-migrate-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            r#"memgraph_uri = "falkor://127.0.0.1:6379"
memgraph_password = "secret"
memgraph_user = "admin"
backend_type = "falkordb"
grafeo_path = "/tmp/graph.db"
memgraph_write_pool_size = 4
"#,
        )
        .unwrap();

        migrate_config_file(&path).unwrap();
        let migrated: CortexConfig = toml::from_str(&std::fs::read_to_string(&path).unwrap())
            .expect("migrated config should deserialize");
        assert_eq!(migrated.falkordb_uri, "falkor://127.0.0.1:6379");
        assert_eq!(migrated.falkordb_password, "secret");
        assert_eq!(migrated.falkordb_write_pool_size, 4);

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("memgraph_uri"));
        assert!(!raw.contains("backend_type"));
        assert!(!raw.contains("grafeo_path"));
        assert!(!raw.contains("memgraph_user"));

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn validate_falkordb_uri_accepts_supported_schemes() {
        assert!(validate_falkordb_uri("falkor://127.0.0.1:6379").is_ok());
        assert!(validate_falkordb_uri("redis://127.0.0.1:6379").is_ok());
        assert!(validate_falkordb_uri("rediss://127.0.0.1:6379").is_ok());
        assert!(validate_falkordb_uri("bolt://127.0.0.1:7687").is_err());
    }

    #[test]
    fn config_path_uses_home() {
        let path = CortexConfig::config_path();
        assert!(path.to_string_lossy().contains(".cortex"));
    }

    #[test]
    fn a2a_config_defaults_disabled() {
        let config = CortexConfig::default();
        assert!(!config.a2a.enabled);
        assert!(!config.a2a.force_in_process);
        assert!(!config.mcp.tools.a2a_spawn_session);
        assert!(config.a2a.validate.command.is_empty());
        assert!(config.a2a.validate.working_directory.is_none());
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
