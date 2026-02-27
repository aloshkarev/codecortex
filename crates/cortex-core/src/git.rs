//! Git Operations for Branch Detection and Repository Information
//!
//! This module provides utilities for extracting Git information without
//! depending on external Git bindings. Uses file I/O and command execution.
//!
//! ## Features
//!
//! - Repository information extraction
//! - Branch listing and comparison
//! - Commit history traversal
//! - Blame information extraction
//!
//! ## Example
//!
//! ```rust
//! use cortex_core::git::GitOperations;
//!
//! let git = GitOperations::new(".");
//! if git.is_git_repo() {
//!     let history = git.traverse_history(10).unwrap();
//!     for commit in history {
//!         println!("{}: {}", commit.short_hash, commit.message);
//!     }
//! }
//! ```

use crate::project::{BranchInfo, GitInfo};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Commit information for history traversal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// Full commit hash
    pub hash: String,
    /// Short commit hash (7 chars)
    pub short_hash: String,
    /// Author name
    pub author: String,
    /// Author email
    pub author_email: String,
    /// Commit timestamp
    pub date: DateTime<Utc>,
    /// Commit message (first line)
    pub message: String,
    /// Full commit message
    pub message_full: String,
    /// Parent commit hashes
    pub parents: Vec<String>,
}

/// Blame line information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLine {
    /// Line number in the file
    pub line_number: usize,
    /// Commit hash that last modified this line
    pub commit_hash: String,
    /// Short commit hash
    pub short_hash: String,
    /// Author of the change
    pub author: String,
    /// Date of the change
    pub date: DateTime<Utc>,
    /// The actual line content
    pub content: String,
}

/// Branch comparison result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDiff {
    /// Source branch name
    pub source_branch: String,
    /// Target branch name
    pub target_branch: String,
    /// Commits in source but not in target
    pub ahead_commits: Vec<CommitInfo>,
    /// Commits in target but not in source
    pub behind_commits: Vec<CommitInfo>,
    /// Files changed between branches
    pub changed_files: Vec<FileDiff>,
    /// Number of commits ahead
    pub ahead_count: usize,
    /// Number of commits behind
    pub behind_count: usize,
}

/// File difference between branches
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// File path
    pub path: String,
    /// Type of change
    pub change_type: FileChangeType,
    /// Additions count
    pub additions: usize,
    /// Deletions count
    pub deletions: usize,
}

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeType {
    /// File was added
    Added,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
}

/// Error type for Git operations
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Not a git repository: {0}")]
    NotAGitRepo(String),

    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Failed to read Git file: {0}")]
    FileReadError(String),

    #[error("Failed to parse Git output: {0}")]
    ParseError(String),

    #[error("Detached HEAD state")]
    DetachedHead,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Git operations helper
pub struct GitOperations {
    repo_path: std::path::PathBuf,
}

impl GitOperations {
    /// Create a new GitOperations instance for a repository path
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            repo_path: path.as_ref().to_path_buf(),
        }
    }

    /// Check if the path is a Git repository
    pub fn is_git_repo(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    /// Get the .git directory path
    pub fn git_dir(&self) -> std::path::PathBuf {
        self.repo_path.join(".git")
    }

    /// Get comprehensive Git information
    pub fn get_git_info(&self) -> Result<GitInfo, GitError> {
        if !self.is_git_repo() {
            return Ok(GitInfo {
                current_branch: String::new(),
                current_commit: String::new(),
                short_commit: String::new(),
                branches: vec![],
                remote_url: None,
                is_git_repo: false,
                uncommitted_changes: 0,
            });
        }

        let current_branch = self.get_current_branch()?;
        let current_commit = self.get_current_commit()?;
        let short_commit = self.get_short_commit()?;
        let branches = self.list_branches()?;
        let remote_url = self.get_remote_url()?;
        let uncommitted_changes = self.count_uncommitted_changes()?;

        Ok(GitInfo {
            current_branch,
            current_commit,
            short_commit,
            branches,
            remote_url,
            is_git_repo: true,
            uncommitted_changes,
        })
    }

    /// Get the current branch name
    pub fn get_current_branch(&self) -> Result<String, GitError> {
        // Try reading .git/HEAD first (faster, no process spawn)
        let head_path = self.git_dir().join("HEAD");

        if head_path.exists() {
            let content = std::fs::read_to_string(&head_path)
                .map_err(|e| GitError::FileReadError(e.to_string()))?;

            let content = content.trim();

            // Format: "ref: refs/heads/branch_name" or commit hash (detached)
            if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
                return Ok(branch.to_string());
            }

            // Detached HEAD state - return short commit
            if content.len() >= 7 {
                return Ok(format!("detached@{}", &content[..7]));
            }
        }

        // Fallback to git command
        self.run_git_command(&["branch", "--show-current"])?
            .first()
            .cloned()
            .ok_or(GitError::ParseError(
                "Could not determine current branch".into(),
            ))
    }

    /// Get the current commit hash (full)
    pub fn get_current_commit(&self) -> Result<String, GitError> {
        let branch = self.get_current_branch()?;

        // For detached HEAD, read from HEAD directly
        if branch.starts_with("detached@") {
            let head_path = self.git_dir().join("HEAD");
            if head_path.exists() {
                let content = std::fs::read_to_string(&head_path)
                    .map_err(|e| GitError::FileReadError(e.to_string()))?;
                return Ok(content.trim().to_string());
            }
        }

        // Read from refs/heads/{branch}
        let ref_path = self.git_dir().join("refs").join("heads").join(&branch);
        if ref_path.exists() {
            let content = std::fs::read_to_string(&ref_path)
                .map_err(|e| GitError::FileReadError(e.to_string()))?;
            return Ok(content.trim().to_string());
        }

        // Fallback to packed-refs
        let packed_refs = self.git_dir().join("packed-refs");
        if packed_refs.exists() {
            let content = std::fs::read_to_string(&packed_refs)
                .map_err(|e| GitError::FileReadError(e.to_string()))?;

            for line in content.lines() {
                if line.ends_with(&format!("refs/heads/{}", branch))
                    && let Some(hash) = line.split_whitespace().next()
                {
                    return Ok(hash.to_string());
                }
            }
        }

        // Last resort: git command
        self.run_git_command(&["rev-parse", "HEAD"])?
            .first()
            .cloned()
            .ok_or(GitError::ParseError("Could not get current commit".into()))
    }

    /// Get short commit hash (7 chars)
    pub fn get_short_commit(&self) -> Result<String, GitError> {
        let full = self.get_current_commit()?;
        if full.len() >= 7 {
            Ok(full[..7].to_string())
        } else {
            Ok(full)
        }
    }

    /// List all branches with their info
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>, GitError> {
        let mut branches = Vec::new();
        let current_branch = self.get_current_branch().unwrap_or_default();

        // List local branches from refs/heads
        let heads_dir = self.git_dir().join("refs").join("heads");
        if heads_dir.exists() {
            self.collect_branches_from_dir(&heads_dir, "", false, &current_branch, &mut branches)?;
        }

        // List remote branches from refs/remotes
        let remotes_dir = self.git_dir().join("refs").join("remotes");
        if remotes_dir.exists() {
            self.collect_branches_from_dir(&remotes_dir, "", true, &current_branch, &mut branches)?;
        }

        // Also try git command for more complete info
        if let Ok(output) = self.run_git_command(&[
            "for-each-ref",
            "--format=%(refname:short)|%(objectname)|%(committerdate:iso8601)|%(subject)",
            "refs/heads/",
            "refs/remotes/",
        ]) {
            for line in output {
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let is_remote = name.contains('/');
                    let last_commit = parts[1].to_string();
                    let last_commit_date = parts
                        .get(2)
                        .and_then(|d| DateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S %z").ok())
                        .map(|d| d.with_timezone(&Utc));
                    let last_commit_message = parts.get(3).map(|s| s.to_string());

                    // Update or add branch
                    if let Some(existing) = branches.iter_mut().find(|b| b.name == name) {
                        existing.last_commit_date = last_commit_date;
                        existing.last_commit_message = last_commit_message;
                    } else {
                        branches.push(BranchInfo {
                            name: name.clone(),
                            is_remote,
                            last_commit: last_commit.clone(),
                            last_commit_date,
                            last_commit_message,
                            is_current: name == current_branch,
                        });
                    }
                }
            }
        }

        // Ensure current branch is marked
        for branch in &mut branches {
            branch.is_current = branch.name == current_branch;
        }

        Ok(branches)
    }

    /// Recursively collect branches from a directory
    fn collect_branches_from_dir(
        &self,
        dir: &Path,
        prefix: &str,
        is_remote: bool,
        current_branch: &str,
        branches: &mut Vec<BranchInfo>,
    ) -> Result<(), GitError> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                let new_prefix = if prefix.is_empty() {
                    name
                } else {
                    format!("{}/{}", prefix, name)
                };
                self.collect_branches_from_dir(
                    &path,
                    &new_prefix,
                    is_remote,
                    current_branch,
                    branches,
                )?;
            } else {
                let branch_name = if prefix.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", prefix, name)
                };

                let commit = std::fs::read_to_string(&path)
                    .map(|c| c.trim().to_string())
                    .unwrap_or_default();

                branches.push(BranchInfo {
                    name: branch_name.clone(),
                    is_remote,
                    last_commit: commit,
                    last_commit_date: None,
                    last_commit_message: None,
                    is_current: branch_name == current_branch,
                });
            }
        }

        Ok(())
    }

    /// Get the remote URL (origin)
    pub fn get_remote_url(&self) -> Result<Option<String>, GitError> {
        let config_path = self.git_dir().join("config");
        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| GitError::FileReadError(e.to_string()))?;

        // Parse git config format for remote.origin.url
        let mut in_remote_origin = false;
        for line in content.lines() {
            let line = line.trim();

            if line == "[remote \"origin\"]" {
                in_remote_origin = true;
                continue;
            }

            if line.starts_with('[') {
                in_remote_origin = false;
                continue;
            }

            if in_remote_origin && line.starts_with("url = ") {
                return Ok(Some(line[6..].to_string()));
            }
        }

        // Fallback to git command
        Ok(self
            .run_git_command(&["remote", "get-url", "origin"])
            .ok()
            .and_then(|v| v.first().cloned()))
    }

    /// Count uncommitted changes (modified, staged, untracked)
    pub fn count_uncommitted_changes(&self) -> Result<usize, GitError> {
        let mut count = 0;

        // Count staged and unstaged changes
        if let Ok(output) = self.run_git_command(&["status", "--porcelain"]) {
            count += output.len();
        }

        Ok(count)
    }

    /// Check if the branch has changed since last check
    pub fn has_branch_changed(
        &self,
        last_branch: &str,
        last_commit: &str,
    ) -> Result<bool, GitError> {
        let current_branch = self.get_current_branch()?;
        let current_commit = self.get_current_commit()?;

        Ok(current_branch != last_branch || current_commit != last_commit)
    }

    /// Traverse commit history from HEAD
    ///
    /// Returns a vector of commits in reverse chronological order (newest first).
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of commits to return (0 = all commits)
    ///
    /// # Example
    ///
    /// ```rust
    /// use cortex_core::git::GitOperations;
    ///
    /// let git = GitOperations::new(".");
    /// let history = git.traverse_history(10).unwrap();
    /// for commit in history {
    ///     println!("{}: {}", commit.short_hash, commit.message);
    /// }
    /// ```
    pub fn traverse_history(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError> {
        if !self.is_git_repo() {
            return Ok(vec![]);
        }

        let limit_arg = if limit > 0 {
            format!("-{}", limit)
        } else {
            String::new()
        };

        // Format: hash|author|email|timestamp|subject|body|parents
        let format_str = "%H|%an|%ae|%at|%s|%b|%P";
        let format_arg = format!("--format={}", format_str);
        let args = vec!["log", &format_arg, &limit_arg];

        let output = self.run_git_command_with_empty(&args)?;
        let mut commits = Vec::new();

        for line in output {
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(7, '|').collect();
            if parts.len() < 6 {
                continue;
            }

            let hash = parts[0].to_string();
            let short_hash = if hash.len() >= 7 {
                hash[..7].to_string()
            } else {
                hash.clone()
            };

            let timestamp: i64 = parts[3].parse().unwrap_or(0);
            let date = DateTime::from_timestamp(timestamp, 0)
                .unwrap_or(DateTime::UNIX_EPOCH)
                .with_timezone(&Utc);

            let parents: Vec<String> = parts
                .get(6)
                .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
                .unwrap_or_default();

            commits.push(CommitInfo {
                hash,
                short_hash,
                author: parts[1].to_string(),
                author_email: parts[2].to_string(),
                date,
                message: parts[4].to_string(),
                message_full: parts[5].trim().to_string(),
                parents,
            });
        }

        Ok(commits)
    }

    /// Get commit history for a specific file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file (relative to repo root)
    /// * `limit` - Maximum number of commits to return
    pub fn file_history(
        &self,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, GitError> {
        if !self.is_git_repo() {
            return Ok(vec![]);
        }

        let limit_arg = if limit > 0 {
            format!("-{}", limit)
        } else {
            "-100".to_string()
        };

        let format_str = "%H|%an|%ae|%at|%s|%b|%P";
        let format_arg = format!("--format={}", format_str);
        let args = vec!["log", &format_arg, &limit_arg, "--", file_path];

        let output = self.run_git_command_with_empty(&args)?;
        let mut commits = Vec::new();

        for line in output {
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(7, '|').collect();
            if parts.len() < 6 {
                continue;
            }

            let hash = parts[0].to_string();
            let short_hash = if hash.len() >= 7 {
                hash[..7].to_string()
            } else {
                hash.clone()
            };

            let timestamp: i64 = parts[3].parse().unwrap_or(0);
            let date = DateTime::from_timestamp(timestamp, 0)
                .unwrap_or(DateTime::UNIX_EPOCH)
                .with_timezone(&Utc);

            let parents: Vec<String> = parts
                .get(6)
                .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
                .unwrap_or_default();

            commits.push(CommitInfo {
                hash,
                short_hash,
                author: parts[1].to_string(),
                author_email: parts[2].to_string(),
                date,
                message: parts[4].to_string(),
                message_full: parts[5].trim().to_string(),
                parents,
            });
        }

        Ok(commits)
    }

    /// Get blame information for a file
    ///
    /// Returns line-by-line blame information showing who last modified each line.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file (relative to repo root)
    ///
    /// # Example
    ///
    /// ```rust
    /// use cortex_core::git::GitOperations;
    ///
    /// let git = GitOperations::new(".");
    /// let blame = git.get_blame_info("src/main.rs").unwrap();
    /// for line in blame {
    ///     println!("{}: {} by {}", line.line_number, line.short_hash, line.author);
    /// }
    /// ```
    pub fn get_blame_info(&self, file_path: &str) -> Result<Vec<BlameLine>, GitError> {
        if !self.is_git_repo() {
            return Ok(vec![]);
        }

        // Format: line_number|hash|author|timestamp|content
        let args = vec![
            "blame",
            "--line-porcelain",
            "--",
            file_path,
        ];

        let output = self.run_git_raw_command(&args)?;

        let mut blame_lines = Vec::new();
        let mut current_hash = String::new();
        let mut current_author = String::new();
        let mut current_time: Option<DateTime<Utc>> = None;
        let mut line_number = 0usize;

        for line in output.lines() {
            if let Some(content) = line.strip_prefix('\t') {
                // This is the actual code line
                let content = content.to_string();
                line_number += 1;

                blame_lines.push(BlameLine {
                    line_number,
                    commit_hash: current_hash.clone(),
                    short_hash: if current_hash.len() >= 7 {
                        current_hash[..7].to_string()
                    } else {
                        current_hash.clone()
                    },
                    author: current_author.clone(),
                    date: current_time.unwrap_or(DateTime::UNIX_EPOCH),
                    content,
                });

                // Reset for next line
                current_hash.clear();
                current_author.clear();
                current_time = None;
            } else if let Some(hash) = line.split_whitespace().next()
                && hash.len() == 40
                && hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                current_hash = hash.to_string();
            }

            if let Some(rest) = line.strip_prefix("author ") {
                current_author = rest.to_string();
            }

            if let Some(rest) = line.strip_prefix("author-time ")
                && let Ok(timestamp) = rest.parse::<i64>()
            {
                current_time = DateTime::from_timestamp(timestamp, 0)
                    .map(|d| d.with_timezone(&Utc));
            }
        }

        Ok(blame_lines)
    }

    /// Compare two branches
    ///
    /// Returns information about commits and file differences between branches.
    ///
    /// # Arguments
    ///
    /// * `source_branch` - The source branch (e.g., "feature/xyz")
    /// * `target_branch` - The target branch (e.g., "main")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use cortex_core::git::GitOperations;
    ///
    /// let git = GitOperations::new(".");
    /// let diff = git.compare_branches("feature/xyz", "main").unwrap();
    /// println!("{} commits ahead, {} behind", diff.ahead_count, diff.behind_count);
    /// ```
    pub fn compare_branches(
        &self,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<BranchDiff, GitError> {
        if !self.is_git_repo() {
            return Err(GitError::NotAGitRepo(self.repo_path.display().to_string()));
        }

        // Get ahead/behind counts
        let ahead_behind_range = format!("{}...{}", source_branch, target_branch);
        let ahead_behind = self.run_git_command(&[
            "rev-list",
            "--left-right",
            "--count",
            &ahead_behind_range,
        ])?;

        let (ahead_count, behind_count) = if let Some(line) = ahead_behind.first() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let ahead = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let behind = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            (ahead, behind)
        } else {
            (0, 0)
        };

        // Get ahead commits
        let ahead_range = format!("{}..{}", target_branch, source_branch);
        let ahead_commits = self.get_commits_between(&ahead_range)?;

        // Get behind commits
        let behind_range = format!("{}..{}", source_branch, target_branch);
        let behind_commits = self.get_commits_between(&behind_range)?;

        // Get changed files
        let changed_files = self.get_changed_files_between(source_branch, target_branch)?;

        Ok(BranchDiff {
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            ahead_commits,
            behind_commits,
            changed_files,
            ahead_count,
            behind_count,
        })
    }

    /// Get commits between two references
    fn get_commits_between(&self, range: &str) -> Result<Vec<CommitInfo>, GitError> {
        let format_str = "%H|%an|%ae|%at|%s|%b|%P";
        let format_arg = format!("--format={}", format_str);
        let args = vec!["log", &format_arg, range];

        let output = self.run_git_command_with_empty(&args)?;
        let mut commits = Vec::new();

        for line in output {
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(7, '|').collect();
            if parts.len() < 6 {
                continue;
            }

            let hash = parts[0].to_string();
            let short_hash = if hash.len() >= 7 {
                hash[..7].to_string()
            } else {
                hash.clone()
            };

            let timestamp: i64 = parts[3].parse().unwrap_or(0);
            let date = DateTime::from_timestamp(timestamp, 0)
                .unwrap_or(DateTime::UNIX_EPOCH)
                .with_timezone(&Utc);

            let parents: Vec<String> = parts
                .get(6)
                .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
                .unwrap_or_default();

            commits.push(CommitInfo {
                hash,
                short_hash,
                author: parts[1].to_string(),
                author_email: parts[2].to_string(),
                date,
                message: parts[4].to_string(),
                message_full: parts[5].trim().to_string(),
                parents,
            });
        }

        Ok(commits)
    }

    /// Get changed files between two branches
    fn get_changed_files_between(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Vec<FileDiff>, GitError> {
        let diff_range = format!("{}...{}", target, source);
        let args = vec![
            "diff",
            "--numstat",
            "--name-status",
            &diff_range,
        ];

        let output = self.run_git_command(&args)?;
        let mut files = HashMap::new();

        for line in &output {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let change_type = match parts[0] {
                    "A" => FileChangeType::Added,
                    "D" => FileChangeType::Deleted,
                    "R" => FileChangeType::Renamed,
                    _ => FileChangeType::Modified,
                };

                let path = parts.last().unwrap_or(&"").to_string();
                files.insert(
                    path.clone(),
                    FileDiff {
                        path,
                        change_type,
                        additions: 0,
                        deletions: 0,
                    },
                );
            }
        }

        // Get numstat for additions/deletions
        let stat_range = format!("{}...{}", target, source);
        let args = vec!["diff", "--numstat", &stat_range];
        let stat_output = self.run_git_command(&args)?;

        for line in stat_output {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let path = parts[2].to_string();
                let additions = parts[0].parse().unwrap_or(0);
                let deletions = parts[1].parse().unwrap_or(0);

                if let Some(file_diff) = files.get_mut(&path) {
                    file_diff.additions = additions;
                    file_diff.deletions = deletions;
                }
            }
        }

        Ok(files.into_values().collect())
    }

    /// Run a git command and return raw output
    fn run_git_raw_command(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                return Err(GitError::CommandFailed(stderr.to_string()));
            }
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a git command that may return empty lines
    fn run_git_command_with_empty(&self, args: &[&str]) -> Result<Vec<String>, GitError> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail for empty repo or no upstream errors
            if !stderr.is_empty()
                && !stderr.contains("no upstream")
                && !stderr.contains("does not have any commits")
                && !stderr.contains("unknown revision")
                && !stderr.contains("ambiguous argument 'HEAD'")
            {
                return Err(GitError::CommandFailed(stderr.to_string()));
            }
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|l| l.to_string()).collect())
    }

    /// Run a git command and return output lines
    fn run_git_command(&self, args: &[&str]) -> Result<Vec<String>, GitError> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .map_err(|e| GitError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail for "no upstream" or "does not have any commits" type errors
            if !stderr.is_empty()
                && !stderr.contains("no upstream")
                && !stderr.contains("does not have any commits")
                && !stderr.contains("unknown revision")
                && !stderr.contains("ambiguous argument 'HEAD'")
            {
                return Err(GitError::CommandFailed(stderr.to_string()));
            }
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a temporary git repository for testing
    fn create_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["init"])
            .output();

        // Configure git user
        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.email", "test@example.com"])
            .output();

        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.name", "Test User"])
            .output();

        (temp_dir, repo_path)
    }

    /// Create a commit in the test repo
    fn create_commit(repo_path: &Path, filename: &str, content: &str, message: &str) {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, content).expect("Failed to write file");

        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["add", filename])
            .output();

        let _ = Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", message])
            .output();
    }

    #[test]
    fn git_operations_for_non_git_dir() {
        let git = GitOperations::new("/tmp");
        assert!(!git.is_git_repo());
    }

    #[test]
    fn git_info_for_non_git_repo() {
        let git = GitOperations::new("/tmp");
        let info = git.get_git_info().unwrap();
        assert!(!info.is_git_repo);
    }

    #[test]
    fn test_is_git_repo() {
        let (_temp_dir, repo_path) = create_test_repo();
        let git = GitOperations::new(&repo_path);
        assert!(git.is_git_repo());
    }

    #[test]
    fn test_get_current_branch_initial() {
        let (_temp_dir, repo_path) = create_test_repo();
        let git = GitOperations::new(&repo_path);

        // Initial branch should be main or master
        let branch = git.get_current_branch().unwrap();
        assert!(branch == "main" || branch == "master" || branch.starts_with("detached"));
    }

    #[test]
    fn test_get_git_info() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial commit");

        let git = GitOperations::new(&repo_path);

        let info = git.get_git_info().unwrap();
        assert!(info.is_git_repo);
        assert!(!info.current_branch.is_empty() || info.current_branch.starts_with("detached"));
    }

    #[test]
    fn test_list_branches() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial commit");

        let git = GitOperations::new(&repo_path);

        let branches = git.list_branches().unwrap();
        // Should have at least one branch (main or master)
        assert!(!branches.is_empty());
    }

    #[test]
    fn test_traverse_history_empty() {
        let (_temp_dir, repo_path) = create_test_repo();
        let git = GitOperations::new(&repo_path);

        // Empty repo has no commits yet - should return empty without error
        let history = git.traverse_history(10).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_traverse_history_with_commits() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial commit");

        let git = GitOperations::new(&repo_path);
        let history = git.traverse_history(10).unwrap();

        assert!(!history.is_empty());
        assert_eq!(history[0].message, "Initial commit");
        assert_eq!(history[0].author, "Test User");
        assert_eq!(history[0].author_email, "test@example.com");
    }

    #[test]
    fn test_traverse_history_limit() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test1.txt", "content1", "Commit 1");
        create_commit(&repo_path, "test2.txt", "content2", "Commit 2");
        create_commit(&repo_path, "test3.txt", "content3", "Commit 3");

        let git = GitOperations::new(&repo_path);
        let history = git.traverse_history(2).unwrap();

        assert_eq!(history.len(), 2);
        // Most recent commit first
        assert_eq!(history[0].message, "Commit 3");
        assert_eq!(history[1].message, "Commit 2");
    }

    #[test]
    fn test_commit_info_structure() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Test commit message");

        let git = GitOperations::new(&repo_path);
        let history = git.traverse_history(1).unwrap();

        let commit = &history[0];
        assert!(!commit.hash.is_empty());
        assert_eq!(commit.hash.len(), 40); // Full SHA-1 hash
        assert_eq!(commit.short_hash.len(), 7);
        assert!(!commit.author.is_empty());
        assert!(!commit.author_email.is_empty());
        assert!(!commit.message.is_empty());
    }

    #[test]
    fn test_file_history() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "tracked.txt", "v1", "First version");
        create_commit(&repo_path, "tracked.txt", "v2", "Second version");
        create_commit(&repo_path, "other.txt", "other", "Other file");

        let git = GitOperations::new(&repo_path);
        let history = git.file_history("tracked.txt", 10).unwrap();

        // Should have 2 commits for this file
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_get_blame_info() {
        let (_temp_dir, repo_path) = create_test_repo();

        // Create a file with multiple lines
        let content = "line 1\nline 2\nline 3\n";
        create_commit(&repo_path, "blame_test.txt", content, "Add blame test file");

        let git = GitOperations::new(&repo_path);
        let blame = git.get_blame_info("blame_test.txt").unwrap();

        assert_eq!(blame.len(), 3); // 3 lines
        assert_eq!(blame[0].line_number, 1);
        assert_eq!(blame[0].content, "line 1");
        assert_eq!(blame[0].author, "Test User");
        assert!(!blame[0].short_hash.is_empty());
    }

    #[test]
    fn test_blame_info_structure() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "single.txt", "single line", "Single commit");

        let git = GitOperations::new(&repo_path);
        let blame = git.get_blame_info("single.txt").unwrap();

        assert_eq!(blame.len(), 1);
        let line = &blame[0];
        assert_eq!(line.line_number, 1);
        assert_eq!(line.content, "single line");
        assert_eq!(line.commit_hash.len(), 40);
        assert_eq!(line.short_hash.len(), 7);
    }

    #[test]
    fn test_compare_branches_same() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial commit");

        let git = GitOperations::new(&repo_path);
        let current_branch = git.get_current_branch().unwrap();

        // Comparing branch with itself
        let diff = git.compare_branches(&current_branch, &current_branch).unwrap();
        assert_eq!(diff.ahead_count, 0);
        assert_eq!(diff.behind_count, 0);
        assert!(diff.changed_files.is_empty());
    }

    #[test]
    fn test_branch_diff_structure() {
        let (_temp_dir, repo_path) = create_test_repo();

        // Create main branch commit
        create_commit(&repo_path, "main.txt", "main content", "Main commit");

        let git = GitOperations::new(&repo_path);
        let main_branch = git.get_current_branch().unwrap();

        // Create feature branch
        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["checkout", "-b", "feature"])
            .output();

        create_commit(&repo_path, "feature.txt", "feature content", "Feature commit");

        let diff = git.compare_branches("feature", &main_branch).unwrap();

        assert_eq!(diff.source_branch, "feature");
        assert_eq!(diff.target_branch, main_branch);
        assert_eq!(diff.ahead_count, 1); // feature has 1 more commit
        assert_eq!(diff.behind_count, 0);
    }

    #[test]
    fn test_file_diff_structure() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "initial", "Initial");

        let git = GitOperations::new(&repo_path);
        let branch = git.get_current_branch().unwrap();

        // Modify file
        fs::write(repo_path.join("test.txt"), "modified").unwrap();
        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-am", "Modified"])
            .output();

        // Create new branch
        let _ = Command::new("git")
            .current_dir(&repo_path)
            .args(["checkout", "-b", "feature2"])
            .output();

        // Add new file
        create_commit(&repo_path, "new.txt", "new file", "Added new file");

        let diff = git.compare_branches("feature2", &branch).unwrap();

        assert!(!diff.changed_files.is_empty());
    }

    #[test]
    fn test_has_branch_changed_false() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial");

        let git = GitOperations::new(&repo_path);
        let branch = git.get_current_branch().unwrap();
        let commit = git.get_current_commit().unwrap();

        assert!(!git.has_branch_changed(&branch, &commit).unwrap());
    }

    #[test]
    fn test_has_branch_changed_true_after_commit() {
        let (_temp_dir, repo_path) = create_test_repo();
        create_commit(&repo_path, "test.txt", "content", "Initial");

        let git = GitOperations::new(&repo_path);
        let old_commit = git.get_current_commit().unwrap();
        let branch = git.get_current_branch().unwrap();

        // Create new commit
        create_commit(&repo_path, "test2.txt", "content2", "Second");

        assert!(git.has_branch_changed(&branch, &old_commit).unwrap());
    }

    #[test]
    fn test_count_uncommitted_changes() {
        let (_temp_dir, repo_path) = create_test_repo();

        let git = GitOperations::new(&repo_path);

        // No changes initially
        assert_eq!(git.count_uncommitted_changes().unwrap(), 0);

        // Add untracked file
        fs::write(repo_path.join("untracked.txt"), "untracked").unwrap();
        assert!(git.count_uncommitted_changes().unwrap() >= 1);
    }

    #[test]
    fn test_non_git_repo_returns_empty_history() {
        let git = GitOperations::new("/tmp/nonexistent");
        let history = git.traverse_history(10).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_non_git_repo_returns_empty_blame() {
        let git = GitOperations::new("/tmp/nonexistent");
        let blame = git.get_blame_info("any.txt").unwrap();
        assert!(blame.is_empty());
    }
}
