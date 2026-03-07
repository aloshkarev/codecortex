//! Hybrid Search combining Graph and Vector Search
//!
//! This module provides the `HybridSearch` struct that combines structural
//! graph queries with semantic vector search for enhanced code discovery.
//!
//! ## Search Types
//!
//! - **Semantic**: Pure vector similarity search
//! - **Structural**: Graph traversal (callers, callees, etc.)
//! - **Hybrid**: Vector search enriched with graph context

use crate::{Embedder, MetadataValue, SearchResult, VectorStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Search type for hybrid queries
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SearchType {
    /// Pure vector similarity search
    #[default]
    Semantic,
    /// Graph traversal queries
    Structural,
    /// Vector search enriched with graph context
    Hybrid,
}

impl std::fmt::Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Semantic => write!(f, "semantic"),
            Self::Structural => write!(f, "structural"),
            Self::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl std::str::FromStr for SearchType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "semantic" | "vector" => Ok(Self::Semantic),
            "structural" | "graph" => Ok(Self::Structural),
            "hybrid" | "combined" => Ok(Self::Hybrid),
            _ => Err(format!("Unknown search type: {}", s)),
        }
    }
}

/// Result from hybrid search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    /// Search result from vector store
    pub result: SearchResult,
    /// Graph context (callers, callees, related symbols)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_context: Option<GraphContext>,
    /// Combined relevance score
    pub combined_score: f32,
}

/// Graph context for a search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphContext {
    /// Number of callers
    pub callers_count: usize,
    /// Number of callees
    pub callees_count: usize,
    /// Related symbols (same file, class, etc.)
    pub related_symbols: Vec<String>,
    /// Centrality score in the graph
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centrality: Option<f32>,
}

/// Hybrid search combining vector and graph search
pub struct HybridSearch {
    vector_store: Arc<dyn VectorStore>,
    embedder: Arc<dyn Embedder>,
}

impl HybridSearch {
    /// Create a new hybrid search instance
    pub fn new(vector_store: Arc<dyn VectorStore>, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            vector_store,
            embedder,
        }
    }

    /// Search for code using the specified search type
    pub async fn search(
        &self,
        query: &str,
        search_type: SearchType,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        match search_type {
            SearchType::Semantic => self.semantic_search(query, k).await,
            SearchType::Structural => {
                // Structural search would need GraphClient - return semantic for now
                self.semantic_search(query, k).await
            }
            SearchType::Hybrid => self.hybrid_search(query, k).await,
        }
    }

    /// Pure semantic (vector) search
    pub async fn semantic_search(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let embedding = self.embedder.embed_query(query).await?;

        let results = self.vector_store.search(embedding, k).await?;

        Ok(results
            .into_iter()
            .map(|r| HybridResult {
                combined_score: r.score,
                graph_context: None,
                result: r,
            })
            .collect())
    }

    /// Semantic search with metadata filter
    pub async fn semantic_search_with_filter(
        &self,
        query: &str,
        k: usize,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let embedding = self.embedder.embed_query(query).await?;

        let results = self
            .vector_store
            .search_with_filter(embedding, k, filter)
            .await?;

        Ok(results
            .into_iter()
            .map(|r| HybridResult {
                combined_score: r.score,
                graph_context: None,
                result: r,
            })
            .collect())
    }

    /// Hybrid search with graph context enrichment
    pub async fn hybrid_search(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        // Get semantic results
        let semantic_results = self.semantic_search(query, k * 2).await?;

        // For now, just return semantic results
        // In a full implementation, we would:
        // 1. Get graph context for each result (callers, callees)
        // 2. Re-rank based on combined scores
        // 3. Add related symbols from graph

        Ok(semantic_results.into_iter().take(k).collect())
    }

    /// Search within a specific repository
    pub async fn search_in_repository(
        &self,
        query: &str,
        repository: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let mut filter = HashMap::new();
        filter.insert(
            "repository".to_string(),
            MetadataValue::String(repository.to_string()),
        );

        self.semantic_search_with_filter(query, k, filter).await
    }

    /// Search within a specific file
    pub async fn search_in_file(
        &self,
        query: &str,
        path: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let mut filter = HashMap::new();
        filter.insert("path".to_string(), MetadataValue::String(path.to_string()));

        self.semantic_search_with_filter(query, k, filter).await
    }

    /// Search for symbols by kind (function, class, etc.)
    pub async fn search_by_kind(
        &self,
        query: &str,
        kind: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let mut filter = HashMap::new();
        filter.insert("kind".to_string(), MetadataValue::String(kind.to_string()));

        self.semantic_search_with_filter(query, k, filter).await
    }

    /// Search for symbols in a specific language
    pub async fn search_by_language(
        &self,
        query: &str,
        language: &str,
        k: usize,
    ) -> Result<Vec<HybridResult>, crate::VectorError> {
        let mut filter = HashMap::new();
        filter.insert(
            "language".to_string(),
            MetadataValue::String(language.to_string()),
        );

        self.semantic_search_with_filter(query, k, filter).await
    }

    /// Index a code document
    pub async fn index_document(
        &self,
        id: &str,
        content: &str,
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<(), crate::VectorError> {
        let embedding = self.embedder.embed_document(content).await?;
        self.vector_store.upsert(id, embedding, metadata).await
    }

    /// Index multiple documents
    pub async fn index_documents(
        &self,
        documents: Vec<crate::schema::VectorDocument>,
    ) -> Result<usize, crate::VectorError> {
        if documents.is_empty() {
            return self.vector_store.upsert_batch(Vec::new()).await;
        }

        // Generate embeddings for all documents
        let texts: Vec<&str> = documents.iter().map(|d| d.content.as_str()).collect();
        let embeddings = self.embedder.embed_documents(&texts).await?;

        if embeddings.len() != documents.len() {
            return Err(crate::VectorError::EmbeddingError(
                crate::EmbeddingError::InvalidResponse(format!(
                    "embedder returned {} embeddings for {} documents",
                    embeddings.len(),
                    documents.len()
                )),
            ));
        }

        // Attach embeddings to documents
        let mut documents_with_embeddings = Vec::new();
        for (mut doc, embedding) in documents.into_iter().zip(embeddings) {
            doc.embedding = embedding;
            documents_with_embeddings.push(doc);
        }

        self.vector_store
            .upsert_batch(documents_with_embeddings)
            .await
    }

    /// Get the underlying vector store
    pub fn vector_store(&self) -> &dyn VectorStore {
        self.vector_store.as_ref()
    }

    /// Get the underlying embedder
    pub fn embedder(&self) -> &dyn Embedder {
        self.embedder.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};

    struct MockEmbedder {
        embeddings: Vec<Vec<f32>>,
    }

    #[async_trait]
    impl Embedder for MockEmbedder {
        async fn embed(&self, _text: &str) -> Result<Vec<f32>, crate::EmbeddingError> {
            Ok(vec![0.0; crate::EMBEDDING_DIMENSION])
        }

        async fn embed_batch(
            &self,
            _texts: &[&str],
        ) -> Result<Vec<Vec<f32>>, crate::EmbeddingError> {
            Ok(self.embeddings.clone())
        }

        fn provider(&self) -> crate::EmbeddingProvider {
            crate::EmbeddingProvider::Ollama
        }

        fn model(&self) -> &str {
            "mock"
        }
    }

    #[derive(Default)]
    struct MockVectorStore {
        upsert_batch_calls: Arc<Mutex<usize>>,
    }

    #[async_trait]
    impl VectorStore for MockVectorStore {
        async fn upsert(
            &self,
            _id: &str,
            _embedding: Vec<f32>,
            _metadata: HashMap<String, MetadataValue>,
        ) -> Result<(), crate::VectorError> {
            Ok(())
        }

        async fn upsert_batch(
            &self,
            documents: Vec<crate::schema::VectorDocument>,
        ) -> Result<usize, crate::VectorError> {
            *self.upsert_batch_calls.lock().expect("lock poisoned") += 1;
            Ok(documents.len())
        }

        async fn search(
            &self,
            _query: Vec<f32>,
            _k: usize,
        ) -> Result<Vec<crate::SearchResult>, crate::VectorError> {
            Ok(Vec::new())
        }

        async fn search_with_filter(
            &self,
            _query: Vec<f32>,
            _k: usize,
            _filter: HashMap<String, MetadataValue>,
        ) -> Result<Vec<crate::SearchResult>, crate::VectorError> {
            Ok(Vec::new())
        }

        async fn get(
            &self,
            _id: &str,
        ) -> Result<Option<crate::schema::VectorDocument>, crate::VectorError> {
            Ok(None)
        }

        async fn delete(&self, _id: &str) -> Result<bool, crate::VectorError> {
            Ok(false)
        }

        async fn delete_by_filter(
            &self,
            _filter: HashMap<String, MetadataValue>,
        ) -> Result<usize, crate::VectorError> {
            Ok(0)
        }

        async fn count(&self) -> Result<usize, crate::VectorError> {
            Ok(0)
        }

        async fn health_check(&self) -> Result<bool, crate::VectorError> {
            Ok(true)
        }
    }

    #[test]
    fn test_search_type_from_str() {
        assert_eq!(
            SearchType::from_str("semantic").unwrap(),
            SearchType::Semantic
        );
        assert_eq!(
            SearchType::from_str("vector").unwrap(),
            SearchType::Semantic
        );
        assert_eq!(
            SearchType::from_str("structural").unwrap(),
            SearchType::Structural
        );
        assert_eq!(
            SearchType::from_str("graph").unwrap(),
            SearchType::Structural
        );
        assert_eq!(SearchType::from_str("hybrid").unwrap(), SearchType::Hybrid);
        assert_eq!(
            SearchType::from_str("combined").unwrap(),
            SearchType::Hybrid
        );
        assert!(SearchType::from_str("unknown").is_err());
    }

    #[tokio::test]
    async fn test_index_documents_rejects_embedding_count_mismatch() {
        let vector_store = Arc::new(MockVectorStore::default());
        let embedder = Arc::new(MockEmbedder {
            embeddings: vec![vec![0.0; crate::EMBEDDING_DIMENSION]],
        });
        let search = HybridSearch::new(vector_store.clone(), embedder);
        let docs = vec![
            crate::schema::VectorDocument::new("a".to_string(), vec![], "alpha".to_string()),
            crate::schema::VectorDocument::new("b".to_string(), vec![], "beta".to_string()),
        ];

        let err = search
            .index_documents(docs)
            .await
            .expect_err("mismatched embeddings should fail");

        assert!(matches!(
            err,
            crate::VectorError::EmbeddingError(crate::EmbeddingError::InvalidResponse(_))
        ));
        assert_eq!(
            *vector_store
                .upsert_batch_calls
                .lock()
                .expect("lock poisoned"),
            0
        );
    }
}
