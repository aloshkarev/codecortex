//! Vector Storage and Embedding Providers for CodeCortex
//!
//! This crate provides:
//! - `VectorStore` trait for abstracting vector database operations
//! - `JsonStore` implementation using JSON storage for simplicity
//! - `Embedder` trait for generating embeddings from text
//! - OpenAI and Ollama embedding provider implementations
//! - `HybridSearch` for combining graph and vector search
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      HybridSearch                           │
//! │  (Combines GraphClient + VectorStore for semantic search)   │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!           ┌───────────────────┴───────────────────┐
//!           ▼                                       ▼
//! ┌─────────────────────┐               ┌─────────────────────┐
//! │    VectorStore      │               │    Embedder         │
//! │    (JsonStore)      │               │ (OpenAI / Ollama)   │
//! └─────────────────────┘               └─────────────────────┘
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_vector::{JsonStore, OpenAIEmbedder, HybridSearch, Embedder, VectorStore};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create vector store
//!     let store = JsonStore::open("./vectors.db").await?;
//!
//!     // Create embedder
//!     let embedder = OpenAIEmbedder::new("sk-...".to_string());
//!
//!     // Insert documents
//!     let embedding = embedder.embed("fn authenticate(user: &str) -> Result<Token>").await?;
//!     store.upsert("func-1", embedding, std::collections::HashMap::new()).await?;
//!
//!     // Search
//!     let query_embedding = embedder.embed("login function").await?;
//!     let results = store.search(query_embedding, 10).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod embedder;
pub mod hybrid;
pub mod json_store;
pub mod lancedb_store;
pub mod schema;

pub use embedder::{Embedder, EmbeddingError, EmbeddingProvider, OllamaEmbedder, OpenAIEmbedder};
pub use hybrid::{HybridResult, HybridSearch, SearchType};
pub use json_store::JsonStore;
pub use lancedb_store::LanceStore;
pub use schema::{VectorDocument, VectorMetadata};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dimension of embeddings (OpenAI text-embedding-3-small = 1536)
pub const EMBEDDING_DIMENSION: usize = 1536;

/// Error type for vector operations
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Embedding error: {0}")]
    EmbeddingError(#[from] EmbeddingError),

    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Invalid dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Trait for vector database operations
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert or update a document with its embedding
    async fn upsert(
        &self,
        id: &str,
        embedding: Vec<f32>,
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<(), VectorError>;

    /// Batch insert documents
    async fn upsert_batch(&self, documents: Vec<VectorDocument>) -> Result<usize, VectorError>;

    /// Search for similar vectors
    async fn search(&self, query: Vec<f32>, k: usize) -> Result<Vec<SearchResult>, VectorError>;

    /// Search with metadata filter
    async fn search_with_filter(
        &self,
        query: Vec<f32>,
        k: usize,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<Vec<SearchResult>, VectorError>;

    /// Get a document by ID
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>, VectorError>;

    /// Delete a document by ID
    async fn delete(&self, id: &str) -> Result<bool, VectorError>;

    /// Delete all documents matching a filter
    async fn delete_by_filter(
        &self,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<usize, VectorError>;

    /// Get the number of documents in the store
    async fn count(&self) -> Result<usize, VectorError>;

    /// Check if the store is healthy
    async fn health_check(&self) -> Result<bool, VectorError>;
}

/// Metadata value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MetadataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<String>),
}

impl From<String> for MetadataValue {
    fn from(s: String) -> Self {
        MetadataValue::String(s)
    }
}

impl From<&str> for MetadataValue {
    fn from(s: &str) -> Self {
        MetadataValue::String(s.to_string())
    }
}

impl From<i64> for MetadataValue {
    fn from(i: i64) -> Self {
        MetadataValue::Integer(i)
    }
}

impl From<bool> for MetadataValue {
    fn from(b: bool) -> Self {
        MetadataValue::Boolean(b)
    }
}

/// Search result with score and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Document ID
    pub id: String,
    /// Similarity score (0-1, higher is more similar)
    pub score: f32,
    /// Document metadata
    pub metadata: HashMap<String, MetadataValue>,
    /// Optional content snippet
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_value_conversions() {
        let s: MetadataValue = "test".into();
        assert!(matches!(s, MetadataValue::String(_)));

        let i: MetadataValue = 42i64.into();
        assert!(matches!(i, MetadataValue::Integer(42)));

        let b: MetadataValue = true.into();
        assert!(matches!(b, MetadataValue::Boolean(true)));
    }
}
