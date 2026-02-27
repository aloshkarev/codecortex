//! Project Registry for Managing Multiple Watched Projects
//!
//! This module provides a thread-safe registry for managing multiple Git-aware
//! project watches with automatic branch detection and context management.

use chrono::Utc;
use cortex_core::{
    BranchIndexInfo, GitError, GitInfo, GitOperations, ProjectConfig, ProjectRef, ProjectState,
    ProjectStatus, ProjectSummary,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Error type for project registry operations
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Project already exists: {0}")]
    ProjectAlreadyExists(String),

    #[error("Git error: {0}")]
    GitError(#[from] GitError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("No current project set")]
    NoCurrentProject,

    #[error("Branch not indexed: {0}")]
    BranchNotIndexed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Global project registry
pub struct ProjectRegistry {
    /// All registered projects
    projects: DashMap<PathBuf, ProjectState>,

    /// Currently active project context
    current_project: RwLock<Option<ProjectRef>>,

    /// Path to the registry state file
    state_path: PathBuf,

    /// Default configuration for new projects (future use)
    #[allow(dead_code)]
    default_config: ProjectConfig,
}

impl ProjectRegistry {
    /// Create a new project registry
    pub fn new() -> Self {
        Self::with_state_path(Self::default_state_path())
    }

    /// Create a registry with a custom state file path
    pub fn with_state_path<P: AsRef<Path>>(path: P) -> Self {
        let registry = Self {
            projects: DashMap::new(),
            current_project: RwLock::new(None),
            state_path: path.as_ref().to_path_buf(),
            default_config: ProjectConfig::default(),
        };

        // Try to load existing state
        let _ = registry.load_state();

        registry
    }

    /// Get the default state file path
    pub fn default_state_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".cortex")
            .join("project_registry.json")
    }

    /// Add a project to the registry
    pub fn add_project<P: AsRef<Path>>(
        &self,
        path: P,
        config: Option<ProjectConfig>,
    ) -> Result<ProjectState, RegistryError> {
        let path = path.as_ref().canonicalize().map_err(|e| {
            RegistryError::InvalidPath(format!("{}: {}", path.as_ref().display(), e))
        })?;

        if self.projects.contains_key(&path) {
            return Err(RegistryError::ProjectAlreadyExists(
                path.display().to_string(),
            ));
        }

        let mut state = ProjectState::new(path.clone());
        if let Some(cfg) = config {
            state.config = cfg;
        }

        // Get Git info if available
        let git_ops = GitOperations::new(&path);
        if git_ops.is_git_repo() {
            state.git_info = Some(git_ops.get_git_info()?);
        }

        state.status = ProjectStatus::Inactive;

        self.projects.insert(path.clone(), state.clone());

        // Set as current if this is the first project
        if self.projects.len() == 1
            && let Some(pr) = state.to_project_ref()
        {
            *self.current_project.write() = Some(pr);
        }

        self.save_state()?;

        Ok(state)
    }

    /// Remove a project from the registry
    pub fn remove_project<P: AsRef<Path>>(&self, path: P) -> Result<(), RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        if self.projects.remove(&path).is_none() {
            return Err(RegistryError::ProjectNotFound(path.display().to_string()));
        }

        // Clear current project if it was this one
        {
            let current = self.current_project.read();
            if let Some(ref pr) = *current
                && pr.path == path
            {
                drop(current);
                *self.current_project.write() = None;
            }
        }

        // Set another project as current if available
        if self.current_project.read().is_none()
            && let Some(entry) = self.projects.iter().next()
            && let Some(pr) = entry.value().to_project_ref()
        {
            *self.current_project.write() = Some(pr);
        }

        self.save_state()?;

        Ok(())
    }

    /// Get a project by path
    pub fn get_project<P: AsRef<Path>>(&self, path: P) -> Option<ProjectState> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());
        self.projects.get(&path).map(|r| r.value().clone())
    }

    /// Update a project's state
    pub fn update_project<P: AsRef<Path>, F>(&self, path: P, f: F) -> Result<(), RegistryError>
    where
        F: FnOnce(&mut ProjectState),
    {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let mut state = self
            .projects
            .get(&path)
            .ok_or_else(|| RegistryError::ProjectNotFound(path.display().to_string()))?
            .clone();

        f(&mut state);

        self.projects.insert(path, state);
        self.save_state()?;

        Ok(())
    }

    /// List all projects
    pub fn list_projects(&self) -> Vec<ProjectSummary> {
        self.projects
            .iter()
            .map(|r| ProjectSummary::from(r.value()))
            .collect()
    }

    /// Get the current project context
    pub fn get_current_project(&self) -> Option<ProjectRef> {
        self.current_project.read().clone()
    }

    /// Set the current project
    pub fn set_current_project<P: AsRef<Path>>(
        &self,
        path: P,
        branch: Option<String>,
    ) -> Result<ProjectRef, RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let state = self
            .projects
            .get(&path)
            .ok_or_else(|| RegistryError::ProjectNotFound(path.display().to_string()))?;

        let branch = match branch {
            Some(b) => b,
            None => state
                .git_info
                .as_ref()
                .map(|g| g.current_branch.clone())
                .unwrap_or_else(|| "main".to_string()),
        };

        let commit = state.git_info.as_ref().map(|g| g.current_commit.clone());

        let pr = ProjectRef {
            path: path.clone(),
            branch,
            commit,
        };

        *self.current_project.write() = Some(pr.clone());

        Ok(pr)
    }

    /// Refresh Git info for a project
    pub fn refresh_git_info<P: AsRef<Path>>(&self, path: P) -> Result<GitInfo, RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let git_ops = GitOperations::new(&path);
        let git_info = git_ops.get_git_info()?;

        self.update_project(&path, |state| {
            state.git_info = Some(git_info.clone());
        })?;

        Ok(git_info)
    }

    /// Check if a project's branch has changed
    pub fn check_branch_change<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Option<(String, String)>, RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let state = self
            .projects
            .get(&path)
            .ok_or_else(|| RegistryError::ProjectNotFound(path.display().to_string()))?;

        if let Some(ref old_git) = state.git_info {
            let git_ops = GitOperations::new(&path);
            let new_branch = git_ops.get_current_branch()?;

            if new_branch != old_git.current_branch {
                return Ok(Some((old_git.current_branch.clone(), new_branch)));
            }
        }

        Ok(None)
    }

    /// Record a branch index
    pub fn record_branch_index<P: AsRef<Path>>(
        &self,
        path: P,
        branch: String,
        commit_hash: String,
        file_count: usize,
        symbol_count: usize,
        duration_ms: u64,
    ) -> Result<(), RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let index_info = BranchIndexInfo::new(branch.clone(), commit_hash)
            .with_counts(file_count, symbol_count)
            .with_duration(duration_ms);

        self.update_project(&path, |state| {
            state.indexed_branches.insert(branch, index_info);
            state.last_indexed_at = Some(Utc::now());
        })?;

        Ok(())
    }

    /// Clean up old branch indexes
    pub fn cleanup_old_branches<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<String>, RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        let removed = {
            let state = self
                .projects
                .get(&path)
                .ok_or_else(|| RegistryError::ProjectNotFound(path.display().to_string()))?;

            let max = state.config.max_branch_indexes;
            let pinned_set: std::collections::HashSet<&str> = state
                .config
                .pinned_branches
                .iter()
                .map(|s| s.as_str())
                .collect();

            // Sort by indexed_at (oldest first)
            let mut branches: Vec<_> = state.indexed_branches.iter().collect::<Vec<_>>();
            branches.sort_by_key(|(_, info)| info.indexed_at);

            let to_remove: Vec<String> = branches
                .iter()
                .filter(|(name, _)| !pinned_set.contains(name.as_str()))
                .rev()
                .skip(max)
                .map(|(name, _)| (*name).clone())
                .collect();

            to_remove
        };

        if !removed.is_empty() {
            self.update_project(&path, |state| {
                for branch in &removed {
                    state.indexed_branches.remove(branch);
                }
            })?;
        }

        Ok(removed)
    }

    /// Update project status
    pub fn set_status<P: AsRef<Path>>(
        &self,
        path: P,
        status: ProjectStatus,
    ) -> Result<(), RegistryError> {
        let path = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        self.update_project(&path, |state| {
            state.status = status;
        })
    }

    /// Save registry state to disk
    pub fn save_state(&self) -> Result<(), RegistryError> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let state = RegistryState {
            projects: self
                .projects
                .iter()
                .map(|r| (r.key().clone(), r.value().clone()))
                .collect(),
            current_project: self.current_project.read().clone(),
        };

        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(&self.state_path, json)?;

        Ok(())
    }

    /// Load registry state from disk
    pub fn load_state(&self) -> Result<(), RegistryError> {
        if !self.state_path.exists() {
            return Ok(());
        }

        let json = std::fs::read_to_string(&self.state_path)?;
        let state: RegistryState = serde_json::from_str(&json)?;

        for (path, project_state) in state.projects {
            self.projects.insert(path, project_state);
        }

        *self.current_project.write() = state.current_project;

        Ok(())
    }

    /// Get the number of registered projects
    pub fn len(&self) -> usize {
        self.projects.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable registry state
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryState {
    projects: HashMap<PathBuf, ProjectState>,
    current_project: Option<ProjectRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_add_remove_project() {
        let registry = ProjectRegistry::new();

        // Add a project - /tmp should exist on most systems
        let state = registry.add_project("/tmp", None);
        match state {
            Ok(_) => {
                // Successfully added
                assert!(!registry.is_empty());
            }
            Err(e) => {
                // Could be InvalidPath if /tmp doesn't exist or ProjectAlreadyExists
                let err_str = e.to_string();
                assert!(
                    err_str.contains("InvalidPath") || err_str.contains("already exists"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn project_ref_context_id() {
        let pr = ProjectRef::new(PathBuf::from("/project"), "main".into());
        assert_eq!(pr.context_id(), "/project@main");
    }
}
