//! Protocol-native cooperation artifacts (A2A spec §4.1.7 Artifact + Part.data).

use crate::wire::{A2aPart, ArtifactWire};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Extension URI for intelligence ↔ MCP cooperation schema.
pub const EXTENSION_INTELLIGENCE_COOPERATION: &str =
    "https://codecortex.dev/extensions/intelligence-cooperation/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CooperationArtifactKind {
    IntelligencePack,
    ToolDelegation,
    CapsuleRef,
    WorkflowResult,
}

impl CooperationArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IntelligencePack => "intelligence_pack",
            Self::ToolDelegation => "tool_delegation",
            Self::CapsuleRef => "capsule_ref",
            Self::WorkflowResult => "workflow_result",
        }
    }
}

/// Typed cooperation artifact stored on tasks and mapped to spec `Artifact`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CooperationArtifact {
    pub artifact_kind: String,
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_tool_id: Option<String>,
    pub freshness: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_next_tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capsule_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_card_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}

impl CooperationArtifact {
    pub fn artifact_id_for(task_id: &str, kind: CooperationArtifactKind, tool: &str) -> String {
        format!("{task_id}/{}/{}", kind.as_str(), tool)
    }

    pub fn from_intelligence_pack(
        task_id: &str,
        mcp_tool_id: &str,
        data: Value,
        freshness: &str,
        suggested_next_tools: Vec<String>,
        budget_tokens: u32,
        estimated_tokens: usize,
        capsule_uri: Option<String>,
    ) -> Self {
        Self {
            artifact_kind: CooperationArtifactKind::IntelligencePack
                .as_str()
                .to_string(),
            artifact_id: Self::artifact_id_for(
                task_id,
                CooperationArtifactKind::IntelligencePack,
                mcp_tool_id,
            ),
            name: Some(format!("codecortex.{mcp_tool_id}")),
            description: Some(format!(
                "Graph-backed intelligence from MCP tool {mcp_tool_id}"
            )),
            data,
            mcp_tool_id: Some(mcp_tool_id.to_string()),
            freshness: freshness.to_string(),
            suggested_next_tools,
            budget_tokens: Some(budget_tokens),
            estimated_tokens: Some(estimated_tokens),
            source_policy: Some("snippets".to_string()),
            capsule_uri,
            agent_card_url: None,
            role: None,
            scope: None,
            extensions: vec![EXTENSION_INTELLIGENCE_COOPERATION.to_string()],
        }
    }

    pub fn from_tool_delegation(
        task_id: &str,
        role: &str,
        agent_card_url: &str,
        suggested_next_tools: Vec<String>,
        scope: Value,
        freshness: &str,
    ) -> Self {
        Self {
            artifact_kind: CooperationArtifactKind::ToolDelegation.as_str().to_string(),
            artifact_id: Self::artifact_id_for(
                task_id,
                CooperationArtifactKind::ToolDelegation,
                role,
            ),
            name: Some(format!("codecortex.delegation.{role}")),
            description: Some(format!("MCP tool delegation for external role {role}")),
            data: json!({
                "role": role,
                "agentCardUrl": agent_card_url,
                "suggestedNextTools": suggested_next_tools,
                "scope": scope,
            }),
            mcp_tool_id: None,
            freshness: freshness.to_string(),
            suggested_next_tools,
            budget_tokens: None,
            estimated_tokens: None,
            source_policy: None,
            capsule_uri: None,
            agent_card_url: Some(agent_card_url.to_string()),
            role: Some(role.to_string()),
            scope: Some(scope),
            extensions: vec![EXTENSION_INTELLIGENCE_COOPERATION.to_string()],
        }
    }

    pub fn from_workflow_result(task_id: &str, workflow: &str, data: Value) -> Self {
        let mcp_tool_id = data
            .get("mcpToolId")
            .or_else(|| data.get("mcp_tool_id"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let freshness = data
            .get("freshness")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let suggested_next_tools = data
            .get("suggestedNextTools")
            .or_else(|| data.get("suggested_next_tools"))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let capsule_uri = data
            .get("capsuleUri")
            .or_else(|| data.get("capsule_uri"))
            .and_then(Value::as_str)
            .map(str::to_string);
        Self {
            artifact_kind: CooperationArtifactKind::WorkflowResult.as_str().to_string(),
            artifact_id: Self::artifact_id_for(
                task_id,
                CooperationArtifactKind::WorkflowResult,
                workflow,
            ),
            name: Some(format!("codecortex.workflow.{workflow}")),
            description: Some(format!("Workflow {workflow} final result")),
            data,
            mcp_tool_id,
            freshness,
            suggested_next_tools,
            budget_tokens: None,
            estimated_tokens: None,
            source_policy: None,
            capsule_uri,
            agent_card_url: None,
            role: None,
            scope: None,
            extensions: vec![EXTENSION_INTELLIGENCE_COOPERATION.to_string()],
        }
    }

    /// Parse legacy flat JSON or typed cooperation artifact.
    pub fn from_value(v: &Value) -> Self {
        if v.get("artifactId").is_some() || v.get("artifact_id").is_some() {
            return serde_json::from_value(v.clone()).unwrap_or_else(|_| legacy_from_value(v));
        }
        legacy_from_value(v)
    }

    pub fn to_wire(&self) -> ArtifactWire {
        ArtifactWire {
            artifact_id: self.artifact_id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            parts: vec![A2aPart {
                text: None,
                data: Some(self.data.clone()),
                metadata: None,
                media_type: Some("application/json".to_string()),
            }],
            metadata: Some(json!({
                "mcpToolId": self.mcp_tool_id,
                "freshness": self.freshness,
                "suggestedNextTools": self.suggested_next_tools,
                "budgetTokens": self.budget_tokens,
                "estimatedTokens": self.estimated_tokens,
                "sourcePolicy": self.source_policy,
                "capsuleUri": self.capsule_uri,
                "artifactKind": self.artifact_kind,
                "extensionUri": EXTENSION_INTELLIGENCE_COOPERATION,
                "agentCardUrl": self.agent_card_url,
                "role": self.role,
                "scope": self.scope,
            })),
            extensions: if self.extensions.is_empty() {
                vec![EXTENSION_INTELLIGENCE_COOPERATION.to_string()]
            } else {
                self.extensions.clone()
            },
        }
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| self.data.clone())
    }
}

fn legacy_from_value(v: &Value) -> CooperationArtifact {
    let kind = v
        .get("artifact_kind")
        .and_then(Value::as_str)
        .unwrap_or("legacy");
    CooperationArtifact {
        artifact_kind: kind.to_string(),
        artifact_id: format!("legacy/{kind}"),
        name: Some("codecortex.result".to_string()),
        description: None,
        data: v.clone(),
        mcp_tool_id: v
            .get("mcp_tool_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        freshness: v
            .get("freshness")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        suggested_next_tools: v
            .get("suggested_next_tools")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        budget_tokens: None,
        estimated_tokens: None,
        source_policy: None,
        capsule_uri: v
            .get("capsule_uri")
            .and_then(Value::as_str)
            .map(str::to_string),
        agent_card_url: v
            .get("agent_card_url")
            .and_then(Value::as_str)
            .map(str::to_string),
        role: v.get("role").and_then(Value::as_str).map(str::to_string),
        scope: v.get("scope").cloned(),
        extensions: vec![EXTENSION_INTELLIGENCE_COOPERATION.to_string()],
    }
}

pub fn task_cooperation_metadata(
    workflow: &str,
    include_paths: &[String],
    suggested_next_tools: &[String],
    freshness: &str,
) -> Value {
    json!({
        "workflow": workflow,
        "includePaths": include_paths,
        "suggestedNextTools": suggested_next_tools,
        "freshness": freshness,
        "extensionUri": EXTENSION_INTELLIGENCE_COOPERATION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_artifact_has_metadata_outside_data() {
        let art = CooperationArtifact::from_intelligence_pack(
            "task-1",
            "get_patch_context",
            json!({"task": "fix auth"}),
            "fresh",
            vec!["get_api_contract".to_string()],
            6000,
            512,
            Some("codecortex://capsule/1".to_string()),
        );
        let wire = art.to_wire();
        assert_eq!(
            wire.artifact_id,
            "task-1/intelligence_pack/get_patch_context"
        );
        assert!(wire.metadata.is_some());
        let meta = wire.metadata.unwrap();
        assert_eq!(meta["mcpToolId"], "get_patch_context");
        assert_eq!(meta["freshness"], "fresh");
        assert!(
            wire.extensions
                .contains(&EXTENSION_INTELLIGENCE_COOPERATION.to_string())
        );
    }
}
