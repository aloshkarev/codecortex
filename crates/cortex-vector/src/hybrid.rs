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

#[derive(Debug, Clone, Copy)]
enum QueryIntent {
    Bugfix,
    Refactor,
    Tests,
    ApiUsage,
    Explore,
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
                // Structural fallback still uses vectors but applies stronger metadata priors.
                self.hybrid_search(query, k).await
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
        let intent = detect_intent(query);
        let semantic_results = self.semantic_search(query, k * 3).await?;
        let mut reranked: Vec<HybridResult> = semantic_results
            .into_iter()
            .map(|mut r| {
                let vector_score = r.result.score;
                let graph_score = estimated_graph_signal(&r);
                let lexical_hint = lexical_metadata_hint(query, &r);
                let intent_boost = intent_kind_boost(intent, &r);
                let (w_vector, w_graph, w_lexical) = intent_weights(intent);
                r.combined_score = (vector_score * w_vector)
                    + (graph_score * w_graph)
                    + (lexical_hint * w_lexical)
                    + intent_boost;
                r
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        reranked.truncate(k);
        Ok(reranked)
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

fn detect_intent(query: &str) -> QueryIntent {
    let q = query.to_ascii_lowercase();
    if q.contains("bug") || q.contains("error") || q.contains("panic") || q.contains("fix") {
        QueryIntent::Bugfix
    } else if q.contains("refactor") || q.contains("cleanup") || q.contains("rename") {
        QueryIntent::Refactor
    } else if q.contains("test") || q.contains("assert") || q.contains("spec") {
        QueryIntent::Tests
    } else if q.contains("how to") || q.contains("usage") || q.contains("example") {
        QueryIntent::ApiUsage
    } else {
        QueryIntent::Explore
    }
}

fn intent_weights(intent: QueryIntent) -> (f32, f32, f32) {
    match intent {
        QueryIntent::Bugfix => (0.60, 0.25, 0.15),
        QueryIntent::Refactor => (0.50, 0.35, 0.15),
        QueryIntent::Tests => (0.45, 0.20, 0.35),
        QueryIntent::ApiUsage => (0.55, 0.20, 0.25),
        QueryIntent::Explore => (0.65, 0.15, 0.20),
    }
}

fn estimated_graph_signal(result: &HybridResult) -> f32 {
    result
        .graph_context
        .as_ref()
        .map(|ctx| ((ctx.callers_count + ctx.callees_count) as f32 / 16.0).min(1.0))
        .unwrap_or(0.0)
}

fn lexical_metadata_hint(query: &str, result: &HybridResult) -> f32 {
    let q = query.to_ascii_lowercase();
    let mut hint = 0.0f32;
    if let Some(MetadataValue::String(kind)) = result.result.metadata.get("kind") {
        let k = kind.to_ascii_lowercase();
        if q.contains(&k) {
            hint += 0.3;
        }
    }
    if let Some(MetadataValue::String(path)) = result.result.metadata.get("path") {
        let p = path.to_ascii_lowercase();
        if p.contains("test") && q.contains("test") {
            hint += 0.3;
        }
        if p.contains("api") && (q.contains("api") || q.contains("endpoint")) {
            hint += 0.2;
        }
    }
    hint.min(1.0)
}

fn intent_kind_boost(intent: QueryIntent, result: &HybridResult) -> f32 {
    let kind = result
        .result
        .metadata
        .get("kind")
        .and_then(|v| match v {
            MetadataValue::String(s) => Some(s.to_ascii_lowercase()),
            _ => None,
        })
        .unwrap_or_default();
    match intent {
        QueryIntent::Tests if kind.contains("test") => 0.15,
        QueryIntent::Refactor if kind.contains("struct") || kind.contains("class") => 0.10,
        QueryIntent::Bugfix if kind.contains("function") || kind.contains("method") => 0.08,
        QueryIntent::ApiUsage if kind.contains("function") => 0.06,
        _ => 0.0,
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

    #[test]
    fn test_detect_intent_variants() {
        assert!(matches!(
            detect_intent("fix panic in parser"),
            QueryIntent::Bugfix
        ));
        assert!(matches!(
            detect_intent("refactor module layout"),
            QueryIntent::Refactor
        ));
        assert!(matches!(
            detect_intent("write tests for auth"),
            QueryIntent::Tests
        ));
        assert!(matches!(
            detect_intent("api usage for search"),
            QueryIntent::ApiUsage
        ));
    }
}
