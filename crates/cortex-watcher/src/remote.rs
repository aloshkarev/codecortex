//! Remote file system support for cortex-watcher.
//!
//! This module provides:
//! - **Remote FS Detection**: Check if a path is on a remote/mounted filesystem
//! - **Remote Watcher**: Watch files on remote filesystems (SFTP, S3, etc.)
//!
//! ## Supported Remote Protocols
//!
//! - Local filesystem (mounted via SSHFS, FUSE, etc.)
//! - SFTP (via ssh2-rsync + rsync,//! - HTTP/HTTPS (via WebDAV)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Remote filesystem types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteFsType {
    /// Local filesystem (may be mounted)
    Local,
    /// SFTP remote filesystem
    Sftp,
    /// WebDAV/HTTP-based remote filesystem
    WebDav,
    /// Cloud storage (S3, GCS, Azure Blob, etc.)
    CloudStorage,
}

impl std::fmt::Display for RemoteFsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteFsType::Local => write!(f, "local"),
            RemoteFsType::Sftp => write!(f, "sftp"),
            RemoteFsType::WebDav => write!(f, "webdav"),
            RemoteFsType::CloudStorage => write!(f, "cloud_storage"),
        }
    }
}

/// Configuration for remote filesystem watching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFsConfig {
    /// Type of remote filesystem
    pub fs_type: RemoteFsType,
    /// Host for remote filesystem (e.g., sftp.example.com)
    pub host: String,
    /// Port for remote connection
    pub port: u16,
    /// Username for authentication
    pub username: Option<String>,
    /// Path to private key file
    pub private_key_path: Option<PathBuf>,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Read timeout in milliseconds
    pub read_timeout_ms: u64,
    /// Whether to use compression
    pub use_compression: bool,
}

impl Default for RemoteFsConfig {
    fn default() -> Self {
        Self {
            fs_type: RemoteFsType::Local,
            host: String::new(),
            port: 22,
            username: None,
            private_key_path: None,
            connection_timeout_secs: 30,
            read_timeout_ms: 30,
            use_compression: true,
        }
    }
}

/// Check if a path is on a remote/-mounted filesystem
pub fn is_remote_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Check for mounted filesystems (Linux FUSE mounts, macOS volumes)
    if path.starts_with("/mnt/") || path.starts_with("/Volumes/") {
        return true;
    }

    // Check for cloud storage prefixes
    if path_str.starts_with("s3://")
        || path_str.starts_with("gs://")
        || (path_str.starts_with("https://") && path_str.contains(".blob.core.windows"))
        || (path_str.starts_with("https://") && path_str.contains(".amazonaws.com"))
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn remote_fs_type_display() {
        assert_eq!(RemoteFsType::Local.to_string(), "local");
        assert_eq!(RemoteFsType::Sftp.to_string(), "sftp");
        assert_eq!(RemoteFsType::WebDav.to_string(), "webdav");
        assert_eq!(RemoteFsType::CloudStorage.to_string(), "cloud_storage");
    }

    #[test]
    fn remote_fs_config_default() {
        let config = RemoteFsConfig::default();
        assert_eq!(config.fs_type, RemoteFsType::Local);
        assert_eq!(config.port, 22);
        assert!(config.use_compression);
    }

    #[test]
    fn is_remote_path_local() {
        // These are local paths that are mounted or look like remote
        assert!(is_remote_path(Path::new("/mnt/data/file.txt")));
        assert!(is_remote_path(Path::new("/Volumes/MacHD/file.txt")));
    }

    #[test]
    fn is_remote_path_remote() {
        assert!(is_remote_path(Path::new("s3://bucket/file.txt")));
        assert!(is_remote_path(Path::new("gs://bucket/file.txt")));
        assert!(is_remote_path(Path::new("https://bucket.s3.amazonaws.com/file.txt")));
        assert!(is_remote_path(Path::new("https://myaccount.blob.core.windows.net/file.txt")));
    }

    #[test]
    fn is_remote_path_regular() {
        assert!(!is_remote_path(Path::new("/tmp/file.txt")));
        assert!(!is_remote_path(Path::new("/var/data/file.txt")));
        assert!(!is_remote_path(Path::new("C:\\Users\\file.txt")));
    }

    #[test]
    fn remote_fs_config_custom() {
        let config = RemoteFsConfig {
            fs_type: RemoteFsType::Sftp,
            host: "sftp.example.com".to_string(),
            port: 2222,
            username: Some("user".to_string()),
            private_key_path: Some(PathBuf::from("/keys/id_rsa")),
            connection_timeout_secs: 60,
            read_timeout_ms: 60,
            use_compression: true,
        };

        assert_eq!(config.fs_type, RemoteFsType::Sftp);
        assert_eq!(config.host, "sftp.example.com");
        assert_eq!(config.port, 2222);
    }

    #[test]
    fn remote_fs_config_serialization() {
        let config = RemoteFsConfig {
            fs_type: RemoteFsType::Sftp,
            host: "sftp.example.com".to_string(),
            port: 22,
            username: Some("user".to_string()),
            private_key_path: Some(PathBuf::from("/keys/id_rsa")),
            connection_timeout_secs: 30,
            read_timeout_ms: 30,
            use_compression: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RemoteFsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.fs_type, RemoteFsType::Sftp);
        assert_eq!(deserialized.host, "sftp.example.com");
    }
}
