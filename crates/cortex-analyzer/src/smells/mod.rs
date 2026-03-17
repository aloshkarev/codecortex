//! Code smell detection modules organized by category (refactoring.guru)
//!
//! This module provides a comprehensive code smell detection system based on
//! the refactoring.guru catalog, organized into five categories:
//!
//! - **Bloaters**: Code that has grown too large (LongFunction, LargeClass, etc.)
//! - **OO Abusers**: Incorrect OOP principles application
//! - **Change Preventers**: Code that resists modification
//! - **Dispensables**: Unnecessary code that should be removed
//! - **Couplers**: Excessive coupling between modules

mod bloaters;
mod change_preventers;
mod couplers;
mod dispensables;
mod language;
mod oo_abusers;

pub use bloaters::{
    detect_data_clumps, detect_large_classes, detect_long_functions, detect_long_parameter_lists,
    detect_primitive_obsession, detect_switch_statements,
};
pub use change_preventers::{
    detect_divergent_change, detect_parallel_inheritance, detect_shotgun_surgery,
    detect_shotgun_surgery_with_context,
};
pub use couplers::{
    detect_feature_envy, detect_feature_envy_with_context, detect_inappropriate_intimacy,
    detect_inappropriate_intimacy_with_context, detect_message_chains, detect_middle_man,
};
pub use dispensables::{
    detect_comments, detect_data_classes, detect_dead_code, detect_dead_code_with_context,
    detect_duplicate_code, detect_duplicate_code_with_context, detect_lazy_classes,
    detect_speculative_generality,
};
pub use oo_abusers::{detect_alternative_classes, detect_refused_bequest, detect_temporary_fields};

/// Detect deep nesting (backward compatibility)
pub fn detect_deep_nesting(
    source: &str,
    file_path: &str,
    config: &crate::SmellConfig,
) -> Vec<crate::CodeSmell> {
    crate::SmellDetector::with_config(config.clone()).detect_deep_nesting(source, file_path)
}

use serde::{Deserialize, Serialize};

/// Category of code smell (refactoring.guru classification)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmellCategory {
    /// Code that has grown too large to handle effectively
    Bloaters,
    /// Incomplete or incorrect application of OOP principles
    ObjectOrientedAbusers,
    /// Code that makes it hard to change and extend
    ChangePreventers,
    /// Unnecessary code that should be removed
    Dispensables,
    /// Excessive coupling between modules
    Couplers,
}

impl std::fmt::Display for SmellCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmellCategory::Bloaters => write!(f, "bloaters"),
            SmellCategory::ObjectOrientedAbusers => write!(f, "oo_abusers"),
            SmellCategory::ChangePreventers => write!(f, "change_preventers"),
            SmellCategory::Dispensables => write!(f, "dispensables"),
            SmellCategory::Couplers => write!(f, "couplers"),
        }
    }
}

/// Helper trait for detecting code smells
pub trait SmellDetector {
    /// Get the category this smell belongs to
    fn category(&self) -> SmellCategory;

    /// Get a brief description of the smell
    fn description(&self) -> &'static str;

    /// Get recommended refactorings for this smell
    fn recommended_refactorings(&self) -> Vec<&'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_deep_nesting_wrapper_uses_nesting_detector() {
        let config = crate::SmellConfig {
            max_nesting_depth: 2,
            ..Default::default()
        };

        let source = r#"
fn nested() {
    if a {
        if b {
            if c {
                do_work();
            }
        }
    }
}
"#;

        let smells = detect_deep_nesting(source, "test.rs", &config);
        assert!(!smells.is_empty());
        assert!(
            smells
                .iter()
                .any(|s| s.smell_type == crate::SmellType::DeepNesting)
        );
        assert!(
            !smells
                .iter()
                .any(|s| s.smell_type == crate::SmellType::LongFunction)
        );
    }
}
