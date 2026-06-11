//! Built-in in-process role runners.

use crate::envelope::A2aEnvelope;
use crate::payload::{A2aPayload, RiskLevel};
use crate::roles::AgentRole;
use crate::runtime::context::RoleContext;
use crate::services::{ImpactSummary, IntelligenceRequest, blackboard_from_envelope};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[async_trait]
pub trait RoleRunner: Send + Sync {
    fn role(&self) -> AgentRole;
    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>>;
}

pub fn build_runners() -> Vec<Arc<dyn RoleRunner>> {
    vec![
        Arc::new(GatewayRunner),
        Arc::new(PatchPlannerRunner),
        Arc::new(AnalyzerRunner),
        Arc::new(ValidatorRunner),
        Arc::new(IndexerRunner),
        Arc::new(PrReviewerRunner),
    ]
}

struct GatewayRunner;

#[async_trait]
impl RoleRunner for GatewayRunner {
    fn role(&self) -> AgentRole {
        AgentRole::Gateway
    }

    async fn handle(&self, envelope: A2aEnvelope, _ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        Ok(vec![envelope])
    }
}

struct PatchPlannerRunner;

#[async_trait]
impl RoleRunner for PatchPlannerRunner {
    fn role(&self) -> AgentRole {
        AgentRole::PatchPlanner
    }

    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        if !ctx
            .manifests
            .accepts_payload("patch_planner", payload_type_name(&envelope.payload))
        {
            return Ok(vec![]);
        }

        let intel = IntelligenceRequest::from_role_context(ctx).with_workflow("consensus_review");
        let capsule = ctx.services.get_patch_context(&intel).await?;

        let demo_deadlock = ctx.config.workflows.consensus_review.demo_fixture
            || ctx.task.contains("transport_deadlock")
            || ctx.task.contains("spin_lock")
            || ctx
                .include_paths
                .iter()
                .any(|p| p.contains("transport_deadlock"));

        let mut patch = if demo_deadlock {
            json!({
                "strategy": "naive_spin_lock",
                "file": ctx.target_path(),
                "change": capsule.summary,
                "capsule_uri": capsule.capsule_uri,
            })
        } else {
            json!({
                "strategy": "indexed_patch_plan",
                "file": ctx.target_path(),
                "change": capsule.summary,
                "capsule_uri": capsule.capsule_uri,
                "include_paths": capsule.include_paths,
            })
        };

        if demo_deadlock && matches!(envelope.payload, A2aPayload::Reject { .. }) {
            patch = json!({
                "strategy": "ordered_mutex",
                "file": ctx.target_path(),
                "change": "acquire locks in consistent global order",
            });
        } else if matches!(envelope.payload, A2aPayload::Reject { .. }) {
            patch = json!({
                "strategy": "revise_from_analyzer_feedback",
                "file": ctx.target_path(),
                "change": capsule.summary,
                "capsule_uri": capsule.capsule_uri,
            });
        }

        let mut replies = Vec::new();

        if matches!(envelope.payload, A2aPayload::TaskDelegation { .. }) {
            let proposal = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::PatchPlanner,
                AgentRole::Analyzer,
                A2aPayload::StrategyProposal {
                    estimated_complexity: 3,
                    required_sub_nodes: vec!["analyzer".to_string(), "validator".to_string()],
                },
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &proposal).await;
            replies.push(proposal);
        } else if matches!(envelope.payload, A2aPayload::Reject { .. }) {
            let proposal = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::PatchPlanner,
                AgentRole::Analyzer,
                A2aPayload::StrategyProposal {
                    estimated_complexity: 2,
                    required_sub_nodes: vec!["analyzer".to_string()],
                },
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &proposal).await;
            replies.push(proposal);
        }

        let out = A2aEnvelope::new(
            ctx.conversation_id,
            AgentRole::PatchPlanner,
            AgentRole::Analyzer,
            A2aPayload::CodeInsight {
                summary: format!("proposed patch for {}", ctx.target_path()),
                target_qualified_name: ctx.target_path(),
                risk_level: if patch
                    .get("strategy")
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("spin"))
                    .unwrap_or(false)
                {
                    RiskLevel::Critical
                } else {
                    RiskLevel::Low
                },
                suggested_action: patch
                    .get("strategy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            },
        )
        .with_task_id(ctx.task_id);

        record_bb(ctx, &out).await;
        replies.push(out);
        Ok(replies)
    }
}

struct AnalyzerRunner;

#[async_trait]
impl RoleRunner for AnalyzerRunner {
    fn role(&self) -> AgentRole {
        AgentRole::Analyzer
    }

    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        if !ctx
            .manifests
            .accepts_payload("analyzer", payload_type_name(&envelope.payload))
        {
            return Ok(vec![]);
        }

        match &envelope.payload {
            A2aPayload::StrategyProposal {
                estimated_complexity,
                required_sub_nodes,
            } => {
                let accept = *estimated_complexity <= 5 && !required_sub_nodes.is_empty();
                let out = if accept {
                    A2aEnvelope::new(
                        ctx.conversation_id,
                        AgentRole::Analyzer,
                        AgentRole::PatchPlanner,
                        A2aPayload::Accept,
                    )
                } else {
                    A2aEnvelope::new(
                        ctx.conversation_id,
                        AgentRole::Analyzer,
                        AgentRole::PatchPlanner,
                        A2aPayload::Reject {
                            reason: "strategy too complex or missing sub-nodes".to_string(),
                        },
                    )
                }
                .with_task_id(ctx.task_id);
                record_bb(ctx, &out).await;
                Ok(vec![out])
            }
            A2aPayload::GraphMutationSignal { affected_files, .. } => {
                let mut intel =
                    IntelligenceRequest::from_role_context(ctx).with_workflow("impact_review");
                if let Some(path) = affected_files.first() {
                    intel.target_path = Some(path.clone());
                }
                let impact = ctx.services.analyze_impact(&intel).await?;
                let out = A2aEnvelope::new(
                    ctx.conversation_id,
                    AgentRole::Analyzer,
                    AgentRole::PatchPlanner,
                    A2aPayload::CodeInsight {
                        summary: impact.summary,
                        target_qualified_name: impact.target,
                        risk_level: impact.risk_level,
                        suggested_action: if impact.has_cycle_risk {
                            "reject_cycle".to_string()
                        } else {
                            "proceed".to_string()
                        },
                    },
                )
                .with_task_id(ctx.task_id);
                record_bb(ctx, &out).await;
                Ok(vec![out])
            }
            A2aPayload::CodeInsight {
                suggested_action,
                target_qualified_name,
                ..
            } => {
                if let Some(bb) = ctx.blackboard.as_deref() {
                    if let Ok(insights) = bb.list_insights(&ctx.session_id).await {
                        let cached = insights.iter().find(|i| {
                            i.target_qualified_name == *target_qualified_name
                                && i.role == "analyzer"
                                && (i.suggested_action == "proceed"
                                    || i.suggested_action == "reject_cycle")
                        });
                        if let Some(hit) = cached {
                            if hit.suggested_action == "proceed" {
                                let accept = A2aEnvelope::new(
                                    ctx.conversation_id,
                                    AgentRole::Analyzer,
                                    AgentRole::PatchPlanner,
                                    A2aPayload::Accept,
                                )
                                .with_task_id(ctx.task_id);
                                return Ok(vec![accept]);
                            }
                            let reject = A2aEnvelope::new(
                                ctx.conversation_id,
                                AgentRole::Analyzer,
                                AgentRole::PatchPlanner,
                                A2aPayload::Reject {
                                    reason: hit.summary.clone(),
                                },
                            )
                            .with_task_id(ctx.task_id);
                            return Ok(vec![reject]);
                        }
                    }
                }

                let mut intel =
                    IntelligenceRequest::from_role_context(ctx).with_workflow("impact_review");
                intel.target_path = Some(target_qualified_name.clone());
                let impact = ctx.services.analyze_impact(&intel).await?;

                if is_impact_stub(&impact) && suggested_action.contains("spin") {
                    let reject = A2aEnvelope::new(
                        ctx.conversation_id,
                        AgentRole::Analyzer,
                        AgentRole::PatchPlanner,
                        A2aPayload::Reject {
                            reason: "detected lock ordering risk: naive spin_lock may deadlock"
                                .to_string(),
                        },
                    )
                    .with_task_id(ctx.task_id);
                    record_bb(ctx, &reject).await;
                    return Ok(vec![reject]);
                }

                let _contracts = ctx
                    .services
                    .get_api_contract(target_qualified_name.as_str(), ctx.budget_tokens.min(4000))
                    .await;

                if impact.has_cycle_risk || impact.risk_level == RiskLevel::Critical {
                    let reject = A2aEnvelope::new(
                        ctx.conversation_id,
                        AgentRole::Analyzer,
                        AgentRole::PatchPlanner,
                        A2aPayload::Reject {
                            reason: impact.summary,
                        },
                    )
                    .with_task_id(ctx.task_id);
                    record_bb(ctx, &reject).await;
                    return Ok(vec![reject]);
                }

                let accept = A2aEnvelope::new(
                    ctx.conversation_id,
                    AgentRole::Analyzer,
                    AgentRole::PatchPlanner,
                    A2aPayload::Accept,
                )
                .with_task_id(ctx.task_id);
                record_bb(ctx, &accept).await;
                Ok(vec![accept])
            }
            _ => Ok(vec![]),
        }
    }
}

struct ValidatorRunner;

#[async_trait]
impl RoleRunner for ValidatorRunner {
    fn role(&self) -> AgentRole {
        AgentRole::Validator
    }

    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        if !matches!(envelope.payload, A2aPayload::Accept) {
            return Ok(vec![]);
        }
        if ctx.config.role_mode("validator") == cortex_core::A2aRoleMode::Disabled {
            return Ok(vec![]);
        }

        if ctx.config.require_fresh_index {
            let fresh = ctx
                .services
                .index_freshness_for_paths(&ctx.include_paths)
                .await?;
            if fresh.label != "fresh" {
                let insight = A2aEnvelope::new(
                    ctx.conversation_id,
                    AgentRole::Validator,
                    AgentRole::PatchPlanner,
                    A2aPayload::Reject {
                        reason: format!(
                            "index freshness is {} — repair index before validation",
                            fresh.label
                        ),
                    },
                )
                .with_task_id(ctx.task_id);
                record_bb(ctx, &insight).await;
                return Ok(vec![insight]);
            }
        }

        let _tests = ctx
            .services
            .get_test_context(ctx.target_path().as_str(), ctx.budget_tokens.min(4000))
            .await;

        let validation = ctx
            .services
            .validate_build(ctx.repo_root.as_deref().map(|p| p.to_str()).flatten())
            .await?;

        if validation.passed {
            let accept = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::Validator,
                AgentRole::Gateway,
                A2aPayload::Accept,
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &accept).await;
            Ok(vec![accept])
        } else {
            let insight = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::Validator,
                AgentRole::PatchPlanner,
                A2aPayload::CodeInsight {
                    summary: validation.summary,
                    target_qualified_name: ctx.target_path(),
                    risk_level: RiskLevel::High,
                    suggested_action: "fix_build".to_string(),
                },
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &insight).await;
            Ok(vec![insight])
        }
    }
}

struct PrReviewerRunner;

#[async_trait]
impl RoleRunner for PrReviewerRunner {
    fn role(&self) -> AgentRole {
        AgentRole::PrReviewer
    }

    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        if !ctx
            .manifests
            .accepts_payload("pr_reviewer", payload_type_name(&envelope.payload))
        {
            return Ok(vec![]);
        }

        let A2aPayload::TaskDelegation {
            task_description,
            context_capsule_uri: _context_capsule_uri,
        } = &envelope.payload
        else {
            return Ok(vec![]);
        };

        let intel = IntelligenceRequest::from_role_context(ctx).with_workflow("pr_review");
        let impact = ctx.services.analyze_impact(&intel).await?;

        let delta_note = if ctx.source_branch.is_some() && ctx.target_branch.is_some() {
            ctx.services
                .get_delta_context(&intel)
                .await
                .map(|d| {
                    d.delta_json
                        .get("summary")
                        .or_else(|| d.delta_json.get("modified_symbols"))
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| {
                            format!(
                                "delta {}..{}",
                                ctx.source_branch.as_deref().unwrap_or("HEAD"),
                                ctx.target_branch.as_deref().unwrap_or("main")
                            )
                        })
                })
                .unwrap_or_default()
        } else {
            String::new()
        };

        let insight = A2aEnvelope::new(
            ctx.conversation_id,
            AgentRole::PrReviewer,
            AgentRole::Gateway,
            A2aPayload::CodeInsight {
                summary: format!(
                    "PR review: {task_description} — {} {}",
                    impact.summary,
                    if delta_note.is_empty() {
                        String::new()
                    } else {
                        format!("(delta: {delta_note})")
                    }
                ),
                target_qualified_name: impact.target.clone(),
                risk_level: impact.risk_level,
                suggested_action: if impact.has_cycle_risk {
                    "reject_high_blast_radius".to_string()
                } else {
                    "approve_pr".to_string()
                },
            },
        )
        .with_task_id(ctx.task_id);
        record_bb(ctx, &insight).await;

        let mut replies = vec![insight];

        if impact.risk_level == RiskLevel::Low && !impact.has_cycle_risk {
            let accept = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::PrReviewer,
                AgentRole::Gateway,
                A2aPayload::Accept,
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &accept).await;
            replies.push(accept);
        } else if impact.risk_level == RiskLevel::Critical || impact.has_cycle_risk {
            let reject = A2aEnvelope::new(
                ctx.conversation_id,
                AgentRole::PrReviewer,
                AgentRole::Gateway,
                A2aPayload::Reject {
                    reason: impact.summary.clone(),
                },
            )
            .with_task_id(ctx.task_id);
            record_bb(ctx, &reject).await;
            replies.push(reject);
        }

        Ok(replies)
    }
}

struct IndexerRunner;

#[async_trait]
impl RoleRunner for IndexerRunner {
    fn role(&self) -> AgentRole {
        AgentRole::Indexer
    }

    async fn handle(&self, envelope: A2aEnvelope, ctx: &RoleContext) -> Result<Vec<A2aEnvelope>> {
        if let A2aPayload::GraphMutationSignal { affected_files, .. } = &envelope.payload {
            ctx.services
                .publish_graph_mutation(
                    &ctx.session_id,
                    &ctx.conversation_id.to_string(),
                    affected_files.clone(),
                )
                .await?;
            let _fresh = ctx
                .services
                .index_freshness_for_paths(&ctx.include_paths)
                .await;
        }
        Ok(vec![])
    }
}

/// True when graph-backed impact is unavailable (NullA2aServices or connect failure).
fn is_impact_stub(impact: &ImpactSummary) -> bool {
    impact.summary.contains("null services: no graph analysis")
        || impact.summary.starts_with("graph unavailable:")
}

fn payload_type_name(payload: &A2aPayload) -> &str {
    match payload {
        A2aPayload::TaskDelegation { .. } => "TaskDelegation",
        A2aPayload::StrategyProposal { .. } => "StrategyProposal",
        A2aPayload::CodeInsight { .. } => "CodeInsight",
        A2aPayload::GraphMutationSignal { .. } => "GraphMutationSignal",
        A2aPayload::Accept => "Accept",
        A2aPayload::Reject { .. } => "Reject",
        A2aPayload::FinalResult { .. } => "FinalResult",
    }
}

async fn record_bb(ctx: &RoleContext, envelope: &A2aEnvelope) {
    if let Some(w) = ctx.blackboard.as_deref() {
        let _ = blackboard_from_envelope(Some(w), &ctx.session_id, envelope).await;
    }
}
