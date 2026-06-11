//! Process-wide MCP policy profile (`[mcp].profile` in config, legacy `CORTEX_MCP_PROFILE` env).

use crate::FeatureFlags;
use cortex_core::McpProfileKind;

/// Deployment profile for MCP defaults and feature gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpProfile {
    /// Developer-friendly defaults; `allow_source` in `recommend_tools` defaults to true.
    Dev,
    /// Tighter defaults for shared or commercial code: disables risky optional surfaces
    /// and defaults `allow_source` to false in routing helpers.
    Strict,
}

impl McpProfile {
    pub fn from_config_kind(kind: McpProfileKind) -> Self {
        match kind {
            McpProfileKind::Strict => Self::Strict,
            McpProfileKind::Dev => Self::Dev,
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("CORTEX_MCP_PROFILE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .trim()
        {
            "strict" | "enterprise" | "corp" => Self::Strict,
            _ => Self::Dev,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Strict => "strict",
        }
    }

    /// Apply profile on top of flags already loaded from env / CLI overrides.
    pub fn apply_to_flags(self, f: &mut FeatureFlags) {
        if !matches!(self, Self::Strict) {
            return;
        }
        f.vector_write = false;
        f.memory_write = false;
        f.memory_read = false;
        f.context_capsule = false;
    }

    pub fn default_allow_source_in_recommendations(self) -> bool {
        !matches!(self, Self::Strict)
    }

    /// Default `budget_tokens` scale factor (1.0 = unchanged). Strict uses slightly lower caps.
    pub fn default_context_budget_multiplier(self) -> f64 {
        match self {
            Self::Dev => 1.0,
            Self::Strict => 0.68,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::McpProfile;

    #[test]
    fn as_str_round_trips_dev_strict() {
        assert_eq!(McpProfile::Dev.as_str(), "dev");
        assert_eq!(McpProfile::Strict.as_str(), "strict");
    }
}
