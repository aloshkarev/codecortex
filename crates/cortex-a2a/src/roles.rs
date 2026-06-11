use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// CodeCortex agent role in the hybrid A2A topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Gateway,
    Indexer,
    Analyzer,
    PatchPlanner,
    PrReviewer,
    Validator,
}

impl AgentRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Indexer => "indexer",
            Self::Analyzer => "analyzer",
            Self::PatchPlanner => "patch_planner",
            Self::PrReviewer => "pr_reviewer",
            Self::Validator => "validator",
        }
    }

    pub fn all() -> &'static [AgentRole] {
        &[
            AgentRole::Gateway,
            AgentRole::Indexer,
            AgentRole::Analyzer,
            AgentRole::PatchPlanner,
            AgentRole::PrReviewer,
            AgentRole::Validator,
        ]
    }
}

impl fmt::Display for AgentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AgentRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gateway" => Ok(Self::Gateway),
            "indexer" => Ok(Self::Indexer),
            "analyzer" => Ok(Self::Analyzer),
            "patch_planner" | "patch-planner" | "patchplanner" => Ok(Self::PatchPlanner),
            "pr_reviewer" | "pr-reviewer" | "prreviewer" => Ok(Self::PrReviewer),
            "validator" => Ok(Self::Validator),
            other => Err(format!("unknown agent role: {other}")),
        }
    }
}
