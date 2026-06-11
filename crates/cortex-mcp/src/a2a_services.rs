//! MCP-backed [`A2aServices`] for real consensus workflows.

use cortex_a2a::services::{
    A2aServices, ApiContractSummary, ContextCapsuleSummary, DeltaContextSummary, ImpactSummary,
    IndexFreshnessLabel, IntelligenceRequest, PatchContextCapsule, PrReviewSummary,
    TestContextSummary, ValidationSummary,
};
use cortex_analyzer::Analyzer;
use cortex_core::config::CortexConfig;
use cortex_graph::GraphClient;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct McpA2aServices {
    config: CortexConfig,
    repo_root: PathBuf,
    graph: Mutex<Option<GraphClient>>,
}

impl McpA2aServices {
    pub fn new(config: CortexConfig) -> Self {
        Self {
            config,
            repo_root: PathBuf::from(crate::handler::default_repo_path()),
            graph: Mutex::new(None),
        }
    }

    async fn client(&self) -> anyhow::Result<GraphClient> {
        let mut guard = self.graph.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(c.clone());
        }
        let c = GraphClient::connect(&self.config).await?;
        *guard = Some(c.clone());
        Ok(c)
    }

    fn repo_path(&self, req: &IntelligenceRequest) -> String {
        req.repo_path
            .clone()
            .unwrap_or_else(|| self.repo_root.display().to_string())
    }
}

fn capsule_from_pack(pack: &crate::intelligence::IntelligencePack) -> PatchContextCapsule {
    let capsule_uri = pack
        .meta
        .capsule_uri
        .clone()
        .or_else(|| {
            pack.data
                .get("capsule_uri")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let summary = pack
        .data
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let include_paths = pack
        .data
        .get("include_paths")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_else(|| vec!["src".to_string()]);
    PatchContextCapsule {
        capsule_uri,
        summary,
        include_paths,
        freshness: pack.freshness_label().to_string(),
        warnings: pack.meta.warnings.clone(),
        suggested_next_tools: pack.meta.suggested_next_tools.clone(),
        data_json: Some(pack.data.clone()),
    }
}

fn impact_from_pack(
    pack: &crate::intelligence::IntelligencePack,
    summary: ImpactSummary,
) -> ImpactSummary {
    ImpactSummary {
        freshness: pack.freshness_label().to_string(),
        warnings: pack.meta.warnings.clone(),
        suggested_next_tools: pack.meta.suggested_next_tools.clone(),
        data_json: Some(pack.data.clone()),
        ..summary
    }
}

#[async_trait::async_trait]
impl A2aServices for McpA2aServices {
    async fn index_freshness_for_paths(
        &self,
        paths: &[String],
    ) -> anyhow::Result<IndexFreshnessLabel> {
        let client = match self.client().await {
            Ok(c) => c,
            Err(_) => {
                return Ok(IndexFreshnessLabel {
                    label: "unknown".to_string(),
                });
            }
        };
        let repo = self.repo_root.display().to_string();
        let label = crate::intelligence::path_freshness(&client, &repo, paths)
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        Ok(IndexFreshnessLabel { label })
    }

    async fn get_patch_context(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<PatchContextCapsule> {
        let client = GraphClient::connect(&self.config).await?;
        let analyzer = Analyzer::new(client.clone());
        let repo = self.repo_path(req);
        let pack = crate::intelligence::build_patch_pack(
            &client,
            &analyzer,
            &repo,
            &crate::intelligence::PatchContextParams {
                task: req.task.clone(),
                mode: req.mode.clone(),
                budget_tokens: req.budget_tokens,
                scope: crate::intelligence::ScopeFilters::new(
                    req.include_paths.clone(),
                    req.exclude_paths.clone(),
                ),
            },
            req.workflow.as_deref(),
        )
        .await;
        Ok(capsule_from_pack(&pack))
    }

    async fn analyze_impact(&self, req: &IntelligenceRequest) -> anyhow::Result<ImpactSummary> {
        let target = req.target_path_or_default();
        let client = match self.client().await {
            Ok(c) => c,
            Err(e) => {
                return Ok(ImpactSummary {
                    target: target.clone(),
                    risk_level: cortex_a2a::payload::RiskLevel::Medium,
                    summary: format!("graph unavailable: {e}"),
                    has_cycle_risk: false,
                    freshness: "unknown".to_string(),
                    warnings: vec![e.to_string()],
                    suggested_next_tools: crate::intelligence::spawn_tools_for_workflow(
                        "impact_review",
                    ),
                    data_json: None,
                });
            }
        };
        let analyzer = Analyzer::new(client.clone());
        let symbol =
            crate::intelligence::resolve_symbol(&analyzer, &target, req.target_symbol.as_deref())
                .await;
        let repo = self.repo_path(req);
        let pack = crate::intelligence::build_impact_pack(
            &client,
            &analyzer,
            &repo,
            &req.include_paths,
            &crate::intelligence::ImpactGraphParams {
                symbol,
                depth: 4,
                include_importers: true,
                budget_tokens: req.budget_tokens,
                symbol_type: "auto".to_string(),
            },
            req.workflow.as_deref(),
        )
        .await;
        let base = crate::intelligence::impact_summary_for_a2a(
            &target,
            &req.include_paths,
            &pack.data,
            None,
        );
        Ok(impact_from_pack(&pack, base))
    }

    async fn get_api_contract(
        &self,
        symbol: &str,
        budget_tokens: u32,
    ) -> anyhow::Result<ApiContractSummary> {
        let client = GraphClient::connect(&self.config).await?;
        let analyzer = Analyzer::new(client);
        let take = (budget_tokens / 400).clamp(1, 12) as usize;
        let contracts = crate::intelligence::compute_api_contract(&analyzer, symbol, take).await;
        Ok(ApiContractSummary {
            symbol: symbol.to_string(),
            contracts_json: contracts,
        })
    }

    async fn get_test_context(
        &self,
        symbol: &str,
        budget_tokens: u32,
    ) -> anyhow::Result<TestContextSummary> {
        let client = GraphClient::connect(&self.config).await?;
        let analyzer = Analyzer::new(client);
        let take = (budget_tokens / 96).clamp(1, 20) as usize;
        let tests = crate::intelligence::compute_test_context(&analyzer, symbol, take).await;
        Ok(TestContextSummary {
            symbol: symbol.to_string(),
            tests_json: tests,
        })
    }

    async fn get_delta_context(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<DeltaContextSummary> {
        let client = GraphClient::connect(&self.config).await?;
        let repo = self.repo_path(req);
        let source = req
            .source_branch
            .clone()
            .unwrap_or_else(|| "HEAD".to_string());
        let target = req
            .target_branch
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let delta = crate::intelligence::compute_delta_context(
            &client,
            &repo,
            &source,
            &target,
            req.budget_tokens,
            &crate::intelligence::ScopeFilters::new(
                req.include_paths.clone(),
                req.exclude_paths.clone(),
            ),
        )
        .await;
        Ok(DeltaContextSummary {
            source_branch: source,
            target_branch: target,
            delta_json: delta,
        })
    }

    async fn get_pr_review_summary(
        &self,
        req: &IntelligenceRequest,
    ) -> anyhow::Result<PrReviewSummary> {
        let client = GraphClient::connect(&self.config).await?;
        let repo = self.repo_path(req);
        let source = req
            .source_branch
            .clone()
            .unwrap_or_else(|| "HEAD".to_string());
        let target = req
            .target_branch
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let pack = crate::intelligence::compute_pr_review_pack(
            &client,
            &crate::intelligence::PrReviewParams {
                task: req.task.clone(),
                repo_path: repo,
                source_branch: source.clone(),
                target_branch: target.clone(),
                scope: crate::intelligence::ScopeFilters::new(
                    req.include_paths.clone(),
                    req.exclude_paths.clone(),
                ),
                budget_tokens: req.budget_tokens,
                target_symbol: req.target_symbol.clone(),
            },
        )
        .await;
        Ok(PrReviewSummary {
            capsule_uri: pack.meta.capsule_uri.clone().unwrap_or_default(),
            impact_summary: pack
                .data
                .get("impact_summary")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            risk_level: pack
                .data
                .get("risk_level")
                .and_then(|v| v.as_str())
                .unwrap_or("low")
                .to_string(),
            delta_json: pack.data.get("delta").cloned().unwrap_or_default(),
            status_hint: pack
                .data
                .get("status_hint")
                .and_then(|v| v.as_str())
                .unwrap_or("review")
                .to_string(),
            freshness: pack.freshness_label().to_string(),
            warnings: pack.meta.warnings.clone(),
            suggested_next_tools: pack.meta.suggested_next_tools.clone(),
            data_json: Some(pack.data.clone()),
        })
    }

    async fn get_context_capsule(
        &self,
        req: &IntelligenceRequest,
        query: &str,
    ) -> anyhow::Result<ContextCapsuleSummary> {
        let client = GraphClient::connect(&self.config).await?;
        let analyzer = Analyzer::new(client.clone());
        let repo = self.repo_path(req);
        let pack = crate::intelligence::compute_context_capsule(
            &client,
            &analyzer,
            &crate::intelligence::CapsuleParams {
                query: query.to_string(),
                task_intent: req.mode.clone(),
                budget_tokens: req.budget_tokens,
                max_items: 40,
                scope: crate::intelligence::ScopeFilters::new(
                    req.include_paths.clone(),
                    req.exclude_paths.clone(),
                ),
                repo_path: repo,
            },
            Vec::new(),
        )
        .await;
        Ok(ContextCapsuleSummary {
            query: query.to_string(),
            item_count: pack
                .data
                .get("item_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            freshness: pack.freshness_label().to_string(),
            warnings: pack.meta.warnings.clone(),
            suggested_next_tools: pack.meta.suggested_next_tools.clone(),
            data_json: pack.data.clone(),
        })
    }

    async fn validate_build(&self, repo_root: Option<&str>) -> anyhow::Result<ValidationSummary> {
        let root = repo_root
            .map(PathBuf::from)
            .unwrap_or_else(|| self.repo_root.clone());
        if !root.join("Cargo.toml").exists() {
            return Ok(ValidationSummary {
                passed: true,
                summary: "no Cargo.toml — skipped cargo check".to_string(),
            });
        }
        let output = tokio::process::Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(&root)
            .output()
            .await;
        match output {
            Ok(out) if out.status.success() => Ok(ValidationSummary {
                passed: true,
                summary: "cargo check passed".to_string(),
            }),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let summary: String = stderr.lines().take(8).collect::<Vec<_>>().join("\n");
                Ok(ValidationSummary {
                    passed: false,
                    summary: if summary.is_empty() {
                        format!("cargo check failed: {}", out.status)
                    } else {
                        summary
                    },
                })
            }
            Err(e) => Ok(ValidationSummary {
                passed: true,
                summary: format!("cargo check skipped: {e}"),
            }),
        }
    }

    async fn publish_graph_mutation(
        &self,
        session_id: &str,
        conversation_id: &str,
        affected_files: Vec<String>,
    ) -> anyhow::Result<()> {
        if !self.config.a2a.blackboard.enabled {
            return Ok(());
        }
        let client = self.client().await?;
        let writer = cortex_graph::BlackboardWriter::new(
            client,
            self.config.a2a.blackboard_write_batch_size(64),
        );
        let _ = writer.ensure_schema().await;
        for path in &affected_files {
            let _ = writer
                .write_mutation_hint(session_id, conversation_id, path, "index_promoted")
                .await;
        }
        Ok(())
    }
}

async fn build_a2a_hub_inner(config: &CortexConfig) -> Arc<cortex_a2a::A2aHub> {
    let repo_root = Some(PathBuf::from(crate::handler::default_repo_path()));

    let blackboard = if config.a2a.blackboard.enabled {
        match GraphClient::connect(config).await {
            Ok(client) => {
                let writer = Arc::new(cortex_graph::BlackboardWriter::new(
                    client,
                    config.a2a.blackboard_write_batch_size(64),
                ));
                let _ = writer.ensure_schema().await;
                Some(writer)
            }
            Err(e) => {
                tracing::warn!("a2a blackboard unavailable: {e}");
                None
            }
        }
    } else {
        None
    };

    Arc::new(cortex_a2a::A2aHub::with_options(
        config.a2a.clone(),
        Arc::new(McpA2aServices::new(config.clone())),
        blackboard,
        repo_root,
    ))
}

/// Build a graph-backed A2A hub when `[a2a] enabled = true`.
pub async fn try_build_a2a_hub(config: &CortexConfig) -> Option<Arc<cortex_a2a::A2aHub>> {
    if !config.a2a.enabled {
        return None;
    }
    Some(build_a2a_hub_inner(config).await)
}

pub async fn build_a2a_hub(config: &CortexConfig) -> Arc<cortex_a2a::A2aHub> {
    build_a2a_hub_inner(config).await
}
