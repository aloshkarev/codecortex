//! Index scope: how filesystem scan paths map to graph `repository_path` keys (FalkorDB).

use std::path::{Path, PathBuf};

/// Walk upward from `path` to find a directory containing `.git`.
pub fn find_git_repository_root(path: &Path) -> Option<PathBuf> {
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

/// Canonical graph `repository_path` for an index operation.
///
/// When no explicit override is given, uses the **scan path** (project folder), not the git
/// repository root. Branch/commit still come from [`crate::GitOperations`] on the git root.
pub fn graph_repository_path_for_index(scan_path: &Path, override_path: Option<&str>) -> String {
    if let Some(p) = override_path {
        return p.to_string();
    }
    canonical_display_path(scan_path)
}

/// Lossy canonical path string for stable FalkorDB `repository_path` / branch-index keys.
pub fn canonical_display_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins() {
        let p = Path::new("/tmp/proj");
        assert_eq!(
            graph_repository_path_for_index(p, Some("/explicit")),
            "/explicit"
        );
    }

    #[test]
    fn default_uses_scan_path_display() {
        let dir = std::env::temp_dir().join("cortex_index_scope_test");
        let _ = std::fs::create_dir_all(&dir);
        let got = graph_repository_path_for_index(&dir, None);
        assert!(got.contains("cortex_index_scope_test"));
    }
}
