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
}

impl From<std::io::Error> for CortexError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}
