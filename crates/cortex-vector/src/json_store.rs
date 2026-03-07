//! Local JSON-based Vector Store Implementation
//!
//! Provides persistent vector storage using JSON files.
//! This is a simple, embedded store suitable for development and small deployments.
//! For production with large datasets, consider using Qdrant or a real LanceDB implementation.
//!
//! ## Storage Format
//!
//! Vectors are stored in a `vectors.json` file in the specified directory:
//! ```text
//! ~/.cortex/vectors/
//! └── vectors.json
//! ```
//!
//! ## Performance
//!
//! - Good for: < 10,000 documents
//! - Search: O(n) cosine similarity (no indexing)
//! - Persistence: Full file rewrite on each change

use crate::{
    MetadataValue, SearchResult, VectorDocument, VectorError, VectorMetadata, VectorStore,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// File name for vector storage
const VECTORS_FILE: &str = "vectors.json";

/// Local vector store using JSON files
///
/// This is a simple embedded store that persists vectors to a JSON file.
/// It's suitable for development and small deployments.
pub struct JsonStore {
    db_path: String,
    data: Arc<RwLock<Vec<VectorDocument>>>,
}

impl JsonStore {
    /// Open or create a vector store at the given path
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, VectorError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        info!("Opening JSON vector store at {}", path_str);

        // Create parent directory if needed
        if let Some(parent) = path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Load existing data
        let data = Self::load_data(path.as_ref()).await?;

        Ok(Self {
            db_path: path_str,
            data: Arc::new(RwLock::new(data)),
        })
    }

    /// Load data from JSON file
    async fn load_data(path: &Path) -> Result<Vec<VectorDocument>, VectorError> {
        let file_path = path.join(VECTORS_FILE);

        if !file_path.exists() {
            return Ok(vec![]);
        }

        let content = tokio::fs::read_to_string(&file_path).await?;
        let documents: Vec<VectorDocument> = serde_json::from_str(&content)?;

        info!("Loaded {} documents from JSON store", documents.len());

        Ok(documents)
    }

    /// Save data to JSON file
    async fn save_data(&self) -> Result<(), VectorError> {
        let file_path = Path::new(&self.db_path).join(VECTORS_FILE);

        let data = self.data.read().await.clone();
        let content = serde_json::to_string_pretty(&data)?;

        tokio::fs::write(&file_path, content).await?;

        debug!("Saved {} documents to JSON store", data.len());

        Ok(())
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            return 0.0;
        }

        dot / (mag_a * mag_b)
    }

    /// Check if document matches filter
    fn matches_filter(doc: &VectorDocument, filter: &HashMap<String, MetadataValue>) -> bool {
        let metadata = doc.metadata_to_map();

        filter
            .iter()
            .all(|(key, value)| metadata.get(key).map(|v| v == value).unwrap_or(false))
    }

    fn map_to_metadata(metadata: &HashMap<String, MetadataValue>) -> VectorMetadata {
        let mut doc_metadata = VectorMetadata::default();

        if let Some(MetadataValue::String(path)) = metadata.get("path") {
            doc_metadata.path = Some(path.clone());
        }
        if let Some(MetadataValue::String(name)) = metadata.get("name") {
            doc_metadata.name = Some(name.clone());
        }
        if let Some(MetadataValue::String(kind)) = metadata.get("kind") {
            doc_metadata.kind = Some(kind.clone());
        }
        if let Some(MetadataValue::String(lang)) = metadata.get("language") {
            doc_metadata.language = Some(lang.clone());
        }
        if let Some(MetadataValue::String(repo)) = metadata.get("repository") {
            doc_metadata.repository = Some(repo.clone());
        }
        if let Some(MetadataValue::String(branch)) = metadata.get("branch") {
            doc_metadata.branch = Some(branch.clone());
        }
        if let Some(MetadataValue::Integer(line)) = metadata.get("line_number")
            && *line >= 0
        {
            doc_metadata.line_number = Some(*line as usize);
        }
        if let Some(MetadataValue::String(doc_type)) = metadata.get("doc_type") {
            doc_metadata.doc_type = Some(doc_type.clone());
        }

        for (key, value) in metadata {
            if matches!(
                key.as_str(),
                "path"
                    | "name"
                    | "kind"
                    | "language"
                    | "repository"
                    | "branch"
                    | "line_number"
                    | "doc_type"
                    | "content"
            ) {
                continue;
            }
            let json_value = match value {
                MetadataValue::String(text) => serde_json::Value::String(text.clone()),
                MetadataValue::Integer(int) => serde_json::json!(*int),
                MetadataValue::Float(float) => serde_json::json!(*float),
                MetadataValue::Boolean(flag) => serde_json::json!(*flag),
                MetadataValue::List(items) => serde_json::json!(items),
            };
            doc_metadata.extra.insert(key.clone(), json_value);
        }

        doc_metadata
    }
}

#[async_trait]
impl VectorStore for JsonStore {
    async fn upsert(
        &self,
        id: &str,
        embedding: Vec<f32>,
        metadata: HashMap<String, MetadataValue>,
    ) -> Result<(), VectorError> {
        let content = metadata
            .get("content")
            .and_then(|v| {
                if let MetadataValue::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let doc_metadata = Self::map_to_metadata(&metadata);
        let doc = VectorDocument::with_metadata(id.to_string(), embedding, content, doc_metadata);

        self.upsert_batch(vec![doc]).await?;

        Ok(())
    }

    async fn upsert_batch(&self, documents: Vec<VectorDocument>) -> Result<usize, VectorError> {
        if documents.is_empty() {
            return Ok(0);
        }

        for doc in &documents {
            doc.validate().map_err(|message| {
                if message.starts_with("Invalid embedding dimension:") {
                    VectorError::InvalidDimension {
                        expected: crate::EMBEDDING_DIMENSION,
                        actual: doc.embedding.len(),
                    }
                } else {
                    VectorError::DatabaseError(message)
                }
            })?;
        }

        let mut data = self.data.write().await;

        // Remove existing documents with same IDs
        let ids: Vec<&str> = documents.iter().map(|d| d.id.as_str()).collect();
        data.retain(|d| !ids.contains(&d.id.as_str()));

        // Add new documents
        let count = documents.len();
        for doc in documents {
            data.push(doc);
        }

        drop(data);

        // Persist to disk
        self.save_data().await?;

        info!("Stored {} documents in JSON vector store", count);

        Ok(count)
    }

    async fn search(&self, query: Vec<f32>, k: usize) -> Result<Vec<SearchResult>, VectorError> {
        if query.len() != crate::EMBEDDING_DIMENSION {
            return Err(VectorError::InvalidDimension {
                expected: crate::EMBEDDING_DIMENSION,
                actual: query.len(),
            });
        }
        let data = self.data.read().await;

        // Compute similarities
        let mut results: Vec<(f32, &VectorDocument)> = data
            .iter()
            .map(|doc| {
                let similarity = Self::cosine_similarity(&query, &doc.embedding);
                (similarity, doc)
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        Ok(results
            .into_iter()
            .take(k)
            .map(|(score, doc)| SearchResult {
                id: doc.id.clone(),
                score,
                metadata: doc.metadata_to_map(),
                content: Some(doc.content.clone()),
            })
            .collect())
    }

    async fn search_with_filter(
        &self,
        query: Vec<f32>,
        k: usize,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        if query.len() != crate::EMBEDDING_DIMENSION {
            return Err(VectorError::InvalidDimension {
                expected: crate::EMBEDDING_DIMENSION,
                actual: query.len(),
            });
        }
        let data = self.data.read().await;

        // Filter and compute similarities
        let mut results: Vec<(f32, &VectorDocument)> = data
            .iter()
            .filter(|doc| Self::matches_filter(doc, &filter))
            .map(|doc| {
                let similarity = Self::cosine_similarity(&query, &doc.embedding);
                (similarity, doc)
            })
            .collect();

        // Sort by similarity (descending)
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        Ok(results
            .into_iter()
            .take(k)
            .map(|(score, doc)| SearchResult {
                id: doc.id.clone(),
                score,
                metadata: doc.metadata_to_map(),
                content: Some(doc.content.clone()),
            })
            .collect())
    }

    async fn get(&self, id: &str) -> Result<Option<VectorDocument>, VectorError> {
        let data = self.data.read().await;

        Ok(data.iter().find(|d| d.id == id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<bool, VectorError> {
        let deleted = {
            let mut data = self.data.write().await;
            let initial_len = data.len();
            data.retain(|d| d.id != id);
            data.len() < initial_len
        };

        if deleted {
            self.save_data().await?;
        }

        Ok(deleted)
    }

    async fn delete_by_filter(
        &self,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<usize, VectorError> {
        if filter.is_empty() {
            return Ok(0);
        }

        let deleted = {
            let mut data = self.data.write().await;
            let initial_len = data.len();
            data.retain(|d| !Self::matches_filter(d, &filter));
            initial_len - data.len()
        };

        if deleted > 0 {
            self.save_data().await?;
        }

        Ok(deleted)
    }

    async fn count(&self) -> Result<usize, VectorError> {
        let data = self.data.read().await;
        Ok(data.len())
    }

    async fn health_check(&self) -> Result<bool, VectorError> {
        // Simple check - can we access the data?
        let _data = self.data.read().await;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EMBEDDING_DIMENSION;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_open() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path()).await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_store_upsert_and_search() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        // Create a test embedding
        let embedding = vec![0.1; EMBEDDING_DIMENSION];
        let mut metadata = HashMap::new();
        metadata.insert(
            "path".to_string(),
            MetadataValue::String("src/main.rs".to_string()),
        );
        metadata.insert(
            "name".to_string(),
            MetadataValue::String("main".to_string()),
        );
        metadata.insert(
            "kind".to_string(),
            MetadataValue::String("function".to_string()),
        );
        metadata.insert(
            "language".to_string(),
            MetadataValue::String("rust".to_string()),
        );
        metadata.insert(
            "content".to_string(),
            MetadataValue::String("fn main() {}".to_string()),
        );

        // Upsert
        store
            .upsert("test-1", embedding.clone(), metadata)
            .await
            .expect("Upsert failed");

        // Search
        let results = store.search(embedding, 10).await.expect("Search failed");
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_store_count() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        let initial_count = store.count().await.expect("Count failed");
        assert_eq!(initial_count, 0);

        let embedding = vec![0.1; EMBEDDING_DIMENSION];
        let metadata = HashMap::new();

        store
            .upsert("test-1", embedding, metadata)
            .await
            .expect("Upsert failed");

        let count_after = store.count().await.expect("Count failed");
        assert_eq!(count_after, 1);
    }

    #[tokio::test]
    async fn test_store_delete() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        let embedding = vec![0.1; EMBEDDING_DIMENSION];
        let metadata = HashMap::new();

        store
            .upsert("test-1", embedding.clone(), metadata)
            .await
            .expect("Upsert failed");
        assert_eq!(store.count().await.expect("Count failed"), 1);

        let deleted = store.delete("test-1").await.expect("Delete failed");
        assert!(deleted);
        assert_eq!(store.count().await.expect("Count failed"), 0);
    }

    #[tokio::test]
    async fn test_store_persistence() {
        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create and add document
        {
            let store = JsonStore::open(dir.path())
                .await
                .expect("Failed to open store");
            let embedding = vec![0.5; EMBEDDING_DIMENSION];
            let mut metadata = HashMap::new();
            metadata.insert(
                "name".to_string(),
                MetadataValue::String("test".to_string()),
            );

            store
                .upsert("persist-1", embedding, metadata)
                .await
                .expect("Upsert failed");
        }

        // Reopen and verify
        {
            let store = JsonStore::open(dir.path())
                .await
                .expect("Failed to open store");
            let count = store.count().await.expect("Count failed");
            assert_eq!(count, 1);

            let doc = store.get("persist-1").await.expect("Get failed");
            assert!(doc.is_some());
        }
    }

    #[tokio::test]
    async fn test_store_search_with_filter() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        let embedding = vec![0.1; EMBEDDING_DIMENSION];

        // Insert documents with different languages
        let mut metadata1 = HashMap::new();
        metadata1.insert(
            "language".to_string(),
            MetadataValue::String("rust".to_string()),
        );
        metadata1.insert(
            "content".to_string(),
            MetadataValue::String("fn foo()".to_string()),
        );
        store
            .upsert("rust-1", embedding.clone(), metadata1)
            .await
            .expect("Upsert failed");

        let mut metadata2 = HashMap::new();
        metadata2.insert(
            "language".to_string(),
            MetadataValue::String("python".to_string()),
        );
        metadata2.insert(
            "content".to_string(),
            MetadataValue::String("def bar()".to_string()),
        );
        store
            .upsert("python-1", embedding.clone(), metadata2)
            .await
            .expect("Upsert failed");

        // Search with filter
        let mut filter = HashMap::new();
        filter.insert(
            "language".to_string(),
            MetadataValue::String("rust".to_string()),
        );

        let results = store
            .search_with_filter(embedding, 10, filter)
            .await
            .expect("Search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "rust-1");
    }

    #[tokio::test]
    async fn test_store_delete_by_empty_filter_is_noop() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        store
            .upsert("doc-1", vec![0.2; EMBEDDING_DIMENSION], HashMap::new())
            .await
            .expect("Upsert failed");

        let deleted = store
            .delete_by_filter(HashMap::new())
            .await
            .expect("Delete by empty filter failed");

        assert_eq!(deleted, 0);
        assert_eq!(store.count().await.expect("Count failed"), 1);
    }

    #[tokio::test]
    async fn test_store_preserves_extra_metadata() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        let mut metadata = HashMap::new();
        metadata.insert(
            "team".to_string(),
            MetadataValue::String("search".to_string()),
        );
        metadata.insert("priority".to_string(), MetadataValue::Integer(3));

        store
            .upsert("doc-extra", vec![0.3; EMBEDDING_DIMENSION], metadata)
            .await
            .expect("Upsert failed");

        let doc = store
            .get("doc-extra")
            .await
            .expect("Get failed")
            .expect("Document should exist");
        let map = doc.metadata_to_map();

        assert_eq!(
            map.get("team"),
            Some(&MetadataValue::String("search".to_string()))
        );
        assert_eq!(map.get("priority"), Some(&MetadataValue::Integer(3)));
    }

    #[tokio::test]
    async fn test_search_rejects_wrong_dimension() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = JsonStore::open(dir.path())
            .await
            .expect("Failed to open store");

        let err = store
            .search(vec![0.1; 8], 5)
            .await
            .expect_err("wrong dimension should fail");

        assert!(matches!(
            err,
            VectorError::InvalidDimension {
                expected: crate::EMBEDDING_DIMENSION,
                actual: 8
            }
        ));
    }
}
