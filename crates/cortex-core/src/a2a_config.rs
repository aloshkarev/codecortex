//! Agent-to-agent (A2A) configuration loaded from `~/.cortex/config.toml`.
//!
//! A2A settings are **not** read from environment variables.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// How a role is executed in the hybrid A2A topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum A2aRoleMode {
    InProcess,
    External,
    Disabled,
}

impl Default for A2aRoleMode {
    fn default() -> Self {
        Self::InProcess
    }
}

/// Per-role A2A endpoint, execution mode, and executable manifest fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aRoleConfig {
    pub mode: A2aRoleMode,
    pub agent_card_url: Option<String>,
    /// Incoming payload types this role handles (e.g. `TaskDelegation`).
    pub subscriptions: Vec<String>,
    /// Outgoing payload types this role may emit.
    pub capabilities: Vec<String>,
    /// Tool/skill ids advertised on the agent card.
    pub skills: Vec<String>,
    /// MCP tool ids this role may delegate to the host agent.
    pub mcp_tools: Vec<String>,
    /// Max seconds to poll an external task for replies in `dispatch_sync`.
    pub reply_timeout_secs: u64,
}

impl Default for A2aRoleConfig {
    fn default() -> Self {
        Self {
            mode: A2aRoleMode::InProcess,
            agent_card_url: None,
            subscriptions: Vec::new(),
            capabilities: Vec::new(),
            skills: Vec::new(),
            mcp_tools: Vec::new(),
            reply_timeout_secs: 30,
        }
    }
}

/// HTTP binding for the A2A server surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aServerConfig {
    pub http_enabled: bool,
    pub base_path: String,
    pub protocol_version: String,
    pub agent_card_path: String,
    pub extension_uri: String,
    pub grpc_enabled: bool,
    pub grpc_listen: String,
}

impl Default for A2aServerConfig {
    fn default() -> Self {
        Self {
            http_enabled: false,
            base_path: "/a2a/v1".to_string(),
            protocol_version: "1.0".to_string(),
            agent_card_path: "/.well-known/agent-card.json".to_string(),
            extension_uri: "https://codecortex.dev/extensions/blackboard/v1".to_string(),
            grpc_enabled: false,
            grpc_listen: "127.0.0.1:50051".to_string(),
        }
    }
}

/// Push notification webhook delivery (spec §3.5); off by default.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aPushRetryConfig {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

impl Default for A2aPushRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aPushConfig {
    pub enabled: bool,
    pub signing_secret_path: PathBuf,
    pub default_callback_timeout_secs: u64,
    pub retry: A2aPushRetryConfig,
}

impl Default for A2aPushConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            enabled: false,
            signing_secret_path: PathBuf::from(home).join(".cortex/a2a/push.secret"),
            default_callback_timeout_secs: 30,
            retry: A2aPushRetryConfig::default(),
        }
    }
}

/// Host-side guards when A2A is enabled (avoid dumping raw graph rows into MCP context).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aHostGuardConfig {
    pub max_cypher_rows: usize,
}

impl Default for A2aHostGuardConfig {
    fn default() -> Self {
        Self {
            max_cypher_rows: 50,
        }
    }
}

/// Graph blackboard settings for cross-agent insight sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aBlackboardConfig {
    pub enabled: bool,
    pub write_batch_size: usize,
    pub max_insights_per_session: usize,
}

impl Default for A2aBlackboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            write_batch_size: 4096,
            max_insights_per_session: 10_000,
        }
    }
}

/// Built-in workflow template configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aConsensusReviewConfig {
    pub enabled: bool,
    pub roles: Vec<String>,
    pub default_budget_tokens: u32,
    /// When true, patch planner uses transport deadlock demo strategies.
    pub demo_fixture: bool,
}

impl Default for A2aConsensusReviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            roles: vec![
                "patch_planner".to_string(),
                "analyzer".to_string(),
                "validator".to_string(),
            ],
            default_budget_tokens: 6000,
            demo_fixture: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aPatchPlanConfig {
    pub enabled: bool,
    pub default_budget_tokens: u32,
}

impl Default for A2aPatchPlanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_budget_tokens: 6000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aImpactReviewConfig {
    pub enabled: bool,
    pub default_budget_tokens: u32,
}

impl Default for A2aImpactReviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_budget_tokens: 4000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aPrReviewConfig {
    pub enabled: bool,
    pub default_budget_tokens: u32,
}

impl Default for A2aPrReviewConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_budget_tokens: 6000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aWorkflowsConfig {
    pub consensus_review: A2aConsensusReviewConfig,
    pub patch_plan: A2aPatchPlanConfig,
    pub impact_review: A2aImpactReviewConfig,
    pub pr_review: A2aPrReviewConfig,
}

impl Default for A2aWorkflowsConfig {
    fn default() -> Self {
        Self {
            consensus_review: A2aConsensusReviewConfig::default(),
            patch_plan: A2aPatchPlanConfig::default(),
            impact_review: A2aImpactReviewConfig::default(),
            pr_review: A2aPrReviewConfig::default(),
        }
    }
}

impl A2aWorkflowsConfig {
    pub fn is_enabled(&self, name: &str) -> bool {
        match name {
            "consensus_review" => self.consensus_review.enabled,
            "patch_plan" => self.patch_plan.enabled,
            "impact_review" => self.impact_review.enabled,
            "pr_review" => self.pr_review.enabled,
            _ => false,
        }
    }

    pub fn known_workflows() -> &'static [&'static str] {
        &[
            "consensus_review",
            "patch_plan",
            "impact_review",
            "pr_review",
        ]
    }
}

/// Task persistence backend for A2A sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum A2aTaskStoreKind {
    Memory,
    Sled,
}

impl Default for A2aTaskStoreKind {
    fn default() -> Self {
        Self::Memory
    }
}

/// Top-level `[a2a]` configuration block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aConfig {
    pub enabled: bool,
    /// When true, hub workflows force in-process dispatch for all roles (ignores per-role `mode`).
    pub force_in_process: bool,
    pub max_parallel_roles: usize,
    pub consensus_max_rounds: u32,
    /// Max StrategyProposal Accept/Reject rounds between planner and analyzer.
    pub max_negotiation_rounds: u32,
    pub insight_ttl_secs: u64,
    pub task_store: A2aTaskStoreKind,
    pub task_store_path: PathBuf,
    pub server: A2aServerConfig,
    pub blackboard: A2aBlackboardConfig,
    pub push: A2aPushConfig,
    pub host_guard: A2aHostGuardConfig,
    pub workflows: A2aWorkflowsConfig,
    /// When true, validator rejects when scoped index freshness is not fresh.
    pub require_fresh_index: bool,
    #[serde(default)]
    pub roles: HashMap<String, A2aRoleConfig>,
    /// Paths scanned for agent markdown manifests (Tier 2).
    #[serde(default)]
    pub agent_manifest_paths: Vec<PathBuf>,
}

impl Default for A2aConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut roles = HashMap::new();
        roles.insert(
            "analyzer".to_string(),
            A2aRoleConfig {
                mode: A2aRoleMode::InProcess,
                subscriptions: vec![
                    "TaskDelegation".to_string(),
                    "GraphMutationSignal".to_string(),
                    "CodeInsight".to_string(),
                    "StrategyProposal".to_string(),
                ],
                capabilities: vec![
                    "CodeInsight".to_string(),
                    "Reject".to_string(),
                    "Accept".to_string(),
                ],
                skills: vec![
                    "analyze_code_relationships".to_string(),
                    "get_impact_graph".to_string(),
                ],
                ..Default::default()
            },
        );
        roles.insert(
            "indexer".to_string(),
            A2aRoleConfig {
                mode: A2aRoleMode::InProcess,
                subscriptions: vec!["GraphMutationSignal".to_string()],
                ..Default::default()
            },
        );
        roles.insert(
            "patch_planner".to_string(),
            A2aRoleConfig {
                mode: A2aRoleMode::External,
                agent_card_url: Some(
                    "http://127.0.0.1:3001/.well-known/agents/patch-planner.json".to_string(),
                ),
                subscriptions: vec!["TaskDelegation".to_string()],
                capabilities: vec!["CodeInsight".to_string()],
                skills: vec!["get_patch_context".to_string()],
                ..Default::default()
            },
        );
        roles.insert(
            "validator".to_string(),
            A2aRoleConfig {
                mode: A2aRoleMode::External,
                agent_card_url: Some(
                    "http://127.0.0.1:3001/.well-known/agents/validator.json".to_string(),
                ),
                subscriptions: vec!["Accept".to_string()],
                capabilities: vec!["CodeInsight".to_string(), "Accept".to_string()],
                ..Default::default()
            },
        );
        roles.insert("gateway".to_string(), A2aRoleConfig::default());
        roles.insert("pr_reviewer".to_string(), A2aRoleConfig::default());

        Self {
            enabled: false,
            force_in_process: false,
            max_parallel_roles: 4,
            consensus_max_rounds: 3,
            max_negotiation_rounds: 3,
            insight_ttl_secs: 86_400,
            task_store: A2aTaskStoreKind::Memory,
            task_store_path: PathBuf::from(home).join(".cortex/a2a/tasks"),
            server: A2aServerConfig::default(),
            blackboard: A2aBlackboardConfig::default(),
            push: A2aPushConfig::default(),
            host_guard: A2aHostGuardConfig::default(),
            workflows: A2aWorkflowsConfig::default(),
            require_fresh_index: false,
            agent_manifest_paths: Vec::new(),
            roles,
        }
    }
}

impl A2aConfig {
    /// Default manifest scan paths relative to a repository root.
    pub fn default_agent_manifest_paths(repo_root: &std::path::Path) -> Vec<PathBuf> {
        vec![
            repo_root.join("docs/agents"),
            repo_root.join(".cursor/agents"),
        ]
    }

    pub fn role_mode(&self, name: &str) -> A2aRoleMode {
        self.roles
            .get(name)
            .map(|r| r.mode)
            .unwrap_or(A2aRoleMode::Disabled)
    }

    pub fn blackboard_write_batch_size(&self, fallback_max_batch: usize) -> usize {
        let n = self.blackboard.write_batch_size;
        if n == 0 { fallback_max_batch.max(1) } else { n }
    }

    /// Enable push delivery for production deployments.
    pub fn apply_production_profile(&mut self) {
        self.push.enabled = true;
        self.server.http_enabled = true;
    }
}
