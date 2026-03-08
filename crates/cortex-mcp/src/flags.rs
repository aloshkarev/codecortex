//! Feature Flag Registry for CodeCortex MCP Tools
//!
//! Centralized management of feature flags that control tool availability
//! and behavior. Flags are configured via environment variables following
//! the pattern: `CORTEX_FLAG_MCP_<FLAG_NAME>_ENABLED`

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::OnceLock;

/// Represents a single feature flag with its current state
#[derive(Debug, Clone, Copy)]
pub struct Flag {
    /// The environment variable key for this flag
    env_key: &'static str,
    /// Default value when environment variable is not set
    default: bool,
    /// Cached value (populated on first access)
    cached: bool,
}

impl Flag {
    /// Create a new flag with the given environment key and default value
    pub const fn new(env_key: &'static str, default: bool) -> Self {
        Self {
            env_key,
            default,
            cached: false,
        }
    }

    /// Check if this flag is enabled, reading from environment if not cached
    pub fn is_enabled(&self) -> bool {
        // Note: For runtime caching, we use OnceLock in FeatureFlags
        read_flag_from_env(self.env_key, self.default)
    }
}

/// Read a flag value from environment, with fallback to default
fn read_flag_from_env(env_key: &str, default: bool) -> bool {
    match std::env::var(env_key) {
        Ok(v) => matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        Err(_) => default,
    }
}

/// Registry of all feature flags for MCP tools
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    /// Enable context capsule tool (hybrid retrieval)
    pub context_capsule: bool,
    /// Enable impact graph tool (blast radius analysis)
    pub impact_graph: bool,
    /// Enable logic flow search tool
    pub logic_flow: bool,
    /// Enable index status tool
    pub index_status: bool,
    /// Enable skeleton extraction tool
    pub skeleton: bool,
    /// Enable workspace setup tool
    pub workspace_setup: bool,
    /// Enable LSP edge ingestion
    pub lsp_ingest: bool,
    /// Enable memory read operations
    pub memory_read: bool,
    /// Enable memory write operations
    pub memory_write: bool,
    /// Enable vector read operations
    pub vector_read: bool,
    /// Enable vector write operations
    pub vector_write: bool,
    /// Enable cache layer
    pub cache_enabled: bool,
    /// Enable telemetry collection
    pub telemetry_enabled: bool,
    /// Enable TF-IDF scoring in retrieval
    pub tfidf_scoring: bool,
    /// Enable graph centrality scoring
    pub centrality_scoring: bool,
}

static FEATURE_FLAGS: OnceLock<FeatureFlags> = OnceLock::new();

impl FeatureFlags {
    /// Environment variable prefix for all MCP flags
    const ENV_PREFIX: &'static str = "CORTEX_FLAG_MCP";

    /// Create a flag key from a flag name
    fn make_env_key(name: &str) -> String {
        let normalized = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();
        format!("{}_{}_ENABLED", Self::ENV_PREFIX, normalized)
    }

    /// Load feature flags from environment variables
    pub fn from_env() -> Self {
        Self {
            context_capsule: read_flag_from_env(&Self::make_env_key("context_capsule"), false),
            impact_graph: read_flag_from_env(&Self::make_env_key("impact_graph"), false),
            logic_flow: read_flag_from_env(&Self::make_env_key("logic_flow"), false),
            index_status: read_flag_from_env(&Self::make_env_key("index_status"), false),
            skeleton: read_flag_from_env(&Self::make_env_key("skeleton"), false),
            workspace_setup: read_flag_from_env(&Self::make_env_key("workspace_setup"), false),
            lsp_ingest: read_flag_from_env(&Self::make_env_key("lsp_ingest"), false),
            memory_read: read_flag_from_env(&Self::make_env_key("memory_read"), false),
            memory_write: read_flag_from_env(&Self::make_env_key("memory_write"), false),
            vector_read: read_flag_from_env(&Self::make_env_key("vector_read"), true),
            vector_write: read_flag_from_env(&Self::make_env_key("vector_write"), true),
            cache_enabled: read_flag_from_env(&Self::make_env_key("cache"), true),
            telemetry_enabled: read_flag_from_env(&Self::make_env_key("telemetry"), true),
            tfidf_scoring: read_flag_from_env(&Self::make_env_key("tfidf_scoring"), true),
            centrality_scoring: read_flag_from_env(&Self::make_env_key("centrality_scoring"), true),
        }
    }

    /// Get the global feature flags instance (lazy-initialized)
    pub fn global() -> &'static Self {
        FEATURE_FLAGS.get_or_init(Self::from_env)
    }

    /// Check if a specific flag is enabled by name
    pub fn is_enabled(&self, flag_name: &str) -> bool {
        match flag_name {
            "context_capsule" | "mcp.context_capsule.enabled" => self.context_capsule,
            "impact_graph" | "mcp.impact_graph.enabled" => self.impact_graph,
            "logic_flow" | "mcp.logic_flow.enabled" => self.logic_flow,
            "index_status" | "mcp.index_status.enabled" => self.index_status,
            "skeleton" | "mcp.skeleton.enabled" => self.skeleton,
            "workspace_setup" | "mcp.workspace_setup.enabled" => self.workspace_setup,
            "lsp_ingest" | "mcp.lsp_ingest.enabled" => self.lsp_ingest,
            "memory_read" | "mcp.memory.read.enabled" => self.memory_read,
            "memory_write" | "mcp.memory.write.enabled" => self.memory_write,
            "vector_read" | "mcp.vector.read.enabled" => self.vector_read,
            "vector_write" | "mcp.vector.write.enabled" => self.vector_write,
            "cache" | "mcp.cache.enabled" => self.cache_enabled,
            "telemetry" | "mcp.telemetry.enabled" => self.telemetry_enabled,
            "tfidf_scoring" => self.tfidf_scoring,
            "centrality_scoring" => self.centrality_scoring,
            _ => {
                tracing::warn!("Unknown feature flag requested: {}", flag_name);
                false
            }
        }
    }

    /// Get all flag names and their current values
    pub fn all_flags(&self) -> HashMap<&'static str, bool> {
        let mut flags = HashMap::new();
        flags.insert("context_capsule", self.context_capsule);
        flags.insert("impact_graph", self.impact_graph);
        flags.insert("logic_flow", self.logic_flow);
        flags.insert("index_status", self.index_status);
        flags.insert("skeleton", self.skeleton);
        flags.insert("workspace_setup", self.workspace_setup);
        flags.insert("lsp_ingest", self.lsp_ingest);
        flags.insert("memory_read", self.memory_read);
        flags.insert("memory_write", self.memory_write);
        flags.insert("vector_read", self.vector_read);
        flags.insert("vector_write", self.vector_write);
        flags.insert("cache_enabled", self.cache_enabled);
        flags.insert("telemetry_enabled", self.telemetry_enabled);
        flags.insert("tfidf_scoring", self.tfidf_scoring);
        flags.insert("centrality_scoring", self.centrality_scoring);
        flags
    }

    /// Create a new instance with all flags enabled (for testing)
    pub fn all_enabled() -> Self {
        Self {
            context_capsule: true,
            impact_graph: true,
            logic_flow: true,
            index_status: true,
            skeleton: true,
            workspace_setup: true,
            lsp_ingest: true,
            memory_read: true,
            memory_write: true,
            vector_read: true,
            vector_write: true,
            cache_enabled: true,
            telemetry_enabled: true,
            tfidf_scoring: true,
            centrality_scoring: true,
        }
    }

    /// Create a new instance with all flags disabled (for testing)
    pub fn all_disabled() -> Self {
        Self {
            context_capsule: false,
            impact_graph: false,
            logic_flow: false,
            index_status: false,
            skeleton: false,
            workspace_setup: false,
            lsp_ingest: false,
            memory_read: false,
            memory_write: false,
            vector_read: false,
            vector_write: false,
            cache_enabled: false,
            telemetry_enabled: false,
            tfidf_scoring: false,
            centrality_scoring: false,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_flags_defaults_from_env() {
        let flags = FeatureFlags::from_env();
        // Without env vars set, most flags should be disabled by default
        assert!(!flags.context_capsule);
        assert!(!flags.impact_graph);
        // Cache and telemetry should be enabled by default
        assert!(flags.cache_enabled);
        assert!(flags.telemetry_enabled);
    }

    #[test]
    fn feature_flags_is_enabled_by_name() {
        let flags = FeatureFlags::all_enabled();
        assert!(flags.is_enabled("context_capsule"));
        assert!(flags.is_enabled("mcp.context_capsule.enabled"));
        assert!(flags.is_enabled("impact_graph"));
        assert!(flags.is_enabled("mcp.impact_graph.enabled"));
    }

    #[test]
    fn feature_flags_unknown_flag_returns_false() {
        let flags = FeatureFlags::all_enabled();
        assert!(!flags.is_enabled("unknown_flag"));
    }

    #[test]
    fn feature_flags_all_enabled() {
        let flags = FeatureFlags::all_enabled();
        assert!(flags.context_capsule);
        assert!(flags.impact_graph);
        assert!(flags.logic_flow);
        assert!(flags.index_status);
        assert!(flags.skeleton);
        assert!(flags.workspace_setup);
        assert!(flags.lsp_ingest);
        assert!(flags.memory_read);
        assert!(flags.memory_write);
        assert!(flags.vector_read);
        assert!(flags.vector_write);
        assert!(flags.cache_enabled);
        assert!(flags.telemetry_enabled);
    }

    #[test]
    fn feature_flags_all_disabled() {
        let flags = FeatureFlags::all_disabled();
        assert!(!flags.context_capsule);
        assert!(!flags.impact_graph);
        assert!(!flags.logic_flow);
        assert!(!flags.index_status);
        assert!(!flags.skeleton);
        assert!(!flags.workspace_setup);
        assert!(!flags.lsp_ingest);
        assert!(!flags.memory_read);
        assert!(!flags.memory_write);
        assert!(!flags.vector_read);
        assert!(!flags.vector_write);
        assert!(!flags.cache_enabled);
        assert!(!flags.telemetry_enabled);
    }

    #[test]
    fn feature_flags_all_flags_returns_map() {
        let flags = FeatureFlags::all_enabled();
        let all = flags.all_flags();
        assert!(all.contains_key("context_capsule"));
        assert!(all.contains_key("impact_graph"));
        assert_eq!(all.len(), 15);
    }

    #[test]
    fn make_env_key_normalizes_name() {
        assert_eq!(
            FeatureFlags::make_env_key("context_capsule"),
            "CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED"
        );
        assert_eq!(
            FeatureFlags::make_env_key("memory.read"),
            "CORTEX_FLAG_MCP_MEMORY_READ_ENABLED"
        );
    }
}
