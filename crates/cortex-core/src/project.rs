//! Project Management Types for Git-Aware Project Watching
//!
//! This module provides types for managing multiple projects with Git branch awareness.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Reference to a specific project and branch context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRef {
    /// Absolute path to project root
    pub path: PathBuf,
    /// Current branch name
    pub branch: String,
    /// Optional commit hash for detached HEAD
    pub commit: Option<String>,
}

impl ProjectRef {
    /// Create a new project reference
    pub fn new(path: PathBuf, branch: String) -> Self {
        Self {
            path,
            branch,
            commit: None,
        }
    }

    /// Create with commit hash
    pub fn with_commit(mut self, commit: String) -> Self {
        self.commit = Some(commit);
        self
    }

    /// Get a unique identifier for this project+branch combination
    pub fn context_id(&self) -> String {
        format!("{}@{}", self.path.display(), self.branch)
    }
}

/// State for a single watched project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Absolute path to project root
    pub path: PathBuf,
    /// Project name (derived from directory name)
    pub name: String,
    /// Git repository information
    pub git_info: Option<GitInfo>,
    /// Branches that have been indexed
    pub indexed_branches: HashMap<String, BranchIndexInfo>,
    /// Current watching status
    pub status: ProjectStatus,
    /// When last indexed
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// Configuration for this project
    pub config: ProjectConfig,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

impl ProjectState {
    /// Create a new project state for a path
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            path,
            name,
            git_info: None,
            indexed_branches: HashMap::new(),
            status: ProjectStatus::Inactive,
            last_indexed_at: None,
            config: ProjectConfig::default(),
            error_message: None,
        }
    }

    /// Create with custom config
    pub fn with_config(mut self, config: ProjectConfig) -> Self {
        self.config = config;
        self
    }

    /// Check if a specific branch is indexed
    pub fn is_branch_indexed(&self, branch: &str) -> bool {
        self.indexed_branches.contains_key(branch)
    }

    /// Get the current branch (if git info is available)
    pub fn current_branch(&self) -> Option<&str> {
        self.git_info.as_ref().map(|g| g.current_branch.as_str())
    }

    /// Check if the current branch index is stale
    pub fn is_current_index_stale(&self) -> bool {
        if let (Some(git_info), Some(branch_info)) = (
            &self.git_info,
            self.current_branch()
                .and_then(|b| self.indexed_branches.get(b)),
        ) {
            branch_info.commit_hash != git_info.current_commit
        } else {
            true
        }
    }

    /// Get a project reference for the current context
    pub fn to_project_ref(&self) -> Option<ProjectRef> {
        self.git_info.as_ref().map(|git| ProjectRef {
            path: self.path.clone(),
            branch: git.current_branch.clone(),
            commit: Some(git.current_commit.clone()),
        })
    }
}

/// Git-specific information for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    /// Current branch name
    pub current_branch: String,
    /// Current commit hash (full)
    pub current_commit: String,
    /// Short commit hash
    pub short_commit: String,
    /// All known branches
    pub branches: Vec<BranchInfo>,
    /// Remote URL (origin, if any)
    pub remote_url: Option<String>,
    /// Whether this is a valid git repository
    pub is_git_repo: bool,
    /// Number of uncommitted changes
    pub uncommitted_changes: usize,
}

impl GitInfo {
    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> bool {
        self.uncommitted_changes > 0
    }

    /// Get local branches only
    pub fn local_branches(&self) -> Vec<&BranchInfo> {
        self.branches.iter().filter(|b| !b.is_remote).collect()
    }

    /// Get remote branches only
    pub fn remote_branches(&self) -> Vec<&BranchInfo> {
        self.branches.iter().filter(|b| b.is_remote).collect()
    }

    /// Find a branch by name
    pub fn find_branch(&self, name: &str) -> Option<&BranchInfo> {
        self.branches.iter().find(|b| b.name == name)
    }
}

/// Information about a Git branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Branch name (e.g., "main", "feature/x", "origin/develop")
    pub name: String,
    /// Whether this is a remote branch
    pub is_remote: bool,
    /// Last commit hash on this branch
    pub last_commit: String,
    /// Last commit date
    pub last_commit_date: Option<DateTime<Utc>>,
    /// Commit message (first line)
    pub last_commit_message: Option<String>,
    /// Is this the current branch
    pub is_current: bool,
}

/// Index information for a specific branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchIndexInfo {
    /// Branch name
    pub branch: String,
    /// Commit hash when indexed
    pub commit_hash: String,
    /// When this branch was indexed
    pub indexed_at: DateTime<Utc>,
    /// Number of files indexed
    pub file_count: usize,
    /// Number of symbols (functions, classes, etc.) indexed
    pub symbol_count: usize,
    /// Whether this index may be stale
    pub is_stale: bool,
    /// Duration of indexing in milliseconds
    pub index_duration_ms: u64,
}

impl BranchIndexInfo {
    /// Create a new branch index info
    pub fn new(branch: String, commit_hash: String) -> Self {
        Self {
            branch,
            commit_hash,
            indexed_at: Utc::now(),
            file_count: 0,
            symbol_count: 0,
            is_stale: false,
            index_duration_ms: 0,
        }
    }

    /// Update counts
    pub fn with_counts(mut self, file_count: usize, symbol_count: usize) -> Self {
        self.file_count = file_count;
        self.symbol_count = symbol_count;
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.index_duration_ms = duration_ms;
        self
    }
}

/// Project watching status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    /// Not being watched
    Inactive,
    /// Actively watching for changes
    Watching,
    /// Currently indexing
    Indexing,
    /// Paused (manual pause)
    Paused,
    /// Error state
    Error,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inactive => write!(f, "inactive"),
            Self::Watching => write!(f, "watching"),
            Self::Indexing => write!(f, "indexing"),
            Self::Paused => write!(f, "paused"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Per-project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Whether to auto-track the current Git branch
    #[serde(default = "default_track_branch")]
    pub track_branch: bool,

    /// Branches to always keep indexed (even when not current)
    #[serde(default)]
    pub pinned_branches: Vec<String>,

    /// Maximum number of branch indexes to keep (oldest removed first)
    #[serde(default = "default_max_branch_indexes")]
    pub max_branch_indexes: usize,

    /// Glob patterns for files to ignore
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,

    /// Whether to automatically index when switching branches
    #[serde(default = "default_index_on_switch")]
    pub index_on_switch: bool,

    /// Debounce time for file changes (milliseconds)
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,

    /// Whether to include hidden files
    #[serde(default)]
    pub include_hidden: bool,

    /// File extensions to index (empty = all supported)
    #[serde(default)]
    pub extensions: Vec<String>,
}

fn default_track_branch() -> bool {
    true
}
fn default_max_branch_indexes() -> usize {
    5
}
fn default_ignore_patterns() -> Vec<String> {
    vec![
        "target/".to_string(),
        "node_modules/".to_string(),
        ".git/".to_string(),
        "dist/".to_string(),
        "build/".to_string(),
        "__pycache__/".to_string(),
        "*.pyc".to_string(),
        ".DS_Store".to_string(),
    ]
}
fn default_index_on_switch() -> bool {
    true
}
fn default_debounce_ms() -> u64 {
    2000
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            track_branch: default_track_branch(),
            pinned_branches: vec!["main".to_string(), "master".to_string()],
            max_branch_indexes: default_max_branch_indexes(),
            ignore_patterns: default_ignore_patterns(),
            index_on_switch: default_index_on_switch(),
            debounce_ms: default_debounce_ms(),
            include_hidden: false,
            extensions: vec![],
        }
    }
}

/// Summary of a project for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    /// Project path
    pub path: PathBuf,
    /// Project name
    pub name: String,
    /// Current branch (if git repo)
    pub current_branch: Option<String>,
    /// Current status
    pub status: ProjectStatus,
    /// Number of indexed branches
    pub indexed_branch_count: usize,
    /// Whether the current index is stale
    pub is_stale: bool,
    /// Last indexed time
    pub last_indexed_at: Option<DateTime<Utc>>,
}

impl From<&ProjectState> for ProjectSummary {
    fn from(state: &ProjectState) -> Self {
        Self {
            path: state.path.clone(),
            name: state.name.clone(),
            current_branch: state.current_branch().map(String::from),
            status: state.status,
            indexed_branch_count: state.indexed_branches.len(),
            is_stale: state.is_current_index_stale(),
            last_indexed_at: state.last_indexed_at,
        }
    }
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Project that was synced
    pub project_path: PathBuf,
    /// Branch that was synced
    pub branch: String,
    /// Commit before sync
    pub old_commit: Option<String>,
    /// Commit after sync
    pub new_commit: String,
    /// Files indexed
    pub files_indexed: usize,
    /// Symbols indexed
    pub symbols_indexed: usize,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether this was a full reindex
    pub was_full_reindex: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_ref_context_id() {
        let pr = ProjectRef::new(PathBuf::from("/home/user/project"), "main".to_string());
        assert_eq!(pr.context_id(), "/home/user/project@main");
    }

    #[test]
    fn project_ref_with_commit() {
        let pr = ProjectRef::new(PathBuf::from("/project"), "develop".to_string())
            .with_commit("abc123".to_string());
        assert_eq!(pr.commit, Some("abc123".to_string()));
    }

    #[test]
    fn project_state_is_branch_indexed() {
        let mut state = ProjectState::new(PathBuf::from("/project"));
        assert!(!state.is_branch_indexed("main"));

        state.indexed_branches.insert(
            "main".to_string(),
            BranchIndexInfo::new("main".into(), "abc".into()),
        );
        assert!(state.is_branch_indexed("main"));
    }

    #[test]
    fn project_config_defaults() {
        let config = ProjectConfig::default();
        assert!(config.track_branch);
        assert_eq!(config.max_branch_indexes, 5);
        assert!(config.index_on_switch);
        assert!(!config.include_hidden);
        assert_eq!(config.debounce_ms, 2000);
    }

    #[test]
    fn project_state_new() {
        let state = ProjectState::new(PathBuf::from("/home/user/myproject"));
        assert_eq!(state.name, "myproject");
        assert_eq!(state.status, ProjectStatus::Inactive);
        assert!(state.git_info.is_none());
        assert!(state.indexed_branches.is_empty());
    }

    #[test]
    fn project_state_current_branch() {
        let mut state = ProjectState::new(PathBuf::from("/project"));
        assert!(state.current_branch().is_none());

        state.git_info = Some(GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![],
            remote_url: None,
            is_git_repo: true,
            uncommitted_changes: 0,
        });
        assert_eq!(state.current_branch(), Some("main"));
    }

    #[test]
    fn project_state_is_current_index_stale() {
        let mut state = ProjectState::new(PathBuf::from("/project"));

        // No git info means stale
        assert!(state.is_current_index_stale());

        // With git info but no indexed branch means stale
        state.git_info = Some(GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![],
            remote_url: None,
            is_git_repo: true,
            uncommitted_changes: 0,
        });
        assert!(state.is_current_index_stale());

        // Indexed with matching commit means not stale
        state.indexed_branches.insert(
            "main".to_string(),
            BranchIndexInfo {
                branch: "main".to_string(),
                commit_hash: "abc".to_string(),
                indexed_at: Utc::now(),
                file_count: 10,
                symbol_count: 50,
                is_stale: false,
                index_duration_ms: 1000,
            },
        );
        assert!(!state.is_current_index_stale());

        // Indexed with different commit means stale
        state.git_info.as_mut().unwrap().current_commit = "def".to_string();
        assert!(state.is_current_index_stale());
    }

    #[test]
    fn branch_index_info_new() {
        let info = BranchIndexInfo::new("main".to_string(), "abc123".to_string());
        assert_eq!(info.branch, "main");
        assert_eq!(info.commit_hash, "abc123");
        assert_eq!(info.file_count, 0);
        assert_eq!(info.symbol_count, 0);
    }

    #[test]
    fn branch_index_info_with_counts() {
        let info = BranchIndexInfo::new("main".to_string(), "abc".to_string())
            .with_counts(10, 50);
        assert_eq!(info.file_count, 10);
        assert_eq!(info.symbol_count, 50);
    }

    #[test]
    fn branch_index_info_with_duration() {
        let info = BranchIndexInfo::new("main".to_string(), "abc".to_string())
            .with_duration(5000);
        assert_eq!(info.index_duration_ms, 5000);
    }

    #[test]
    fn git_info_has_uncommitted_changes() {
        let git_info = GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![],
            remote_url: None,
            is_git_repo: true,
            uncommitted_changes: 0,
        };
        assert!(!git_info.has_uncommitted_changes());

        let git_info = GitInfo {
            uncommitted_changes: 5,
            ..git_info
        };
        assert!(git_info.has_uncommitted_changes());
    }

    #[test]
    fn git_info_local_and_remote_branches() {
        let git_info = GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![
                BranchInfo {
                    name: "main".to_string(),
                    is_remote: false,
                    last_commit: "abc".to_string(),
                    last_commit_date: None,
                    last_commit_message: None,
                    is_current: true,
                },
                BranchInfo {
                    name: "develop".to_string(),
                    is_remote: false,
                    last_commit: "def".to_string(),
                    last_commit_date: None,
                    last_commit_message: None,
                    is_current: false,
                },
                BranchInfo {
                    name: "origin/main".to_string(),
                    is_remote: true,
                    last_commit: "abc".to_string(),
                    last_commit_date: None,
                    last_commit_message: None,
                    is_current: false,
                },
            ],
            remote_url: Some("git@github.com:user/repo.git".to_string()),
            is_git_repo: true,
            uncommitted_changes: 0,
        };

        let local: Vec<_> = git_info.local_branches();
        assert_eq!(local.len(), 2);

        let remote: Vec<_> = git_info.remote_branches();
        assert_eq!(remote.len(), 1);
    }

    #[test]
    fn git_info_find_branch() {
        let git_info = GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![
                BranchInfo {
                    name: "main".to_string(),
                    is_remote: false,
                    last_commit: "abc".to_string(),
                    last_commit_date: None,
                    last_commit_message: None,
                    is_current: true,
                },
            ],
            remote_url: None,
            is_git_repo: true,
            uncommitted_changes: 0,
        };

        assert!(git_info.find_branch("main").is_some());
        assert!(git_info.find_branch("nonexistent").is_none());
    }

    #[test]
    fn project_status_display() {
        assert_eq!(format!("{}", ProjectStatus::Inactive), "inactive");
        assert_eq!(format!("{}", ProjectStatus::Watching), "watching");
        assert_eq!(format!("{}", ProjectStatus::Indexing), "indexing");
        assert_eq!(format!("{}", ProjectStatus::Paused), "paused");
        assert_eq!(format!("{}", ProjectStatus::Error), "error");
    }

    #[test]
    fn project_summary_from_state() {
        let mut state = ProjectState::new(PathBuf::from("/project"));
        state.git_info = Some(GitInfo {
            current_branch: "main".to_string(),
            current_commit: "abc".to_string(),
            short_commit: "abc".to_string(),
            branches: vec![],
            remote_url: None,
            is_git_repo: true,
            uncommitted_changes: 0,
        });
        state.status = ProjectStatus::Watching;
        state.indexed_branches.insert(
            "main".to_string(),
            BranchIndexInfo::new("main".into(), "abc".into()),
        );

        let summary = ProjectSummary::from(&state);
        assert_eq!(summary.name, "project");
        assert_eq!(summary.current_branch, Some("main".to_string()));
        assert_eq!(summary.status, ProjectStatus::Watching);
        assert_eq!(summary.indexed_branch_count, 1);
    }

    #[test]
    fn project_state_serialization() {
        let state = ProjectState::new(PathBuf::from("/project"));
        let json = serde_json::to_string(&state).unwrap();
        let parsed: ProjectState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, state.name);
        assert_eq!(parsed.path, state.path);
    }

    #[test]
    fn project_config_serialization() {
        let config = ProjectConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.track_branch, config.track_branch);
        assert_eq!(parsed.max_branch_indexes, config.max_branch_indexes);
    }

    #[test]
    fn project_ref_serialization() {
        let pr = ProjectRef::new(PathBuf::from("/project"), "main".to_string())
            .with_commit("abc".to_string());
        let json = serde_json::to_string(&pr).unwrap();
        let parsed: ProjectRef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, pr.path);
        assert_eq!(parsed.branch, pr.branch);
        assert_eq!(pr.commit, Some("abc".to_string()));
    }

    #[test]
    fn sync_result_serialization() {
        let result = SyncResult {
            project_path: PathBuf::from("/project"),
            branch: "main".to_string(),
            old_commit: Some("abc".to_string()),
            new_commit: "def".to_string(),
            files_indexed: 10,
            symbols_indexed: 50,
            duration_ms: 1000,
            was_full_reindex: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SyncResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.branch, result.branch);
        assert_eq!(parsed.files_indexed, result.files_indexed);
    }
}
