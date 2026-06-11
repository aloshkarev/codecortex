//! Shared `.cortexignore` / `.gitignore` discovery for graph index, vector index, and watch.
//!
//! Policy excludes (`exclude_patterns`, config, CLI) are applied **after** ignore-file rules
//! and cannot un-ignore paths negated in `.cortexignore` / `.gitignore`.
//!
//! Ignore-file semantics come from a single [`WalkBuilder`](ignore::WalkBuilder) configuration
//! shared by [`CortexIgnoreWalker::collect_files`] and [`CortexIgnoreWalker::is_ignored`].

use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{CortexError, Result};

pub const CORTEXIGNORE_FILENAME: &str = ".cortexignore";

/// Default relative path under `$HOME` for global ignore rules.
pub const DEFAULT_GLOBAL_CORTEXIGNORE_REL: &str = ".cortex/cortexignore";

/// Resolve the default global `.cortexignore` path (`~/.cortex/cortexignore`).
pub fn default_global_cortexignore_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(DEFAULT_GLOBAL_CORTEXIGNORE_REL))
}

/// Options for [`CortexIgnoreWalker`].
#[derive(Debug, Clone)]
pub struct CortexIgnoreOptions {
    /// Git / project root where ignore files are loaded from ancestors.
    pub repo_root: PathBuf,
    /// Filesystem subtree to emit during collection; defaults to [`Self::repo_root`].
    pub scan_root: Option<PathBuf>,
    pub global_ignore_path: Option<PathBuf>,
    pub respect_gitignore: bool,
    pub respect_cortexignore: bool,
    pub include_hidden: bool,
    /// Hard excludes merged from build detection, project policy, config, and CLI.
    pub policy_excludes: Vec<String>,
    /// When true, [`CortexIgnoreWalker::collect_files_with_stats`] counts ignore-rule skips.
    pub count_ignored_skips: bool,
}

impl Default for CortexIgnoreOptions {
    fn default() -> Self {
        Self {
            repo_root: PathBuf::new(),
            scan_root: None,
            global_ignore_path: default_global_cortexignore_path(),
            respect_gitignore: true,
            respect_cortexignore: true,
            include_hidden: false,
            policy_excludes: Vec::new(),
            count_ignored_skips: false,
        }
    }
}

/// Result of a file collection pass with optional ignore statistics.
#[derive(Debug, Clone, Default)]
pub struct CollectFilesResult {
    pub files: Vec<PathBuf>,
    /// Files that would match `accept` but were skipped by ignore rules (not policy).
    pub ignored_by_rules: usize,
}

/// Walks a repository applying `.gitignore`, nested `.cortexignore`, optional global ignore,
/// and policy exclude patterns.
#[derive(Debug, Clone)]
pub struct CortexIgnoreWalker {
    options: CortexIgnoreOptions,
}

impl CortexIgnoreWalker {
    pub fn new(options: CortexIgnoreOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> &CortexIgnoreOptions {
        &self.options
    }

    fn repo_root(&self) -> PathBuf {
        normalize_root(&self.options.repo_root)
    }

    fn scan_root(&self) -> PathBuf {
        self.options
            .scan_root
            .as_ref()
            .map(|p| normalize_root(p))
            .unwrap_or_else(|| self.repo_root())
    }

    fn walk_builder(&self, root: &Path) -> WalkBuilder {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(self.options.include_hidden);
        builder.git_ignore(self.options.respect_gitignore);
        builder.git_exclude(self.options.respect_gitignore);
        if self.options.respect_cortexignore {
            builder.add_custom_ignore_filename(CORTEXIGNORE_FILENAME);
        }
        if let Some(ref global) = self.options.global_ignore_path {
            if global.is_file() {
                builder.add_ignore(global);
            }
        }
        builder
    }

    fn walk_builder_unfiltered(&self, root: &Path) -> WalkBuilder {
        let mut builder = WalkBuilder::new(root);
        builder.hidden(self.options.include_hidden);
        builder.git_ignore(false);
        builder.git_exclude(false);
        builder
    }

    /// Collect files under the effective scan root, optionally restricted to `include_files`.
    pub fn collect_files<F>(
        &self,
        root: &Path,
        include_files: Option<&[PathBuf]>,
        accept: F,
    ) -> Result<Vec<PathBuf>>
    where
        F: Fn(&Path) -> bool,
    {
        Ok(self
            .collect_files_with_stats(root, include_files, accept)?
            .files)
    }

    /// Like [`Self::collect_files`] but returns ignore-rule skip counts when enabled.
    pub fn collect_files_with_stats<F>(
        &self,
        root: &Path,
        include_files: Option<&[PathBuf]>,
        accept: F,
    ) -> Result<CollectFilesResult>
    where
        F: Fn(&Path) -> bool,
    {
        let _ = root;
        let scan_root = self.scan_root();
        let repo_root = self.repo_root();

        if scan_root.is_file() {
            if accept(&scan_root) && !self.is_ignored(&scan_root) {
                return Ok(CollectFilesResult {
                    files: vec![scan_root],
                    ignored_by_rules: 0,
                });
            }
            return Ok(CollectFilesResult {
                files: Vec::new(),
                ignored_by_rules: if accept(&scan_root) && self.is_ignored_by_rules(&scan_root) {
                    1
                } else {
                    0
                },
            });
        }

        if let Some(selected) = include_files {
            if !selected.is_empty() {
                return Ok(self.collect_explicit_files(&scan_root, selected, &accept));
            }
        }

        let mut files = Vec::new();
        let mut policy_excluded = 0usize;
        for entry in self.walk_builder(&repo_root).build().flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let entry_path = entry.path();
            if !is_under_or_equal(&entry_path, &scan_root) {
                continue;
            }
            if !self.options.include_hidden && is_hidden_entry(entry_path) {
                continue;
            }
            if is_policy_excluded(entry_path, &self.options.policy_excludes) {
                policy_excluded += 1;
                continue;
            }
            if accept(entry_path) {
                files.push(entry_path.to_path_buf());
            }
        }

        let ignored_by_rules = if self.options.count_ignored_skips {
            self.count_ignored_rule_skips(&repo_root, &scan_root, &accept)?
        } else {
            0
        };
        let _ = policy_excluded;

        Ok(CollectFilesResult {
            files,
            ignored_by_rules,
        })
    }

    fn count_ignored_rule_skips<F>(
        &self,
        repo_root: &Path,
        scan_root: &Path,
        accept: &F,
    ) -> Result<usize>
    where
        F: Fn(&Path) -> bool,
    {
        let mut unfiltered = 0usize;
        for entry in self.walk_builder_unfiltered(repo_root).build().flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let entry_path = entry.path();
            if !is_under_or_equal(entry_path, scan_root) {
                continue;
            }
            if !self.options.include_hidden && is_hidden_entry(entry_path) {
                continue;
            }
            if is_policy_excluded(entry_path, &self.options.policy_excludes) {
                continue;
            }
            if accept(entry_path) {
                unfiltered += 1;
            }
        }

        let mut filtered = 0usize;
        for entry in self.walk_builder(repo_root).build().flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let entry_path = entry.path();
            if !is_under_or_equal(entry_path, scan_root) {
                continue;
            }
            if !self.options.include_hidden && is_hidden_entry(entry_path) {
                continue;
            }
            if is_policy_excluded(entry_path, &self.options.policy_excludes) {
                continue;
            }
            if accept(entry_path) {
                filtered += 1;
            }
        }

        Ok(unfiltered.saturating_sub(filtered))
    }

    fn collect_explicit_files<F>(
        &self,
        root: &Path,
        include_files: &[PathBuf],
        accept: &F,
    ) -> CollectFilesResult
    where
        F: Fn(&Path) -> bool,
    {
        let mut seen = HashSet::new();
        let mut files = Vec::new();
        let mut ignored_by_rules = 0usize;
        for selected in include_files {
            let candidate = if selected.is_absolute() {
                selected.clone()
            } else {
                root.join(selected)
            };
            let candidate = candidate.canonicalize().unwrap_or(candidate);
            if !candidate.is_file() {
                continue;
            }
            if !accept(&candidate) {
                continue;
            }
            if self.is_ignored(&candidate) {
                if self.is_ignored_by_rules(&candidate) {
                    ignored_by_rules += 1;
                }
                continue;
            }
            if seen.insert(candidate.clone()) {
                files.push(candidate);
            }
        }
        CollectFilesResult {
            files,
            ignored_by_rules,
        }
    }

    /// Returns true when `path` should not be indexed or watched.
    pub fn is_ignored(&self, path: &Path) -> bool {
        if is_policy_excluded(path, &self.options.policy_excludes) {
            return true;
        }
        self.is_ignored_by_rules(path)
    }

    fn is_ignored_by_rules(&self, path: &Path) -> bool {
        if !self.options.include_hidden && is_hidden_entry(path) {
            return true;
        }
        let repo_root = self.repo_root();
        let target = canonicalize_lossy(path);
        if !is_under_or_equal(&target, &repo_root) {
            return true;
        }
        !self.path_visible_in_walk(&repo_root, &target)
    }

    /// True when `target` appears in an ignore-filtered walk from `repo_root`.
    fn path_visible_in_walk(&self, repo_root: &Path, target: &Path) -> bool {
        let target = canonicalize_lossy(target);
        let mut builder = self.walk_builder(repo_root);
        let target_for_filter = target.clone();
        builder.filter_entry(move |entry| on_path_to_target(entry.path(), &target_for_filter));

        for entry in builder.build().flatten() {
            let entry_path = canonicalize_lossy(entry.path());
            if entry_path == target {
                return true;
            }
        }

        if target.exists() {
            return false;
        }

        // Non-existent paths: visible when their deepest existing ancestor is visible
        // and the missing leaf is not inside an ignored directory entry.
        let mut ancestor = target.as_path();
        while !ancestor.exists() {
            ancestor = match ancestor.parent() {
                Some(p) if is_under_or_equal(p, repo_root) => p,
                _ => return false,
            };
        }

        let ancestor = canonicalize_lossy(ancestor);
        if !self.path_visible_in_walk(repo_root, &ancestor) {
            return false;
        }

        // Missing file under a visible directory is not ignored (e.g. delete events).
        true
    }
}

/// Policy-layer exclude (build detector, project policy, config, CLI).
pub fn is_policy_excluded(path: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    patterns
        .iter()
        .any(|pattern| policy_pattern_matches(&path_str, pattern))
}

fn policy_pattern_matches(path_str: &str, pattern: &str) -> bool {
    if pattern.ends_with("/**") {
        let dir = &pattern[..pattern.len() - 3];
        if dir.contains('/') || dir.contains('\\') {
            let dir_with_sep = format!("{}/", dir.replace('\\', "/"));
            let normalized = path_str.replace('\\', "/");
            normalized.contains(&dir_with_sep)
                || normalized.ends_with(&dir_with_sep[..dir_with_sep.len() - 1])
        } else {
            Path::new(path_str)
                .components()
                .any(|c| c.as_os_str() == std::ffi::OsStr::new(dir))
        }
    } else if pattern.starts_with("*.") {
        let ext = &pattern[1..];
        Path::new(path_str)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()) == ext)
            .unwrap_or(false)
    } else {
        path_str.contains(pattern)
    }
}

fn normalize_root(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent()
            .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_under_or_equal(path: &Path, root: &Path) -> bool {
    let path = canonicalize_lossy(path);
    let root = canonicalize_lossy(root);
    path.starts_with(&root)
}

/// Keep only entries on the path from walk root to `target` (inclusive).
fn on_path_to_target(entry_path: &Path, target: &Path) -> bool {
    let entry = canonicalize_lossy(entry_path);
    let target = canonicalize_lossy(target);
    target.starts_with(&entry)
}

fn is_hidden_entry(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| {
            name.starts_with('.') && name != ".gitignore" && name != CORTEXIGNORE_FILENAME
        })
}

/// Write a starter `.cortexignore` when missing, seeded from `default_patterns`.
pub fn ensure_cortexignore_template(repo_root: &Path, default_patterns: &[String]) -> Result<()> {
    let path = repo_root.join(CORTEXIGNORE_FILENAME);
    if path.exists() {
        return Ok(());
    }
    let mut lines =
        vec!["# CodeCortex indexing ignores (git may still track these paths)".to_string()];
    for pattern in default_patterns {
        if !pattern.trim().is_empty() {
            lines.push(pattern.trim().to_string());
        }
    }
    std::fs::write(&path, lines.join("\n") + "\n")
        .map_err(|e| CortexError::Io(format!("failed to write {}: {e}", path.display())))?;
    Ok(())
}

/// Parity helper: `is_ignored(p)` should equal `!collected.contains(p)`.
#[cfg(test)]
pub fn assert_collect_is_ignored_parity(
    walker: &CortexIgnoreWalker,
    root: &Path,
    paths: &[PathBuf],
) {
    let collected = walker
        .collect_files(root, None, |_| true)
        .unwrap_or_default();
    let collected_set: HashSet<_> = collected.iter().map(|p| canonicalize_lossy(p)).collect();
    for path in paths {
        let canon = canonicalize_lossy(path);
        let ignored = walker.is_ignored(path);
        let in_collect = collected_set.contains(&canon);
        assert_eq!(
            ignored,
            !in_collect,
            "parity mismatch for {}: is_ignored={ignored}, in_collect={in_collect}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn walker_for(root: &Path, policy: Vec<String>) -> CortexIgnoreWalker {
        CortexIgnoreWalker::new(CortexIgnoreOptions {
            repo_root: root.to_path_buf(),
            scan_root: None,
            global_ignore_path: None,
            respect_gitignore: true,
            respect_cortexignore: true,
            include_hidden: false,
            policy_excludes: policy,
            count_ignored_skips: false,
        })
    }

    fn parity_paths(root: &Path, walker: &CortexIgnoreWalker) {
        let collected = walker.collect_files(root, None, |_| true).unwrap();
        for path in &collected {
            assert!(
                !walker.is_ignored(path),
                "collected path should not be ignored: {}",
                path.display()
            );
        }
        fn walk_dir(
            dir: &Path,
            root: &Path,
            walker: &CortexIgnoreWalker,
            collected: &HashSet<PathBuf>,
        ) {
            let Ok(read) = fs::read_dir(dir) else {
                return;
            };
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, root, walker, collected);
                } else if path.is_file() {
                    let canon = canonicalize_lossy(&path);
                    if collected.contains(&canon) {
                        continue;
                    }
                    assert!(
                        walker.is_ignored(&path),
                        "non-collected file should be ignored: {}",
                        path.display()
                    );
                }
            }
        }
        let set: HashSet<_> = collected.iter().map(|p| canonicalize_lossy(p)).collect();
        walk_dir(root, root, walker, &set);
    }

    #[test]
    fn cortexignore_excludes_nested_generated() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".cortexignore"), "generated/\n").unwrap();
        let gen_dir = root.join("src").join("generated");
        fs::create_dir_all(&gen_dir).unwrap();
        fs::write(gen_dir.join("skip.rs"), "fn skip() {}").unwrap();
        fs::write(root.join("src").join("keep.rs"), "fn keep() {}").unwrap();

        let walker = walker_for(root, vec![]);
        let files = walker
            .collect_files(root, None, |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<HashSet<_>>();

        assert!(files.contains("keep.rs"));
        assert!(!files.contains("skip.rs"));
        parity_paths(root, &walker);
    }

    #[test]
    fn cortexignore_negation_unignores_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".cortexignore"), "vendor/**\n!important.rs\n").unwrap();
        let vendor = root.join("vendor");
        fs::create_dir_all(&vendor).unwrap();
        fs::write(vendor.join("ignored.rs"), "fn i() {}").unwrap();
        fs::write(vendor.join("important.rs"), "fn imp() {}").unwrap();

        let walker = walker_for(root, vec![]);
        let names = walker
            .collect_files(root, None, |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<HashSet<_>>();

        assert!(!names.contains("ignored.rs"));
        assert!(names.contains("important.rs"));
        parity_paths(root, &walker);
    }

    #[test]
    fn nested_cortexignore_in_subdirectory() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let pkg = root.join("pkg");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join(".cortexignore"), "*.tmp\n").unwrap();
        fs::write(pkg.join("code.rs"), "fn c() {}").unwrap();
        fs::write(pkg.join("note.tmp"), "x").unwrap();

        let walker = walker_for(root, vec![]);
        let names = walker
            .collect_files(root, None, |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<HashSet<_>>();

        assert!(names.contains("code.rs"));
        assert!(!names.contains("note.tmp"));
        assert!(walker.is_ignored(&pkg.join("note.tmp")));
        assert!(!walker.is_ignored(&pkg.join("code.rs")));
        parity_paths(root, &walker);
    }

    #[test]
    fn ignored_directory_pattern_blocks_nested_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".cortexignore"), "ignored/\n").unwrap();
        let ignored_dir = root.join("ignored");
        fs::create_dir_all(&ignored_dir).unwrap();
        fs::write(ignored_dir.join("skip.rs"), "fn skip() {}").unwrap();
        fs::write(root.join("keep.rs"), "fn keep() {}").unwrap();

        let walker = walker_for(root, vec![]);
        assert!(walker.is_ignored(&ignored_dir.join("skip.rs")));
        assert!(!walker.is_ignored(&root.join("keep.rs")));
        parity_paths(root, &walker);
    }

    #[test]
    fn global_cortexignore_applies_at_repo_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let global = tmp.path().join("global.cortexignore");
        fs::write(&global, "secret/\n").unwrap();
        let secret = root.join("secret");
        fs::create_dir_all(&secret).unwrap();
        fs::write(secret.join("key.rs"), "fn k() {}").unwrap();
        fs::write(root.join("open.rs"), "fn o() {}").unwrap();

        let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
            repo_root: root.to_path_buf(),
            scan_root: None,
            global_ignore_path: Some(global),
            respect_gitignore: false,
            respect_cortexignore: true,
            include_hidden: false,
            policy_excludes: vec![],
            count_ignored_skips: false,
        });
        let names = walker
            .collect_files(root, None, |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<HashSet<_>>();

        assert!(names.contains("open.rs"));
        assert!(!names.contains("key.rs"));
        parity_paths(root, &walker);
    }

    #[test]
    fn policy_exclude_overlays_ignore_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("allowed.rs"), "fn a() {}").unwrap();

        let walker = walker_for(root, vec!["allowed.rs".to_string()]);
        assert!(walker.is_ignored(&root.join("allowed.rs")));
    }

    #[test]
    fn explicit_include_skips_ignored_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".cortexignore"), "skip.rs\n").unwrap();
        fs::write(root.join("skip.rs"), "fn s() {}").unwrap();

        let walker = walker_for(root, vec![]);
        let files = walker
            .collect_files(root, Some(&[PathBuf::from("skip.rs")]), |_| true)
            .unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn scan_root_subdir_respects_parent_cortexignore() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join(".cortexignore"), "generated/\n").unwrap();
        let pkg = root.join("pkg");
        fs::create_dir_all(pkg.join("src")).unwrap();
        fs::create_dir_all(root.join("generated")).unwrap();
        fs::write(root.join("generated").join("auto.rs"), "fn a() {}").unwrap();
        fs::write(pkg.join("src").join("main.rs"), "fn m() {}").unwrap();

        let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
            repo_root: root.to_path_buf(),
            scan_root: Some(pkg.clone()),
            global_ignore_path: None,
            respect_gitignore: false,
            respect_cortexignore: true,
            include_hidden: false,
            policy_excludes: vec![],
            count_ignored_skips: false,
        });
        let names = walker
            .collect_files(&pkg, None, |_| true)
            .unwrap()
            .into_iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect::<HashSet<_>>();

        assert!(names.contains("main.rs"));
        assert!(!names.contains("auto.rs"));
    }

    #[test]
    fn ensure_cortexignore_template_writes_once() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        ensure_cortexignore_template(root, &["target/".to_string()]).unwrap();
        assert!(root.join(".cortexignore").exists());
        let before = fs::read_to_string(root.join(".cortexignore")).unwrap();
        ensure_cortexignore_template(root, &["node_modules/".to_string()]).unwrap();
        let after = fs::read_to_string(root.join(".cortexignore")).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn is_policy_excluded_directory_component() {
        assert!(is_policy_excluded(
            Path::new("/repo/target/debug/foo.rs"),
            &["target/**".to_string()]
        ));
        assert!(is_policy_excluded(
            Path::new("/repo/src/foo.pyc"),
            &["*.pyc".to_string()]
        ));
    }
}
