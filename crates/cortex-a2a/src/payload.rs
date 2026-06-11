use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Risk level for structured code insights shared on the blackboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Internal message payloads (mapped to A2A `Part.data` under the blackboard extension).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2aPayload {
    TaskDelegation {
        task_description: String,
        context_capsule_uri: String,
    },
    StrategyProposal {
        estimated_complexity: u32,
        required_sub_nodes: Vec<String>,
    },
    CodeInsight {
        summary: String,
        target_qualified_name: String,
        risk_level: RiskLevel,
        suggested_action: String,
    },
    GraphMutationSignal {
        event_type: String,
        affected_files: Vec<String>,
    },
    Accept,
    Reject {
        reason: String,
    },
    FinalResult {
        data: Value,
    },
}
