//! Feature Flag Registry for CodeCortex MCP Tools
//!
//! Primary source: `[mcp.tools]` in `~/.cortex/config.toml` via [`FeatureFlags::from_config`].
//! Legacy `CORTEX_FLAG_MCP_*_ENABLED` environment variables are still honored when config
//! does not override a tool (env is read as fallback in [`from_config`]).

#![allow(dead_code)]

use cortex_core::CortexConfig;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable `cortex_a2a_spawn_session` meta-tool (also requires `[a2a].enabled`).
    pub a2a_spawn_session: bool,
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
    /// Enable manage_codecortex orchestration tool
    pub manage_codecortex: bool,
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

    /// Default when `CORTEX_FLAG_MCP_<NAME>_ENABLED` is unset (opt-out via `=0`).
    const DEFAULT_TOOL_ENABLED: bool = true;

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

    fn env_tool_flag(name: &str) -> bool {
        read_flag_from_env(&Self::make_env_key(name), Self::DEFAULT_TOOL_ENABLED)
    }

    /// Load from TOML config; env vars fill gaps for legacy deployments.
    pub fn from_config(config: &CortexConfig) -> Self {
        let t = &config.mcp.tools;
        let mut flags = Self {
            a2a_spawn_session: t.a2a_spawn_session,
            context_capsule: merge_tool_flag(t.context_capsule, "context_capsule"),
            impact_graph: merge_tool_flag(t.impact_graph, "impact_graph"),
            logic_flow: merge_tool_flag(t.logic_flow, "logic_flow"),
            index_status: merge_tool_flag(t.index_status, "index_status"),
            skeleton: merge_tool_flag(t.skeleton, "skeleton"),
            workspace_setup: merge_tool_flag(t.workspace_setup, "workspace_setup"),
            manage_codecortex: merge_tool_flag(t.manage_codecortex, "manage_codecortex"),
            lsp_ingest: merge_tool_flag(t.lsp_ingest, "lsp_ingest"),
            memory_read: merge_tool_flag(t.memory_read, "memory_read"),
            memory_write: merge_tool_flag(t.memory_write, "memory_write"),
            vector_read: merge_tool_flag(t.vector_read, "vector_read"),
            vector_write: merge_tool_flag(t.vector_write, "vector_write"),
            cache_enabled: merge_tool_flag(t.cache_enabled, "cache"),
            telemetry_enabled: merge_tool_flag(t.telemetry_enabled, "telemetry"),
            tfidf_scoring: merge_tool_flag(t.tfidf_scoring, "tfidf_scoring"),
            centrality_scoring: merge_tool_flag(t.centrality_scoring, "centrality_scoring"),
        };
        crate::mcp_profile::McpProfile::from_config_kind(config.mcp.resolved_profile())
            .apply_to_flags(&mut flags);
        flags
    }

    /// Load feature flags from environment variables (legacy).
    pub fn from_env() -> Self {
        let mut flags = Self {
            a2a_spawn_session: Self::env_tool_flag("a2a_spawn_session"),
            context_capsule: Self::env_tool_flag("context_capsule"),
            impact_graph: Self::env_tool_flag("impact_graph"),
            logic_flow: Self::env_tool_flag("logic_flow"),
            index_status: Self::env_tool_flag("index_status"),
            skeleton: Self::env_tool_flag("skeleton"),
            workspace_setup: Self::env_tool_flag("workspace_setup"),
            manage_codecortex: Self::env_tool_flag("manage_codecortex"),
            lsp_ingest: Self::env_tool_flag("lsp_ingest"),
            memory_read: Self::env_tool_flag("memory_read"),
            memory_write: Self::env_tool_flag("memory_write"),
            vector_read: Self::env_tool_flag("vector_read"),
            vector_write: Self::env_tool_flag("vector_write"),
            cache_enabled: Self::env_tool_flag("cache"),
            telemetry_enabled: Self::env_tool_flag("telemetry"),
            tfidf_scoring: Self::env_tool_flag("tfidf_scoring"),
            centrality_scoring: Self::env_tool_flag("centrality_scoring"),
        };
        crate::mcp_profile::McpProfile::from_env().apply_to_flags(&mut flags);
        flags
    }

    /// Create from environment variables, then enable named CLI overrides.
    ///
    /// Names accept either `snake_case` or `kebab-case`. The group override
    /// `memory` enables both memory read and memory write tools.
    pub fn from_config_with_overrides(config: &CortexConfig, enabled: &[String]) -> Self {
        let mut flags = Self::from_config(config);
        Self::apply_cli_enable_overrides(&mut flags, enabled);
        crate::mcp_profile::McpProfile::from_config_kind(config.mcp.resolved_profile())
            .apply_to_flags(&mut flags);
        flags
    }

    pub fn from_env_with_overrides(enabled: &[String]) -> Self {
        let mut flags = Self::from_env();
        Self::apply_cli_enable_overrides(&mut flags, enabled);
        crate::mcp_profile::McpProfile::from_env().apply_to_flags(&mut flags);
        flags
    }

    fn apply_cli_enable_overrides(flags: &mut Self, enabled: &[String]) {
        for name in enabled {
            match name.replace('-', "_").to_ascii_lowercase().as_str() {
                "a2a_spawn_session" => flags.a2a_spawn_session = true,
                "context_capsule" => flags.context_capsule = true,
                "impact_graph" => flags.impact_graph = true,
                "logic_flow" => flags.logic_flow = true,
                "index_status" => flags.index_status = true,
                "skeleton" => flags.skeleton = true,
                "workspace_setup" => flags.workspace_setup = true,
                "manage_codecortex" => flags.manage_codecortex = true,
                "lsp_ingest" => flags.lsp_ingest = true,
                "memory" => {
                    flags.memory_read = true;
                    flags.memory_write = true;
                }
                "memory_read" => flags.memory_read = true,
                "memory_write" => flags.memory_write = true,
                "vector_read" => flags.vector_read = true,
                "vector_write" => flags.vector_write = true,
                other => tracing::warn!("Unknown MCP --enable flag ignored: {other}"),
            }
        }
    }

    /// Get the global feature flags instance (lazy-initialized)
    pub fn global() -> &'static Self {
        FEATURE_FLAGS.get_or_init(Self::from_env)
    }

    /// Check if a specific flag is enabled by name
    pub fn is_enabled(&self, flag_name: &str) -> bool {
        self.is_enabled_or(flag_name, false)
    }

    /// Check a flag, returning `default` for unknown flag names.
    pub fn is_enabled_or(&self, flag_name: &str, default: bool) -> bool {
        match flag_name {
            "a2a_spawn_session" | "mcp.a2a_spawn_session.enabled" => self.a2a_spawn_session,
            "context_capsule" | "mcp.context_capsule.enabled" => self.context_capsule,
            "impact_graph" | "mcp.impact_graph.enabled" => self.impact_graph,
            "logic_flow" | "mcp.logic_flow.enabled" => self.logic_flow,
            "index_status" | "mcp.index_status.enabled" => self.index_status,
            "skeleton" | "mcp.skeleton.enabled" => self.skeleton,
            "workspace_setup" | "mcp.workspace_setup.enabled" => self.workspace_setup,
            "manage_codecortex" | "mcp.manage_codecortex.enabled" => self.manage_codecortex,
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
                default
            }
        }
    }

    /// Get all flag names and their current values
    pub fn all_flags(&self) -> HashMap<&'static str, bool> {
        let mut flags = HashMap::new();
        flags.insert("a2a_spawn_session", self.a2a_spawn_session);
        flags.insert("context_capsule", self.context_capsule);
        flags.insert("impact_graph", self.impact_graph);
        flags.insert("logic_flow", self.logic_flow);
        flags.insert("index_status", self.index_status);
        flags.insert("skeleton", self.skeleton);
        flags.insert("workspace_setup", self.workspace_setup);
        flags.insert("manage_codecortex", self.manage_codecortex);
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
            a2a_spawn_session: true,
            context_capsule: true,
            impact_graph: true,
            logic_flow: true,
            index_status: true,
            skeleton: true,
            workspace_setup: true,
            manage_codecortex: true,
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
            a2a_spawn_session: false,
            context_capsule: false,
            impact_graph: false,
            logic_flow: false,
            index_status: false,
            skeleton: false,
            workspace_setup: false,
            manage_codecortex: false,
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

fn merge_tool_flag(config_value: bool, env_name: &str) -> bool {
    if std::env::var(FeatureFlags::make_env_key(env_name)).is_ok() {
        FeatureFlags::env_tool_flag(env_name)
    } else {
        config_value
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
        // Without env vars set, MCP tools are enabled by default (opt-out via =0)
        assert!(flags.context_capsule);
        assert!(flags.impact_graph);
        assert!(flags.manage_codecortex);
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
        assert_eq!(all.len(), 17);
    }

    #[test]
    fn from_config_respects_a2a_spawn_session() {
        let mut config = CortexConfig::default();
        config.mcp.tools.a2a_spawn_session = true;
        let flags = FeatureFlags::from_config(&config);
        assert!(flags.a2a_spawn_session);
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

    #[test]
    fn strict_mcp_profile_tightens_flags() {
        let mut flags = FeatureFlags::all_enabled();
        crate::mcp_profile::McpProfile::Strict.apply_to_flags(&mut flags);
        assert!(!flags.vector_write);
        assert!(!flags.memory_write);
        assert!(!flags.memory_read);
        assert!(!flags.context_capsule);
        assert!(flags.vector_read);
    }
}
