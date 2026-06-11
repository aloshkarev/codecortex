//! Hierarchical `.cortexignore` oracle suite — parity between `is_ignored` and `collect_files`.

use cortex_core::{CortexIgnoreOptions, CortexIgnoreWalker, ensure_cortexignore_template};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct FixtureCase {
    name: &'static str,
    layout: &'static [(&'static str, &'static str)],
    repo_cortexignore: Option<&'static str>,
    nested_cortexignore: Option<(&'static str, &'static str)>,
    gitignore: Option<&'static str>,
    scan_subdir: Option<&'static str>,
    checks: &'static [(&'static str, bool)],
}

fn write_layout(root: &Path, layout: &[(&str, &str)]) {
    for (rel, content) in layout {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
}

fn canon(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
        .canonicalize()
        .unwrap_or_else(|_| root.join(rel))
}

fn assert_parity(walker: &CortexIgnoreWalker, root: &Path, rel: &str, expect_ignored: bool) {
    let path = root.join(rel);
    let ignored = walker.is_ignored(&path);
    assert_eq!(
        ignored,
        expect_ignored,
        "is_ignored mismatch for {rel} in {}",
        root.display()
    );

    let collected = walker
        .collect_files(root, None, |_| true)
        .unwrap_or_default();
    let set: HashSet<_> = collected
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();
    let cpath = path.canonicalize().unwrap_or_else(|_| path.clone());
    if path.is_file() {
        assert_eq!(
            !ignored,
            set.contains(&cpath),
            "collect/is_ignored parity for {rel}"
        );
    }
}

fn run_case(case: &FixtureCase) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    write_layout(root, case.layout);

    if let Some(content) = case.repo_cortexignore {
        fs::write(root.join(".cortexignore"), content).unwrap();
    }
    if let Some((dir, content)) = case.nested_cortexignore {
        let p = root.join(dir).join(".cortexignore");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }
    if let Some(content) = case.gitignore {
        fs::write(root.join(".gitignore"), content).unwrap();
        let _ = std::process::Command::new("git")
            .current_dir(root)
            .args(["init", "-q"])
            .status();
    }

    let scan_root = case
        .scan_subdir
        .map(|d| root.join(d))
        .unwrap_or_else(|| root.to_path_buf());

    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.to_path_buf(),
        scan_root: case.scan_subdir.map(|d| root.join(d)),
        global_ignore_path: None,
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });

    for (rel, expect_ignored) in case.checks {
        assert_parity(&walker, &scan_root, rel, *expect_ignored);
    }
}

const CASES: &[FixtureCase] = &[
    FixtureCase {
        name: "root_glob_ext",
        layout: &[("a.rs", ""), ("a.tmp", ""), ("pkg/b.rs", "")],
        repo_cortexignore: Some("*.tmp\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("a.rs", false), ("a.tmp", true), ("pkg/b.rs", false)],
    },
    FixtureCase {
        name: "dir_slash",
        layout: &[("keep.rs", ""), ("build/out.rs", "")],
        repo_cortexignore: Some("build/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("keep.rs", false), ("build/out.rs", true)],
    },
    FixtureCase {
        name: "dir_globstar",
        layout: &[
            ("keep.rs", ""),
            ("build/out.rs", ""),
            ("build/deep/x.rs", ""),
        ],
        repo_cortexignore: Some("build/**\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[
            ("keep.rs", false),
            ("build/out.rs", true),
            ("build/deep/x.rs", true),
        ],
    },
    FixtureCase {
        name: "anchored_root",
        layout: &[("tmp/x.rs", ""), ("src/tmp/x.rs", "")],
        repo_cortexignore: Some("/tmp/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("tmp/x.rs", true), ("src/tmp/x.rs", false)],
    },
    FixtureCase {
        name: "anywhere_globstar",
        layout: &[("a/vendor/x.rs", ""), ("vendor/y.rs", "")],
        repo_cortexignore: Some("**/vendor/**\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("a/vendor/x.rs", true), ("vendor/y.rs", true)],
    },
    FixtureCase {
        name: "negation_root",
        layout: &[("vendor/a.rs", ""), ("vendor/keep.rs", "")],
        repo_cortexignore: Some("vendor/**\n!vendor/keep.rs\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("vendor/a.rs", true), ("vendor/keep.rs", false)],
    },
    FixtureCase {
        name: "nested_cortexignore",
        layout: &[("pkg/a.rs", ""), ("pkg/a.tmp", ""), ("other.rs", "")],
        repo_cortexignore: None,
        nested_cortexignore: Some(("pkg", "*.tmp\n")),
        gitignore: None,
        scan_subdir: None,
        checks: &[
            ("pkg/a.rs", false),
            ("pkg/a.tmp", true),
            ("other.rs", false),
        ],
    },
    FixtureCase {
        name: "nested_negation",
        layout: &[("pkg/gen/a.rs", ""), ("pkg/gen/keep.rs", "")],
        repo_cortexignore: Some("generated/\n"),
        nested_cortexignore: Some(("pkg", "gen/**\n!gen/keep.rs\n")),
        gitignore: None,
        scan_subdir: None,
        checks: &[("pkg/gen/a.rs", true), ("pkg/gen/keep.rs", false)],
    },
    FixtureCase {
        name: "gitignore_plus_cortexignore",
        layout: &[("tracked.rs", ""), ("secret.rs", ""), ("local.rs", "")],
        repo_cortexignore: Some("local.rs\n"),
        nested_cortexignore: None,
        gitignore: Some("secret.rs\n"),
        scan_subdir: None,
        checks: &[
            ("tracked.rs", false),
            ("secret.rs", true),
            ("local.rs", true),
        ],
    },
    FixtureCase {
        name: "scan_subdir_monorepo",
        layout: &[
            ("generated/auto.rs", ""),
            ("pkg/src/main.rs", ""),
            ("pkg/other.txt", ""),
        ],
        repo_cortexignore: Some("generated/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: Some("pkg"),
        checks: &[("src/main.rs", false), ("other.txt", false)],
    },
    FixtureCase {
        name: "ignored_directory_nested",
        layout: &[("ignored/skip.rs", ""), ("keep.rs", "")],
        repo_cortexignore: Some("ignored/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("ignored/skip.rs", true), ("keep.rs", false)],
    },
    FixtureCase {
        name: "hidden_not_included",
        layout: &[(".hidden.rs", ""), ("visible.rs", "")],
        repo_cortexignore: None,
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[(".hidden.rs", true), ("visible.rs", false)],
    },
    FixtureCase {
        name: "double_star_filename",
        layout: &[("src/foo.generated.rs", ""), ("src/bar.rs", "")],
        repo_cortexignore: Some("*.generated.rs\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("src/foo.generated.rs", true), ("src/bar.rs", false)],
    },
    FixtureCase {
        name: "parent_dir_not_ignored",
        layout: &[("src/a.rs", ""), ("lib/b.rs", "")],
        repo_cortexignore: Some("src/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("src/a.rs", true), ("lib/b.rs", false)],
    },
    FixtureCase {
        name: "empty_cortexignore",
        layout: &[("a.rs", ""), ("b.rs", "")],
        repo_cortexignore: Some("\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("a.rs", false), ("b.rs", false)],
    },
    FixtureCase {
        name: "comment_lines",
        layout: &[("target/x.rs", ""), ("src/y.rs", "")],
        repo_cortexignore: Some("# build output\ntarget/\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("target/x.rs", true), ("src/y.rs", false)],
    },
    FixtureCase {
        name: "multiple_nested_ignores",
        layout: &[("a/x.rs", ""), ("a/y.tmp", ""), ("b/z.tmp", "")],
        repo_cortexignore: None,
        nested_cortexignore: Some(("a", "*.tmp\n")),
        gitignore: None,
        scan_subdir: None,
        checks: &[("a/x.rs", false), ("a/y.tmp", true), ("b/z.tmp", false)],
    },
    FixtureCase {
        name: "gitignore_negation",
        layout: &[("logs/a.log", ""), ("logs/keep.log", "")],
        repo_cortexignore: None,
        nested_cortexignore: None,
        gitignore: Some("logs/**\n!logs/keep.log\n"),
        scan_subdir: None,
        checks: &[("logs/a.log", true), ("logs/keep.log", false)],
    },
    FixtureCase {
        name: "scan_subdir_nested_ignore",
        layout: &[("pkg/build/out.tmp", ""), ("pkg/src/code.rs", "")],
        repo_cortexignore: None,
        nested_cortexignore: Some(("pkg", "*.tmp\n")),
        gitignore: None,
        scan_subdir: Some("pkg"),
        checks: &[("build/out.tmp", true), ("src/code.rs", false)],
    },
    FixtureCase {
        name: "root_file_only_pattern",
        layout: &[("skip.rs", ""), ("src/skip.rs", "")],
        repo_cortexignore: Some("/skip.rs\n"),
        nested_cortexignore: None,
        gitignore: None,
        scan_subdir: None,
        checks: &[("skip.rs", true), ("src/skip.rs", false)],
    },
];

#[test]
fn hierarchical_oracle_table() {
    for case in CASES {
        run_case(case);
    }
}

#[test]
fn hierarchical_oracle_generated_batch() {
    for i in 0..22 {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let dir = format!("gen{i}");
        fs::write(root.join(".cortexignore"), format!("{dir}/\n")).unwrap();
        fs::write(root.join("keep.rs"), "").unwrap();
        let gen_dir = root.join(&dir);
        fs::create_dir_all(&gen_dir).unwrap();
        fs::write(gen_dir.join("file.rs"), "").unwrap();

        let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
            repo_root: root.to_path_buf(),
            scan_root: None,
            global_ignore_path: None,
            respect_gitignore: false,
            respect_cortexignore: true,
            include_hidden: false,
            policy_excludes: vec![],
            count_ignored_skips: false,
        });
        assert_parity(&walker, root, "keep.rs", false);
        assert_parity(&walker, root, &format!("{dir}/file.rs"), true);
    }
}

#[test]
fn ensure_template_does_not_affect_ignore() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::write(root.join("src.rs"), "fn s() {}").unwrap();
    ensure_cortexignore_template(root, &["target/".to_string()]).unwrap();
    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.to_path_buf(),
        scan_root: None,
        global_ignore_path: None,
        respect_gitignore: false,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });
    assert!(!walker.is_ignored(&root.join("src.rs")));
}

#[test]
fn nonexistent_path_under_visible_dir_not_ignored() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.to_path_buf(),
        scan_root: None,
        global_ignore_path: None,
        respect_gitignore: false,
        respect_cortexignore: false,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });
    assert!(!walker.is_ignored(&root.join("src/deleted.rs")));
}

#[test]
fn nonexistent_path_under_ignored_dir_is_ignored() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    fs::write(root.join(".cortexignore"), "ignored/\n").unwrap();
    fs::create_dir_all(root.join("ignored")).unwrap();
    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root: root.to_path_buf(),
        scan_root: None,
        global_ignore_path: None,
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: vec![],
        count_ignored_skips: false,
    });
    assert!(walker.is_ignored(&root.join("ignored/deleted.rs")));
}
