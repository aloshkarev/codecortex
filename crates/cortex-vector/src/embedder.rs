//! Embedding Providers for Vector Generation
//!
//! This module provides the `Embedder` trait and implementations for:
//! - **OpenAI**: Uses the `text-embedding-3-small` model (1536 dimensions)
//! - **Ollama**: Uses local models like `nomic-embed-text`
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_vector::{Embedder, OpenAIEmbedder};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let embedder = OpenAIEmbedder::new(std::env::var("OPENAI_API_KEY")?);
//!
//!     let embedding = embedder.embed("fn authenticate(user: &str) -> Result<Token>").await?;
//!     println!("Embedding dimension: {}", embedding.len());
//!
//!     // Batch embedding
//!     let texts = vec!["fn foo()", "fn bar()", "fn baz()"];
//!     let embeddings = embedder.embed_batch(&texts).await?;
//!     println!("Generated {} embeddings", embeddings.len());
//!
//!     Ok(())
//! }
//! ```

use crate::EMBEDDING_DIMENSION;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Error type for embedding operations
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("OpenAI API error: {0}")]
    OpenAIError(String),

    #[error("Ollama API error: {0}")]
    OllamaError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("API key not configured")]
    MissingApiKey,

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

/// Embedding provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    OpenAI,
    Ollama,
}

impl std::fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for EmbeddingProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("openai") {
            Ok(Self::OpenAI)
        } else if s.eq_ignore_ascii_case("ollama") {
            Ok(Self::Ollama)
        } else {
            Err(format!("Unknown embedding provider: {}", s))
        }
    }
}

/// Trait for embedding providers
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate an embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;

    /// Generate embeddings for multiple texts in a batch
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Generate an embedding for a search query.
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embed(text).await
    }

    /// Generate an embedding for a code/document payload.
    async fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embed(text).await
    }

    /// Generate embeddings for multiple documents.
    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.embed_batch(texts).await
    }

    /// Get the provider type
    fn provider(&self) -> EmbeddingProvider;

    /// Get the model name
    fn model(&self) -> &str;

    /// Get the embedding dimension
    fn dimension(&self) -> usize {
        EMBEDDING_DIMENSION
    }
}

// ============================================================================
// OpenAI Embedder
// ============================================================================

/// OpenAI embedding client
pub struct OpenAIEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIEmbedder {
    /// Default model for embeddings
    pub const DEFAULT_MODEL: &str = "text-embedding-3-small";

    /// Create a new OpenAI embedder with API key
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: Self::DEFAULT_MODEL.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create with custom model
    pub fn with_model(api_key: String, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.into(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Set custom base URL (for proxies or Azure)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create from environment variable
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| EmbeddingError::MissingApiKey)?;
        Ok(Self::new(api_key))
    }

    fn validate_embedding_count(actual: usize, expected: usize) -> Result<(), EmbeddingError> {
        if actual != expected {
            return Err(EmbeddingError::InvalidResponse(format!(
                "expected {} embeddings, got {}",
                expected, actual
            )));
        }
        Ok(())
    }

    fn validate_embedding_dimension(embedding: &[f32]) -> Result<(), EmbeddingError> {
        if embedding.len() != EMBEDDING_DIMENSION {
            return Err(EmbeddingError::InvalidResponse(format!(
                "expected embedding dimension {}, got {}",
                EMBEDDING_DIMENSION,
                embedding.len()
            )));
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct OpenAIEmbedRequest {
    model: String,
    input: OpenAIInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenAIInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedding>,
    model: String,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    total_tokens: u32,
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let request = OpenAIEmbedRequest {
            model: self.model.clone(),
            input: OpenAIInput::Single(text.to_string()),
            dimensions: Some(EMBEDDING_DIMENSION),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::OpenAIError(error_text));
        }

        let embed_response: OpenAIEmbedResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;
        Self::validate_embedding_count(embed_response.data.len(), 1)?;

        let embedding = embed_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embedding in response".to_string()))?
            .embedding;
        Self::validate_embedding_dimension(&embedding)?;
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Process in batches of 100 to avoid rate limits
        const BATCH_SIZE: usize = 100;
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(BATCH_SIZE) {
            let request = OpenAIEmbedRequest {
                model: self.model.clone(),
                input: OpenAIInput::Batch(chunk.iter().map(|s| s.to_string()).collect()),
                dimensions: Some(EMBEDDING_DIMENSION),
            };

            let response = self
                .client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .timeout(Duration::from_secs(60))
                .send()
                .await?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(EmbeddingError::OpenAIError(error_text));
            }

            let embed_response: OpenAIEmbedResponse = response
                .json()
                .await
                .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;

            // Sort by index to maintain order
            let mut sorted: Vec<_> = embed_response.data.into_iter().collect();
            sorted.sort_by_key(|e| e.index);
            let embeddings: Vec<Vec<f32>> = sorted.into_iter().map(|e| e.embedding).collect();
            Self::validate_embedding_count(embeddings.len(), chunk.len())?;
            for embedding in &embeddings {
                Self::validate_embedding_dimension(embedding)?;
            }

            all_embeddings.extend(embeddings);
        }

        Ok(all_embeddings)
    }

    fn provider(&self) -> EmbeddingProvider {
        EmbeddingProvider::OpenAI
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// ============================================================================
// Ollama Embedder
// ============================================================================

/// Ollama embedding client
pub struct OllamaEmbedder {
    client: reqwest::Client,
    base_url: String,
    model: String,
    max_input_chars: usize,
    min_retry_chars: usize,
    max_retry_attempts: usize,
    target_dimension: usize,
    enable_bge_query_prefix: bool,
}

impl OllamaEmbedder {
    /// Default model for embeddings
    pub const DEFAULT_MODEL: &str = "nomic-embed-text";
    const BGE_M3_MODEL_PREFIX: &'static str = "bge-m3";
    const BGE_M3_QUERY_PREFIX: &'static str =
        "Represent this sentence for searching relevant passages: ";
    const BGE_M3_NATIVE_DIMENSION: usize = 1024;
    const DEFAULT_MAX_INPUT_CHARS: usize = 12_000;
    const BGE_M3_DEFAULT_MAX_INPUT_CHARS: usize = 8_000;
    const DEFAULT_MIN_RETRY_CHARS: usize = 256;
    const DEFAULT_MAX_RETRY_ATTEMPTS: usize = 6;
    const DEFAULT_TARGET_DIMENSION: usize = EMBEDDING_DIMENSION;

    /// Create a new Ollama embedder
    pub fn new() -> Self {
        let mut embedder = Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
            model: Self::DEFAULT_MODEL.to_string(),
            max_input_chars: Self::default_max_input_chars_for_model(Self::DEFAULT_MODEL),
            min_retry_chars: Self::DEFAULT_MIN_RETRY_CHARS,
            max_retry_attempts: Self::DEFAULT_MAX_RETRY_ATTEMPTS,
            target_dimension: Self::DEFAULT_TARGET_DIMENSION,
            enable_bge_query_prefix: true,
        };
        embedder.apply_env_overrides();
        embedder
    }

    /// Create with custom model
    pub fn with_model(model: impl Into<String>) -> Self {
        let mut embedder = Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
            model: model.into(),
            max_input_chars: Self::default_max_input_chars_for_model(Self::DEFAULT_MODEL),
            min_retry_chars: Self::DEFAULT_MIN_RETRY_CHARS,
            max_retry_attempts: Self::DEFAULT_MAX_RETRY_ATTEMPTS,
            target_dimension: Self::DEFAULT_TARGET_DIMENSION,
            enable_bge_query_prefix: true,
        };
        embedder.max_input_chars = Self::default_max_input_chars_for_model(&embedder.model);
        embedder.apply_env_overrides();
        embedder
    }

    /// Set custom Ollama base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the maximum input characters sent to Ollama in one request.
    pub fn with_max_input_chars(mut self, max_input_chars: usize) -> Self {
        self.max_input_chars = max_input_chars.max(256);
        self
    }

    /// Set the output embedding dimension target.
    pub fn with_target_dimension(mut self, target_dimension: usize) -> Self {
        self.target_dimension = target_dimension.max(1);
        self
    }

    /// Pad embedding to target dimension
    fn pad_embedding(&self, mut embedding: Vec<f32>) -> Vec<f32> {
        if embedding.len() >= self.target_dimension {
            embedding.truncate(self.target_dimension);
            embedding
        } else {
            embedding.resize(self.target_dimension, 0.0);
            embedding
        }
    }

    fn default_max_input_chars_for_model(model: &str) -> usize {
        if Self::is_bge_m3_model_name(model) {
            Self::BGE_M3_DEFAULT_MAX_INPUT_CHARS
        } else {
            Self::DEFAULT_MAX_INPUT_CHARS
        }
    }

    fn is_bge_m3_model_name(model: &str) -> bool {
        let m = model.trim_start();
        let prefix = Self::BGE_M3_MODEL_PREFIX.as_bytes();
        let b = m.as_bytes();
        b.len() >= prefix.len() && b[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    fn is_bge_m3_model(&self) -> bool {
        Self::is_bge_m3_model_name(&self.model)
    }

    fn native_dimension(&self) -> usize {
        if self.is_bge_m3_model() {
            Self::BGE_M3_NATIVE_DIMENSION
        } else {
            self.target_dimension
        }
    }

    fn request_dimensions(&self) -> Option<usize> {
        if self.is_bge_m3_model() {
            Some(self.target_dimension.min(self.native_dimension()))
        } else {
            None
        }
    }

    fn prepare_query_text(&self, text: &str) -> String {
        if self.is_bge_m3_model() && self.enable_bge_query_prefix {
            format!("{}{}", Self::BGE_M3_QUERY_PREFIX, text)
        } else {
            text.to_string()
        }
    }

    fn prepare_document_text(&self, text: &str) -> String {
        text.to_string()
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("CORTEX_OLLAMA_MAX_INPUT_CHARS")
            && let Ok(parsed) = v.parse::<usize>()
        {
            self.max_input_chars = parsed.max(256);
        }
        if let Ok(v) = std::env::var("CORTEX_OLLAMA_MIN_RETRY_CHARS")
            && let Ok(parsed) = v.parse::<usize>()
        {
            self.min_retry_chars = parsed.max(64);
        }
        if let Ok(v) = std::env::var("CORTEX_OLLAMA_MAX_RETRY_ATTEMPTS")
            && let Ok(parsed) = v.parse::<usize>()
        {
            self.max_retry_attempts = parsed.max(1);
        }
        if let Ok(v) = std::env::var("CORTEX_OLLAMA_TARGET_DIMENSION")
            && let Ok(parsed) = v.parse::<usize>()
        {
            self.target_dimension = parsed.max(1);
        }
        if let Ok(v) = std::env::var("CORTEX_VECTOR_TARGET_DIM")
            && let Ok(parsed) = v.parse::<usize>()
        {
            self.target_dimension = parsed.max(1);
        }
        if let Ok(v) = std::env::var("CORTEX_OLLAMA_ENABLE_BGE_QUERY_PREFIX") {
            let v = v.trim().to_ascii_lowercase();
            self.enable_bge_query_prefix = !matches!(v.as_str(), "0" | "false" | "no" | "off");
        }
    }

    fn char_len(text: &str) -> usize {
        text.chars().count()
    }

    fn truncate_middle(text: &str, max_chars: usize) -> String {
        if max_chars == 0 {
            return String::new();
        }
        if Self::char_len(text) <= max_chars {
            return text.to_string();
        }
        if max_chars <= 16 {
            return text.chars().take(max_chars).collect();
        }

        let marker = "\n/* ... truncated for embedding ... */\n";
        let marker_chars = marker.chars().count();
        if max_chars <= marker_chars + 2 {
            return text.chars().take(max_chars).collect();
        }

        let keep_total = max_chars - marker_chars;
        let head_chars = (keep_total * 2) / 3;
        let tail_chars = keep_total - head_chars;

        let head: String = text.chars().take(head_chars).collect();
        let tail_vec: Vec<char> = text.chars().rev().take(tail_chars).collect();
        let tail: String = tail_vec.into_iter().rev().collect();

        format!("{head}{marker}{tail}")
    }

    fn is_context_length_error(error_text: &str) -> bool {
        let lower = error_text.to_ascii_lowercase();
        lower.contains("input length exceeds the context length")
            || lower.contains("context length")
            || lower.contains("prompt is too long")
            || lower.contains("token limit")
    }

    async fn request_single_embedding(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            input: OllamaInput::Single(text.to_string()),
            truncate: true,
            dimensions: self.request_dimensions(),
        };

        let response = self
            .client
            .post(format!("{}/api/embed", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(EmbeddingError::OllamaError(error_text));
        }

        let embed_response: OllamaEmbedResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;
        Self::validate_embedding_count(embed_response.embeddings.len(), 1)?;

        embed_response
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::InvalidResponse("No embedding in response".to_string()))
    }

    fn validate_embedding_count(actual: usize, expected: usize) -> Result<(), EmbeddingError> {
        if actual != expected {
            return Err(EmbeddingError::InvalidResponse(format!(
                "expected {} embeddings, got {}",
                expected, actual
            )));
        }
        Ok(())
    }

    async fn embed_single_with_retry(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let mut candidate = Self::truncate_middle(text, self.max_input_chars);
        let mut attempts = 0usize;

        loop {
            match self.request_single_embedding(&candidate).await {
                Ok(embedding) => return Ok(self.pad_embedding(embedding)),
                Err(EmbeddingError::OllamaError(msg))
                    if Self::is_context_length_error(&msg)
                        && attempts < self.max_retry_attempts
                        && Self::char_len(&candidate) > self.min_retry_chars =>
                {
                    attempts += 1;
                    let next_len = (Self::char_len(&candidate) * 2) / 3;
                    let target = next_len.max(self.min_retry_chars);
                    candidate = Self::truncate_middle(&candidate, target);
                }
                Err(err) => return Err(err),
            }
        }
    }
}

impl Default for OllamaEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct OllamaEmbedRequest {
    model: String,
    input: OllamaInput,
    truncate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OllamaInput {
    Single(String),
    Batch(Vec<String>),
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
    #[serde(default)]
    model: Option<String>,
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.embed_document(text).await
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let prepared = self.prepare_query_text(text);
        self.embed_single_with_retry(&prepared).await
    }

    async fn embed_document(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let prepared = self.prepare_document_text(text);
        self.embed_single_with_retry(&prepared).await
    }

    async fn embed_documents(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        self.embed_batch(texts).await
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let prepared: Vec<String> = texts
            .iter()
            .map(|text| {
                let doc = self.prepare_document_text(text);
                Self::truncate_middle(&doc, self.max_input_chars)
            })
            .collect();

        let request = OllamaEmbedRequest {
            model: self.model.clone(),
            input: OllamaInput::Batch(prepared.clone()),
            truncate: true,
            dimensions: self.request_dimensions(),
        };

        let response = self
            .client
            .post(format!("{}/api/embed", self.base_url))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(Duration::from_secs(120))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if Self::is_context_length_error(&error_text) {
                let mut embeddings = Vec::with_capacity(prepared.len());
                for text in &prepared {
                    embeddings.push(self.embed_single_with_retry(text).await?);
                }
                return Ok(embeddings);
            }
            return Err(EmbeddingError::OllamaError(error_text));
        }

        let embed_response: OllamaEmbedResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::InvalidResponse(e.to_string()))?;
        Self::validate_embedding_count(embed_response.embeddings.len(), prepared.len())?;

        Ok(embed_response
            .embeddings
            .into_iter()
            .map(|e| self.pad_embedding(e))
            .collect())
    }

    fn provider(&self) -> EmbeddingProvider {
        EmbeddingProvider::Ollama
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimension(&self) -> usize {
        self.target_dimension
    }
}

/// Create an embedder based on configuration
pub fn create_embedder(
    provider: EmbeddingProvider,
    api_key: Option<String>,
    model: Option<String>,
) -> Result<Box<dyn Embedder>, EmbeddingError> {
    match provider {
        EmbeddingProvider::OpenAI => {
            let key = api_key.ok_or(EmbeddingError::MissingApiKey)?;
            let embedder = if let Some(m) = model {
                OpenAIEmbedder::with_model(key, m)
            } else {
                OpenAIEmbedder::new(key)
            };
            Ok(Box::new(embedder))
        }
        EmbeddingProvider::Ollama => {
            let embedder = if let Some(m) = model {
                OllamaEmbedder::with_model(m)
            } else {
                OllamaEmbedder::new()
            };
            Ok(Box::new(embedder))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(
            EmbeddingProvider::from_str("openai").unwrap(),
            EmbeddingProvider::OpenAI
        );
        assert_eq!(
            EmbeddingProvider::from_str("OPENAI").unwrap(),
            EmbeddingProvider::OpenAI
        );
        assert_eq!(
            EmbeddingProvider::from_str("ollama").unwrap(),
            EmbeddingProvider::Ollama
        );
        assert!(EmbeddingProvider::from_str("unknown").is_err());
    }

    #[test]
    fn test_ollama_padding() {
        let embedder = OllamaEmbedder::new();

        // Test padding
        let short = vec![0.5; 100];
        let padded = embedder.pad_embedding(short.clone());

        assert_eq!(padded.len(), EMBEDDING_DIMENSION);
        assert_eq!(&padded[..100], &short[..]);
        assert!(padded[100..].iter().all(|&x| x == 0.0));

        // Test truncation
        let long = vec![0.5; 2000];
        let truncated = embedder.pad_embedding(long);
        assert_eq!(truncated.len(), EMBEDDING_DIMENSION);
    }

    #[test]
    fn test_truncate_middle_short_input_unchanged() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let truncated = OllamaEmbedder::truncate_middle(source, 200);
        assert_eq!(truncated, source);
    }

    #[test]
    fn test_truncate_middle_long_input_is_bounded() {
        let source = "x".repeat(20_000);
        let truncated = OllamaEmbedder::truncate_middle(&source, 4_000);
        assert!(truncated.chars().count() <= 4_000);
        assert!(truncated.contains("truncated for embedding"));
    }

    #[test]
    fn test_context_length_error_detection() {
        let err = "{\"error\":\"the input length exceeds the context length\"}";
        assert!(OllamaEmbedder::is_context_length_error(err));
    }

    #[test]
    fn test_bge_m3_model_profile() {
        let embedder = OllamaEmbedder::with_model("bge-m3");
        assert!(embedder.is_bge_m3_model());
        assert_eq!(embedder.native_dimension(), 1024);
        assert_eq!(embedder.request_dimensions(), Some(1024));
    }

    #[test]
    fn test_bge_m3_query_prefix_applied() {
        let embedder = OllamaEmbedder::with_model("bge-m3");
        let prepared = embedder.prepare_query_text("find auth code");
        assert!(prepared.starts_with("Represent this sentence for searching relevant passages: "));
    }

    #[test]
    fn test_custom_target_dimension_respected() {
        let embedder = OllamaEmbedder::with_model("bge-m3").with_target_dimension(768);
        assert_eq!(embedder.dimension(), 768);
        assert_eq!(embedder.request_dimensions(), Some(768));
    }

    #[test]
    fn test_openai_default() {
        let embedder = OpenAIEmbedder::new("test-key".to_string());
        assert_eq!(embedder.model(), OpenAIEmbedder::DEFAULT_MODEL);
        assert_eq!(embedder.provider(), EmbeddingProvider::OpenAI);
    }

    #[test]
    fn test_ollama_default() {
        let embedder = OllamaEmbedder::new();
        assert_eq!(embedder.model(), OllamaEmbedder::DEFAULT_MODEL);
        assert_eq!(embedder.provider(), EmbeddingProvider::Ollama);
    }

    #[test]
    fn test_openai_validate_embedding_count() {
        let err =
            OpenAIEmbedder::validate_embedding_count(1, 2).expect_err("count mismatch should fail");
        assert!(matches!(err, EmbeddingError::InvalidResponse(_)));
    }

    #[test]
    fn test_openai_validate_embedding_dimension() {
        let err = OpenAIEmbedder::validate_embedding_dimension(&[0.0, 1.0, 2.0])
            .expect_err("dimension mismatch should fail");
        assert!(matches!(err, EmbeddingError::InvalidResponse(_)));
    }

    #[test]
    fn test_ollama_validate_embedding_count() {
        let err =
            OllamaEmbedder::validate_embedding_count(1, 2).expect_err("count mismatch should fail");
        assert!(matches!(err, EmbeddingError::InvalidResponse(_)));
    }
}
