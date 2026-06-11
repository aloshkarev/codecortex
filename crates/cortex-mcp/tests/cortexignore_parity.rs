//! Cross-surface parity: graph indexer discovery vs vector collection vs watch filter.

use cortex_core::{CortexConfig, CortexIgnoreOptions, CortexIgnoreWalker};
use cortex_indexer::{build_detector::ProjectConfig, collect_discoverable_source_files};
use cortex_mcp::collect_indexable_code_files;
use cortex_watcher::{EventFilter, WatchEventKind};
use std::collections::HashSet;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/cortexignore")
}

fn canonical_set(paths: impl IntoIterator<Item = PathBuf>) -> HashSet<PathBuf> {
    paths
        .into_iter()
        .map(|p| p.canonicalize().unwrap_or(p))
        .collect()
}

#[test]
fn graph_and_vector_file_sets_match_fixture() {
    let root = fixture_root();
    if !root.is_dir() {
        return;
    }

    let project = ProjectConfig::default();
    let graph_files = collect_discoverable_source_files(&root, &project, &[], &[], None);
    let vector_files = collect_indexable_code_files(&root, &CortexConfig::default(), &[]).unwrap();

    assert_eq!(
        canonical_set(graph_files),
        canonical_set(vector_files),
        "graph and vector discovery must agree"
    );
}

#[test]
fn nested_pkg_honors_root_and_nested_cortexignore() {
    let root = fixture_root();
    let pkg = root.join("pkg");
    if !pkg.is_dir() {
        return;
    }

    let project = ProjectConfig::default();
    let files = collect_discoverable_source_files(&pkg, &project, &[], &[], None);
    let names: HashSet<_> = files
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();

    assert!(names.contains("keep.rs"));
    assert!(!names.contains("out.tmp"));
    assert!(!names.contains("auto.rs"));
}

#[test]
fn event_filter_agrees_with_collect_for_sample_paths() {
    let root = fixture_root();
    if !root.is_dir() {
        return;
    }

    let filter = EventFilter::new().with_cortex_ignore(&root, &CortexConfig::default(), &[]);
    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.canonicalize().unwrap_or_else(|_| root.clone()),
        scan_root: None,
        global_ignore_path: None,
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });

    let samples = [
        root.join("keep.rs"),
        root.join("skip.rs"),
        root.join("generated/auto.rs"),
        root.join("pkg/src/keep.rs"),
        root.join("pkg/build/out.tmp"),
    ];

    for path in samples {
        if !path.exists() {
            continue;
        }
        let ignored = walker.is_ignored(&path);
        let should_process = filter.should_process(&path, WatchEventKind::Modified);
        assert_eq!(
            !ignored,
            should_process,
            "watch filter mismatch for {}",
            path.display()
        );
    }
}
