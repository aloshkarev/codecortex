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

/// Build validation invoked by the A2A validator role (`validate_build`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aValidateConfig {
    /// Explicit command (program + args). When empty, auto-detect from repo layout.
    pub command: Vec<String>,
    /// Working directory relative to the repo root, or an absolute path.
    pub working_directory: Option<PathBuf>,
}

impl Default for A2aValidateConfig {
    fn default() -> Self {
        Self {
            command: Vec::new(),
            working_directory: None,
        }
    }
}

/// Resolved build-validation invocation for a repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateBuildPlan {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: std::path::PathBuf,
    pub label: String,
}

impl A2aValidateConfig {
    /// Resolve the command and working directory for `repo_root`.
    ///
    /// When `command` is empty, auto-detect:
    /// - `CMakeLists.txt` → `./build.sh` when present, else `cmake --build build`
    /// - `Cargo.toml` → `cargo check --quiet`
    pub fn resolve(&self, repo_root: &std::path::Path) -> Option<ValidateBuildPlan> {
        let cwd = self
            .working_directory
            .as_ref()
            .map(|wd| {
                if wd.is_absolute() {
                    wd.clone()
                } else {
                    repo_root.join(wd)
                }
            })
            .unwrap_or_else(|| repo_root.to_path_buf());

        if !self.command.is_empty() {
            let program = self.command[0].clone();
            let args = self.command[1..].to_vec();
            let label = self.command.join(" ");
            return Some(ValidateBuildPlan {
                program,
                args,
                cwd,
                label,
            });
        }

        Self::auto_detect(&cwd)
    }

    fn auto_detect(cwd: &std::path::Path) -> Option<ValidateBuildPlan> {
        if cwd.join("CMakeLists.txt").exists() {
            if cwd.join("build.sh").exists() {
                return Some(ValidateBuildPlan {
                    program: "./build.sh".to_string(),
                    args: Vec::new(),
                    cwd: cwd.to_path_buf(),
                    label: "./build.sh".to_string(),
                });
            }
            return Some(ValidateBuildPlan {
                program: "cmake".to_string(),
                args: vec!["--build".to_string(), "build".to_string()],
                cwd: cwd.to_path_buf(),
                label: "cmake --build build".to_string(),
            });
        }
        if cwd.join("Cargo.toml").exists() {
            return Some(ValidateBuildPlan {
                program: "cargo".to_string(),
                args: vec!["check".to_string(), "--quiet".to_string()],
                cwd: cwd.to_path_buf(),
                label: "cargo check --quiet".to_string(),
            });
        }
        None
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
    /// Build validation command for the validator role (`[a2a.validate]`).
    pub validate: A2aValidateConfig,
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
            validate: A2aValidateConfig::default(),
            agent_manifest_paths: Vec::new(),
            roles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn validate_config_deserializes_from_toml() {
        let raw = r#"
            [a2a.validate]
            command = ["cargo", "check", "--quiet"]
            working_directory = "crates/foo"
        "#;
        #[derive(Deserialize)]
        struct Wrapper {
            a2a: A2aConfig,
        }
        let parsed: Wrapper = toml::from_str(raw).expect("toml");
        assert_eq!(
            parsed.a2a.validate.command,
            vec!["cargo", "check", "--quiet"]
        );
        assert_eq!(
            parsed.a2a.validate.working_directory,
            Some(PathBuf::from("crates/foo"))
        );
    }

    #[test]
    fn auto_detect_cargo_workspace() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"rdiameter\"\n",
        )
        .unwrap();
        let plan = A2aValidateConfig::default()
            .resolve(dir.path())
            .expect("plan");
        assert_eq!(plan.program, "cargo");
        assert_eq!(plan.args, vec!["check", "--quiet"]);
        assert_eq!(plan.label, "cargo check --quiet");
    }

    #[test]
    fn auto_detect_cmake_prefers_build_sh() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.16)\n",
        )
        .unwrap();
        fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        let plan = A2aValidateConfig::default()
            .resolve(dir.path())
            .expect("plan");
        assert_eq!(plan.program, "./build.sh");
        assert_eq!(plan.args, Vec::<String>::new());
    }

    #[test]
    fn auto_detect_cmake_falls_back_to_cmake_build() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.16)\n",
        )
        .unwrap();
        let plan = A2aValidateConfig::default()
            .resolve(dir.path())
            .expect("plan");
        assert_eq!(plan.program, "cmake");
        assert_eq!(plan.args, vec!["--build", "build"]);
    }

    #[test]
    fn cmake_takes_precedence_over_cargo_at_same_root() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.16)\n",
        )
        .unwrap();
        fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"twag\"\n",
        )
        .unwrap();
        let plan = A2aValidateConfig::default()
            .resolve(dir.path())
            .expect("plan");
        assert_eq!(plan.program, "./build.sh");
    }

    #[test]
    fn explicit_command_overrides_auto_detect() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let cfg = A2aValidateConfig {
            command: vec!["make".to_string(), "check".to_string()],
            working_directory: None,
        };
        let plan = cfg.resolve(dir.path()).expect("plan");
        assert_eq!(plan.program, "make");
        assert_eq!(plan.args, vec!["check"]);
    }

    #[test]
    fn working_directory_relative_to_repo_root() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("third_party/rdiameter");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("Cargo.toml"), "[package]\nname = \"rdiameter\"\n").unwrap();
        let cfg = A2aValidateConfig {
            command: Vec::new(),
            working_directory: Some(PathBuf::from("third_party/rdiameter")),
        };
        let plan = cfg.resolve(dir.path()).expect("plan");
        assert_eq!(plan.cwd, sub);
        assert_eq!(plan.program, "cargo");
    }

    #[test]
    fn twag_monorepo_root_auto_detects_build_sh() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.16)\n",
        )
        .unwrap();
        fs::write(dir.path().join("build.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        let rdiameter = dir.path().join("third_party/tngf_cp/rdiameter");
        fs::create_dir_all(&rdiameter).unwrap();
        fs::write(
            rdiameter.join("Cargo.toml"),
            "[package]\nname = \"rdiameter\"\n",
        )
        .unwrap();

        let twag_plan = A2aValidateConfig::default()
            .resolve(dir.path())
            .expect("twag plan");
        assert_eq!(twag_plan.program, "./build.sh");

        let rdiameter_cfg = A2aValidateConfig {
            command: Vec::new(),
            working_directory: Some(PathBuf::from("third_party/tngf_cp/rdiameter")),
        };
        let rdiameter_plan = rdiameter_cfg.resolve(dir.path()).expect("rdiameter plan");
        assert_eq!(rdiameter_plan.program, "cargo");
        assert_eq!(rdiameter_plan.cwd, rdiameter);
    }

    #[test]
    fn resolve_returns_none_without_build_manifest() {
        let dir = TempDir::new().unwrap();
        assert!(A2aValidateConfig::default().resolve(dir.path()).is_none());
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
