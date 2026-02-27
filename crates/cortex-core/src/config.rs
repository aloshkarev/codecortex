use crate::{CortexError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CortexConfig {
    pub memgraph_uri: String,
    pub memgraph_user: String,
    pub memgraph_password: String,
    pub max_batch_size: usize,
    pub watched_paths: Vec<PathBuf>,
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            memgraph_uri: "bolt://127.0.0.1:7687".to_string(),
            memgraph_user: "memgraph".to_string(),
            memgraph_password: "memgraph".to_string(),
            max_batch_size: 500,
            watched_paths: Vec::new(),
        }
    }
}

impl CortexConfig {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/config.toml")
    }

    pub fn ensure_parent_dir() -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)?;
        toml::from_str(&raw).map_err(|e| CortexError::Config(e.to_string()))
    }

    pub fn save(&self) -> Result<()> {
        Self::ensure_parent_dir()?;
        let path = Self::config_path();
        let data = toml::to_string_pretty(self).map_err(|e| CortexError::Config(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_values() {
        let config = CortexConfig::default();
        assert_eq!(config.memgraph_uri, "bolt://127.0.0.1:7687");
        assert_eq!(config.memgraph_user, "memgraph");
        assert_eq!(config.memgraph_password, "memgraph");
        assert_eq!(config.max_batch_size, 500);
        assert!(config.watched_paths.is_empty());
    }

    #[test]
    fn config_custom_values() {
        let config = CortexConfig {
            memgraph_uri: "bolt://custom:7687".to_string(),
            memgraph_user: "admin".to_string(),
            memgraph_password: "secret".to_string(),
            max_batch_size: 1000,
            watched_paths: vec![PathBuf::from("/path/to/repo")],
        };

        assert_eq!(config.memgraph_uri, "bolt://custom:7687");
        assert_eq!(config.max_batch_size, 1000);
        assert_eq!(config.watched_paths.len(), 1);
    }

    #[test]
    fn config_serialization() {
        let config = CortexConfig {
            memgraph_uri: "bolt://localhost:7687".to_string(),
            memgraph_user: "test".to_string(),
            memgraph_password: "pass".to_string(),
            max_batch_size: 250,
            watched_paths: vec![],
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("memgraph_uri"));
        assert!(toml_str.contains("bolt://localhost:7687"));
        assert!(toml_str.contains("max_batch_size = 250"));
    }

    #[test]
    fn config_deserialization() {
        let toml_str = r#"
memgraph_uri = "bolt://192.168.1.1:7687"
memgraph_user = "admin"
memgraph_password = "secret"
max_batch_size = 750
watched_paths = []
"#;
        let config: CortexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.memgraph_uri, "bolt://192.168.1.1:7687");
        assert_eq!(config.memgraph_user, "admin");
        assert_eq!(config.max_batch_size, 750);
    }

    #[test]
    fn config_roundtrip() {
        let original = CortexConfig {
            memgraph_uri: "bolt://test:7687".to_string(),
            memgraph_user: "user".to_string(),
            memgraph_password: "pwd".to_string(),
            max_batch_size: 100,
            watched_paths: vec![PathBuf::from("/repo1"), PathBuf::from("/repo2")],
        };

        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: CortexConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.memgraph_uri, original.memgraph_uri);
        assert_eq!(parsed.memgraph_user, original.memgraph_user);
        assert_eq!(parsed.max_batch_size, original.max_batch_size);
        assert_eq!(parsed.watched_paths, original.watched_paths);
    }

    #[test]
    fn config_path_uses_home() {
        let path = CortexConfig::config_path();
        assert!(path.to_string_lossy().contains(".cortex"));
        assert!(path.file_name().unwrap().to_string_lossy() == "config.toml");
    }
}
