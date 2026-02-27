//! Event Filtering for File Watching
//!
//! Provides configurable event filtering to reduce noise:
//! - Pattern-based file filtering (glob patterns)
//! - Event type filtering (create, modify, delete)
//! - Custom filter rules
//! - Path-based inclusion/exclusion

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashSet;

/// Types of file system events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchEventKind {
    /// File or directory created
    Created,
    /// File or directory modified
    Modified,
    /// File or directory deleted
    Removed,
    /// Any event type
    Any,
}

/// A filter rule for file events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Unique name for this rule
    pub name: String,
    /// Whether this is an inclusion (true) or exclusion (false) rule
    pub include: bool,
    /// Glob pattern for matching paths
    pub pattern: String,
    /// Event types this rule applies to (empty = all)
    pub event_kinds: Vec<WatchEventKind>,
    /// Whether the rule is enabled
    pub enabled: bool,
}

impl FilterRule {
    /// Create a new inclusion rule
    pub fn include(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            include: true,
            pattern: pattern.to_string(),
            event_kinds: vec![],
            enabled: true,
        }
    }

    /// Create a new exclusion rule
    pub fn exclude(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            include: false,
            pattern: pattern.to_string(),
            event_kinds: vec![],
            enabled: true,
        }
    }

    /// Set event types this rule applies to
    pub fn with_event_kinds(mut self, kinds: Vec<WatchEventKind>) -> Self {
        self.event_kinds = kinds;
        self
    }

    /// Check if this rule matches a path and event type
    pub fn matches(&self, path: &Path, event_kind: WatchEventKind) -> bool {
        if !self.enabled {
            return false;
        }

        // Check event kind
        if !self.event_kinds.is_empty() && !self.event_kinds.contains(&event_kind) {
            return false;
        }

        // Check pattern
        self.matches_pattern(path)
    }

    /// Check if the path matches the pattern
    fn matches_pattern(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Simple glob matching
        if self.pattern == "*" {
            return true;
        }

        // Handle ** for recursive matching
        if self.pattern.contains("**") {
            let parts: Vec<&str> = self.pattern.split("**").collect();
            if parts.len() == 2 {
                let prefix = parts[0].trim_end_matches('/');
                let suffix = parts[1].trim_start_matches('/');

                let path_str = path_str.to_string();

                // Check prefix
                if !prefix.is_empty() && !path_str.starts_with(prefix) {
                    // Check if any path component matches
                    let matches_prefix = path.components().any(|c| {
                        c.as_os_str().to_string_lossy().starts_with(prefix.trim_start_matches('/'))
                    });
                    if !matches_prefix && !path_str.contains(prefix) {
                        return false;
                    }
                }

                // Check suffix
                if !suffix.is_empty() {
                    return path_str.ends_with(suffix) ||
                           path.extension()
                               .map(|e| suffix.contains(&e.to_string_lossy().to_string()))
                               .unwrap_or(false);
                }

                return true;
            }
        }

        // Handle extension patterns like *.rs
        if self.pattern.starts_with("*.") && !self.pattern.contains('*') {
            let ext = &self.pattern[2..];
            return path.extension()
                .map(|e| e.to_string_lossy() == ext)
                .unwrap_or(false);
        }

        // Handle patterns with wildcards like *test*
        if self.pattern.starts_with('*') && self.pattern.ends_with('*') && self.pattern.len() > 2 {
            let middle = &self.pattern[1..self.pattern.len()-1];
            return path_str.contains(middle);
        }

        // Handle prefix patterns like test*
        if self.pattern.ends_with('*') && self.pattern.len() > 1 {
            let prefix = &self.pattern[..self.pattern.len()-1];
            return path_str.starts_with(prefix) ||
                   path.file_name()
                       .map(|n| n.to_string_lossy().starts_with(prefix))
                       .unwrap_or(false);
        }

        // Handle suffix patterns like *test
        if self.pattern.starts_with('*') && self.pattern.len() > 1 {
            let suffix = &self.pattern[1..];
            return path_str.ends_with(suffix);
        }

        // Handle directory patterns like target/
        if self.pattern.ends_with('/') {
            let dir = &self.pattern[..self.pattern.len() - 1];
            return path.components().any(|c| {
                c.as_os_str().to_string_lossy() == dir
            });
        }

        // Exact match or contains
        path_str.contains(&self.pattern) || path_str == self.pattern
    }
}

/// Configuration for event filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Default action when no rules match (true = pass, false = block)
    pub default_pass: bool,
    /// List of filter rules (processed in order)
    pub rules: Vec<FilterRule>,
    /// File extensions to always include
    pub include_extensions: HashSet<String>,
    /// File extensions to always exclude
    pub exclude_extensions: HashSet<String>,
    /// Directories to always exclude
    pub exclude_dirs: HashSet<String>,
    /// Minimum file size to process (0 = no minimum)
    pub min_file_size: u64,
    /// Maximum file size to process (0 = no maximum)
    pub max_file_size: u64,
}

impl Default for FilterConfig {
    fn default() -> Self {
        let mut exclude_dirs = HashSet::new();
        exclude_dirs.insert("node_modules".to_string());
        exclude_dirs.insert("target".to_string());
        exclude_dirs.insert(".git".to_string());
        exclude_dirs.insert("__pycache__".to_string());
        exclude_dirs.insert("build".to_string());
        exclude_dirs.insert("dist".to_string());
        exclude_dirs.insert(".cache".to_string());

        let mut exclude_extensions = HashSet::new();
        exclude_extensions.insert("pyc".to_string());
        exclude_extensions.insert("pyo".to_string());
        exclude_extensions.insert("o".to_string());
        exclude_extensions.insert("a".to_string());
        exclude_extensions.insert("so".to_string());
        exclude_extensions.insert("dylib".to_string());
        exclude_extensions.insert("dll".to_string());
        exclude_extensions.insert("exe".to_string());
        exclude_extensions.insert("log".to_string());
        exclude_extensions.insert("lock".to_string());

        let mut include_extensions = HashSet::new();
        include_extensions.insert("rs".to_string());
        include_extensions.insert("py".to_string());
        include_extensions.insert("js".to_string());
        include_extensions.insert("ts".to_string());
        include_extensions.insert("go".to_string());
        include_extensions.insert("c".to_string());
        include_extensions.insert("cpp".to_string());
        include_extensions.insert("h".to_string());
        include_extensions.insert("java".to_string());
        include_extensions.insert("php".to_string());
        include_extensions.insert("rb".to_string());

        Self {
            default_pass: false,
            rules: Vec::new(),
            include_extensions,
            exclude_extensions,
            exclude_dirs,
            min_file_size: 0,
            max_file_size: 10 * 1024 * 1024, // 10 MB max
        }
    }
}

/// Event filter for file watching
#[derive(Debug, Clone)]
pub struct EventFilter {
    config: FilterConfig,
}

impl EventFilter {
    /// Create a new event filter with default configuration
    pub fn new() -> Self {
        Self::with_config(FilterConfig::default())
    }

    /// Create an event filter with custom configuration
    pub fn with_config(config: FilterConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &FilterConfig {
        &self.config
    }

    /// Check if an event should be processed
    pub fn should_process(&self, path: &Path, event_kind: WatchEventKind) -> bool {
        // Check directory exclusions
        if self.is_excluded_dir(path) {
            return false;
        }

        // Check extension exclusions/inclusions
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_string();

            // Explicitly excluded extension
            if self.config.exclude_extensions.contains(&ext_str) {
                return false;
            }

            // If we have include_extensions and this IS in it, include immediately
            if self.config.include_extensions.contains(&ext_str) {
                return true;
            }

            // If we have include_extensions and this isn't in it, exclude
            if !self.config.include_extensions.is_empty() {
                return false;
            }
        }

        // Check file size if path exists
        if (self.config.min_file_size > 0 || self.config.max_file_size > 0)
            && let Ok(metadata) = std::fs::metadata(path)
        {
            let size = metadata.len();
            if size < self.config.min_file_size {
                return false;
            }
            if self.config.max_file_size > 0 && size > self.config.max_file_size {
                return false;
            }
        }

        // Process custom rules
        for rule in &self.config.rules {
            if !rule.enabled {
                continue;
            }

            if rule.matches(path, event_kind) {
                return rule.include;
            }
        }

        // No rule matched, use default
        self.config.default_pass
    }

    /// Check if path is in an excluded directory
    fn is_excluded_dir(&self, path: &Path) -> bool {
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str()
                && self.config.exclude_dirs.contains(name)
            {
                return true;
            }
        }
        false
    }

    /// Add an inclusion rule
    pub fn add_include(&mut self, name: &str, pattern: &str) {
        self.config.rules.push(FilterRule::include(name, pattern));
    }

    /// Add an exclusion rule
    pub fn add_exclude(&mut self, name: &str, pattern: &str) {
        self.config.rules.push(FilterRule::exclude(name, pattern));
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.config.rules.clear();
    }

    /// Add an extension to include
    pub fn include_extension(&mut self, ext: &str) {
        self.config.include_extensions.insert(ext.to_string());
    }

    /// Add an extension to exclude
    pub fn exclude_extension(&mut self, ext: &str) {
        self.config.exclude_extensions.insert(ext.to_string());
    }

    /// Add a directory to exclude
    pub fn exclude_directory(&mut self, dir: &str) {
        self.config.exclude_dirs.insert(dir.to_string());
    }

    /// Get filter statistics
    pub fn stats(&self) -> FilterStats {
        FilterStats {
            rules_count: self.config.rules.len(),
            include_extensions: self.config.include_extensions.len(),
            exclude_extensions: self.config.exclude_extensions.len(),
            exclude_dirs: self.config.exclude_dirs.len(),
        }
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStats {
    pub rules_count: usize,
    pub include_extensions: usize,
    pub exclude_extensions: usize,
    pub exclude_dirs: usize,
}

/// Builder for creating event filters
pub struct EventFilterBuilder {
    config: FilterConfig,
}

impl EventFilterBuilder {
    /// Create a new filter builder
    pub fn new() -> Self {
        Self {
            config: FilterConfig::default(),
        }
    }

    /// Set default pass behavior
    pub fn default_pass(mut self, pass: bool) -> Self {
        self.config.default_pass = pass;
        self
    }

    /// Add an inclusion rule
    pub fn include(mut self, name: &str, pattern: &str) -> Self {
        self.config.rules.push(FilterRule::include(name, pattern));
        self
    }

    /// Add an exclusion rule
    pub fn exclude(mut self, name: &str, pattern: &str) -> Self {
        self.config.rules.push(FilterRule::exclude(name, pattern));
        self
    }

    /// Add an extension to include
    pub fn include_ext(mut self, ext: &str) -> Self {
        self.config.include_extensions.insert(ext.to_string());
        self
    }

    /// Add an extension to exclude
    pub fn exclude_ext(mut self, ext: &str) -> Self {
        self.config.exclude_extensions.insert(ext.to_string());
        self
    }

    /// Add a directory to exclude
    pub fn exclude_dir(mut self, dir: &str) -> Self {
        self.config.exclude_dirs.insert(dir.to_string());
        self
    }

    /// Set minimum file size
    pub fn min_size(mut self, bytes: u64) -> Self {
        self.config.min_file_size = bytes;
        self
    }

    /// Set maximum file size
    pub fn max_size(mut self, bytes: u64) -> Self {
        self.config.max_file_size = bytes;
        self
    }

    /// Build the event filter
    pub fn build(self) -> EventFilter {
        EventFilter::with_config(self.config)
    }
}

impl Default for EventFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_rule_include() {
        let rule = FilterRule::include("rust_files", "*.rs");
        assert!(rule.matches(Path::new("src/main.rs"), WatchEventKind::Modified));
        assert!(!rule.matches(Path::new("src/main.py"), WatchEventKind::Modified));
    }

    #[test]
    fn filter_rule_exclude() {
        let rule = FilterRule::exclude("test_files", "*test*");
        assert!(!rule.include);
        assert!(rule.matches(Path::new("src/test_main.rs"), WatchEventKind::Modified));
    }

    #[test]
    fn filter_rule_event_kinds() {
        let rule = FilterRule::include("create_only", "*.rs")
            .with_event_kinds(vec![WatchEventKind::Created]);

        assert!(rule.matches(Path::new("main.rs"), WatchEventKind::Created));
        assert!(!rule.matches(Path::new("main.rs"), WatchEventKind::Modified));
    }

    #[test]
    fn filter_rule_disabled() {
        let mut rule = FilterRule::include("disabled", "*.rs");
        rule.enabled = false;

        assert!(!rule.matches(Path::new("main.rs"), WatchEventKind::Modified));
    }

    #[test]
    fn event_filter_default() {
        let filter = EventFilter::new();

        // Should include Rust files
        assert!(filter.should_process(Path::new("src/main.rs"), WatchEventKind::Modified));

        // Should exclude node_modules
        assert!(!filter.should_process(Path::new("node_modules/package/index.js"), WatchEventKind::Modified));

        // Should exclude .pyc files
        assert!(!filter.should_process(Path::new("__pycache__/module.pyc"), WatchEventKind::Modified));
    }

    #[test]
    fn event_filter_custom_rules() {
        let mut filter = EventFilter::new();
        filter.add_exclude("docs", "*.md");

        assert!(!filter.should_process(Path::new("README.md"), WatchEventKind::Modified));
        assert!(filter.should_process(Path::new("src/main.rs"), WatchEventKind::Modified));
    }

    #[test]
    fn event_filter_builder() {
        let filter = EventFilterBuilder::new()
            .include_ext("rs")
            .exclude_dir("vendor")
            .max_size(1024 * 1024)
            .build();

        let stats = filter.stats();
        assert!(stats.include_extensions > 0);
        assert!(stats.exclude_dirs > 0);
    }

    #[test]
    fn filter_config_default() {
        let config = FilterConfig::default();

        assert!(!config.default_pass);
        assert!(config.exclude_dirs.contains("node_modules"));
        assert!(config.exclude_dirs.contains("target"));
        assert!(config.exclude_extensions.contains("pyc"));
        assert!(config.include_extensions.contains("rs"));
    }

    #[test]
    fn filter_stats() {
        let filter = EventFilter::new();
        let stats = filter.stats();

        assert_eq!(stats.rules_count, 0);
        assert!(stats.include_extensions > 0);
        assert!(stats.exclude_extensions > 0);
        assert!(stats.exclude_dirs > 0);
    }

    #[test]
    fn watch_event_kind_variants() {
        assert_eq!(WatchEventKind::Created, WatchEventKind::Created);
        assert_ne!(WatchEventKind::Created, WatchEventKind::Modified);
        assert_ne!(WatchEventKind::Modified, WatchEventKind::Removed);
    }

    #[test]
    fn filter_rule_wildcard() {
        let rule = FilterRule::include("all", "*");
        assert!(rule.matches(Path::new("any/path.txt"), WatchEventKind::Modified));
    }

    #[test]
    fn filter_rule_directory() {
        let rule = FilterRule::exclude("target_dir", "target/");
        assert!(rule.matches(Path::new("target/debug/main"), WatchEventKind::Modified));
        assert!(!rule.matches(Path::new("src/main.rs"), WatchEventKind::Modified));
    }
}
