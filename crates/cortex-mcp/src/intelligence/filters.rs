//! Path scope filters (aligned with MCP include/exclude policy).

#[derive(Debug, Clone, Default)]
pub struct ScopeFilters {
    pub include_paths: Vec<String>,
    pub exclude_paths: Vec<String>,
}

impl ScopeFilters {
    pub fn new(include_paths: Vec<String>, exclude_paths: Vec<String>) -> Self {
        Self {
            include_paths,
            exclude_paths,
        }
    }
}

/// True when `path` passes include (if any) and is not excluded.
pub fn path_matches_scope(path: &str, scope: &ScopeFilters) -> bool {
    if scope
        .exclude_paths
        .iter()
        .any(|p| !p.is_empty() && path.contains(p))
    {
        return false;
    }
    if scope.include_paths.is_empty() {
        return true;
    }
    scope
        .include_paths
        .iter()
        .any(|p| !p.is_empty() && path.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn include_and_exclude() {
        let scope = ScopeFilters::new(
            vec!["crates/cortex-mcp".to_string()],
            vec!["generated".to_string()],
        );
        assert!(path_matches_scope(
            "crates/cortex-mcp/src/handler.rs",
            &scope
        ));
        assert!(!path_matches_scope(
            "crates/cortex-mcp/src/generated/foo.rs",
            &scope
        ));
        assert!(!path_matches_scope("crates/other/lib.rs", &scope));
    }
}
