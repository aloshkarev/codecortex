//! MCP server configuration loaded from `~/.cortex/config.toml`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Deployment profile for MCP defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpProfileKind {
    Dev,
    Strict,
}

impl Default for McpProfileKind {
    fn default() -> Self {
        Self::Dev
    }
}

impl McpProfileKind {
    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "strict" | "enterprise" | "corp" => Self::Strict,
            _ => Self::Dev,
        }
    }
}

/// Network transport settings for `cortex mcp start`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpNetworkConfig {
    pub transport: String,
    pub listen: String,
    pub allow_remote: bool,
    pub max_clients: usize,
    pub idle_timeout_secs: u64,
    pub bearer_token_file: Option<PathBuf>,
}

impl Default for McpNetworkConfig {
    fn default() -> Self {
        Self {
            transport: "stdio".to_string(),
            listen: "127.0.0.1:3001".to_string(),
            allow_remote: false,
            max_clients: 8,
            idle_timeout_secs: 300,
            bearer_token_file: None,
        }
    }
}

/// Per-tool enablement under `[mcp.tools]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpToolsConfig {
    pub a2a_spawn_session: bool,
    pub context_capsule: bool,
    pub impact_graph: bool,
    pub logic_flow: bool,
    pub index_status: bool,
    pub skeleton: bool,
    pub workspace_setup: bool,
    pub manage_codecortex: bool,
    pub lsp_ingest: bool,
    pub memory_read: bool,
    pub memory_write: bool,
    pub vector_read: bool,
    pub vector_write: bool,
    pub cache_enabled: bool,
    pub telemetry_enabled: bool,
    pub tfidf_scoring: bool,
    pub centrality_scoring: bool,
}

impl Default for McpToolsConfig {
    fn default() -> Self {
        Self {
            a2a_spawn_session: false,
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
}

/// `[mcp]` section in config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    pub profile: String,
    pub network: McpNetworkConfig,
    pub tools: McpToolsConfig,
    /// When true, bounded MCP tools record token savings to `~/.cortex/savings.json`.
    pub savings_enabled: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            profile: "dev".to_string(),
            network: McpNetworkConfig::default(),
            tools: McpToolsConfig::default(),
            savings_enabled: true,
        }
    }
}

impl McpConfig {
    pub fn resolved_profile(&self) -> McpProfileKind {
        McpProfileKind::from_str_lossy(&self.profile)
    }
}
