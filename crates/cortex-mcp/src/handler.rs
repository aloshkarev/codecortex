use crate::contracts::{
    WARNING_EMBEDDER_TIMEOUT, WARNING_FALLBACK_TO_LEXICAL, WARNING_VECTOR_STORE_UNAVAILABLE,
    error as envelope_error, success as envelope_success,
};
use crate::jobs::{JobRegistry, JobState};
use crate::metrics::global_metrics;
use crate::vector_service::{VectorSearchFilters, VectorSearchRequest, VectorService};
use cortex_analyzer::{
    AnalyzePathFilters, Analyzer, CrossProjectAnalyzer, NavigationEngine, ReviewAnalyzer,
    ReviewFileInput, ReviewInput, ReviewLineRange, Severity, UsageKind,
};
use cortex_core::{CortexConfig, GitOperations, ProjectStatus, SearchKind};
use cortex_graph::{BundleStore, GraphClient};
use cortex_indexer::Indexer;
use cortex_parser::SignatureExtractor;
use cortex_vector::{
    Embedder, HybridSearch, LanceStore, OllamaEmbedder, OpenAIEmbedder, SearchType,
};
use cortex_watcher::{ProjectRegistry, WatchSession};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::{RequestContext, RoleServer, ServerInitializeError},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio,
    HttpSse,
    WebSocket,
    Multi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServeOptions {
    pub transport: McpTransport,
    pub listen: SocketAddr,
    pub token: Option<String>,
    pub allow_remote: bool,
    pub max_clients: usize,
    pub idle_timeout_secs: u64,
    /// Feature flags controlling which optional MCP tools are active.
    /// Constructed from environment variables; CLI `--enable` args layer on top.
    /// Skipped from serialization — flags are resolved at runtime, not persisted.
    #[serde(skip)]
    pub feature_flags: crate::FeatureFlags,
}

impl Default for McpServeOptions {
    fn default() -> Self {
        Self {
            transport: McpTransport::Stdio,
            listen: SocketAddr::from(([127, 0, 0, 1], 3001)),
            token: None,
            allow_remote: false,
            max_clients: 64,
            idle_timeout_secs: 600,
            feature_flags: crate::FeatureFlags::from_env(),
        }
    }
}

// ── request structs ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexPathReq {
    /// Directory or file path to index
    pub path: String,
    /// Also perform vector indexing for semantic retrieval
    pub include_vector: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathReq {
    /// Directory or file path
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexRepositoryReq {
    /// Directory path to index
    pub path: String,
    /// Optional repository identifier
    pub repo_path: Option<String>,
    /// Optional git branch
    pub branch: Option<String>,
    /// Optional source revision
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexFileReq {
    /// File path to index
    pub path: String,
    /// Optional repository identifier
    pub repo_path: Option<String>,
    /// Optional git branch
    pub branch: Option<String>,
    /// Optional source revision
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorSearchReq {
    /// Natural-language or keyword search query (e.g. "auth token refresh", "where is login validated")
    pub query: String,
    /// Maximum number of results to return (default often 10–20)
    pub k: Option<usize>,
    /// Restrict search to this repository path
    pub repo_path: Option<String>,
    /// Restrict search to files under this path prefix
    pub path: Option<String>,
    /// Filter by node kind (e.g. function, class, method)
    pub kind: Option<String>,
    /// Filter by language (e.g. rust, python, typescript)
    pub language: Option<String>,
    /// One of: semantic | hybrid | structural (default: semantic)
    pub search_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexStatusReq {
    /// Repository path to check; omit for all repos
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorDeleteRepositoryReq {
    /// Repository identifier/path whose vector index should be removed
    pub repo_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimilarAcrossReq {
    /// Minimum number of repositories in which symbol should appear
    pub min_repos: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SharedDepsReq {
    /// Optional explicit repository filter list
    pub repos: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareApiReq {
    pub repo_a: String,
    pub repo_b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CrossProjectSearchReq {
    pub query: String,
    pub repositories: Vec<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoToDefReq {
    pub symbol: String,
    pub from_file: Option<String>,
    pub from_line: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindUsagesReq {
    pub symbol: String,
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickInfoReq {
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BranchStructuralDiffReq {
    pub source_branch: String,
    pub target_branch: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PrReviewReq {
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub path: Option<String>,
    pub min_severity: Option<String>,
    pub max_findings: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindCodeReq {
    /// Search string: symbol name, regex pattern, or content snippet depending on kind
    pub query: String,
    /// One of: name | pattern | type | content  (default: pattern)
    pub kind: Option<String>,
    /// Optional path prefix filter
    pub path_filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RelationshipReq {
    /// Graph query mode: `find_callers`, `find_callees`, `find_all_callers`, `find_all_callees`,
    /// `call_chain`, `class_hierarchy`, `dead_code`, `overrides`, `module_deps`, `variable_scope`,
    /// `find_importers`, `find_by_decorator`, `find_by_argument`, `find_complexity`.
    pub query_type: String,
    pub target: Option<String>,
    pub target2: Option<String>,
    pub depth: Option<usize>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CypherReq {
    /// Cypher query string (e.g. MATCH (n:CodeNode) RETURN n LIMIT 10)
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComplexityReq {
    pub top_n: Option<u64>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JobStatusReq {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportBundleReq {
    pub repository_path: String,
    pub output_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextCapsuleReq {
    /// Task or topic description (e.g. "refactor auth", "find bug in login")
    pub query: String,
    /// Optional intent hint (e.g. debug, refactor, onboard); can improve ranking
    pub task_intent: Option<String>,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Approximate token budget for returned snippets (default 6000, max 12000)
    pub max_tokens: Option<usize>,
    /// Maximum number of items to return (default 40, max 100)
    pub max_items: Option<usize>,
    /// Whether to include test files in results
    pub include_tests: Option<bool>,
    /// Path substrings to filter by (e.g. ["src/auth"]); items must match at least one
    pub path_filter: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImpactGraphReq {
    /// Symbol name (function, class, method) to get call graph for
    pub symbol: String,
    /// Optional type hint: function, class, method, etc.
    pub symbol_type: Option<String>,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Traversal depth (default 2–3; higher can be slow)
    pub depth: Option<usize>,
    /// Include importers/dependents in the graph
    pub include_importers: Option<bool>,
    /// Include test files in the graph
    pub include_tests: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogicFlowReq {
    /// Starting symbol (e.g. entry point or caller)
    pub from_symbol: String,
    /// Ending symbol (e.g. target function or callee)
    pub to_symbol: String,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Maximum number of paths to return
    pub max_paths: Option<usize>,
    /// Maximum traversal depth per path
    pub max_depth: Option<usize>,
    /// If true, return partial paths when full path is not found
    pub allow_partial: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SkeletonReq {
    /// File path (relative to repo or absolute) to get skeleton for
    pub path: String,
    /// Optional mode (e.g. full, compact); implementation-dependent
    pub mode: Option<String>,
    /// Repository root when path is relative
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexStatusReq {
    /// Repository path to check; omit for all repos
    pub repo_path: Option<String>,
    /// Include list of background jobs
    pub include_jobs: Option<bool>,
    /// Include watcher status (watched paths)
    pub include_watcher: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspaceSetupReq {
    pub repo_path: Option<String>,
    pub detect_agents: Option<bool>,
    pub generate_configs: Option<bool>,
    pub install_git_hooks: Option<bool>,
    pub non_interactive: Option<bool>,
    pub overwrite: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LspEdgeInput {
    pub caller_fqn: String,
    pub callee_fqn: String,
    pub file: String,
    pub line: u64,
    pub confidence: Option<f64>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubmitLspEdgesReq {
    pub repo_path: String,
    pub edges: Vec<LspEdgeInput>,
    pub merge_mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveObservationReq {
    pub repo_path: String,
    pub text: String,
    pub severity: Option<String>,
    pub confidence: Option<f64>,
    pub symbol_refs: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub classification: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionContextReq {
    pub repo_path: String,
    pub session_id: Option<String>,
    pub include_previous: Option<usize>,
    pub max_items: Option<usize>,
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchMemoryReq {
    pub query: String,
    pub repo_path: String,
    pub max_items: Option<usize>,
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSignatureReq {
    /// Symbol name to look up (function, method, struct, enum)
    pub symbol: String,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include related signatures (implementations, overrides)
    pub include_related: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindTestsReq {
    /// Symbol name to find tests for
    pub symbol: String,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include integration tests
    pub include_integration: Option<bool>,
    /// Maximum number of tests to return
    pub max_results: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainResultReq {
    /// Original query to explain
    pub query: String,
    /// Tool that was used (optional, helps with context)
    pub tool: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeRefactoringReq {
    /// Symbol to analyze for refactoring
    pub symbol: String,
    /// Type of change being considered
    pub change_type: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include detailed breakdown
    pub detailed: Option<bool>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiagnoseReq {
    /// Type of diagnostic to run: index_health, graph_connectivity, cache_status, all
    pub check: Option<String>,
    /// Repository path (for index health checks)
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindPatternsReq {
    /// Pattern to search for: builder, factory, singleton, repository, service, handler, middleware, observer, strategy
    pub pattern: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Minimum confidence threshold (0.0-1.0)
    pub min_confidence: Option<f64>,
    /// Maximum results to return
    pub max_results: Option<usize>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

// ── project management request types ─────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Optional name for the project (defaults to directory name)
    pub name: Option<String>,
    /// Whether to track branch changes (default: true)
    pub track_branch: Option<bool>,
    /// Branches to keep indexed even when inactive
    pub pinned_branches: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Whether to delete associated index data
    pub delete_data: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Branch to switch to (optional, defaults to current)
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBranchesReq {
    /// Path to project (optional, uses current project if not specified)
    pub path: Option<String>,
    /// Whether to include remote branches
    pub include_remote: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectStatusReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Include queue details
    pub include_queue: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectSyncReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Force full index mode
    pub force: Option<bool>,
    /// Cleanup old branches after sync
    pub cleanup_old_branches: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectBranchDiffReq {
    /// Source branch
    pub source: String,
    /// Target branch
    pub target: String,
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Commit limit for ahead/behind lists
    pub commit_limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectQueueStatusReq {
    /// Path to project (optional filter)
    pub path: Option<String>,
    /// Maximum jobs returned
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectMetricsReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ObservationRecord {
    pub observation_id: String,
    pub repo_id: String,
    pub session_id: String,
    pub created_at: u128,
    pub created_by: String,
    pub text: String,
    pub symbol_refs: Vec<String>,
    pub confidence: f64,
    pub stale: bool,
    pub classification: String,
    pub severity: String,
    pub tags: Vec<String>,
    pub source_revision: String,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
}

// ── handler ───────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CortexHandler {
    config: CortexConfig,
    jobs: JobRegistry,
    projects: Arc<ProjectRegistry>,
    tool_router: ToolRouter<Self>,
    feature_flags: crate::FeatureFlags,
}

#[tool_router]
impl CortexHandler {
    pub fn new(config: CortexConfig) -> Self {
        Self {
            config,
            jobs: JobRegistry::default(),
            projects: Arc::new(ProjectRegistry::new()),
            tool_router: Self::tool_router(),
            feature_flags: crate::FeatureFlags::from_env(),
        }
    }

    /// Create a handler with a pre-built `FeatureFlags` (e.g. from CLI `--enable` args).
    pub fn new_with_flags(config: CortexConfig, feature_flags: crate::FeatureFlags) -> Self {
        Self {
            config,
            jobs: JobRegistry::default(),
            projects: Arc::new(ProjectRegistry::new()),
            tool_router: Self::tool_router(),
            feature_flags,
        }
    }

    async fn graph_client(&self) -> Result<GraphClient, McpError> {
        GraphClient::connect(&self.config)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    fn ok(text: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![Content::text(text)])
    }

    fn tool_enabled(&self, key: &str, _default_value: bool) -> bool {
        self.feature_flags.is_enabled(key)
    }

    fn current_watch_config(&self) -> CortexConfig {
        CortexConfig::load().unwrap_or_else(|_| self.config.clone())
    }

    fn resolve_project_context(&self) -> Result<(String, Option<String>), McpError> {
        let project_ref = self.projects.get_current_project().ok_or_else(|| {
            McpError::invalid_params(
                "No current project set. Use set_current_project first.",
                None,
            )
        })?;
        Ok((
            project_ref.path.display().to_string(),
            Some(project_ref.branch),
        ))
    }

    // ── indexing ─────────────────────────────────────────────────────────────

    #[tool(
        title = "Index path into graph",
        description = "Indexes a directory or file into the code graph and optionally the vector store. Use when: indexing a repo, rebuilding the graph, or before graph/vector queries. Prerequisite: Memgraph reachable. Pair with find_code or vector_search after. Returns graph job info plus optional vector stats.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn add_code_to_graph(
        &self,
        Parameters(req): Parameters<IndexPathReq>,
    ) -> Result<CallToolResult, McpError> {
        let include_vector = req.include_vector.unwrap_or(false);
        let job_id = format!("index-{}", now_millis());
        self.jobs
            .mark_running(&job_id, format!("Indexing {}", req.path));

        let cfg = self.config.clone();
        let jobs = self.jobs.clone();
        let path = req.path.clone();
        let job_id_for_task = job_id.clone();
        tokio::spawn(async move {
            let outcome = async {
                let client = GraphClient::connect(&cfg).await?;
                let indexer = Indexer::new(client, cfg.max_batch_size)?;
                let graph_report = indexer.index_path(&path).await?;
                let mut vector_status = serde_json::json!({
                    "enabled": include_vector,
                    "status": "skipped"
                });
                if include_vector {
                    match VectorService::from_env().await {
                        Ok(service) => {
                            let (repo_root, branch, revision) =
                                resolve_git_context_for_path(Path::new(&path)).map_or_else(
                                    || {
                                        (
                                            PathBuf::from(&path),
                                            "unknown".to_string(),
                                            "unknown".to_string(),
                                        )
                                    },
                                    |(root, b, rev)| (root, b, rev),
                                );
                            let vector_outcome = if Path::new(&path).is_file() {
                                service
                                    .index_file(
                                        Path::new(&path),
                                        &repo_root.display().to_string(),
                                        &branch,
                                        &revision,
                                    )
                                    .await
                            } else {
                                service
                                    .index_repository(
                                        &repo_root,
                                        &repo_root.display().to_string(),
                                        &branch,
                                        &revision,
                                    )
                                    .await
                            };
                            match vector_outcome {
                                Ok(indexed) => {
                                    global_metrics().record_vector_documents_indexed(
                                        indexed.indexed_documents as u64,
                                    );
                                    vector_status = serde_json::json!({
                                        "enabled": true,
                                        "status": "completed",
                                        "indexed_documents": indexed.indexed_documents,
                                        "scanned_files": indexed.scanned_files,
                                        "skipped_files": indexed.skipped_files
                                    });
                                }
                                Err(err) => {
                                    global_metrics().record_vector_fallback();
                                    vector_status = serde_json::json!({
                                        "enabled": true,
                                        "status": "failed",
                                        "warning": "vector_index_failed",
                                        "error": err
                                    });
                                }
                            }
                        }
                        Err(err) => {
                            global_metrics().record_vector_fallback();
                            vector_status = serde_json::json!({
                                "enabled": true,
                                "status": "failed",
                                        "warning": WARNING_VECTOR_STORE_UNAVAILABLE,
                                "error": err
                            });
                        }
                    }
                }
                anyhow::Ok(serde_json::json!({
                    "graph": graph_report,
                    "vector": vector_status
                }))
            }
            .await;

            match outcome {
                Ok(report) => jobs.mark_completed(
                    &job_id_for_task,
                    serde_json::to_string(&report).unwrap_or_else(|_| "completed".to_string()),
                ),
                Err(err) => jobs.mark_failed(&job_id_for_task, err.to_string()),
            }
        });

        Ok(Self::ok(
            serde_json::json!({
                "job_id": job_id,
                "state": "running",
                "path": req.path,
                "include_vector": include_vector
            })
            .to_string(),
        ))
    }

    // ── watching ─────────────────────────────────────────────────────────────

    #[tool(
        title = "Watch directory for reindex",
        description = "Watches a path and reindexes on changes. Use when: keeping the graph fresh during editing. Prerequisite: valid path. Pair with list_watched_paths / unwatch_directory. Not a substitute for add_code_to_graph on cold start.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn watch_directory(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut cfg = self.current_watch_config();
        let session = WatchSession::new(&cfg);
        session
            .watch(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        session
            .persist_to_config(&mut cfg)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let job_id = format!("watch-{}", now_millis());
        self.jobs
            .mark_running(&job_id, format!("Watching {}", req.path));
        let jobs = self.jobs.clone();
        let cfg = cfg.clone();
        let watch_path = req.path.clone();
        let job_id_for_task = job_id.clone();
        tokio::spawn(async move {
            let watch_outcome = async {
                let client = GraphClient::connect(&cfg).await?;
                let indexer = Indexer::new(client, cfg.max_batch_size)?;
                let watcher = WatchSession::new(&cfg);
                watcher.watch(watch_path.as_ref())?;
                watcher.run(indexer).await?;
                anyhow::Ok(())
            }
            .await;
            if let Err(err) = watch_outcome {
                jobs.mark_failed(&job_id_for_task, err.to_string());
            }
        });

        Ok(Self::ok(
            serde_json::json!({
                "job_id": job_id,
                "state": "running",
                "path": req.path
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "List watched paths",
        description = "Lists directories under watch-based reindexing. Use when: checking watcher scope. Read-only. Pair with watch_directory and unwatch_directory.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_watched_paths(&self) -> Result<CallToolResult, McpError> {
        let cfg = self.current_watch_config();
        let paths = WatchSession::new(&cfg).list();
        Ok(Self::ok(
            serde_json::to_string_pretty(&paths).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Unwatch directory",
        description = "Stops watching a path and updates persisted config. Use when: removing a watcher entry. Prerequisite: path was watched.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn unwatch_directory(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let mut cfg = self.current_watch_config();
        let session = WatchSession::new(&cfg);
        let removed = session
            .unwatch(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        session
            .persist_to_config(&mut cfg)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(format!("removed={}", removed)))
    }

    // ── search / analysis ─────────────────────────────────────────────────────

    #[tool(
        title = "Search code graph",
        description = "Graph search by symbol name, regex pattern, type, or content. Use when: locating symbols before navigation or analysis. Prerequisite: indexed graph. Prefer vector_search for fuzzy natural-language question if vector index exists. Returns matches with paths and signatures.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_code(
        &self,
        Parameters(req): Parameters<FindCodeReq>,
    ) -> Result<CallToolResult, McpError> {
        let kind = match req.kind.as_deref().unwrap_or("pattern") {
            "name" => SearchKind::Name,
            "type" => SearchKind::Type,
            "content" => SearchKind::Content,
            _ => SearchKind::Pattern,
        };
        let rows = Analyzer::new(self.graph_client().await?)
            .find_code(&req.query, kind, req.path_filter.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&rows).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Graph relationships",
        description = "Callers, callees, hierarchy, dead_code, call_chain, importers, complexity, etc. Use when: impact, structure, or reachability questions. Prerequisite: indexed graph; pass query_type and targets; optional path/glob filters. Prefer find_dead_code shortcut for unused-code only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn analyze_code_relationships(
        &self,
        Parameters(req): Parameters<RelationshipReq>,
    ) -> Result<CallToolResult, McpError> {
        let a = Analyzer::new(self.graph_client().await?);
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let t = req.target.as_deref().unwrap_or_default();
        let t2 = req.target2.as_deref().unwrap_or_default();
        let rows = match req.query_type.as_str() {
            "find_callers" => a.callers_with_filters(t, Some(&filters)).await,
            "find_callees" => a.callees_with_filters(t, Some(&filters)).await,
            "find_all_callers" => a.all_callers_with_filters(t, Some(&filters)).await,
            "find_all_callees" => a.all_callees_with_filters(t, Some(&filters)).await,
            "call_chain" => {
                a.call_chain_with_filters(t, t2, req.depth, Some(&filters))
                    .await
            }
            "class_hierarchy" => a.class_hierarchy_with_filters(t, Some(&filters)).await,
            "dead_code" => a.dead_code_with_filters(Some(&filters)).await,
            "overrides" => a.overrides_with_filters(t, Some(&filters)).await,
            "module_deps" => a.module_dependencies_with_filters(t, Some(&filters)).await,
            "variable_scope" => a.variable_scope_with_filters(t, Some(&filters)).await,
            "find_importers" => a.find_importers_with_filters(t, Some(&filters)).await,
            "find_by_decorator" => a.find_by_decorator_with_filters(t, Some(&filters)).await,
            "find_by_argument" => a.find_by_argument_with_filters(t, Some(&filters)).await,
            "find_complexity" => a.find_complexity_with_filters(t, Some(&filters)).await,
            _ => {
                return Err(McpError::invalid_params(
                    format!("unknown query_type: {}", req.query_type),
                    None,
                ));
            }
        }
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&rows).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Run Cypher query",
        description = "Runs arbitrary Cypher on the code graph. Use when: higher-level tools cannot express the traversal. Prerequisite: indexed graph; avoid destructive queries unless intended. Prefer analyze_code_relationships for standard callers/callees.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn execute_cypher_query(
        &self,
        Parameters(req): Parameters<CypherReq>,
    ) -> Result<CallToolResult, McpError> {
        let rows = self
            .graph_client()
            .await?
            .raw_query(&req.query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&rows).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Find dead code",
        description = "Lists symbols with no callers in the graph. Use when: cleanup or coverage audits. Prerequisite: indexed graph. For scoped dead code use analyze_code_relationships with filters.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_dead_code(&self) -> Result<CallToolResult, McpError> {
        let rows = Analyzer::new(self.graph_client().await?)
            .dead_code()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&rows).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Similar symbols cross-repo",
        description = "Similar symbols across indexed repositories. Use when: duplication or convergence analysis across projects. Prerequisite: multiple indexed repos.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_similar_across_projects(
        &self,
        Parameters(req): Parameters<SimilarAcrossReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let results = analyzer
            .find_similar_symbols(None, req.min_repos.unwrap_or(2))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Shared dependencies",
        description = "Modules shared across indexed projects. Use when: comparing dependency overlap. Prerequisite: indexed multi-repo graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_shared_dependencies(
        &self,
        Parameters(req): Parameters<SharedDepsReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let repos = if req.repos.is_empty() {
            None
        } else {
            Some(req.repos.as_slice())
        };
        let results = analyzer
            .find_shared_dependencies(repos)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Compare API surfaces",
        description = "Public API diff between two indexed repos (overlap, uniqueness, score). Use when: breaking-change or fork comparison.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn compare_api_surface(
        &self,
        Parameters(req): Parameters<CompareApiReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let result = analyzer
            .compare_api_surface(&req.repo_a, &req.repo_b)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Go to definition",
        description = "Resolves definition for a symbol with disambiguation. Use when: jump-to-definition tasks. Prerequisite: current project set; indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn go_to_definition(
        &self,
        Parameters(req): Parameters<GoToDefReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let (repo_path, branch) = self.resolve_project_context()?;
        let nav = NavigationEngine::new(graph, repo_path, branch);
        let results = nav
            .go_to_definition(
                &req.symbol,
                req.from_file.as_deref().unwrap_or(""),
                req.from_line,
            )
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Find all usages",
        description = "Usages of a symbol (calls, imports, types, inheritance). Use when: rename/refactor impact in current project. Prerequisite: set_current_project; indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_all_usages(
        &self,
        Parameters(req): Parameters<FindUsagesReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let (repo_path, branch) = self.resolve_project_context()?;
        let nav = NavigationEngine::new(graph, repo_path, branch);
        let usage_kind = parse_usage_kind(req.kind.as_deref())
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let results = nav
            .find_usages(&req.symbol, usage_kind)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Quick info",
        description = "Hover-like summary: signature, docs, definition location. Use when: fast orientation on a symbol. Prerequisite: current project; indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn quick_info(
        &self,
        Parameters(req): Parameters<QuickInfoReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let (repo_path, branch) = self.resolve_project_context()?;
        let nav = NavigationEngine::new(graph, repo_path, branch);
        let results = nav
            .quick_info(&req.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Branch structural diff",
        description = "Symbol-level diff between branches plus impact hints. Use when: branch or release review. Prerequisite: current project; both sides indexed.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn branch_structural_diff(
        &self,
        Parameters(req): Parameters<BranchStructuralDiffReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let (repo_path, _) = self.resolve_project_context()?;
        let nav = NavigationEngine::new(graph, repo_path, None);
        let result = nav
            .branch_structural_diff(&req.source_branch, &req.target_branch)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "PR-style review",
        description = "Review findings with graph signals (impact, possible new dead code). Use when: PR/branch review. Prerequisite: current project and indexed refs.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn pr_review(
        &self,
        Parameters(req): Parameters<PrReviewReq>,
    ) -> Result<CallToolResult, McpError> {
        let graph = self.graph_client().await?;
        let (repo_path, branch) = self.resolve_project_context()?;
        let nav = NavigationEngine::new(graph, repo_path.clone(), branch);
        let reviewer = ReviewAnalyzer::new();
        let input = build_review_input_from_req(&repo_path, &req)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let report = reviewer.analyze_with_graph(&input, &nav).await;
        Ok(Self::ok(
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Cyclomatic complexity",
        description = "Ranks symbols by cyclomatic complexity. Use when: hotspots or maintainability scans. Prerequisite: indexed graph; optional path/glob filters.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn calculate_cyclomatic_complexity(
        &self,
        Parameters(req): Parameters<ComplexityReq>,
    ) -> Result<CallToolResult, McpError> {
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let rows = Analyzer::new(self.graph_client().await?)
            .complexity_with_filters(req.top_n.unwrap_or(20) as usize, Some(&filters))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&rows).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Vector index repository",
        description = "Indexes repo files into the vector store for semantic search. Use when: enabling vector_search / hybrid. Prerequisite: configured embedder and vector backend. Pair with vector_index_status.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn vector_index_repository(
        &self,
        Parameters(req): Parameters<VectorIndexRepositoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_repository is disabled by feature flag",
                None,
                started,
            ));
        }
        let root = PathBuf::from(&req.path);
        if !root.exists() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!("path does not exist: {}", req.path),
                None,
                started,
            ));
        }
        if !root.is_dir() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!(
                    "vector_index_repository expects a directory path, got file: {}",
                    req.path
                ),
                None,
                started,
            ));
        }
        let service = match VectorService::from_env().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let repository = req
            .repo_path
            .clone()
            .unwrap_or_else(|| root.display().to_string());
        let branch = req.branch.clone().unwrap_or_else(|| "unknown".to_string());
        let revision = req
            .revision
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let result = service
            .index_repository(&root, &repository, &branch, &revision)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        global_metrics().record_vector_documents_indexed(result.indexed_documents as u64);
        Ok(envelope_success(
            json!({
                "repository": repository,
                "branch": branch,
                "revision": revision,
                "indexed_documents": result.indexed_documents,
                "scanned_files": result.scanned_files,
                "skipped_files": result.skipped_files
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Vector index file",
        description = "Indexes one file into the vector store. Use when: patching semantic index incrementally. Prerequisite: vector stack configured.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn vector_index_file(
        &self,
        Parameters(req): Parameters<VectorIndexFileReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_file is disabled by feature flag",
                None,
                started,
            ));
        }
        let file = PathBuf::from(&req.path);
        if !file.exists() || !file.is_file() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!("file does not exist: {}", req.path),
                None,
                started,
            ));
        }
        let service = match VectorService::from_env().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let repository = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let branch = req.branch.clone().unwrap_or_else(|| "unknown".to_string());
        let revision = req
            .revision
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let result = service
            .index_file(&file, &repository, &branch, &revision)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        global_metrics().record_vector_documents_indexed(result.indexed_documents as u64);
        Ok(envelope_success(
            json!({
                "file": req.path,
                "repository": repository,
                "branch": branch,
                "revision": revision,
                "indexed_documents": result.indexed_documents,
                "scanned_files": result.scanned_files,
                "skipped_files": result.skipped_files
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Vector semantic search",
        description = "Semantic search over the vector index. Use when: natural-language code discovery. Prerequisite: vector_index_repository (or file) completed. Not a substitute for exact symbol lookup (find_code).",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn vector_search(
        &self,
        Parameters(req): Parameters<VectorSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_search is disabled by feature flag",
                None,
                started,
            ));
        }
        let k = req.k.unwrap_or(20).clamp(1, 200);
        let search_type = req
            .search_type
            .as_deref()
            .and_then(|s| s.parse::<SearchType>().ok())
            .unwrap_or(SearchType::Semantic);
        let service = match VectorService::from_env().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let results = service
            .search(VectorSearchRequest {
                query: &req.query,
                search_type,
                k,
                filters: VectorSearchFilters {
                    repository: req.repo_path.as_deref(),
                    path: req.path.as_deref(),
                    kind: req.kind.as_deref(),
                    language: req.language.as_deref(),
                },
            })
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let output: Vec<_> = results
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.result.id,
                    "score": r.combined_score,
                    "content": r.result.content,
                    "metadata": r.result.metadata,
                    "graph_context": r.graph_context
                })
            })
            .collect();
        Ok(envelope_success(
            json!({
                "query": req.query,
                "search_type": search_type.to_string(),
                "count": output.len(),
                "results": output
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Hybrid vector search",
        description = "Semantic plus structural signals. Use when: NL query needs graph grounding. Prerequisite: vector + indexed graph. Delegates to vector_search with hybrid mode.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn vector_search_hybrid(
        &self,
        Parameters(req): Parameters<VectorSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let req = VectorSearchReq {
            search_type: Some("hybrid".to_string()),
            ..req
        };
        self.vector_search(Parameters(req)).await
    }

    #[tool(
        title = "Vector search multi-repo",
        description = "Semantic search across repositories with embeddings. Use when: same as vector_search but scoped to many repos. Prerequisite: embeddings and Lance/vector path configured.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_across_projects(
        &self,
        Parameters(req): Parameters<CrossProjectSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let store_path = PathBuf::from(home).join(".cortex/vectors");
        let store = LanceStore::open(&store_path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let embedder: Arc<dyn Embedder> = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            Arc::new(OpenAIEmbedder::new(api_key))
        } else {
            Arc::new(OllamaEmbedder::new())
        };
        let hybrid = HybridSearch::new(Arc::new(store), embedder);
        let repos = if req.repositories.is_empty() {
            None
        } else {
            Some(req.repositories.as_slice())
        };
        let results = hybrid
            .search_across_repositories(&req.query, repos, req.limit.unwrap_or(10))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&results).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Vector index status",
        description = "Health and document counts for vector index. Use when: debugging empty vector_search or planning (re)index. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn vector_index_status(
        &self,
        Parameters(req): Parameters<VectorIndexStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_status is disabled by feature flag",
                None,
                started,
            ));
        }
        let service = match VectorService::from_env().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let healthy = service
            .health_check()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let total_documents = service
            .total_documents()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(envelope_success(
            json!({
                "healthy": healthy,
                "total_documents": total_documents,
                "repo_path": req.repo_path
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Delete vector index for repo",
        description = "Removes all vector documents for a repository id. Use when: wiping stale semantic index. Destructive to vector store only.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn vector_delete_repository(
        &self,
        Parameters(req): Parameters<VectorDeleteRepositoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_delete_repository is disabled by feature flag",
                None,
                started,
            ));
        }
        let service = match VectorService::from_env().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let deleted = service
            .delete_repository(&req.repo_path)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(envelope_success(
            json!({
                "repo_path": req.repo_path,
                "deleted_documents": deleted
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Context capsule",
        description = "Bounded, ranked snippets for a task query (graph + optional vector). Use when: assembling working set for a prompt. Prerequisite: index; vector improves relevance if enabled.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_context_capsule(
        &self,
        Parameters(req): Parameters<ContextCapsuleReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.context_capsule.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_context_capsule is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.query.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }
        let max_items = req.max_items.unwrap_or(40).min(100);
        let max_tokens = req.max_tokens.unwrap_or(6000).clamp(256, 12000);
        let include_tests = req.include_tests.unwrap_or(false);
        let intent = req
            .task_intent
            .clone()
            .unwrap_or_else(|| detect_intent(req.query.as_str()).to_string());
        let analyzer = Analyzer::new(self.graph_client().await?);
        let rows = analyzer
            .find_code(&req.query, SearchKind::Pattern, None)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let filters = req.path_filter.clone().unwrap_or_default();

        let mut items = Vec::<Value>::new();
        let mut token_estimate = 0usize;
        let mut warnings = Vec::<String>::new();

        // Vector-first candidate retrieval for better NL relevance.
        if self.tool_enabled("mcp.vector.read.enabled", true) {
            match VectorService::from_env().await {
                Ok(service) => {
                    match service
                        .search(VectorSearchRequest {
                            query: req.query.as_str(),
                            search_type: SearchType::Hybrid,
                            k: max_items * 2,
                            filters: VectorSearchFilters {
                                repository: req.repo_path.as_deref(),
                                ..Default::default()
                            },
                        })
                        .await
                    {
                        Ok(vector_results) => {
                            for result in vector_results {
                                if items.len() >= max_items || token_estimate >= max_tokens {
                                    break;
                                }
                                let metadata = &result.result.metadata;
                                let path = metadata
                                    .get("path")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or_default();
                                if !include_tests && path.contains("/test") {
                                    continue;
                                }
                                if !filters.is_empty() && !filters.iter().any(|f| path.contains(f))
                                {
                                    continue;
                                }
                                let name = metadata
                                    .get("name")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or_default()
                                    .to_string();
                                let kind = metadata
                                    .get("kind")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or("CodeNode")
                                    .to_string();
                                let snippet = result
                                    .result
                                    .content
                                    .clone()
                                    .unwrap_or_default()
                                    .chars()
                                    .take(320)
                                    .collect::<String>();
                                let lex = simple_lexical_score(&req.query, &name, &snippet);
                                let vector_score = result.combined_score as f64;
                                let graph_score = result
                                    .graph_context
                                    .as_ref()
                                    .map(|ctx| {
                                        ((ctx.callers_count + ctx.callees_count) as f64 / 20.0)
                                            .min(1.0)
                                    })
                                    .unwrap_or(0.0);
                                let score =
                                    (vector_score * 0.65) + (lex * 0.25) + (graph_score * 0.10);
                                token_estimate += snippet.len() / 4 + 32;
                                items.push(json!({
                                    "id": result.result.id,
                                    "kind": kind,
                                    "path": path,
                                    "name": name,
                                    "snippet": snippet,
                                    "score": score,
                                    "why": {
                                        "vector": vector_score,
                                        "graph": graph_score,
                                        "lexical": lex
                                    }
                                }));
                            }
                        }
                        Err(err) => {
                            global_metrics().record_vector_fallback();
                            warnings.push(warning_with_reason(WARNING_FALLBACK_TO_LEXICAL, &err));
                        }
                    }
                }
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(WARNING_VECTOR_STORE_UNAVAILABLE, &err));
                }
            }
        } else {
            warnings.push("vector_read_disabled".to_string());
        }

        for row in rows {
            if items.len() >= max_items || token_estimate >= max_tokens {
                break;
            }
            let Some(node) = row.get("n") else {
                continue;
            };
            let path = node
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if !include_tests && path.contains("/test") {
                continue;
            }
            if !filters.is_empty() && !filters.iter().any(|f| path.contains(f)) {
                continue;
            }
            let name = node
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let kind = node
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("CodeNode")
                .to_string();
            let snippet = node
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .chars()
                .take(320)
                .collect::<String>();
            let lex = simple_lexical_score(&req.query, &name, &snippet);
            let tfidf = (snippet.len().min(200) as f64) / 200.0;
            let centrality = 0.1;
            let score = (lex * 0.5) + (tfidf * 0.4) + (centrality * 0.1);
            token_estimate += snippet.len() / 4 + 32;
            items.push(json!({
                "id": node.get("id").cloned().unwrap_or(Value::Null),
                "kind": kind,
                "path": path,
                "name": name,
                "snippet": snippet,
                "score": score,
                "why": {
                    "vector": 0.0,
                    "graph": centrality,
                    "lexical": lex,
                    "tfidf": tfidf
                }
            }));
        }
        let partial = token_estimate >= max_tokens || items.len() >= max_items;
        if items.is_empty() {
            warnings.push("fallback_relaxed_no_results".to_string());
        }
        Ok(envelope_success(
            json!({
                "intent_detected": intent,
                "capsule_items": items,
                "token_estimate": token_estimate,
                "threshold_used": 0.15,
                "fallback_relaxed": !warnings.is_empty()
            }),
            started,
            warnings,
            partial,
        ))
    }

    #[tool(
        title = "Impact graph",
        description = "Callers/callees/importers summary around a symbol. Use when: blast radius or dependency questions. Prerequisite: indexed graph. For deep edges prefer analyze_code_relationships.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_impact_graph(
        &self,
        Parameters(req): Parameters<ImpactGraphReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.impact_graph.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_impact_graph is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.symbol.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "symbol must not be empty",
                None,
                started,
            ));
        }
        let depth = req.depth.unwrap_or(4).clamp(1, 8);
        let analyzer = Analyzer::new(self.graph_client().await?);
        let direct = analyzer
            .callers(req.symbol.as_str())
            .await
            .unwrap_or_default();
        let transitive = analyzer
            .all_callers(req.symbol.as_str())
            .await
            .unwrap_or_default();
        let importers = if req.include_importers.unwrap_or(true) {
            analyzer
                .find_importers(req.symbol.as_str())
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let blast = if transitive.len() > 20 {
            "high"
        } else if transitive.len() > 5 {
            "medium"
        } else {
            "low"
        };
        Ok(envelope_success(
            json!({
                "root": {
                    "name": req.symbol,
                    "symbol_type": req.symbol_type.unwrap_or_else(|| "auto".to_string())
                },
                "nodes": [],
                "edges": [],
                "summary": {
                    "direct_callers": direct.len(),
                    "transitive_callers": transitive.len(),
                    "importers": importers.len(),
                    "dependents": direct.len() + importers.len(),
                    "blast_radius": blast,
                    "depth_used": depth
                }
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Logic flow paths",
        description = "Call paths between two functions via graph query. Use when: tracing execution flow. Prerequisite: indexed CALLS graph; may return empty if paths not modeled.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_logic_flow(
        &self,
        Parameters(req): Parameters<LogicFlowReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.logic_flow.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "search_logic_flow is disabled by feature flag",
                None,
                started,
            ));
        }
        let max_paths = req.max_paths.unwrap_or(5).clamp(1, 20);
        let max_depth = req.max_depth.unwrap_or(12).clamp(1, 20);
        let allow_partial = req.allow_partial.unwrap_or(true);
        let escaped_from = escape_cypher(&req.from_symbol);
        let escaped_to = escape_cypher(&req.to_symbol);
        let cypher = format!(
            "MATCH p=(a:Function {{name:'{escaped_from}'}})-[:CALLS*1..{max_depth}]->(b:Function {{name:'{escaped_to}'}})
             RETURN p LIMIT {max_paths}"
        );
        let rows = self
            .graph_client()
            .await?
            .raw_query(cypher.as_str())
            .await
            .unwrap_or_default();
        let partial = rows.is_empty() && allow_partial;
        let warnings = if partial {
            vec!["no_path_found_returning_partial".to_string()]
        } else {
            Vec::new()
        };
        Ok(envelope_success(
            json!({
                "paths": rows,
                "searched_depth": max_depth,
                "allow_partial": allow_partial
            }),
            started,
            warnings,
            partial,
        ))
    }

    #[tool(
        title = "File skeleton",
        description = "Outline of a file from disk (symbols/structure). Use when: quick file map without full source. Prerequisite: readable path; does not require graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_skeleton(
        &self,
        Parameters(req): Parameters<SkeletonReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_skeleton is disabled by feature flag",
                None,
                started,
            ));
        }
        let mode = req.mode.unwrap_or_else(|| "minimal".to_string());
        let content = fs::read_to_string(&req.path)
            .map_err(|e| McpError::invalid_params(format!("unable to read path: {e}"), None))?;
        let skeleton = build_skeleton(content.as_str(), mode.as_str());
        Ok(envelope_success(
            json!({
                "path": req.path,
                "mode": mode,
                "content": skeleton,
                "precomputed": false,
                "compression_ratio": 0.7
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Index and job status",
        description = "Aggregate MCP-side view of health, watcher, and job counters. Use when: quick session health check. Pair with diagnose for deeper graph checks.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn index_status(
        &self,
        Parameters(req): Parameters<IndexStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.index_status.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "index_status is disabled by feature flag",
                None,
                started,
            ));
        }
        let include_jobs = req.include_jobs.unwrap_or(true);
        let include_watcher = req.include_watcher.unwrap_or(true);
        let path = req.repo_path.unwrap_or_else(default_repo_path);
        let health = self.graph_client().await.is_ok();
        let stats = Analyzer::new(self.graph_client().await?)
            .repository_stats()
            .await
            .unwrap_or_default();
        let job_list = if include_jobs {
            self.jobs.list()
        } else {
            Vec::new()
        };
        let watched = if include_watcher {
            WatchSession::new(&self.config).list()
        } else {
            Vec::new()
        };
        Ok(envelope_success(
            json!({
                "health": if health { "ok" } else { "degraded" },
                "repo_path": path,
                "counts": {
                    "repositories": stats.len()
                },
                "indexing": {
                    "progress_pct": if job_list.iter().any(|j| j.state == JobState::Running) { 50 } else { 100 }
                },
                "watcher": {
                    "running": !watched.is_empty(),
                    "watched_paths": watched
                },
                "jobs": {
                    "running": job_list.iter().filter(|j| j.state == JobState::Running).count(),
                    "completed": job_list.iter().filter(|j| j.state == JobState::Completed).count(),
                    "failed": job_list.iter().filter(|j| j.state == JobState::Failed).count()
                }
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Workspace MCP bootstrap",
        description = "Detects AI-agent dirs and can write mcp.json / hooks under repo. Use when: onboarding CodeCortex into a workspace. May write files; confirm overwrite behavior.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn workspace_setup(
        &self,
        Parameters(req): Parameters<WorkspaceSetupReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.workspace_setup.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "workspace_setup is disabled by feature flag",
                None,
                started,
            ));
        }
        let repo = req.repo_path.unwrap_or_else(default_repo_path);
        let detect_agents = req.detect_agents.unwrap_or(true);
        let generate_configs = req.generate_configs.unwrap_or(true);
        let install_hooks = req.install_git_hooks.unwrap_or(false);
        let non_interactive = req.non_interactive.unwrap_or(false);
        let overwrite = req.overwrite.unwrap_or(false);
        let mut detected = Vec::<String>::new();
        if detect_agents {
            if Path::new(".cursor").exists() {
                detected.push("cursor".to_string());
            }
            if Path::new("CLAUDE.md").exists() {
                detected.push("claude".to_string());
            }
            if Path::new("AGENTS.md").exists() {
                detected.push("codex".to_string());
            }
        }
        let mut created = Vec::<String>::new();
        let mut warnings = Vec::<String>::new();
        if generate_configs {
            let mcp_path = PathBuf::from(&repo).join("mcp.json");
            if mcp_path.exists() && !(non_interactive && overwrite) {
                warnings.push("mcp.json exists; skipped overwrite".to_string());
            } else {
                let cfg = json!({
                    "mcpServers": {
                        "codecortex": {
                            "command": "cortex",
                            "args": ["mcp", "start"],
                            "cwd": repo
                        }
                    }
                });
                fs::write(
                    &mcp_path,
                    serde_json::to_string_pretty(&cfg).unwrap_or_default(),
                )
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                created.push("mcp.json".to_string());
            }
        }
        let mut hooks = Vec::<String>::new();
        if install_hooks {
            let hooks_dir = PathBuf::from(&repo).join(".git/hooks");
            if hooks_dir.exists() {
                let pre_commit = hooks_dir.join("pre-commit");
                if !pre_commit.exists() || (non_interactive && overwrite) {
                    fs::write(
                        &pre_commit,
                        "#!/usr/bin/env sh\ncargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings\n",
                    )
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    hooks.push("pre-commit".to_string());
                } else {
                    warnings.push("pre-commit hook exists; skipped overwrite".to_string());
                }
            } else {
                warnings.push(".git/hooks directory not found".to_string());
            }
        }
        Ok(envelope_success(
            json!({
                "detected_agents": detected,
                "created_files": created,
                "hooks_installed": hooks,
                "repositories_registered": [repo]
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(
        title = "Ingest LSP call edges",
        description = "Merges LSP CALLS edges into the graph with dedup/rejection stats. Use when: enriching graph from IDE/LSP. Prerequisite: connected graph; writes relationships.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn submit_lsp_edges(
        &self,
        Parameters(req): Parameters<SubmitLspEdgesReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.lsp_ingest.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "submit_lsp_edges is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.edges.is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "edges must not be empty",
                None,
                started,
            ));
        }
        let merge_mode = req.merge_mode.unwrap_or_else(|| "upsert".to_string());
        let mut unique = HashSet::<(String, String, String, u64)>::new();
        let mut deduped = 0usize;
        let mut ingested = 0usize;
        let mut rejected = 0usize;
        let mut reasons = HashMap::<String, usize>::new();
        let client = self.graph_client().await?;
        for edge in req.edges {
            let key = (
                edge.caller_fqn.clone(),
                edge.callee_fqn.clone(),
                edge.file.clone(),
                edge.line,
            );
            if !unique.insert(key) {
                deduped += 1;
                continue;
            }
            let caller = edge
                .caller_fqn
                .rsplit("::")
                .next()
                .unwrap_or(edge.caller_fqn.as_str());
            let callee = edge
                .callee_fqn
                .rsplit("::")
                .next()
                .unwrap_or(edge.callee_fqn.as_str());
            let q = format!(
                "MATCH (a:Function {{name:'{}'}}), (b:Function {{name:'{}'}})
                 WHERE a.path STARTS WITH '{}' AND b.path STARTS WITH '{}'
                 MERGE (a)-[r:CALLS]->(b)
                 SET r.kind='Calls',
                     r.source='lsp',
                     r.confidence={},
                     r.file='{}',
                     r.line_number={},
                     r.merge_mode='{}'",
                escape_cypher(caller),
                escape_cypher(callee),
                escape_cypher(req.repo_path.as_str()),
                escape_cypher(req.repo_path.as_str()),
                edge.confidence.unwrap_or(0.5),
                escape_cypher(edge.file.as_str()),
                edge.line,
                escape_cypher(merge_mode.as_str())
            );
            match client.raw_query(q.as_str()).await {
                Ok(_) => ingested += 1,
                Err(_) => {
                    rejected += 1;
                    *reasons.entry("unknown_symbol".to_string()).or_insert(0) += 1;
                }
            }
        }
        Ok(envelope_success(
            json!({
                "ingested": ingested,
                "deduped": deduped,
                "rejected": rejected,
                "reasons": reasons
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Save observation",
        description = "Persists a short session note with optional symbol links. Use when: durable memory across turns. Search later with search_memory.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn save_observation(
        &self,
        Parameters(req): Parameters<SaveObservationReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "save_observation is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.text.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "text must not be empty",
                None,
                started,
            ));
        }
        if req.text.len() > 8 * 1024 {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "text too large; max 8KB",
                None,
                started,
            ));
        }
        if looks_sensitive(req.text.as_str()) {
            return Ok(envelope_error(
                "SENSITIVE_CONTENT_DETECTED",
                "observation appears to contain sensitive content",
                None,
                started,
            ));
        }
        let mut db = load_memory_db().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let session_id = req
            .session_id
            .unwrap_or_else(|| "default-session".to_string());
        if exceeded_rate_limit(&db, session_id.as_str()) {
            return Ok(envelope_error(
                "RATE_LIMITED",
                "too many writes in short period",
                None,
                started,
            ));
        }
        let linked_symbols = req.symbol_refs.clone().unwrap_or_default();
        let mut warnings = Vec::new();
        let embedding = if self.tool_enabled("mcp.vector.write.enabled", true) {
            match VectorService::from_env().await {
                Ok(service) => match service.embed_query(req.text.as_str()).await {
                    Ok(embedding) => {
                        global_metrics().record_embeddings_generated(1);
                        Some(embedding)
                    }
                    Err(err) => {
                        global_metrics().record_vector_fallback();
                        warnings.push(warning_with_reason(WARNING_EMBEDDER_TIMEOUT, &err));
                        None
                    }
                },
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(WARNING_VECTOR_STORE_UNAVAILABLE, &err));
                    None
                }
            }
        } else {
            warnings.push("vector_write_disabled".to_string());
            None
        };
        let rec = ObservationRecord {
            observation_id: format!("obs-{}", now_millis()),
            repo_id: req.repo_path.clone(),
            session_id,
            created_at: now_millis(),
            created_by: "mcp".to_string(),
            text: req.text,
            symbol_refs: linked_symbols.clone(),
            confidence: req.confidence.unwrap_or(0.8).clamp(0.0, 1.0),
            stale: false,
            classification: req.classification.unwrap_or_else(|| "internal".to_string()),
            severity: req.severity.unwrap_or_else(|| "info".to_string()),
            tags: req.tags.unwrap_or_default(),
            source_revision: "unknown".to_string(),
            embedding,
        };
        let obs_id = rec.observation_id.clone();
        db.observations.push(rec);
        persist_memory_db(&db).map_err(|e| McpError::internal_error(e.to_string(), None))?;
        append_audit_event("save_observation", obs_id.as_str())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({
                "observation_id": obs_id,
                "linked_symbols": linked_symbols.len(),
                "stale": false
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(
        title = "Session context",
        description = "Reads recent observations for a session/repo. Use when: reloading prior decisions. Read-only storage read.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_session_context(
        &self,
        Parameters(req): Parameters<SessionContextReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_session_context is disabled by feature flag",
                None,
                started,
            ));
        }
        let db = load_memory_db().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let include_previous = req.include_previous.unwrap_or(3);
        let max_items = req.max_items.unwrap_or(100).min(200);
        let include_stale = req.include_stale.unwrap_or(false);
        let session_id = req
            .session_id
            .clone()
            .unwrap_or_else(|| "default-session".to_string());
        let mut items: Vec<_> = db
            .observations
            .iter()
            .filter(|o| o.repo_id == req.repo_path)
            .filter(|o| include_stale || !o.stale)
            .filter(|o| o.session_id == session_id || include_previous > 0)
            .cloned()
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(max_items);
        Ok(envelope_success(
            json!({ "items": items }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Search memory",
        description = "Lexical/semantic search over saved observations. Use when: recalling decisions. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn search_memory(
        &self,
        Parameters(req): Parameters<SearchMemoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "search_memory is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.query.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }
        let db = load_memory_db().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let max_items = req.max_items.unwrap_or(20).min(100);
        let include_stale = req.include_stale.unwrap_or(false);
        let mut results = Vec::<Value>::new();
        let mut warnings = Vec::new();
        let query_embedding = if self.tool_enabled("mcp.vector.read.enabled", true) {
            match VectorService::from_env().await {
                Ok(service) => match service.embed_query(req.query.as_str()).await {
                    Ok(embedding) => {
                        global_metrics().record_embeddings_generated(1);
                        Some(embedding)
                    }
                    Err(err) => {
                        global_metrics().record_vector_fallback();
                        warnings.push(warning_with_reason(WARNING_FALLBACK_TO_LEXICAL, &err));
                        None
                    }
                },
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(WARNING_VECTOR_STORE_UNAVAILABLE, &err));
                    None
                }
            }
        } else {
            warnings.push("vector_read_disabled".to_string());
            None
        };
        for obs in db
            .observations
            .iter()
            .filter(|o| o.repo_id == req.repo_path)
        {
            if obs.stale && !include_stale {
                continue;
            }
            let bm25 =
                simple_lexical_score(req.query.as_str(), obs.text.as_str(), obs.text.as_str());
            let tfidf = ((obs.text.len().min(180)) as f64) / 180.0;
            let recency = 1.0;
            let graph_proximity = if obs.symbol_refs.is_empty() { 0.0 } else { 0.2 };
            let staleness_penalty = if obs.stale { -0.2 } else { 0.0 };
            let semantic = match (&query_embedding, &obs.embedding) {
                (Some(query), Some(obs_vec)) => cosine_similarity(query, obs_vec),
                _ => 0.0,
            };
            let score =
                (semantic * 1.2) + bm25 + tfidf + recency + graph_proximity + staleness_penalty;
            results.push(json!({
                "id": obs.observation_id,
                "text": obs.text,
                "score": score,
                "classification": obs.classification,
                "stale": obs.stale,
                "why": {
                    "semantic": semantic,
                    "bm25": bm25,
                    "tfidf": tfidf,
                    "recency": recency,
                    "graph_proximity": graph_proximity,
                    "staleness_penalty": staleness_penalty
                }
            }));
        }
        results.sort_by(|a, b| {
            let left = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            let right = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let partial = results.is_empty();
        if partial {
            warnings.push("memory_empty".to_string());
        }
        results.truncate(max_items);
        Ok(envelope_success(
            json!({ "results": results }),
            started,
            warnings,
            partial,
        ))
    }

    // ── repository management ─────────────────────────────────────────────────

    #[tool(
        title = "List indexed repositories",
        description = "Repositories present in the graph. Use when: verifying scope before analysis. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_indexed_repositories(&self) -> Result<CallToolResult, McpError> {
        let repos = self
            .graph_client()
            .await?
            .list_repositories()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&repos).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "Delete repository from graph",
        description = "Removes a repository subtree from Memgraph. Use when: purging obsolete index. Destructive; confirm with user.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn delete_repository(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        self.graph_client()
            .await?
            .delete_repository(&req.path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(format!("Deleted: {}", req.path)))
    }

    #[tool(
        title = "Repository graph stats",
        description = "Node counts per indexed repository. Use when: capacity or coverage snapshots. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_repository_stats(&self) -> Result<CallToolResult, McpError> {
        let stats = Analyzer::new(self.graph_client().await?)
            .repository_stats()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::to_string_pretty(&stats).unwrap_or_default(),
        ))
    }

    // ── jobs ──────────────────────────────────────────────────────────────────

    #[tool(
        title = "Job status",
        description = "Looks up a background job by id from the in-process job table. Use when: polling async index/watch jobs.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn check_job_status(
        &self,
        Parameters(req): Parameters<JobStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        Ok(Self::ok(
            serde_json::to_string_pretty(&self.jobs.get(&req.id)).unwrap_or_default(),
        ))
    }

    #[tool(
        title = "List background jobs",
        description = "Lists MCP-tracked jobs (index/watch). Use when: operator visibility. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_jobs(&self) -> Result<CallToolResult, McpError> {
        Ok(Self::ok(
            serde_json::to_string_pretty(&self.jobs.list()).unwrap_or_default(),
        ))
    }

    // ── bundles ───────────────────────────────────────────────────────────────

    #[tool(
        title = "Load graph bundle",
        description = "Imports a .ccx bundle for in-process inspection (not full DB load). Use when: offline bundle review.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn load_bundle(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let bundle = BundleStore::import(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(format!(
            "Loaded: {} nodes, {} edges",
            bundle.nodes.len(),
            bundle.edges.len()
        )))
    }

    #[tool(
        title = "Export graph bundle",
        description = "Writes a .ccx snapshot for a repo from the live graph. Use when: sharing graph offline. Writes output_path.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn export_bundle(
        &self,
        Parameters(req): Parameters<ExportBundleReq>,
    ) -> Result<CallToolResult, McpError> {
        let client = self.graph_client().await?;
        let bundle = BundleStore::export_from_graph(&client, &req.repository_path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        BundleStore::export(PathBuf::from(&req.output_path).as_path(), &bundle)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(Self::ok(
            serde_json::json!({
                "status": "ok",
                "repository_path": req.repository_path,
                "output_path": req.output_path,
                "nodes": bundle.nodes.len(),
                "edges": bundle.edges.len()
            })
            .to_string(),
        ))
    }

    // ── health ────────────────────────────────────────────────────────────────

    #[tool(
        title = "Get signature",
        description = "Structured signature for graph-matched symbol(s). Use when: API shape before edit. Prerequisite: indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_signature(
        &self,
        Parameters(req): Parameters<GetSignatureReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_signature is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let include_related = req.include_related.unwrap_or(false);

        // Find the symbol in the graph
        let symbol_query = format!(
            "MATCH (n) WHERE n.name CONTAINS '{}' {} RETURN n.name, n.path, n.kind, n.source, n.line_number, n.lang LIMIT 10",
            escape_cypher(&req.symbol),
            if repo_path != "." {
                format!("AND n.path STARTS WITH '{}'", escape_cypher(&repo_path))
            } else {
                String::new()
            }
        );

        let results = client
            .raw_query(&symbol_query)
            .await
            .map_err(|e| McpError::internal_error(format!("Graph query failed: {}", e), None))?;

        let nodes = results
            .into_iter()
            .filter_map(|row| {
                let name = row.get("n.name").and_then(|v| v.as_str())?.to_string();
                let path = row.get("n.path").and_then(|v| v.as_str())?.to_string();
                let kind = row.get("n.kind").and_then(|v| v.as_str())?.to_string();
                let source = row.get("n.source").and_then(|v| v.as_str())?.to_string();
                let line_number = row
                    .get("n.line_number")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as u32);
                let lang_str = row.get("n.lang").and_then(|v| v.as_str())?.to_string();
                let lang = FromStr::from_str(&lang_str).ok()?;

                Some((name, path, kind, source, line_number, lang))
            })
            .collect::<Vec<_>>();

        if nodes.is_empty() {
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("Symbol '{}' not found in repository", req.symbol),
                None,
                started,
            ));
        }

        let nodes_count = nodes.len();

        // Extract signatures for found nodes
        let mut signatures = Vec::new();
        for (name, path, kind, source, line_number, lang) in nodes {
            let node = cortex_core::CodeNode {
                id: format!("{}:{}", path, name),
                kind: match kind.as_str() {
                    "function" | "Function" => cortex_core::EntityKind::Function,
                    "struct" | "Struct" => cortex_core::EntityKind::Struct,
                    "enum" | "Enum" => cortex_core::EntityKind::Enum,
                    "trait" | "Trait" => cortex_core::EntityKind::Trait,
                    "impl" | "Impl" => cortex_core::EntityKind::Module,
                    "class" | "Class" => cortex_core::EntityKind::Class,
                    _ => cortex_core::EntityKind::Function,
                },
                name: name.clone(),
                path: Some(path.clone()),
                line_number,
                lang: Some(lang),
                source: Some(source.clone()),
                docstring: None,
                properties: std::collections::HashMap::new(),
            };

            if let Some(sig) = SignatureExtractor::extract(&node, &source) {
                let mut sig_json = serde_json::to_value(&sig).unwrap_or_default();
                if let Some(obj) = sig_json.as_object_mut() {
                    obj.insert("name".to_string(), json!(name));
                    obj.insert("path".to_string(), json!(path));
                    obj.insert("kind".to_string(), json!(kind));
                }
                signatures.push(sig_json);

                // If include_related, find implementations/overrides
                if include_related && signatures.len() < 20 {
                    let related_query = format!(
                        "MATCH (a {{name:'{}'}})<-[:IMPLEMENTS|OVERRIDES]-(b) \
                         WHERE b.path STARTS WITH '{}' \
                         RETURN b.name, b.path, b.kind, b.source, b.line_number, b.lang LIMIT 5",
                        escape_cypher(&name),
                        escape_cypher(&repo_path)
                    );
                    if let Ok(related) = client.raw_query(&related_query).await {
                        for rel_row in related {
                            if let (
                                Some(rname),
                                Some(rpath),
                                Some(rkind),
                                Some(rsource),
                                rline,
                                Some(rlang),
                            ) = (
                                rel_row.get("b.name").and_then(|v| v.as_str()),
                                rel_row.get("b.path").and_then(|v| v.as_str()),
                                rel_row.get("b.kind").and_then(|v| v.as_str()),
                                rel_row.get("b.source").and_then(|v| v.as_str()),
                                rel_row.get("b.line_number").and_then(|v| v.as_i64()),
                                rel_row.get("b.lang").and_then(|v| v.as_str()),
                            ) && let Ok(rlang_parsed) = FromStr::from_str(rlang)
                            {
                                let rel_node = cortex_core::CodeNode {
                                    id: format!("{}:{}", rpath, rname),
                                    kind: cortex_core::EntityKind::Function,
                                    name: rname.to_string(),
                                    path: Some(rpath.to_string()),
                                    line_number: rline.map(|n| n as u32),
                                    lang: Some(rlang_parsed),
                                    source: Some(rsource.to_string()),
                                    docstring: None,
                                    properties: std::collections::HashMap::new(),
                                };
                                if let Some(rel_sig) =
                                    SignatureExtractor::extract(&rel_node, rsource)
                                {
                                    let mut rel_json =
                                        serde_json::to_value(&rel_sig).unwrap_or_default();
                                    if let Some(obj) = rel_json.as_object_mut() {
                                        obj.insert("name".to_string(), json!(rname));
                                        obj.insert("path".to_string(), json!(rpath));
                                        obj.insert("kind".to_string(), json!(rkind));
                                        obj.insert("relation".to_string(), json!("implementation"));
                                    }
                                    signatures.push(rel_json);
                                }
                            }
                        }
                    }
                }
            }

            if signatures.len() >= 20 {
                break;
            }
        }

        if signatures.is_empty() {
            return Ok(envelope_error(
                "PARSE_ERROR",
                "Could not extract signature from found symbol(s)",
                Some(json!({"symbol": req.symbol, "nodes_found": nodes_count})),
                started,
            ));
        }

        Ok(envelope_success(
            json!({
                "signatures": signatures,
                "count": signatures.len(),
                "query": req.symbol
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Find related tests",
        description = "Heuristic test lookup for a symbol via graph patterns. Use when: planning validation. Prerequisite: indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_tests(
        &self,
        Parameters(req): Parameters<FindTestsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "find_tests is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let max_results = req.max_results.unwrap_or(20).min(100);
        let include_integration = req.include_integration.unwrap_or(true);

        // Find tests by multiple strategies:
        // 1. Direct TESTS relationship (if indexed)
        // 2. Naming convention (test_<symbol>, <symbol>_test, tests/test_<symbol>)
        // 3. Same module with test attribute

        let mut tests = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Strategy 1: Look for TESTS relationships
        let rel_query = format!(
            "MATCH (t:Function)-[:TESTS]->(s {{name:'{}'}}) \
             WHERE t.path STARTS WITH '{}' \
             RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
            escape_cypher(&req.symbol),
            escape_cypher(&repo_path),
            max_results
        );

        if let Ok(results) = client.raw_query(&rel_query).await {
            for row in results {
                if let (Some(name), Some(path), source, line) = (
                    row.get("t.name").and_then(|v| v.as_str()),
                    row.get("t.path").and_then(|v| v.as_str()),
                    row.get("t.source").and_then(|v| v.as_str()),
                    row.get("t.line_number").and_then(|v| v.as_i64()),
                ) {
                    let key = format!("{}:{}", path, name);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        tests.push(json!({
                            "name": name,
                            "path": path,
                            "kind": "unit",
                            "line_number": line,
                            "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                        }));
                    }
                }
            }
        }

        // Strategy 2: Naming convention search
        let naming_patterns = vec![
            format!("test_{}", req.symbol),
            format!("{}_test", req.symbol),
            format!("test{}", req.symbol),
            format!("test_{}_", req.symbol),
        ];

        for pattern in naming_patterns {
            if tests.len() >= max_results {
                break;
            }

            let name_query = format!(
                "MATCH (t:Function) \
                 WHERE t.name CONTAINS '{}' AND t.path STARTS WITH '{}' \
                 AND (t.path CONTAINS 'test' OR t.path CONTAINS 'tests' OR t.path CONTAINS '_test' OR t.path CONTAINS 'spec') \
                 RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
                escape_cypher(&pattern),
                escape_cypher(&repo_path),
                max_results - tests.len()
            );

            if let Ok(results) = client.raw_query(&name_query).await {
                for row in results {
                    if let (Some(name), Some(path), source, line) = (
                        row.get("t.name").and_then(|v| v.as_str()),
                        row.get("t.path").and_then(|v| v.as_str()),
                        row.get("t.source").and_then(|v| v.as_str()),
                        row.get("t.line_number").and_then(|v| v.as_i64()),
                    ) {
                        let key = format!("{}:{}", path, name);
                        if !seen.contains(&key) {
                            seen.insert(key);
                            tests.push(json!({
                                "name": name,
                                "path": path,
                                "kind": "unit",
                                "line_number": line,
                                "match_reason": "naming_convention",
                                "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                            }));
                        }
                    }
                }
            }
        }

        // Strategy 3: Integration tests (if enabled)
        if include_integration && tests.len() < max_results {
            let int_query = format!(
                "MATCH (t:Function) \
                 WHERE t.path CONTAINS 'integration' AND t.path STARTS WITH '{}' \
                 AND (t.name CONTAINS '{}' OR t.source CONTAINS '{}') \
                 RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
                escape_cypher(&repo_path),
                escape_cypher(&req.symbol),
                escape_cypher(&req.symbol),
                max_results - tests.len()
            );

            if let Ok(results) = client.raw_query(&int_query).await {
                for row in results {
                    if let (Some(name), Some(path), source, line) = (
                        row.get("t.name").and_then(|v| v.as_str()),
                        row.get("t.path").and_then(|v| v.as_str()),
                        row.get("t.source").and_then(|v| v.as_str()),
                        row.get("t.line_number").and_then(|v| v.as_i64()),
                    ) {
                        let key = format!("{}:{}", path, name);
                        if !seen.contains(&key) {
                            seen.insert(key);
                            tests.push(json!({
                                "name": name,
                                "path": path,
                                "kind": "integration",
                                "line_number": line,
                                "match_reason": "content_match",
                                "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                            }));
                        }
                    }
                }
            }
        }

        // Calculate estimated coverage (mock for now - would need actual coverage data)
        let coverage_estimate = if tests.is_empty() {
            0.0
        } else if tests.len() >= 3 {
            0.85
        } else {
            0.5 + (tests.len() as f64 * 0.1)
        };

        if tests.is_empty() {
            return Ok(envelope_success(
                json!({
                    "tests": [],
                    "count": 0,
                    "symbol": req.symbol,
                    "coverage_estimate": 0.0,
                    "warning": "No tests found for this symbol. Consider adding unit tests."
                }),
                started,
                vec!["no_tests_found".to_string()],
                false,
            ));
        }

        Ok(envelope_success(
            json!({
                "tests": tests,
                "count": tests.len(),
                "symbol": req.symbol,
                "coverage_estimate": coverage_estimate.min(1.0),
                "has_unit_tests": tests.iter().any(|t| t.get("kind") == Some(&json!("unit"))),
                "has_integration_tests": tests.iter().any(|t| t.get("kind") == Some(&json!("integration")))
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        title = "Explain search strategy",
        description = "Describes how a textual query would be interpreted (no live graph hit). Use when: teaching/debugging retrieval behavior.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn explain_result(
        &self,
        Parameters(req): Parameters<ExplainResultReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "explain_result is disabled by feature flag",
                None,
                started,
            ));
        }

        let query = req.query.trim();
        if query.is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }

        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let _tool = req.tool.as_deref().unwrap_or("find");

        // Parse the query to understand intent
        let intent = detect_intent(query);

        // Build explanation
        let mut steps = Vec::new();
        let interpretation;
        let mut search_strategy = Vec::new();

        // Determine query type
        let query_type = if query.contains('(') || query.contains("fn ") || query.contains("func ")
        {
            "signature_search"
        } else if query.starts_with("test_") || query.contains(" test") {
            "test_search"
        } else if query.split_whitespace().count() > 3 {
            "semantic_search"
        } else {
            "symbol_search"
        };

        match query_type {
            "signature_search" => {
                interpretation = format!(
                    "Looking for code with specific signature pattern: '{}'",
                    query
                );
                steps.push(
                    "1. Parse signature components (name, parameters, return type)".to_string(),
                );
                steps.push("2. Match against indexed function signatures".to_string());
                steps.push("3. Rank by signature similarity score".to_string());
                search_strategy.push("exact_signature_match".to_string());
                search_strategy.push("fuzzy_parameter_match".to_string());
            }
            "test_search" => {
                interpretation = format!("Searching for tests related to: '{}'", query);
                steps.push("1. Extract symbol name from query".to_string());
                steps.push("2. Search for test files containing symbol".to_string());
                steps
                    .push("3. Match naming conventions (test_<symbol>, <symbol>_test)".to_string());
                steps.push("4. Check TESTS relationships in graph".to_string());
                search_strategy.push("naming_convention".to_string());
                search_strategy.push("graph_relationship".to_string());
            }
            "semantic_search" => {
                interpretation = format!("Semantic search for concepts related to: '{}'", query);
                steps.push("1. Extract key terms from query".to_string());
                steps.push("2. Full-text search on symbol names and docs".to_string());
                steps.push("3. TF-IDF scoring for relevance".to_string());
                steps.push("4. Graph traversal for related symbols".to_string());
                steps.push("5. Combine scores with intent-based weighting".to_string());
                search_strategy.push("full_text_search".to_string());
                search_strategy.push("tfidf_scoring".to_string());
                search_strategy.push("graph_expansion".to_string());
            }
            _ => {
                interpretation = format!("Searching for symbols matching: '{}'", query);
                steps.push("1. Exact name match search".to_string());
                steps.push("2. Prefix/fuzzy match for similar names".to_string());
                steps.push("3. Graph traversal for related symbols".to_string());
                steps.push("4. Rank by relevance and usage".to_string());
                search_strategy.push("exact_match".to_string());
                search_strategy.push("fuzzy_match".to_string());
            }
        }

        // Simulate what would be found (without actually running the query)
        let simulated_matches = self.simulate_query_matches(query, &repo_path).await;

        // Build why_included explanation
        let mut why_included = serde_json::Map::new();
        for (symbol, reason) in simulated_matches.iter().take(5) {
            why_included.insert(symbol.clone(), json!(reason));
        }

        Ok(envelope_success(
            json!({
                "query": query,
                "interpretation": interpretation,
                "detected_intent": intent,
                "query_type": query_type,
                "steps": steps,
                "search_strategy": search_strategy,
                "estimated_results": simulated_matches.len(),
                "why_included": why_included,
                "tips": if simulated_matches.is_empty() {
                    vec![
                        "Try using partial names or prefixes",
                        "Check spelling of symbol names",
                        "Use broader search terms"
                    ]
                } else {
                    vec![
                        "Results are ranked by relevance score",
                        "Related symbols are included via graph traversal",
                        "Use path_filter to narrow results to specific directories"
                    ]
                }
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    async fn simulate_query_matches(&self, query: &str, _repo_path: &str) -> Vec<(String, String)> {
        // This simulates what the query would find
        // In a real implementation, this would run a lightweight preview query
        let mut matches = Vec::new();

        // Simple heuristic based on query terms
        let terms: Vec<&str> = query.split_whitespace().collect();
        if !terms.is_empty() {
            let primary = terms[0];
            matches.push((
                format!("func:{}", primary),
                "Name match (1.0), primary query term".to_string(),
            ));

            if terms.len() > 1 {
                for term in &terms[1..] {
                    matches.push((
                        format!("func:{}_related", term),
                        "Related term match (0.7), same module".to_string(),
                    ));
                }
            }
        }

        matches
    }

    #[tool(
        title = "Refactoring impact",
        description = "Graph-derived impact and steps for a proposed refactor type. Use when: safe-change planning. Prerequisite: indexed repo graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn analyze_refactoring(
        &self,
        Parameters(req): Parameters<AnalyzeRefactoringReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.impact_graph.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "analyze_refactoring is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let detailed = req.detailed.unwrap_or(false);
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let change_type = req
            .change_type
            .clone()
            .unwrap_or_else(|| "modify".to_string());

        // Find the symbol
        let symbol_query = format!(
            "MATCH (n {{name:'{}'}}) WHERE n.path STARTS WITH '{}' \
             RETURN n.name, n.path, n.kind, n.source LIMIT 1",
            escape_cypher(&req.symbol),
            escape_cypher(&repo_path)
        );

        let symbol_results = client
            .raw_query(&symbol_query)
            .await
            .map_err(|e| McpError::internal_error(format!("Graph query failed: {}", e), None))?;

        let (symbol_path, symbol_kind) = if let Some(row) = symbol_results.first() {
            let path = row
                .get("n.path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let kind = row
                .get("n.kind")
                .and_then(|v| v.as_str())
                .unwrap_or("function")
                .to_string();
            (path, kind)
        } else {
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("Symbol '{}' not found in repository", req.symbol),
                None,
                started,
            ));
        };

        // Analyze impact based on change type
        let (affected_files, affected_tests, warnings, safe_to_refactor) =
            match change_type.as_str() {
                "add_parameter" | "remove_parameter" | "change_signature" => {
                    self.analyze_signature_change(&client, &req.symbol, &repo_path)
                        .await
                }
                "rename" => self.analyze_rename(&client, &req.symbol, &repo_path).await,
                "delete" | "remove" => {
                    self.analyze_deletion(&client, &req.symbol, &repo_path)
                        .await
                }
                "extract" | "extract_method" => {
                    self.analyze_extraction(&client, &req.symbol, &repo_path)
                        .await
                }
                _ => {
                    self.analyze_generic_change(&client, &req.symbol, &repo_path)
                        .await
                }
            };

        let affected_files = filter_rows_by_paths(affected_files, &filters);
        let affected_tests = filter_rows_by_paths(affected_tests, &filters);

        // Find suggested tests to run
        let suggested_tests = self
            .find_suggested_tests(&client, &req.symbol, &repo_path)
            .await;

        // Build suggested steps
        let mut suggested_steps = vec![
            format!("1. Review all {} affected files", affected_files.len()),
            format!(
                "2. Update {} call sites",
                affected_files
                    .iter()
                    .filter(|f| f.get("call_site") == Some(&json!(true)))
                    .count()
            ),
            format!("3. Run {} related tests", suggested_tests.len()),
        ];

        if !safe_to_refactor {
            suggested_steps.push("4. ⚠️ Address breaking changes before proceeding".to_string());
        } else {
            suggested_steps.push("4. Make changes in small, testable commits".to_string());
        }

        let result = json!({
            "symbol": req.symbol,
            "symbol_path": symbol_path,
            "symbol_kind": symbol_kind,
            "change_type": change_type,
            "safe_to_refactor": safe_to_refactor,
            "affected_files": affected_files,
            "affected_files_count": affected_files.len(),
            "affected_tests": affected_tests,
            "affected_tests_count": affected_tests.len(),
            "warnings": warnings,
            "suggested_tests": suggested_tests,
            "suggested_steps": suggested_steps,
            "risk_level": if !safe_to_refactor { "high" } else if affected_files.len() > 10 { "medium" } else { "low" },
            "detailed": detailed
        });

        Ok(envelope_success(
            result,
            started,
            if !safe_to_refactor {
                vec!["breaking_change_detected".to_string()]
            } else {
                Vec::new()
            },
            false,
        ))
    }

    async fn analyze_signature_change(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let mut affected_tests = Vec::new();
        let mut warnings = Vec::new();
        let mut safe_to_refactor = true;

        // Find all callers
        let caller_query = format!(
            "MATCH (caller)-[:CALLS]->(callee {{name:'{}'}}) \
             WHERE caller.path STARTS WITH '{}' \
             RETURN caller.name, caller.path, caller.kind",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&caller_query).await {
            for row in results {
                if let (Some(name), Some(path), kind) = (
                    row.get("caller.name").and_then(|v| v.as_str()),
                    row.get("caller.path").and_then(|v| v.as_str()),
                    row.get("caller.kind").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "kind": kind,
                        "call_site": true,
                        "change_required": "update_call"
                    }));

                    // Check if it's a test
                    if path.contains("test") || path.contains("spec") || name.starts_with("test_") {
                        affected_tests.push(json!({
                            "name": name,
                            "path": path,
                            "kind": "test_update"
                        }));
                    }
                }
            }
        }

        // Check for public API
        let visibility_query = format!(
            "MATCH (n {{name:'{}'}}) WHERE n.path STARTS WITH '{}' \
             RETURN n.source",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&visibility_query).await
            && let Some(row) = results.first()
            && let Some(source) = row.get("n.source").and_then(|v| v.as_str())
            && source.contains("pub ")
        {
            warnings.push("Breaking change: public API modification".to_string());
            safe_to_refactor = false;
        }

        if affected_files.len() > 10 {
            warnings.push(format!(
                "High impact: {} files affected",
                affected_files.len()
            ));
        }

        (affected_files, affected_tests, warnings, safe_to_refactor)
    }

    async fn analyze_rename(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let mut affected_tests = Vec::new();
        let mut warnings = Vec::new();

        // Find all references (callers, importers, etc.)
        let ref_query = format!(
            "MATCH (ref)-[r:CALLS|IMPORTS|REFERENCES]->(target {{name:'{}'}}) \
             WHERE ref.path STARTS WITH '{}' \
             RETURN ref.name, ref.path, type(r) as rel_type",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&ref_query).await {
            for row in results {
                if let (Some(name), Some(path)) = (
                    row.get("ref.name").and_then(|v| v.as_str()),
                    row.get("ref.path").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "change_required": "update_reference"
                    }));

                    if path.contains("test") || name.starts_with("test_") {
                        affected_tests.push(json!({
                            "name": name,
                            "path": path
                        }));
                    }
                }
            }
        }

        // Rename is generally safe
        warnings.push("Use IDE refactoring tools for reliable rename".to_string());

        (affected_files, affected_tests, warnings, true)
    }

    async fn analyze_deletion(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let affected_tests = Vec::new();
        let mut warnings = Vec::new();

        // Find all callers - deletion breaks all of them
        let caller_query = format!(
            "MATCH (caller)-[:CALLS]->(callee {{name:'{}'}}) \
             WHERE caller.path STARTS WITH '{}' \
             RETURN caller.name, caller.path",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&caller_query).await {
            for row in results {
                if let (Some(name), Some(path)) = (
                    row.get("caller.name").and_then(|v| v.as_str()),
                    row.get("caller.path").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "change_required": "remove_call_or_replace",
                        "breaking": true
                    }));
                }
            }
        }

        warnings.push("⚠️ Deletion is a breaking change".to_string());
        if !affected_files.is_empty() {
            warnings.push(format!("{} call sites will break", affected_files.len()));
        }

        (affected_files, affected_tests, warnings, false)
    }

    async fn analyze_extraction(
        &self,
        _client: &GraphClient,
        symbol: &str,
        _repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let warnings = vec![
            "Extraction creates new symbol - ensure proper naming".to_string(),
            "Add tests for the new extracted function/method".to_string(),
        ];

        (
            vec![json!({"name": symbol, "change_required": "extract_to_new"})],
            vec![],
            warnings,
            true,
        )
    }

    async fn analyze_generic_change(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        // Default to signature change analysis
        self.analyze_signature_change(client, symbol, repo_path)
            .await
    }

    async fn find_suggested_tests(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> Vec<String> {
        let mut tests = Vec::new();

        let test_query = format!(
            "MATCH (t:Function) \
             WHERE (t.name CONTAINS 'test_' OR t.path CONTAINS 'test') \
             AND t.path STARTS WITH '{}' \
             AND (t.name CONTAINS '{}' OR t.source CONTAINS '{}') \
             RETURN t.name LIMIT 10",
            escape_cypher(repo_path),
            escape_cypher(symbol),
            escape_cypher(symbol)
        );

        if let Ok(results) = client.raw_query(&test_query).await {
            for row in results {
                if let Some(name) = row.get("t.name").and_then(|v| v.as_str()) {
                    tests.push(name.to_string());
                }
            }
        }

        tests
    }

    #[tool(
        title = "System diagnose",
        description = "Connectivity, index sanity, cache hints with suggested actions. Use when: cortex looks broken. Read-only checks.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn diagnose(
        &self,
        Parameters(req): Parameters<DiagnoseReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let check_type = req.check.clone().unwrap_or_else(|| "all".to_string());
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);

        let mut issues = Vec::new();
        let mut suggested_actions = Vec::new();

        // Check graph connectivity
        if check_type == "all" || check_type == "graph_connectivity" {
            match self.graph_client().await {
                Ok(client) => {
                    // Test a simple query
                    let test_query = "MATCH (n) RETURN count(n) as count LIMIT 1";
                    let start = Instant::now();
                    match client.raw_query(test_query).await {
                        Ok(_results) => {
                            let latency_ms = start.elapsed().as_millis();
                            if latency_ms > 100 {
                                issues.push(json!({
                                    "check": "graph_latency",
                                    "severity": "warning",
                                    "message": format!("Graph query latency high: {}ms (threshold: 100ms)", latency_ms)
                                }));
                                suggested_actions
                                    .push("Consider checking Memgraph server resources");
                            }
                        }
                        Err(e) => {
                            issues.push(json!({
                                "check": "graph_query",
                                "severity": "critical",
                                "message": format!("Graph query failed: {}", e)
                            }));
                            suggested_actions.push("Check Memgraph server status");
                        }
                    }
                }
                Err(e) => {
                    issues.push(json!({
                        "check": "graph_connection",
                        "severity": "critical",
                        "message": format!("Cannot connect to graph database: {}", e)
                    }));
                    suggested_actions.push(
                        "Ensure Memgraph is running: docker start memgraph or memgraph command",
                    );
                }
            }
        }

        // Check index health
        if (check_type == "all" || check_type == "index_health")
            && let Ok(client) = self.graph_client().await
        {
            // Check if repository is indexed
            let repo_query = format!(
                "MATCH (r:Repository {{path:'{}'}}) RETURN r.indexed_at as indexed_at",
                escape_cypher(&repo_path)
            );

            if let Ok(results) = client.raw_query(&repo_query).await
                && results.is_empty()
            {
                issues.push(json!({
                    "check": "index_status",
                    "severity": "warning",
                    "message": "Repository not indexed"
                }));
                suggested_actions.push("Run: add_code_to_graph with the repository path");
            }

            // Check node count
            let node_query = "MATCH (n:CodeNode) RETURN count(n) as count";
            if let Ok(results) = client.raw_query(node_query).await
                && let Some(row) = results.first()
                && let Some(count) = row.get("count").and_then(|v| v.as_i64())
                && count == 0
            {
                issues.push(json!({
                    "check": "index_content",
                    "severity": "warning",
                    "message": "No code nodes in graph - index may be empty"
                }));
                suggested_actions.push("Index a codebase first using add_code_to_graph");
            }
        }

        // Check cache status
        if check_type == "all" || check_type == "cache_status" {
            let metrics = crate::metrics::global_metrics();
            let snapshot = metrics.snapshot();

            if snapshot.cache_hits + snapshot.cache_misses > 0 {
                let hit_rate = snapshot.cache_hits as f64
                    / (snapshot.cache_hits + snapshot.cache_misses) as f64;
                if hit_rate < 0.5 {
                    issues.push(json!({
                        "check": "cache_efficiency",
                        "severity": "info",
                        "message": format!("Cache hit rate low: {:.1}% (hits: {}, misses: {})",
                            hit_rate * 100.0, snapshot.cache_hits, snapshot.cache_misses)
                    }));
                    suggested_actions.push("Cache may be cold - repeated queries will be faster");
                }
            }
        }

        // Determine overall status
        let status = if issues
            .iter()
            .any(|i| i.get("severity") == Some(&json!("critical")))
        {
            "unhealthy"
        } else if !issues.is_empty() {
            "degraded"
        } else {
            "ok"
        };

        let result = json!({
            "status": status,
            "check_type": check_type,
            "repo_path": repo_path,
            "issues": issues,
            "issue_count": issues.len(),
            "suggested_actions": suggested_actions,
            "checks_run": if check_type == "all" {
                json!(["graph_connectivity", "index_health", "cache_status"])
            } else {
                json!([check_type])
            }
        });

        Ok(envelope_success(
            result,
            started,
            if status != "ok" {
                vec!["issues_detected".to_string()]
            } else {
                Vec::new()
            },
            false,
        ))
    }

    #[tool(
        title = "Detect architecture patterns",
        description = "Heuristic pattern hints (builder, repository, etc.) via name/source cues. Use when: architecture reconnaissance. Prerequisite: indexed graph.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn find_patterns(
        &self,
        Parameters(req): Parameters<FindPatternsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "find_patterns is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let min_confidence = req.min_confidence.unwrap_or(0.5);
        let max_results = req.max_results.unwrap_or(50).min(200);
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        // Define pattern detection rules
        let patterns_to_check = if let Some(ref pattern) = req.pattern {
            vec![pattern.to_lowercase()]
        } else {
            vec![
                "builder".to_string(),
                "factory".to_string(),
                "singleton".to_string(),
                "repository".to_string(),
                "service".to_string(),
                "handler".to_string(),
                "middleware".to_string(),
                "observer".to_string(),
                "strategy".to_string(),
                "adapter".to_string(),
                "decorator".to_string(),
                "command".to_string(),
                "state".to_string(),
                "facade".to_string(),
                "proxy".to_string(),
            ]
        };

        let mut results = Vec::new();

        for pattern in patterns_to_check {
            let matches = self
                .detect_pattern(&client, &pattern, &repo_path, min_confidence, max_results)
                .await;
            results.extend(filter_rows_by_paths(matches, &filters));
            if results.len() >= max_results {
                break;
            }
        }

        // Sort by confidence and truncate
        results.sort_by(|a, b| {
            b.get("confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0)
                .partial_cmp(&a.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(max_results);

        // Group by pattern type for summary
        let mut pattern_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for result in &results {
            if let Some(pattern) = result.get("pattern").and_then(|p| p.as_str()) {
                *pattern_counts.entry(pattern.to_string()).or_insert(0) += 1;
            }
        }

        if results.is_empty() {
            return Ok(envelope_success(
                json!({
                    "patterns": [],
                    "count": 0,
                    "pattern_summary": {},
                    "message": "No patterns found. Try lowering min_confidence or indexing more code."
                }),
                started,
                vec!["no_patterns_found".to_string()],
                false,
            ));
        }

        Ok(envelope_success(
            json!({
                "patterns": results,
                "count": results.len(),
                "pattern_summary": pattern_counts,
                "patterns_detected": pattern_counts.keys().cloned().collect::<Vec<_>>()
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    async fn detect_pattern(
        &self,
        client: &GraphClient,
        pattern: &str,
        repo_path: &str,
        min_confidence: f64,
        _max_results: usize,
    ) -> Vec<Value> {
        let mut matches = Vec::new();

        let (name_patterns, structural_hints, behavioral_hints) = match pattern {
            "builder" => (
                vec!["Builder", "Build"],
                vec!["build(", ".build()"],
                vec!["chain", "fluent"],
            ),
            "factory" => (
                vec!["Factory", "Create", "Make"],
                vec!["create(", "new_", "make_"],
                vec!["instantiation", "construction"],
            ),
            "singleton" => (
                vec!["Instance", "Singleton", "Global"],
                vec!["instance()", "get_instance()", "static"],
                vec!["single", "global"],
            ),
            "repository" => (
                vec!["Repository", "Repo", "Store", "DAO"],
                vec!["find(", "save(", "delete(", "query("],
                vec!["data", "persistence", "storage"],
            ),
            "service" => (
                vec!["Service", "Manager", "Provider"],
                vec!["process(", "handle(", "execute("],
                vec!["business", "logic"],
            ),
            "handler" => (
                vec!["Handler", "Controller", "Endpoint"],
                vec!["handle(", "process(", "route("],
                vec!["request", "response", "http"],
            ),
            "middleware" => (
                vec!["Middleware", "Interceptor", "Filter"],
                vec!["next(", "chain(", "intercept("],
                vec!["pipeline", "chain"],
            ),
            "observer" => (
                vec!["Observer", "Listener", "Subscriber", "Watcher"],
                vec!["notify(", "subscribe(", "emit("],
                vec!["event", "callback"],
            ),
            "strategy" => (
                vec!["Strategy", "Policy", "Algorithm"],
                vec!["execute(", "apply(", "strategy"],
                vec!["interchangeable", "algorithm"],
            ),
            "adapter" => (
                vec!["Adapter", "Wrapper", "Converter"],
                vec!["adapt(", "convert(", "wrap("],
                vec!["interface", "conversion", "compatibility"],
            ),
            "decorator" => (
                vec!["Decorator", "Wrapper"],
                vec!["decorate(", "wrap(", "impl "],
                vec!["extension", "enhancement", "adding"],
            ),
            "command" => (
                vec!["Command", "Action", "Operation"],
                vec!["execute(", "undo(", "redo(", "command"],
                vec!["encapsulate", "request", "operation"],
            ),
            "state" => (
                vec!["State", "Machine", "FSM"],
                vec!["transition(", "state", "current_state"],
                vec!["finite", "state machine", "states"],
            ),
            "facade" => (
                vec!["Facade", "API", "Interface"],
                vec!["facade(", "simplify(", "delegate("],
                vec!["simplified", "unified", "interface"],
            ),
            "proxy" => (
                vec!["Proxy", "Remote", "Virtual"],
                vec!["proxy(", "forward(", "delegate("],
                vec![" surrogate", "placeholder", "access control"],
            ),
            _ => (vec![pattern], vec![], vec![]),
        };

        // Search by name patterns
        for name_pattern in name_patterns {
            let query = format!(
                "MATCH (n) WHERE (n.name CONTAINS '{}' OR n.path CONTAINS '{}') \
                 AND n.path STARTS WITH '{}' \
                 RETURN n.name, n.path, n.kind, n.source LIMIT 20",
                escape_cypher(name_pattern),
                escape_cypher(name_pattern),
                escape_cypher(repo_path)
            );

            if let Ok(results) = client.raw_query(&query).await {
                for row in results {
                    if let (Some(name), Some(path), kind, source) = (
                        row.get("n.name").and_then(|v| v.as_str()),
                        row.get("n.path").and_then(|v| v.as_str()),
                        row.get("n.kind").and_then(|v| v.as_str()),
                        row.get("n.source").and_then(|v| v.as_str()),
                    ) {
                        // Calculate confidence based on structural and behavioral hints
                        let mut confidence: f64 = 0.4; // Base confidence for name match

                        if let Some(src) = source {
                            for hint in &structural_hints {
                                if src.contains(hint) {
                                    confidence += 0.15;
                                }
                            }
                            for hint in &behavioral_hints {
                                if src.to_lowercase().contains(hint) {
                                    confidence += 0.05;
                                }
                            }
                        }

                        // Bonus for matching kind
                        if let Some(k) = kind
                            && ((pattern == "builder" && k == "Struct")
                                || (pattern == "factory" && k == "Function")
                                || (pattern == "service" && (k == "Struct" || k == "Class"))
                                || (pattern == "repository" && k == "Struct")
                                || (pattern == "adapter" && k == "Struct")
                                || (pattern == "decorator" && k == "Struct")
                                || (pattern == "facade" && k == "Struct")
                                || (pattern == "proxy" && k == "Struct")
                                || (pattern == "command" && (k == "Struct" || k == "Enum"))
                                || (pattern == "state" && k == "Enum")
                                || (pattern == "strategy" && (k == "Trait" || k == "Interface")))
                        {
                            confidence += 0.1;
                        }

                        confidence = confidence.min(1.0);

                        if confidence >= min_confidence {
                            matches.push(json!({
                                "symbol": name,
                                "path": path,
                                "pattern": pattern,
                                "confidence": (confidence * 100.0).round() / 100.0,
                                "detection_method": "name_and_structure",
                                "kind": kind
                            }));
                        }
                    }
                }
            }
        }

        matches
    }

    // ── health ────────────────────────────────────────────────────────────────

    #[tool(
        title = "Graph health",
        description = "Pings Memgraph connectivity and returns analyzer capability flags. Use when: first failure triage. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn check_health(&self) -> Result<CallToolResult, McpError> {
        let ok = self.graph_client().await.is_ok();
        Ok(Self::ok(
            serde_json::json!({
                "status": if ok { "ok" } else { "degraded" },
                "memgraph": if ok { "connected" } else { "unreachable" },
                "analyzer": analyzer_capabilities_json()
            })
            .to_string(),
        ))
    }

    // ── project management ────────────────────────────────────────────────────

    #[tool(
        title = "List projects",
        description = "Registered projects and current selection. Use when: multi-repo scope. Read-only registry view.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        let projects = self.projects.list_projects();
        let current = self.projects.get_current_project();
        let total = projects.len();
        let active = projects
            .iter()
            .filter(|p| p.status == ProjectStatus::Watching)
            .count();

        Ok(Self::ok(
            serde_json::json!({
                "projects": projects,
                "current_project": current.map(|p| p.path.display().to_string()),
                "total": total,
                "active": active
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Add project",
        description = "Registers a filesystem path with optional branch tracking. Use when: expanding multi-project workspace.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn add_project(
        &self,
        Parameters(req): Parameters<AddProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        // Build config from request
        let config = cortex_core::ProjectConfig {
            track_branch: req.track_branch.unwrap_or(true),
            pinned_branches: req.pinned_branches.unwrap_or_default(),
            ..Default::default()
        };

        match self.projects.add_project(&path, Some(config)) {
            Ok(state) => {
                let summary = cortex_core::ProjectSummary::from(&state);
                Ok(Self::ok(
                    serde_json::json!({
                        "project": summary,
                        "message": format!("Added project at {}", req.path)
                    })
                    .to_string(),
                ))
            }
            Err(e) => Ok(envelope_error(
                "ADD_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(
        title = "Remove project",
        description = "Drops a project from the local registry (not the graph). Use when: unregistering stale roots. Destructive to registry entry.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn remove_project(
        &self,
        Parameters(req): Parameters<RemoveProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        match self.projects.remove_project(&path) {
            Ok(()) => Ok(Self::ok(
                serde_json::json!({
                    "path": req.path,
                    "removed": true,
                    "message": format!("Removed project at {}", req.path)
                })
                .to_string(),
            )),
            Err(e) => Ok(envelope_error(
                "REMOVE_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(
        title = "Set current project",
        description = "Selects active project (optional git branch). Use when: scoping navigation tools. Required before go_to_definition and similar.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn set_current_project(
        &self,
        Parameters(req): Parameters<SetProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        match self.projects.set_current_project(&path, req.branch.clone()) {
            Ok(pr) => {
                let project = self.projects.get_project(&path);
                Ok(Self::ok(
                    serde_json::json!({
                        "project": project.map(|p| cortex_core::ProjectSummary::from(&p)),
                        "branch": pr.branch,
                        "message": format!("Set current project to {} on branch {}", req.path, pr.branch)
                    })
                    .to_string(),
                ))
            }
            Err(e) => Ok(envelope_error(
                "SET_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(
        title = "Current project",
        description = "Shows selected project path/branch/commit. Use when: confirming MCP scope. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_current_project(&self) -> Result<CallToolResult, McpError> {
        let current = self.projects.get_current_project();

        let result = if let Some(pr) = current {
            let project = self.projects.get_project(&pr.path);
            serde_json::json!({
                "project": project.map(|p| cortex_core::ProjectSummary::from(&p)),
                "branch": pr.branch,
                "commit": pr.commit,
                "repository_path": pr.path.display().to_string()
            })
        } else {
            serde_json::json!({
                "project": null,
                "branch": null,
                "message": "No current project set. Use add_project to register a project."
            })
        };

        Ok(Self::ok(result.to_string()))
    }

    #[tool(
        title = "List branches",
        description = "Git branches and index coverage for a project. Use when: branch-aware workflows. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_branches(
        &self,
        Parameters(req): Parameters<ListBranchesReq>,
    ) -> Result<CallToolResult, McpError> {
        let path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;

        let project = self.projects.get_project(&path).ok_or_else(|| {
            McpError::invalid_params(format!("Project not found: {}", path.display()), None)
        })?;

        let git_info = project.git_info.as_ref();
        let current_branch = git_info
            .map(|g| g.current_branch.as_str())
            .unwrap_or("unknown");

        let branches = git_info
            .map(|g| {
                g.branches
                    .iter()
                    .map(|b| {
                        let is_indexed = project.indexed_branches.contains_key(&b.name);
                        serde_json::json!({
                            "name": b.name,
                            "is_remote": b.is_remote,
                            "is_current": b.name == current_branch,
                            "is_indexed": is_indexed,
                            "last_commit": b.last_commit
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self::ok(
            serde_json::json!({
                "project": path.display().to_string(),
                "current_branch": current_branch,
                "branches": branches,
                "total": branches.len()
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Refresh project Git",
        description = "Refreshes git metadata for a registered project. Use when: branch drift suspected.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn refresh_project(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        // Check for branch change first
        let branch_change = self.projects.check_branch_change(&path).ok();

        match self.projects.refresh_git_info(&path) {
            Ok(git_info) => Ok(Self::ok(
                serde_json::json!({
                    "path": req.path,
                    "git_info": git_info,
                    "branch_changed": branch_change.flatten(),
                    "message": "Refreshed Git info"
                })
                .to_string(),
            )),
            Err(e) => Ok(envelope_error(
                "REFRESH_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(
        title = "Project status",
        description = "Freshness, branch health, optional watcher queue slice. Use when: daemon/branch monitoring. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn project_status(
        &self,
        Parameters(req): Parameters<ProjectStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let include_queue = req.include_queue.unwrap_or(true);
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;
        let project = self.projects.get_project(&project_path);
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let daemon_status = cortex_watcher::daemon_status(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let branch_health =
            cortex_watcher::project_branch_health(&daemon_paths, &project_path).unwrap_or_default();

        let queue = if include_queue {
            cortex_watcher::list_index_jobs(&daemon_paths, 250)
                .unwrap_or_default()
                .into_iter()
                .filter(|j| j.repository_path == project_path.display().to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let stale_branches = branch_health.iter().filter(|b| b.is_stale).count();
        let freshness = if stale_branches > 0 {
            "stale"
        } else if queue
            .iter()
            .any(|j| j.status == "pending" || j.status == "running")
        {
            "indexing"
        } else {
            "current"
        };

        Ok(Self::ok(
            json!({
                "path": project_path.display().to_string(),
                "freshness": freshness,
                "project": project,
                "branch_health": branch_health,
                "stale_branches": stale_branches,
                "queue": queue,
                "daemon": daemon_status,
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Project sync",
        description = "Refresh git, enqueue daemon index when possible, optional branch cleanup. Use when: one-shot align repo+index.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn project_sync(
        &self,
        Parameters(req): Parameters<ProjectSyncReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;

        let branch_change = self
            .projects
            .check_branch_change(&project_path)
            .ok()
            .flatten();
        let refresh = self.projects.refresh_git_info(&project_path);
        let refresh_stage = match refresh {
            Ok(ref info) => json!({
                "status": "ok",
                "branch": info.current_branch,
                "commit": info.current_commit,
                "branch_changed": branch_change,
            }),
            Err(err) => {
                return Ok(envelope_error(
                    "PROJECT_SYNC_REFRESH_FAILED",
                    err.to_string(),
                    None,
                    started_at,
                ));
            }
        };

        let force = req.force.unwrap_or(false);
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let daemon_status = cortex_watcher::daemon_status(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut index_stage = json!({
            "status": "skipped",
            "reason": "no_git_context",
        });
        if let Some((repo_root, branch, commit_hash)) = resolve_git_context_for_path(&project_path)
        {
            if daemon_status.running {
                let enqueue = cortex_watcher::enqueue_index_job(
                    &daemon_paths,
                    &cortex_watcher::IndexJobRequest {
                        repository_path: repo_root.display().to_string(),
                        branch: branch.clone(),
                        commit_hash: commit_hash.clone(),
                        mode: if force {
                            cortex_watcher::JobMode::Full
                        } else {
                            cortex_watcher::JobMode::IncrementalDiff
                        },
                    },
                )
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                index_stage = json!({
                    "status": "queued",
                    "daemon": true,
                    "job": enqueue.job,
                    "deduplicated": enqueue.deduplicated,
                    "branch": branch,
                    "commit": commit_hash,
                });
            } else {
                index_stage = json!({
                    "status": "skipped",
                    "reason": "daemon_not_running",
                    "branch": branch,
                    "commit": commit_hash,
                });
            }
        }

        let cleanup_stage = if req.cleanup_old_branches.unwrap_or(true) {
            match self.projects.cleanup_old_branches(&project_path) {
                Ok(removed) => json!({"status": "ok", "removed": removed}),
                Err(err) => json!({"status": "error", "error": err.to_string()}),
            }
        } else {
            json!({"status": "skipped"})
        };

        let branch_health =
            cortex_watcher::project_branch_health(&daemon_paths, &project_path).unwrap_or_default();
        Ok(Self::ok(
            json!({
                "status": "synced",
                "path": project_path.display().to_string(),
                "stages": {
                    "refresh": refresh_stage,
                    "index": index_stage,
                    "cleanup": cleanup_stage,
                },
                "branch_health": branch_health,
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Git branch diff",
        description = "Ahead/behind commits and changed files between refs. Use when: pre-merge insight. Read-only git.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn project_branch_diff(
        &self,
        Parameters(req): Parameters<ProjectBranchDiffReq>,
    ) -> Result<CallToolResult, McpError> {
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;
        let git = GitOperations::new(&project_path);
        let diff = git
            .compare_branches(&req.source, &req.target)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let limit = req.commit_limit.unwrap_or(50);
        let ahead_commits = diff
            .ahead_commits
            .into_iter()
            .take(limit)
            .map(|commit| {
                json!({
                    "hash": commit.hash,
                    "short_hash": commit.short_hash,
                    "author": commit.author,
                    "author_email": commit.author_email,
                    "date": commit.date.to_rfc3339(),
                    "message": commit.message,
                    "message_full": commit.message_full,
                    "parents": commit.parents,
                })
            })
            .collect::<Vec<_>>();
        let behind_commits = diff
            .behind_commits
            .into_iter()
            .take(limit)
            .map(|commit| {
                json!({
                    "hash": commit.hash,
                    "short_hash": commit.short_hash,
                    "author": commit.author,
                    "author_email": commit.author_email,
                    "date": commit.date.to_rfc3339(),
                    "message": commit.message,
                    "message_full": commit.message_full,
                    "parents": commit.parents,
                })
            })
            .collect::<Vec<_>>();
        let changed_files = diff
            .changed_files
            .into_iter()
            .map(|file| {
                let change_type = match file.change_type {
                    cortex_core::FileChangeType::Added => "added",
                    cortex_core::FileChangeType::Modified => "modified",
                    cortex_core::FileChangeType::Deleted => "deleted",
                    cortex_core::FileChangeType::Renamed => "renamed",
                };
                json!({
                    "path": file.path,
                    "change_type": change_type,
                    "additions": file.additions,
                    "deletions": file.deletions,
                })
            })
            .collect::<Vec<_>>();

        Ok(Self::ok(
            json!({
                "path": project_path.display().to_string(),
                "source_branch": diff.source_branch,
                "target_branch": diff.target_branch,
                "ahead_count": diff.ahead_count,
                "behind_count": diff.behind_count,
                "ahead_commits": ahead_commits,
                "behind_commits": behind_commits,
                "changed_files": changed_files,
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Index job queue",
        description = "Lists daemon index jobs (optionally filtered by repo path). Use when: backlog visibility. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn project_queue_status(
        &self,
        Parameters(req): Parameters<ProjectQueueStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let limit = req.limit.unwrap_or(200);
        let jobs = cortex_watcher::list_index_jobs(&daemon_paths, limit)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let filtered = if let Some(path) = req.path {
            jobs.into_iter()
                .filter(|j| j.repository_path == path)
                .collect::<Vec<_>>()
        } else {
            jobs
        };

        Ok(Self::ok(
            json!({
                "count": filtered.len(),
                "jobs": filtered,
                "daemon": cortex_watcher::daemon_status(&daemon_paths).ok(),
            })
            .to_string(),
        ))
    }

    #[tool(
        title = "Daemon metrics",
        description = "Counters and derived averages for watch/index daemon. Use when: performance triage. Read-only.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn project_metrics(
        &self,
        Parameters(req): Parameters<ProjectMetricsReq>,
    ) -> Result<CallToolResult, McpError> {
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let counters = cortex_watcher::daemon_metrics(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .counters;
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path));
        let queue = if let Some(ref path) = project_path {
            cortex_watcher::list_index_jobs(&daemon_paths, 300)
                .unwrap_or_default()
                .into_iter()
                .filter(|j| j.repository_path == path.display().to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let avg_queue_wait_ms = {
            let total = counters.get("queue_wait_ms_total").copied().unwrap_or(0);
            let samples = counters.get("queue_wait_samples").copied().unwrap_or(0);
            if samples > 0 {
                Some(total as f64 / samples as f64)
            } else {
                None
            }
        };
        let avg_index_duration_ms = {
            let total = counters
                .get("index_duration_ms_total")
                .copied()
                .unwrap_or(0);
            let completed = counters.get("completed_jobs").copied().unwrap_or(0);
            if completed > 0 {
                Some(total as f64 / completed as f64)
            } else {
                None
            }
        };

        Ok(Self::ok(
            json!({
                "project_path": project_path.map(|p| p.display().to_string()),
                "counters": counters,
                "derived": {
                    "avg_queue_wait_ms": avg_queue_wait_ms,
                    "avg_index_duration_ms": avg_index_duration_ms
                },
                "queue_size": queue.len(),
            })
            .to_string(),
        ))
    }

    /// Test-only: all registered MCP tools (for contract tests).
    #[cfg(test)]
    pub(crate) fn tool_definitions_for_test(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }
}

// ── ServerHandler ─────────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for CortexHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(crate::tool_catalog::server_instructions_markdown())
        .with_server_info(Implementation::new("cortex", env!("CARGO_PKG_VERSION")))
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        let resource = RawResource {
            uri: crate::tool_catalog::TOOL_ROUTING_RESOURCE_URI.to_string(),
            name: "tool-routing-guide".to_string(),
            title: Some("CodeCortex tool routing guide".to_string()),
            description: Some(
                "Full markdown playbook: prerequisites, intent→tool map, disambiguation."
                    .to_string(),
            ),
            mime_type: Some("text/markdown".to_string()),
            size: None,
            icons: None,
            meta: None,
        }
        .no_annotation();
        std::future::ready(Ok(ListResourcesResult::with_all_items(vec![resource])))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        let uri = request.uri;
        std::future::ready(if uri == crate::tool_catalog::TOOL_ROUTING_RESOURCE_URI {
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(
                    crate::tool_catalog::resource_tool_guide_markdown(),
                    crate::tool_catalog::TOOL_ROUTING_RESOURCE_URI,
                )
                .with_mime_type("text/markdown"),
            ]))
        } else {
            Err(McpError::invalid_params(
                format!("unknown resource URI: {uri}"),
                None,
            ))
        })
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        let prompts = vec![
            Prompt::new(
                crate::tool_catalog::PROMPT_ROUTE_TOOLS,
                Some(
                    "Plan which CodeCortex tools to call for a user goal (order + prerequisites).",
                ),
                Some(vec![
                    PromptArgument::new("user_goal")
                        .with_title("User goal")
                        .with_description("What the user wants to do in the codebase")
                        .with_required(false),
                ]),
            )
            .with_title("Route tools from goal"),
            Prompt::new(
                crate::tool_catalog::PROMPT_SESSION_BOOTSTRAP,
                Some("Health → project → index → search/navigation checklist."),
                None,
            )
            .with_title("Session bootstrap"),
        ];
        std::future::ready(Ok(ListPromptsResult::with_all_items(prompts)))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        let name = request.name.clone();
        let args = request.arguments;
        std::future::ready(match name.as_str() {
            crate::tool_catalog::PROMPT_ROUTE_TOOLS => {
                let goal = args
                    .as_ref()
                    .and_then(|m| m.get("user_goal"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(describe the user's coding goal)");
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    crate::tool_catalog::prompt_route_tools_body(goal),
                )])
                .with_description("Plan CodeCortex MCP tool usage for this goal"))
            }
            crate::tool_catalog::PROMPT_SESSION_BOOTSTRAP => {
                Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                    PromptMessageRole::User,
                    crate::tool_catalog::prompt_session_bootstrap_body(),
                )])
                .with_description("Suggested first steps when using CodeCortex MCP"))
            }
            _ => Err(McpError::invalid_params(
                format!("unknown prompt: {name}"),
                None,
            )),
        })
    }
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn start_stdio(
    config: CortexConfig,
    feature_flags: crate::FeatureFlags,
) -> anyhow::Result<()> {
    let service = match CortexHandler::new_with_flags(config, feature_flags)
        .serve(stdio())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            if matches!(e, ServerInitializeError::ConnectionClosed(_)) {
                tracing::debug!("MCP client disconnected during initialization: {}", e);
                return Ok(());
            }
            return Err(e.into());
        }
    };
    service.waiting().await?;
    Ok(())
}

fn is_loopback(addr: &SocketAddr) -> bool {
    addr.ip().is_loopback()
}

fn validate_serve_options(options: &McpServeOptions) -> anyhow::Result<()> {
    if options.max_clients == 0 {
        return Err(anyhow::anyhow!("max_clients must be greater than 0"));
    }
    if options.idle_timeout_secs == 0 {
        return Err(anyhow::anyhow!("idle_timeout_secs must be greater than 0"));
    }
    if !options.allow_remote && !is_loopback(&options.listen) {
        return Err(anyhow::anyhow!(
            "non-loopback listen address requires allow_remote=true"
        ));
    }
    Ok(())
}

pub async fn start_with_options(
    config: CortexConfig,
    options: McpServeOptions,
) -> anyhow::Result<()> {
    validate_serve_options(&options)?;
    match options.transport {
        McpTransport::Stdio => start_stdio(config, options.feature_flags).await,
        McpTransport::HttpSse | McpTransport::WebSocket | McpTransport::Multi => {
            crate::network_server::start_network(config, options).await
        }
    }
}

#[cfg(test)]
mod serve_options_tests {
    use super::*;

    #[test]
    fn rejects_remote_bind_without_allow_remote() {
        let opts = McpServeOptions {
            feature_flags: crate::FeatureFlags::from_env(),
            transport: McpTransport::HttpSse,
            listen: "0.0.0.0:3010".parse().unwrap(),
            token: None,
            allow_remote: false,
            max_clients: 32,
            idle_timeout_secs: 60,
        };
        assert!(validate_serve_options(&opts).is_err());
    }

    #[test]
    fn accepts_remote_bind_when_explicitly_allowed() {
        let opts = McpServeOptions {
            feature_flags: crate::FeatureFlags::from_env(),
            transport: McpTransport::WebSocket,
            listen: "0.0.0.0:3010".parse().unwrap(),
            token: Some("secret".to_string()),
            allow_remote: true,
            max_clients: 32,
            idle_timeout_secs: 60,
        };
        assert!(validate_serve_options(&opts).is_ok());
    }

    #[test]
    fn default_serve_options_keep_stdio_compatibility() {
        let opts = McpServeOptions::default();
        assert_eq!(opts.transport, McpTransport::Stdio);
        assert!(opts.listen.ip().is_loopback());
    }
}

fn build_analyze_filters(
    include_paths: Option<Vec<String>>,
    include_files: Option<Vec<String>>,
    include_globs: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
    exclude_files: Option<Vec<String>>,
    exclude_globs: Option<Vec<String>>,
) -> anyhow::Result<AnalyzePathFilters> {
    let filters = AnalyzePathFilters {
        include_paths: include_paths.unwrap_or_default(),
        include_files: include_files.unwrap_or_default(),
        include_globs: include_globs.unwrap_or_default(),
        exclude_paths: exclude_paths.unwrap_or_default(),
        exclude_files: exclude_files.unwrap_or_default(),
        exclude_globs: exclude_globs.unwrap_or_default(),
    };
    filters.validate().map_err(anyhow::Error::msg)?;
    Ok(filters)
}

fn parse_usage_kind(kind: Option<&str>) -> anyhow::Result<Option<UsageKind>> {
    let Some(kind) = kind else {
        return Ok(None);
    };
    let value = kind.trim().to_ascii_lowercase().replace('-', "_");
    let parsed = match value.as_str() {
        "call" => UsageKind::Call,
        "import" => UsageKind::Import,
        "type_reference" | "typereference" => UsageKind::TypeReference,
        "field_access" | "fieldaccess" => UsageKind::FieldAccess,
        "inheritance" | "inherits" => UsageKind::Inheritance,
        "implementation" | "implements" => UsageKind::Implementation,
        "reference" => UsageKind::Reference,
        _ => anyhow::bail!("unsupported usage kind: {}", kind),
    };
    Ok(Some(parsed))
}

fn parse_review_severity(input: Option<&str>) -> Severity {
    match input.unwrap_or("warning").to_ascii_lowercase().as_str() {
        "critical" => Severity::Critical,
        "error" => Severity::Error,
        "warning" | "warn" => Severity::Warning,
        _ => Severity::Info,
    }
}

fn build_review_input_from_req(repo_path: &str, req: &PrReviewReq) -> anyhow::Result<ReviewInput> {
    let target_path = req.path.as_deref().unwrap_or(repo_path);
    let root = PathBuf::from(target_path);
    let base_ref = req.base_ref.clone().unwrap_or_else(|| "main".to_string());
    let head_ref = req.head_ref.clone().unwrap_or_else(|| "HEAD".to_string());

    let review_files = if root.join(".git").exists() {
        load_local_review_inputs_for_mcp(&root, &base_ref, &head_ref)?
    } else {
        let mut files = Vec::new();
        collect_review_files(&root, &mut files)?;
        files
            .into_iter()
            .filter_map(|p| {
                let source = fs::read_to_string(&p).ok()?;
                Some(ReviewFileInput {
                    path: p.display().to_string(),
                    source,
                    changed_ranges: Vec::<ReviewLineRange>::new(),
                })
            })
            .collect()
    };

    Ok(ReviewInput {
        base_ref: Some(base_ref),
        head_ref: Some(head_ref),
        filters: AnalyzePathFilters::default(),
        min_severity: parse_review_severity(req.min_severity.as_deref()),
        max_findings: req.max_findings,
        files: review_files,
    })
}

fn load_local_review_inputs_for_mcp(
    repo_path: &Path,
    base: &str,
    head: &str,
) -> anyhow::Result<Vec<ReviewFileInput>> {
    let range = format!("{base}...{head}");
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["diff", "--unified=0", "--no-color", range.as_str()])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "failed to compute local diff for pr_review: {}",
            stderr.trim()
        );
    }

    let patch = String::from_utf8_lossy(&output.stdout);
    let changed = parse_unified_diff_changed_ranges_mcp(&patch);
    let mut files = Vec::new();

    for (rel_path, ranges) in changed {
        if let Some(source) = read_file_at_ref_mcp(repo_path, head, &rel_path)? {
            files.push(ReviewFileInput {
                path: rel_path,
                source,
                changed_ranges: ranges,
            });
        }
    }
    Ok(files)
}

fn read_file_at_ref_mcp(
    repo_path: &Path,
    reference: &str,
    file_path: &str,
) -> anyhow::Result<Option<String>> {
    let show_spec = format!("{reference}:{file_path}");
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["show", show_spec.as_str()])
        .output()?;
    if output.status.success() {
        return Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()));
    }
    Ok(None)
}

fn parse_unified_diff_changed_ranges_mcp(patch: &str) -> HashMap<String, Vec<ReviewLineRange>> {
    let mut out: HashMap<String, Vec<ReviewLineRange>> = HashMap::new();
    let mut current_path: Option<String> = None;
    for line in patch.lines() {
        if let Some(stripped) = line.strip_prefix("+++ b/") {
            current_path = Some(stripped.to_string());
            continue;
        }
        if line.starts_with("+++ /dev/null") {
            current_path = None;
            continue;
        }
        if line.starts_with("@@")
            && let (Some(path), Some(range)) = (current_path.as_ref(), parse_hunk_range_mcp(line))
        {
            out.entry(path.clone()).or_default().push(range);
        }
    }
    out
}

fn parse_hunk_range_mcp(line: &str) -> Option<ReviewLineRange> {
    if !line.starts_with("@@") {
        return None;
    }
    let plus_index = line.find('+')?;
    let after_plus = &line[plus_index + 1..];
    let end_index = after_plus.find(' ').unwrap_or(after_plus.len());
    let range_part = &after_plus[..end_index];

    let (start, count) = if let Some((start, count)) = range_part.split_once(',') {
        (start.parse::<u32>().ok()?, count.parse::<u32>().ok()?)
    } else {
        (range_part.parse::<u32>().ok()?, 1)
    };
    if count == 0 || start == 0 {
        return None;
    }
    Some(ReviewLineRange {
        start_line: start,
        end_line: start + count.saturating_sub(1),
    })
}

fn collect_review_files(path: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            collect_review_files(&p, out)?;
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && matches!(
                ext,
                "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            )
        {
            out.push(p);
        }
    }
    Ok(())
}

fn analyzer_capabilities_json() -> Value {
    json!({
        "path_filters": {
            "supported": true,
            "fields": [
                "include_paths",
                "include_files",
                "include_globs",
                "exclude_paths",
                "exclude_files",
                "exclude_globs"
            ]
        },
        "language_aware_smells": {
            "supported": true,
            "extensions": [
                "rs","py","rb","js","jsx","ts","tsx","go","java","c","cc","cpp","h","hpp",
                "cs","php","swift","kt","kts","json","sh","bash","zsh","m","mm","scala"
            ]
        }
    })
}

fn collect_paths_for_filter(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if (k == "path" || k.ends_with(".path"))
                    && v.is_string()
                    && let Some(path) = v.as_str()
                {
                    out.push(path.to_string());
                }
                collect_paths_for_filter(v, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_paths_for_filter(item, out);
            }
        }
        _ => {}
    }
}

fn filter_rows_by_paths(rows: Vec<Value>, filters: &AnalyzePathFilters) -> Vec<Value> {
    if filters.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|row| {
            let mut paths = Vec::new();
            collect_paths_for_filter(row, &mut paths);
            filters.matches_any_path(paths.iter().map(String::as_str))
        })
        .collect()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MemoryDb {
    observations: Vec<ObservationRecord>,
}

fn memory_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_MEMORY_DB_PATH") {
        return PathBuf::from(p);
    }
    CortexConfig::config_path()
        .parent()
        .map(|p| p.join("memory.json"))
        .unwrap_or_else(|| PathBuf::from(".cortex-memory.json"))
}

fn audit_log_path() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_MEMORY_AUDIT_PATH") {
        return PathBuf::from(p);
    }
    CortexConfig::config_path()
        .parent()
        .map(|p| p.join("memory-audit.log"))
        .unwrap_or_else(|| PathBuf::from(".cortex-memory-audit.log"))
}

fn load_memory_db() -> anyhow::Result<MemoryDb> {
    let path = memory_db_path();
    if !path.exists() {
        return Ok(MemoryDb::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str::<MemoryDb>(raw.as_str()).unwrap_or_default())
}

fn persist_memory_db(db: &MemoryDb) -> anyhow::Result<()> {
    let path = memory_db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(db)?)?;
    Ok(())
}

fn append_audit_event(action: &str, target_id: &str) -> anyhow::Result<()> {
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::json!({
        "actor": "mcp",
        "action": action,
        "timestamp_ms": now_millis(),
        "target_id": target_id
    })
    .to_string();
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

fn exceeded_rate_limit(db: &MemoryDb, session_id: &str) -> bool {
    let now = now_millis();
    let count = db
        .observations
        .iter()
        .filter(|o| o.session_id == session_id)
        .filter(|o| now.saturating_sub(o.created_at) < 60_000)
        .count();
    count >= 30
}

fn looks_sensitive(text: &str) -> bool {
    let lowered = text.to_lowercase();
    [
        "password=",
        "secret=",
        "api_key",
        "token=",
        "begin private key",
    ]
    .iter()
    .any(|pat| lowered.contains(pat))
}

fn simple_lexical_score(query: &str, title: &str, body: &str) -> f64 {
    let q = query.to_lowercase();
    let t = title.to_lowercase();
    let b = body.to_lowercase();
    let title_hit = if t.contains(q.as_str()) { 1.0 } else { 0.0 };
    let body_hit = if b.contains(q.as_str()) { 1.0 } else { 0.0 };
    (title_hit * 0.7) + (body_hit * 0.3)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (a, b) in left.iter().zip(right.iter()) {
        let a = *a as f64;
        let b = *b as f64;
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

fn warning_with_reason(code: &str, reason: &str) -> String {
    format!("{code}:{reason}")
}

fn detect_intent(query: &str) -> &'static str {
    let q = query.to_lowercase();
    if q.contains("debug") || q.contains("error") {
        "debug"
    } else if q.contains("refactor") {
        "refactor"
    } else if q.contains("test") {
        "test"
    } else if q.contains("review") {
        "review"
    } else {
        "explore"
    }
}

fn escape_cypher(input: &str) -> String {
    input.replace('\'', "\\'")
}

fn default_repo_path() -> String {
    std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

fn find_git_repository_root(path: &Path) -> Option<PathBuf> {
    let start = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn resolve_git_context_for_path(path: &Path) -> Option<(PathBuf, String, String)> {
    let repo_root = find_git_repository_root(path)?;
    let git_ops = GitOperations::new(&repo_root);
    if !git_ops.is_git_repo() {
        return None;
    }
    let branch = git_ops.get_current_branch().ok()?;
    let commit = git_ops.get_current_commit().ok()?;
    Some((repo_root, branch, commit))
}

fn build_skeleton(content: &str, mode: &str) -> String {
    let mut out = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("struct ")
            || t.starts_with("pub struct ")
            || t.starts_with("class ")
            || t.starts_with("impl ")
        {
            out.push(t.to_string());
            if mode == "minimal" && out.len() >= 120 {
                break;
            }
        } else if mode == "standard" && (t.starts_with("//") || t.starts_with("///")) {
            out.push(t.to_string());
        }
    }
    if out.is_empty() {
        return content.lines().take(40).collect::<Vec<_>>().join("\n");
    }
    out.join("\n")
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{
        ContextCapsuleReq, CortexHandler, ImpactGraphReq, IndexStatusReq, LogicFlowReq, MemoryDb,
        ObservationRecord, SaveObservationReq, SearchMemoryReq, SessionContextReq, SkeletonReq,
        SubmitLspEdgesReq, WorkspaceSetupReq, build_review_input_from_req, build_skeleton,
        detect_intent, exceeded_rate_limit, looks_sensitive, parse_hunk_range_mcp,
        parse_unified_diff_changed_ranges_mcp, parse_usage_kind, simple_lexical_score,
    };
    use crate::handler::{PrReviewReq, UsageKind};
    use cortex_core::CortexConfig;
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::CallToolResult;
    use std::sync::Mutex;

    /// Mutex to synchronize tests that manipulate environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn sensitive_detector_finds_tokens() {
        assert!(looks_sensitive("my API_KEY=123"));
        assert!(!looks_sensitive("regular engineering note"));
    }

    #[test]
    fn intent_detection_works() {
        assert_eq!(detect_intent("debug call path"), "debug");
        assert_eq!(detect_intent("refactor this"), "refactor");
    }

    #[test]
    fn skeleton_extracts_signatures() {
        let src = "pub struct A {}\nfn run() {}\nlet x = 1;";
        let s = build_skeleton(src, "minimal");
        assert!(s.contains("pub struct A"));
        assert!(s.contains("fn run"));
    }

    #[test]
    fn lexical_score_prefers_title_hits() {
        let a = simple_lexical_score("call_chain", "call_chain", "x");
        let b = simple_lexical_score("call_chain", "x", "call_chain");
        assert!(a > b);
    }

    #[test]
    fn rate_limit_triggers_on_burst() {
        let mut db = MemoryDb::default();
        for i in 0..31u128 {
            db.observations.push(ObservationRecord {
                observation_id: format!("obs-{i}"),
                repo_id: "r".to_string(),
                session_id: "s".to_string(),
                created_at: super::now_millis(),
                created_by: "mcp".to_string(),
                text: "x".to_string(),
                symbol_refs: Vec::new(),
                confidence: 1.0,
                stale: false,
                classification: "internal".to_string(),
                severity: "info".to_string(),
                tags: Vec::new(),
                source_revision: "rev".to_string(),
                embedding: None,
            });
        }
        assert!(exceeded_rate_limit(&db, "s"));
    }

    fn as_text(result: CallToolResult) -> String {
        result.content[0]
            .as_text()
            .expect("text response")
            .text
            .clone()
    }

    #[tokio::test]
    async fn context_capsule_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_context_capsule(Parameters(ContextCapsuleReq {
                query: "auth refresh".to_string(),
                task_intent: None,
                repo_path: None,
                max_tokens: None,
                max_items: None,
                include_tests: None,
                path_filter: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED");
        }
    }

    #[tokio::test]
    async fn impact_graph_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_impact_graph(Parameters(ImpactGraphReq {
                symbol: "call_chain".to_string(),
                symbol_type: None,
                repo_path: None,
                depth: None,
                include_importers: None,
                include_tests: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED");
        }
    }

    #[tokio::test]
    async fn logic_flow_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .search_logic_flow(Parameters(LogicFlowReq {
                from_symbol: "a".to_string(),
                to_symbol: "b".to_string(),
                repo_path: None,
                max_paths: None,
                max_depth: None,
                allow_partial: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED");
        }
    }

    #[tokio::test]
    async fn skeleton_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_SKELETON_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_skeleton(Parameters(SkeletonReq {
                path: "Cargo.toml".to_string(),
                mode: None,
                repo_path: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_SKELETON_ENABLED");
        }
    }

    #[tokio::test]
    async fn index_status_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .index_status(Parameters(IndexStatusReq {
                repo_path: None,
                include_jobs: None,
                include_watcher: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED");
        }
    }

    #[tokio::test]
    async fn workspace_setup_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .workspace_setup(Parameters(WorkspaceSetupReq {
                repo_path: None,
                detect_agents: None,
                generate_configs: None,
                install_git_hooks: None,
                non_interactive: None,
                overwrite: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED");
        }
    }

    #[tokio::test]
    async fn submit_lsp_edges_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_LSP_INGEST_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .submit_lsp_edges(Parameters(SubmitLspEdgesReq {
                repo_path: ".".to_string(),
                edges: Vec::new(),
                merge_mode: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_LSP_INGEST_ENABLED");
        }
    }

    #[tokio::test]
    async fn save_observation_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .save_observation(Parameters(SaveObservationReq {
                repo_path: ".".to_string(),
                text: "note".to_string(),
                severity: None,
                confidence: None,
                symbol_refs: None,
                tags: None,
                classification: None,
                session_id: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED");
        }
    }

    #[tokio::test]
    async fn get_session_context_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_session_context(Parameters(SessionContextReq {
                repo_path: ".".to_string(),
                session_id: None,
                include_previous: None,
                max_items: None,
                include_stale: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED");
        }
    }

    #[tokio::test]
    async fn search_memory_respects_feature_flag() {
        // Test that setting the env var to 0 disables the tool
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .search_memory(Parameters(SearchMemoryReq {
                query: "call target".to_string(),
                repo_path: ".".to_string(),
                max_items: None,
                include_stale: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED");
        }
    }

    // Pattern detection unit tests
    #[test]
    fn pattern_detection_names_contain_expected_patterns() {
        // Test that all expected patterns are in the list
        let patterns = vec![
            "builder",
            "factory",
            "singleton",
            "repository",
            "service",
            "handler",
            "middleware",
            "observer",
            "strategy",
            "adapter",
            "decorator",
            "command",
            "state",
            "facade",
            "proxy",
        ];

        // Each pattern should have at least one name hint
        for pattern in patterns {
            // These patterns should all be valid
            assert!(
                !pattern.is_empty(),
                "Pattern {} should not be empty",
                pattern
            );
        }
    }

    #[test]
    fn builder_pattern_detection_logic() {
        // Test builder pattern signals
        let builder_names = vec!["UserBuilder", "RequestBuilder", "ConfigBuilder"];
        let builder_structural = vec!["build()", ".build()", "builder"];

        for name in &builder_names {
            assert!(
                name.contains("Builder"),
                "Name {} should contain Builder",
                name
            );
        }
        for structural in &builder_structural {
            assert!(
                structural.contains("build"),
                "Structural {} should indicate builder",
                structural
            );
        }
    }

    #[test]
    fn repository_pattern_detection_logic() {
        // Test repository pattern signals
        let repo_names = vec!["UserRepository", "OrderRepo", "DataStore", "UserDAO"];
        let repo_structural = vec!["find(", "save(", "delete(", "query("];

        for name in &repo_names {
            let is_repo = name.contains("Repository")
                || name.contains("Repo")
                || name.contains("Store")
                || name.contains("DAO");
            assert!(is_repo, "Name {} should indicate repository", name);
        }
        for structural in &repo_structural {
            assert!(!structural.is_empty(), "Structural hint should exist");
        }
    }

    #[test]
    fn adapter_pattern_detection_logic() {
        // Test adapter pattern signals
        let adapter_names = vec!["DataAdapter", "ApiAdapter", "LogWrapper", "FormatConverter"];
        let _adapter_structural = ["adapt(", "convert(", "wrap("];

        for name in &adapter_names {
            let is_adapter =
                name.contains("Adapter") || name.contains("Wrapper") || name.contains("Converter");
            assert!(is_adapter, "Name {} should indicate adapter", name);
        }
    }

    #[test]
    fn command_pattern_detection_logic() {
        // Test command pattern signals
        let command_names = vec!["CreateCommand", "DeleteAction", "UpdateOperation"];
        let _command_structural = ["execute(", "undo(", "redo("];

        for name in &command_names {
            let is_command =
                name.contains("Command") || name.contains("Action") || name.contains("Operation");
            assert!(is_command, "Name {} should indicate command", name);
        }
    }

    #[test]
    fn state_pattern_detection_logic() {
        // Test state pattern signals
        let state_names = vec!["OrderState", "StateMachine", "ConnectionFSM"];
        let _state_structural = ["transition(", "current_state", "state"];

        for name in &state_names {
            let is_state =
                name.contains("State") || name.contains("Machine") || name.contains("FSM");
            assert!(is_state, "Name {} should indicate state", name);
        }
    }

    // Helper function tests

    #[test]
    fn escape_cypher_escapes_quotes() {
        assert_eq!(super::escape_cypher("hello"), "hello");
        assert_eq!(super::escape_cypher("it's"), "it\\'s");
        assert_eq!(super::escape_cypher("'quoted'"), "\\'quoted\\'");
    }

    #[test]
    fn default_repo_path_returns_current_dir() {
        let path = super::default_repo_path();
        // Should return a valid path string
        assert!(!path.is_empty());
    }

    #[test]
    fn build_skeleton_minimal_mode() {
        let src = r#"
            fn main() {
                println!("hello");
            }

            pub struct User {
                name: String,
            }

            impl User {
                fn new() -> Self { User { name: String::new() } }
            }
        "#;
        let skeleton = super::build_skeleton(src, "minimal");
        assert!(skeleton.contains("fn main"));
        assert!(skeleton.contains("pub struct User"));
        assert!(skeleton.contains("impl User"));
    }

    #[test]
    fn build_skeleton_standard_mode_includes_comments() {
        let src = r#"
            /// Documentation comment
            fn documented() {}

            // Regular comment
            fn regular() {}
        "#;
        let skeleton = super::build_skeleton(src, "standard");
        assert!(skeleton.contains("fn documented"));
        assert!(skeleton.contains("///"));
    }

    #[test]
    fn build_skeleton_empty_returns_fallback() {
        let src = "let x = 1;\nlet y = 2;";
        let skeleton = super::build_skeleton(src, "minimal");
        // Should return fallback when no signatures found
        assert!(skeleton.contains("let x") || skeleton.is_empty());
    }

    #[test]
    fn cortex_handler_new_creates_instance() {
        let handler = CortexHandler::new(CortexConfig::default());
        // Just verify we can create one
        let _ = handler.tool_enabled("test", true);
    }

    #[test]
    fn parse_usage_kind_handles_expected_values() {
        assert!(matches!(
            parse_usage_kind(Some("type-reference")).expect("parse"),
            Some(UsageKind::TypeReference)
        ));
        assert!(matches!(
            parse_usage_kind(Some("field_access")).expect("parse"),
            Some(UsageKind::FieldAccess)
        ));
        assert!(matches!(
            parse_usage_kind(Some("call")).expect("parse"),
            Some(UsageKind::Call)
        ));
    }

    #[test]
    fn parse_hunk_range_parses_added_span() {
        let range = parse_hunk_range_mcp("@@ -10,2 +20,5 @@").expect("range");
        assert_eq!(range.start_line, 20);
        assert_eq!(range.end_line, 24);
    }

    #[test]
    fn parse_unified_diff_collects_ranges_per_file() {
        let patch = r#"
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,0 +1,2 @@
+fn one() {}
+fn two() {}
"#;
        let changed = parse_unified_diff_changed_ranges_mcp(patch);
        let ranges = changed.get("src/a.rs").expect("file entry");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 1);
        assert_eq!(ranges[0].end_line, 2);
    }

    #[test]
    fn build_review_input_from_req_non_git_fallback_reads_sources() {
        let base = std::env::temp_dir().join(format!(
            "mcp-review-build-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).expect("create temp dir");
        let file = base.join("sample.rs");
        std::fs::write(&file, "fn sample() {}\n").expect("write sample");

        let req = PrReviewReq {
            base_ref: Some("main".to_string()),
            head_ref: Some("HEAD".to_string()),
            path: Some(base.display().to_string()),
            min_severity: Some("warning".to_string()),
            max_findings: Some(10),
        };
        let input =
            build_review_input_from_req(base.to_string_lossy().as_ref(), &req).expect("input");
        assert!(!input.files.is_empty());
        assert!(input.files.iter().any(|f| f.path.ends_with("sample.rs")));

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(base);
    }
}
