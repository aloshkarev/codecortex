//! Unified intelligence response pack for MCP envelopes and A2A artifacts.

use crate::contracts::{
    EnvelopeBuilder, FreshnessState, OmittedItem, ResponseScope, SourcePolicy, TokenBudget,
};
use rmcp::model::CallToolResult;
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Instant;

/// Metadata shared by MCP context tools and A2A intelligence artifacts.
#[derive(Debug, Clone, Serialize)]
pub struct IntelligenceMeta {
    pub freshness: FreshnessState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub budget_tokens: u32,
    pub estimated_tokens: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_next_tools: Vec<String>,
    pub source_policy: SourcePolicy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_tool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capsule_uri: Option<String>,
}

impl Default for IntelligenceMeta {
    fn default() -> Self {
        Self {
            freshness: FreshnessState::Unknown,
            warnings: Vec::new(),
            budget_tokens: 6000,
            estimated_tokens: 0,
            suggested_next_tools: Vec::new(),
            source_policy: SourcePolicy::Snippets,
            mcp_tool_id: None,
            capsule_uri: None,
        }
    }
}

/// Tool-specific payload plus agent-facing metadata.
#[derive(Debug, Clone, Serialize)]
pub struct IntelligencePack {
    pub data: Value,
    pub meta: IntelligenceMeta,
}

impl IntelligencePack {
    pub fn new(data: Value, meta: IntelligenceMeta) -> Self {
        Self { data, meta }
    }

    pub fn with_tool(mut self, tool_id: impl Into<String>) -> Self {
        self.meta.mcp_tool_id = Some(tool_id.into());
        self
    }

    pub fn with_capsule_uri(mut self, uri: impl Into<String>) -> Self {
        self.meta.capsule_uri = Some(uri.into());
        self
    }

    pub fn freshness_label(&self) -> &'static str {
        match self.meta.freshness {
            FreshnessState::Fresh => "fresh",
            FreshnessState::Warming => "warming",
            FreshnessState::Stale => "stale",
            FreshnessState::Partial => "partial",
            FreshnessState::Unknown => "unknown",
        }
    }

    /// Build MCP tool envelope (additive meta on standard envelope shape).
    pub fn to_envelope(
        self,
        tool_id: &str,
        started: Instant,
        include_paths: Vec<String>,
        exclude_paths: Vec<String>,
        omitted: Vec<OmittedItem>,
    ) -> CallToolResult {
        EnvelopeBuilder::new(started)
            .audit_tool(tool_id)
            .cost_class("bounded")
            .freshness(self.meta.freshness)
            .token_budget(TokenBudget {
                requested_tokens: self.meta.budget_tokens as usize,
                estimated_tokens: self.meta.estimated_tokens,
                hard_cap: true,
            })
            .scope(ResponseScope {
                repo_path: None,
                branch: None,
                include_paths,
                exclude_paths,
            })
            .source_policy(self.meta.source_policy)
            .omitted(omitted)
            .next_tools(self.meta.suggested_next_tools)
            .warnings(self.meta.warnings)
            .success(self.data)
    }

    /// JSON artifact part for A2A task history / blackboard (legacy flat JSON).
    pub fn to_a2a_artifact_json(&self) -> Value {
        json!({
            "artifact_kind": "intelligence_pack",
            "mcp_tool_id": self.meta.mcp_tool_id,
            "capsule_uri": self.meta.capsule_uri,
            "freshness": self.freshness_label(),
            "warnings": self.meta.warnings,
            "budget_tokens": self.meta.budget_tokens,
            "estimated_tokens": self.meta.estimated_tokens,
            "suggested_next_tools": self.meta.suggested_next_tools,
            "source_policy": "snippets",
            "data": self.data,
        })
    }

    /// Protocol-native cooperation artifact for A2A task store.
    pub fn to_cooperation_artifact(&self, task_id: &str) -> cortex_a2a::CooperationArtifact {
        cortex_a2a::CooperationArtifact::from_intelligence_pack(
            task_id,
            self.meta.mcp_tool_id.as_deref().unwrap_or("unknown"),
            self.data.clone(),
            self.freshness_label(),
            self.meta.suggested_next_tools.clone(),
            self.meta.budget_tokens,
            self.meta.estimated_tokens,
            self.meta.capsule_uri.clone(),
        )
    }
}

pub fn parse_freshness_label(label: &str) -> FreshnessState {
    match label.to_ascii_lowercase().as_str() {
        "fresh" => FreshnessState::Fresh,
        "warming" => FreshnessState::Warming,
        "stale" => FreshnessState::Stale,
        "partial" => FreshnessState::Partial,
        _ => FreshnessState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_artifact_includes_meta_fields() {
        let pack = IntelligencePack::new(
            json!({"task": "test"}),
            IntelligenceMeta {
                freshness: FreshnessState::Fresh,
                suggested_next_tools: vec!["get_api_contract".to_string()],
                mcp_tool_id: Some("get_patch_context".to_string()),
                ..Default::default()
            },
        );
        let art = pack.to_a2a_artifact_json();
        assert_eq!(art["freshness"], "fresh");
        assert_eq!(art["mcp_tool_id"], "get_patch_context");
        assert!(art["suggested_next_tools"].is_array());
    }
}
