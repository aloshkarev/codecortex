//! FalkorDB `repository_path` keys should match the indexed project folder, not only the git root.

use cortex_core::graph_repository_path_for_index;
use std::path::Path;

#[test]
fn graph_scope_defaults_to_scan_path_not_git_root() {
    let dir = std::env::temp_dir().join("cortex_graph_scope_nested");
    let _ = std::fs::create_dir_all(&dir);
    let scope = graph_repository_path_for_index(&dir, None);
    assert!(
        scope.contains("cortex_graph_scope_nested"),
        "expected scan path in graph scope, got {scope}"
    );
}

/// Daemon index jobs must use the same key as `index_path_with_branch_context`.
#[test]
fn graph_scope_is_stable_canonical_display() {
    let dir = std::env::temp_dir().join("cortex_graph_scope_canonical");
    let _ = std::fs::create_dir_all(&dir);
    let a = graph_repository_path_for_index(&dir, None);
    let b = graph_repository_path_for_index(&dir, None);
    assert_eq!(a, b);
}

#[test]
fn graph_scope_override_wins() {
    let dir = Path::new("/tmp/ignored");
    assert_eq!(
        graph_repository_path_for_index(dir, Some("/explicit/project")),
        "/explicit/project"
    );
}
