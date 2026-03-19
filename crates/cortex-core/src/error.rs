use thiserror::Error;

pub type Result<T> = std::result::Result<T, CortexError>;

#[derive(Debug, Error)]
pub enum CortexError {
    #[error("unsupported language for path: {0}")]
    UnsupportedLanguage(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("parse error in {path}: {message}")]
    Parse { path: String, message: String },
    #[error("database error: {0}")]
    Database(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("operation timed out: {0}")]
    Timeout(String),
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("embedding error: {0}")]
    Embedding(String),
    #[error("index error: {0}")]
    Index(String),
}

impl From<std::io::Error> for CortexError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl CortexError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Connection(_) | Self::Timeout(_) | Self::Database(_)
        )
    }

    /// Get a short error category
    pub fn category(&self) -> &'static str {
        match self {
            Self::UnsupportedLanguage(_) => "language",
            Self::Config(_) => "config",
            Self::Parse { .. } => "parse",
            Self::Database(_) => "database",
            Self::Io(_) => "io",
            Self::InvalidArgument(_) => "argument",
            Self::Timeout(_) => "timeout",
            Self::Runtime(_) => "runtime",
            Self::NotFound(_) => "not_found",
            Self::AlreadyExists(_) => "conflict",
            Self::Unauthorized(_) => "auth",
            Self::Connection(_) => "connection",
            Self::Embedding(_) => "embedding",
            Self::Index(_) => "index",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_is_retryable() {
        assert!(CortexError::Connection("test".to_string()).is_retryable());
        assert!(CortexError::Timeout("test".to_string()).is_retryable());
        assert!(CortexError::Database("test".to_string()).is_retryable());
        assert!(!CortexError::NotFound("test".to_string()).is_retryable());
        assert!(!CortexError::Config("test".to_string()).is_retryable());
    }

    #[test]
    fn error_category() {
        assert_eq!(
            CortexError::NotFound("test".to_string()).category(),
            "not_found"
        );
        assert_eq!(
            CortexError::Unauthorized("test".to_string()).category(),
            "auth"
        );
        assert_eq!(
            CortexError::Embedding("test".to_string()).category(),
            "embedding"
        );
        assert_eq!(CortexError::Index("test".to_string()).category(), "index");
        assert_eq!(CortexError::Config("test".to_string()).category(), "config");
    }

    #[test]
    fn error_display() {
        let err = CortexError::NotFound("resource".to_string());
        assert_eq!(format!("{}", err), "not found: resource");

        let err = CortexError::AlreadyExists("item".to_string());
        assert_eq!(format!("{}", err), "already exists: item");

        let err = CortexError::Unauthorized("access denied".to_string());
        assert_eq!(format!("{}", err), "unauthorized: access denied");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let cortex_err: CortexError = io_err.into();
        assert!(matches!(cortex_err, CortexError::Io(_)));
    }
}
