//! Git-aware indexing helpers (FalkorDB `repository_path` scoped to project folder).

use cortex_core::{CortexConfig, GitOperations};
use cortex_graph::{GraphClient, is_branch_index_current};
use cortex_indexer::{IndexChangePlan, Indexer};
use std::path::{Path, PathBuf};

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

pub fn resolve_git_context(path: &Path) -> Option<(PathBuf, String, String)> {
    let repo_root = find_git_repository_root(path)?;
    let git_ops = GitOperations::new(&repo_root);
    if !git_ops.is_git_repo() {
        return None;
    }
    let branch = git_ops.get_current_branch().ok()?;
    let commit = git_ops.get_current_commit().ok()?;
    Some((repo_root, branch, commit))
}

pub async fn index_with_git_context(
    config: &CortexConfig,
    path: &Path,
    force: bool,
    skip_if_current: bool,
    change_plan: Option<IndexChangePlan>,
    extra_include_files: &[PathBuf],
    extra_exclude_patterns: &[String],
    graph_repository_path: Option<&str>,
) -> anyhow::Result<(cortex_indexer::IndexReport, Option<PathBuf>)> {
    let client = GraphClient::connect(config).await?;
    let indexer = Indexer::from_cortex_config_with_scan_extras(
        client,
        config,
        extra_include_files,
        extra_exclude_patterns,
    )?;

    if let Some((repo_root, branch, commit)) = resolve_git_context(path) {
        let graph_root = cortex_core::graph_repository_path_for_index(path, graph_repository_path);
        let report = if let Some(change_plan) = change_plan {
            indexer
                .index_path_with_branch_change_plan(
                    path,
                    &branch,
                    &commit,
                    graph_root,
                    change_plan,
                    skip_if_current,
                )
                .await?
        } else {
            indexer
                .index_path_with_branch_context(
                    path,
                    &branch,
                    &commit,
                    graph_root,
                    force,
                    skip_if_current,
                )
                .await?
        };
        Ok((report, Some(repo_root)))
    } else {
        let report = indexer.index_path_with_options(path, force).await?;
        Ok((report, None))
    }
}

/// Graph `repository_path` key for daemon jobs and FalkorDB scoping.
pub fn graph_repository_scope(project_path: &Path) -> String {
    cortex_core::graph_repository_path_for_index(project_path, None)
}

/// True when the project registry already has an index for the current branch/commit.
pub fn is_project_branch_index_current(project_path: &Path) -> bool {
    let registry = cortex_watcher::ProjectRegistry::new();
    let Some(state) = registry.get_project(project_path) else {
        return false;
    };
    let Some(git) = state.git_info.as_ref() else {
        return false;
    };
    state
        .indexed_branches
        .get(&git.current_branch)
        .is_some_and(|info| info.commit_hash == git.current_commit && !info.is_stale)
}

/// Whether an index job should be enqueued (registry-only; tests).
#[cfg(test)]
pub fn should_enqueue_index_for_project(project_path: &Path, force: bool) -> bool {
    if force {
        return true;
    }
    !is_project_branch_index_current(project_path)
}

/// Graph-backed freshness when the project registry has no branch record yet.
pub async fn is_graph_branch_index_current(
    config: &CortexConfig,
    project_path: &Path,
) -> Option<bool> {
    let Some((_root, branch, commit)) = resolve_git_context(project_path) else {
        return None;
    };
    let scope = graph_repository_scope(project_path);
    let client = GraphClient::connect(config).await.ok()?;
    is_branch_index_current(&client, &scope, &branch, &commit)
        .await
        .ok()
}

/// Whether to run or queue indexing (registry first, then graph `BranchIndex`).
pub async fn should_enqueue_index_for_project_async(
    config: &CortexConfig,
    project_path: &Path,
    force: bool,
) -> bool {
    if force {
        return true;
    }
    if is_project_branch_index_current(project_path) {
        return false;
    }
    if is_graph_branch_index_current(config, project_path).await == Some(true) {
        return false;
    }
    true
}

pub fn record_project_branch_index(project_path: &Path, report: &cortex_indexer::IndexReport) {
    let (Some(branch), Some(commit_hash)) = (&report.branch, &report.commit_hash) else {
        return;
    };

    let registry = cortex_watcher::ProjectRegistry::new();
    if registry.get_project(project_path).is_none() {
        let _ = registry.add_project(project_path, None);
    }

    let duration_ms = (report.duration_secs * 1000.0).round() as u64;
    let _ = registry.record_branch_index(
        project_path,
        branch.clone(),
        commit_hash.clone(),
        report.indexed_files,
        report.symbol_count,
        duration_ms,
    );
    let _ = registry.refresh_git_info(project_path);
    let _ = registry.cleanup_old_branches(project_path);

    let paths = cortex_watcher::DaemonPaths::default_paths();
    let _ = cortex_watcher::record_branch_health_indexed(&paths, project_path, branch, commit_hash);
}

pub async fn auto_index_project_current_branch(
    config: &CortexConfig,
    project_path: &Path,
    force: bool,
) -> anyhow::Result<Option<serde_json::Value>> {
    if !force && !should_enqueue_index_for_project_async(config, project_path, false).await {
        let graph_scope = graph_repository_scope(project_path);
        return Ok(Some(serde_json::json!({
            "repository_path": graph_scope,
            "skipped": true,
            "reason": "branch_index_already_current",
        })));
    }

    let Some((repo_root, branch, commit_hash)) = resolve_git_context(project_path) else {
        return Ok(None);
    };

    let policy = project_config_for_path(project_path);
    let extra_excludes: Vec<String> = policy
        .as_ref()
        .map(|p| p.exclude_patterns.clone())
        .unwrap_or_default();
    let graph_scope = cortex_core::graph_repository_path_for_index(project_path, None);
    let (report, repo_root_for_record) = index_with_git_context(
        config,
        project_path,
        force,
        !force,
        None,
        &[],
        &extra_excludes,
        Some(graph_scope.as_str()),
    )
    .await?;
    if repo_root_for_record.is_some() {
        record_project_branch_index(project_path, &report);
    }

    let _ = repo_root;
    Ok(Some(serde_json::json!({
        "repository_path": graph_scope,
        "branch": branch,
        "commit": commit_hash,
        "report": report
    })))
}

pub fn project_config_for_path(path: &Path) -> Option<cortex_core::ProjectConfig> {
    let registry = cortex_watcher::ProjectRegistry::new();
    let repo_root = find_git_repository_root(path).unwrap_or_else(|| path.to_path_buf());
    registry.get_project(&repo_root).map(|p| p.config)
}

pub async fn auto_index_project_current_branch_best_effort(
    config: &CortexConfig,
    project_path: &Path,
    force: bool,
) -> (Option<serde_json::Value>, Option<String>) {
    match auto_index_project_current_branch(config, project_path, force).await {
        Ok(indexed) => (indexed, None),
        Err(err) => (None, Some(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_repository_scope_matches_core_helper() {
        let dir = std::env::temp_dir().join("cortex_cli_scope_test");
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(
            graph_repository_scope(&dir),
            cortex_core::graph_repository_path_for_index(&dir, None),
        );
    }

    #[test]
    fn should_enqueue_when_project_not_registered() {
        let dir = std::env::temp_dir().join("cortex_cli_enqueue_test");
        let _ = std::fs::create_dir_all(&dir);
        assert!(should_enqueue_index_for_project(&dir, false));
        assert!(should_enqueue_index_for_project(&dir, true));
    }
}
