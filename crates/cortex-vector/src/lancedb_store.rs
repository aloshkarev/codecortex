//! LanceDB Vector Store Implementation
//!
//! Provides vector storage using LanceDB - an embedded, serverless vector database.
//! LanceDB uses Apache Arrow as its underlying storage format for efficient vector operations.
//!
//! ## Features
//!
//! - Embedded (no server required)
//! - Persistent storage to disk
//! - Vector indexing for fast similarity search
//! - Supports filtering on metadata
//!
//! ## Storage Location
//!
//! Data is stored in Lance format at the specified path:
//! ```text
//! ~/.cortex/vectors/
//! └── .lance/
//! ```
//!
//! ## Performance
//!
//! - Good for: millions of documents
//! - Search: Sub-second with vector index
//! - Automatic indexing for vectors > 256 dimensions

use crate::{
    EMBEDDING_DIMENSION, MetadataValue, SearchResult, VectorDocument, VectorError, VectorMetadata,
    VectorStore,
};
use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Float32Array, Int64Array, RecordBatch,
    RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Table name for vector documents
const TABLE_NAME: &str = "vectors";

/// LanceDB-based vector store
pub struct LanceStore {
    conn: Connection,
    schema: SchemaRef,
}

impl LanceStore {
    /// Open or create a LanceDB vector store at the given path
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self, VectorError> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        info!("Opening LanceDB vector store at {}", path_str);

        // Create parent directory if needed
        if let Some(parent) = path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Connect to LanceDB
        let conn = lancedb::connect(&path_str).execute().await.map_err(|e| {
            VectorError::DatabaseError(format!("Failed to connect to LanceDB: {}", e))
        })?;

        let schema = Self::create_schema();

        // Check if table exists, create if not
        let table_names = conn
            .table_names()
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to list tables: {}", e)))?;

        if !table_names.contains(&TABLE_NAME.to_string()) {
            info!("Creating new LanceDB table: {}", TABLE_NAME);

            // Create empty table with schema
            let empty_batch = RecordBatch::new_empty(schema.clone());
            let batches = vec![Ok(empty_batch)];
            let iter = RecordBatchIterator::new(batches, schema.clone());

            conn.create_table(TABLE_NAME, Box::new(iter))
                .execute()
                .await
                .map_err(|e| {
                    VectorError::DatabaseError(format!("Failed to create table: {}", e))
                })?;
        } else {
            info!("Opening existing LanceDB table: {}", TABLE_NAME);
        }

        Ok(Self { conn, schema })
    }

    /// Create the Arrow schema for vector documents
    fn create_schema() -> SchemaRef {
        let fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBEDDING_DIMENSION as i32,
                ),
                false,
            ),
            Field::new("content", DataType::Utf8, false),
            // Metadata fields
            Field::new("path", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
            Field::new("kind", DataType::Utf8, true),
            Field::new("language", DataType::Utf8, true),
            Field::new("repository", DataType::Utf8, true),
            Field::new("branch", DataType::Utf8, true),
            Field::new("line_number", DataType::Int64, true),
            Field::new("doc_type", DataType::Utf8, true),
        ];

        Arc::new(Schema::new(fields))
    }

    /// Convert VectorDocument to RecordBatch
    fn documents_to_batch(&self, documents: &[VectorDocument]) -> Result<RecordBatch, VectorError> {
        let mut ids: Vec<String> = Vec::new();
        let mut contents: Vec<String> = Vec::new();
        let mut paths: Vec<Option<String>> = Vec::new();
        let mut names: Vec<Option<String>> = Vec::new();
        let mut kinds: Vec<Option<String>> = Vec::new();
        let mut languages: Vec<Option<String>> = Vec::new();
        let mut repositories: Vec<Option<String>> = Vec::new();
        let mut branches: Vec<Option<String>> = Vec::new();
        let mut line_numbers: Vec<Option<i64>> = Vec::new();
        let mut doc_types: Vec<Option<String>> = Vec::new();

        for doc in documents {
            ids.push(doc.id.clone());
            contents.push(doc.content.clone());
            paths.push(doc.metadata.path.clone());
            names.push(doc.metadata.name.clone());
            kinds.push(doc.metadata.kind.clone());
            languages.push(doc.metadata.language.clone());
            repositories.push(doc.metadata.repository.clone());
            branches.push(doc.metadata.branch.clone());
            line_numbers.push(doc.metadata.line_number.map(|n| n as i64));
            doc_types.push(doc.metadata.doc_type.clone());
        }

        // Create embedding FixedSizeListArray
        let embedding_array = Self::create_embedding_array(&documents)?;

        let id_array: ArrayRef = Arc::new(StringArray::from(ids));
        let content_array: ArrayRef = Arc::new(StringArray::from(contents));
        let path_array: ArrayRef = Arc::new(StringArray::from(paths));
        let name_array: ArrayRef = Arc::new(StringArray::from(names));
        let kind_array: ArrayRef = Arc::new(StringArray::from(kinds));
        let language_array: ArrayRef = Arc::new(StringArray::from(languages));
        let repository_array: ArrayRef = Arc::new(StringArray::from(repositories));
        let branch_array: ArrayRef = Arc::new(StringArray::from(branches));
        let line_number_array: ArrayRef = Arc::new(Int64Array::from(line_numbers));
        let doc_type_array: ArrayRef = Arc::new(StringArray::from(doc_types));

        RecordBatch::try_new(
            self.schema.clone(),
            vec![
                id_array,
                embedding_array,
                content_array,
                path_array,
                name_array,
                kind_array,
                language_array,
                repository_array,
                branch_array,
                line_number_array,
                doc_type_array,
            ],
        )
        .map_err(|e| VectorError::DatabaseError(format!("Failed to create record batch: {}", e)))
    }

    /// Create FixedSizeListArray for embeddings
    fn create_embedding_array(documents: &[VectorDocument]) -> Result<ArrayRef, VectorError> {
        use arrow_array::builder::FixedSizeListBuilder;
        use arrow_array::builder::Float32Builder;

        let mut builder =
            FixedSizeListBuilder::new(Float32Builder::new(), EMBEDDING_DIMENSION as i32);

        for doc in documents {
            if doc.embedding.len() != EMBEDDING_DIMENSION {
                return Err(VectorError::DatabaseError(format!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    EMBEDDING_DIMENSION,
                    doc.embedding.len()
                )));
            }
            for &val in &doc.embedding {
                builder.values().append_value(val);
            }
            builder.append(true);
        }

        Ok(Arc::new(builder.finish()))
    }

    /// Create vector index for faster search
    pub async fn create_index(&self) -> Result<(), VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        info!("Creating vector index on embedding column");

        table
            .create_index(&["embedding"], Index::Auto)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to create index: {}", e)))?;

        info!("Vector index created successfully");
        Ok(())
    }

    /// Convert metadata HashMap to VectorMetadata
    fn map_to_metadata(metadata: &HashMap<String, MetadataValue>) -> VectorMetadata {
        let get_string = |key: &str| -> Option<String> {
            metadata.get(key).and_then(|v| {
                if let MetadataValue::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
        };

        let get_int = |key: &str| -> Option<usize> {
            metadata.get(key).and_then(|v| {
                if let MetadataValue::Integer(i) = v {
                    Some(*i as usize)
                } else {
                    None
                }
            })
        };

        VectorMetadata {
            path: get_string("path"),
            name: get_string("name"),
            kind: get_string("kind"),
            language: get_string("language"),
            repository: get_string("repository"),
            branch: get_string("branch"),
            line_number: get_int("line_number"),
            doc_type: get_string("doc_type"),
            extra: HashMap::new(),
        }
    }

    /// Convert RecordBatch row to VectorDocument
    fn row_to_document(batch: &RecordBatch, row_idx: usize) -> Option<VectorDocument> {
        let id_array = batch
            .column_by_name("id")?
            .as_any()
            .downcast_ref::<StringArray>()?;
        let content_array = batch
            .column_by_name("content")?
            .as_any()
            .downcast_ref::<StringArray>()?;

        let embedding_array = batch
            .column_by_name("embedding")?
            .as_any()
            .downcast_ref::<FixedSizeListArray>()?;

        let id = id_array.value(row_idx).to_string();
        let content = content_array.value(row_idx).to_string();

        // Extract embedding
        let embedding_list = embedding_array.value(row_idx);
        let embedding_floats = embedding_list.as_any().downcast_ref::<Float32Array>()?;
        let embedding: Vec<f32> = (0..embedding_floats.len())
            .map(|i| embedding_floats.value(i))
            .collect();

        // Extract metadata
        let get_string = |col: &str| -> Option<String> {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .and_then(|arr| {
                    if arr.is_null(row_idx) {
                        None
                    } else {
                        Some(arr.value(row_idx).to_string())
                    }
                })
        };

        let get_int = |col: &str| -> Option<usize> {
            batch
                .column_by_name(col)
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                .and_then(|arr| {
                    if arr.is_null(row_idx) {
                        None
                    } else {
                        Some(arr.value(row_idx) as usize)
                    }
                })
        };

        let metadata = VectorMetadata {
            path: get_string("path"),
            name: get_string("name"),
            kind: get_string("kind"),
            language: get_string("language"),
            repository: get_string("repository"),
            branch: get_string("branch"),
            line_number: get_int("line_number"),
            doc_type: get_string("doc_type"),
            extra: HashMap::new(),
        };

        Some(VectorDocument::with_metadata(
            id, embedding, content, metadata,
        ))
    }

    /// Build filter SQL from HashMap
    fn build_filter_sql(filter: &HashMap<String, MetadataValue>) -> Option<String> {
        if filter.is_empty() {
            return None;
        }

        let conditions: Vec<String> = filter
            .iter()
            .map(|(key, value)| match value {
                MetadataValue::String(s) => format!("{} = '{}'", key, s.replace("'", "''")),
                MetadataValue::Integer(i) => format!("{} = {}", key, i),
                MetadataValue::Float(f) => format!("{} = {}", key, f),
                MetadataValue::Boolean(b) => format!("{} = {}", key, b),
                MetadataValue::List(list) => {
                    let items: Vec<String> = list
                        .iter()
                        .map(|s| format!("'{}'", s.replace("'", "''")))
                        .collect();
                    format!("{} IN ({})", key, items.join(", "))
                }
            })
            .collect();

        Some(conditions.join(" AND "))
    }

    /// Helper to collect RecordBatch stream results
    async fn collect_stream(
        stream: impl futures::Stream<Item = Result<RecordBatch, lancedb::error::Error>> + Send,
    ) -> Result<Vec<RecordBatch>, VectorError> {
        use futures::StreamExt;
        let mut results = Vec::new();
        let mut stream = std::pin::pin!(stream);

        while let Some(batch_result) = stream.next().await {
            let batch = batch_result
                .map_err(|e| VectorError::DatabaseError(format!("Failed to read batch: {}", e)))?;
            results.push(batch);
        }

        Ok(results)
    }
}

#[async_trait]
impl VectorStore for LanceStore {
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

        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        // First, delete existing documents with same IDs
        let ids: Vec<String> = documents.iter().map(|d| d.id.clone()).collect();
        for id in &ids {
            let filter = format!("id = '{}'", id.replace("'", "''"));
            if let Err(e) = table.delete(&filter).await {
                warn!("Failed to delete existing document {}: {}", id, e);
            }
        }

        // Convert to RecordBatch
        let batch = self.documents_to_batch(&documents)?;
        let batches = vec![Ok(batch)];
        let iter = RecordBatchIterator::new(batches, self.schema.clone());

        // Add documents
        table
            .add(Box::new(iter))
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to add documents: {}", e)))?;

        info!("Stored {} documents in LanceDB", documents.len());

        Ok(documents.len())
    }

    async fn search(&self, query: Vec<f32>, k: usize) -> Result<Vec<SearchResult>, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        // Perform vector search
        let results = Self::collect_stream(
            table
                .query()
                .nearest_to(query)
                .map_err(|e| {
                    VectorError::DatabaseError(format!("Failed to create vector query: {}", e))
                })?
                .limit(k)
                .execute()
                .await
                .map_err(|e| {
                    VectorError::DatabaseError(format!("Failed to execute search: {}", e))
                })?,
        )
        .await?;

        let mut search_results = Vec::new();

        for batch in results {
            // Get distance column if present
            let distances: Vec<f32> = batch
                .column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>())
                .map(|arr| {
                    (0..arr.len())
                        .map(|i| if arr.is_null(i) { 0.0 } else { arr.value(i) })
                        .collect()
                })
                .unwrap_or_default();

            for row_idx in 0..batch.num_rows() {
                if let Some(doc) = Self::row_to_document(&batch, row_idx) {
                    let score = distances
                        .get(row_idx)
                        .map(|d| 1.0 - (d / 2.0).min(1.0).max(0.0)) // Convert distance to similarity
                        .unwrap_or(1.0);

                    search_results.push(SearchResult {
                        id: doc.id.clone(),
                        score,
                        metadata: doc.metadata_to_map(),
                        content: Some(doc.content.clone()),
                    });
                }
            }
        }

        Ok(search_results)
    }

    async fn search_with_filter(
        &self,
        query: Vec<f32>,
        k: usize,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        // Build query with filter
        let mut query_builder = table
            .query()
            .nearest_to(query)
            .map_err(|e| {
                VectorError::DatabaseError(format!("Failed to create vector query: {}", e))
            })?
            .limit(k);

        if let Some(filter_sql) = Self::build_filter_sql(&filter) {
            query_builder = query_builder.only_if(&filter_sql);
        }

        let results =
            Self::collect_stream(query_builder.execute().await.map_err(|e| {
                VectorError::DatabaseError(format!("Failed to execute search: {}", e))
            })?)
            .await?;

        let mut search_results = Vec::new();

        for batch in results {
            let distances: Vec<f32> = batch
                .column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>())
                .map(|arr| {
                    (0..arr.len())
                        .map(|i| if arr.is_null(i) { 0.0 } else { arr.value(i) })
                        .collect()
                })
                .unwrap_or_default();

            for row_idx in 0..batch.num_rows() {
                if let Some(doc) = Self::row_to_document(&batch, row_idx) {
                    let score = distances
                        .get(row_idx)
                        .map(|d| 1.0 - (d / 2.0).min(1.0).max(0.0))
                        .unwrap_or(1.0);

                    search_results.push(SearchResult {
                        id: doc.id.clone(),
                        score,
                        metadata: doc.metadata_to_map(),
                        content: Some(doc.content.clone()),
                    });
                }
            }
        }

        Ok(search_results)
    }

    async fn get(&self, id: &str) -> Result<Option<VectorDocument>, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        let filter = format!("id = '{}'", id.replace("'", "''"));

        let results = Self::collect_stream(
            table
                .query()
                .only_if(&filter)
                .execute()
                .await
                .map_err(|e| {
                    VectorError::DatabaseError(format!("Failed to execute query: {}", e))
                })?,
        )
        .await?;

        for batch in results {
            if batch.num_rows() > 0 {
                return Ok(Self::row_to_document(&batch, 0));
            }
        }

        Ok(None)
    }

    async fn delete(&self, id: &str) -> Result<bool, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        // Check if document exists first
        let exists = self.get(id).await?.is_some();

        if exists {
            let filter = format!("id = '{}'", id.replace("'", "''"));
            table.delete(&filter).await.map_err(|e| {
                VectorError::DatabaseError(format!("Failed to delete document: {}", e))
            })?;

            info!("Deleted document: {}", id);
        }

        Ok(exists)
    }

    async fn delete_by_filter(
        &self,
        filter: HashMap<String, MetadataValue>,
    ) -> Result<usize, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        // Count documents before deletion
        let count_before = self.count().await?;

        if let Some(filter_sql) = Self::build_filter_sql(&filter) {
            table.delete(&filter_sql).await.map_err(|e| {
                VectorError::DatabaseError(format!("Failed to delete documents: {}", e))
            })?;
        }

        // Count after deletion
        let count_after = self.count().await?;

        let deleted = count_before.saturating_sub(count_after);
        if deleted > 0 {
            info!("Deleted {} documents matching filter", deleted);
        }

        Ok(deleted)
    }

    async fn count(&self) -> Result<usize, VectorError> {
        let table = self
            .conn
            .open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| VectorError::DatabaseError(format!("Failed to open table: {}", e)))?;

        let mut stream = table.query().execute().await.map_err(|e| {
            VectorError::DatabaseError(format!("Failed to execute count query: {}", e))
        })?;

        let mut count = 0usize;
        use futures::StreamExt;
        while let Some(batch_result) = stream.next().await {
            let batch = batch_result
                .map_err(|e| VectorError::DatabaseError(format!("Failed to read batch: {}", e)))?;
            count += batch.num_rows();
        }

        Ok(count)
    }

    async fn health_check(&self) -> Result<bool, VectorError> {
        // Verify connection and table exist
        let _ = self.count().await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_open() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = LanceStore::open(dir.path()).await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_store_upsert_and_search() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = LanceStore::open(dir.path())
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
        let store = LanceStore::open(dir.path())
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
        let store = LanceStore::open(dir.path())
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
            let store = LanceStore::open(dir.path())
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
            let store = LanceStore::open(dir.path())
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
        let store = LanceStore::open(dir.path())
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
    async fn test_create_index() {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let store = LanceStore::open(dir.path())
            .await
            .expect("Failed to open store");

        // Add enough documents for index creation (requires at least 256 rows for PQ)
        for i in 0..300 {
            let embedding = vec![0.1 * (i as f32 / 300.0); EMBEDDING_DIMENSION];
            let mut metadata = HashMap::new();
            metadata.insert(
                "content".to_string(),
                MetadataValue::String(format!("doc {}", i)),
            );
            store
                .upsert(&format!("doc-{}", i), embedding, metadata)
                .await
                .expect("Upsert failed");
        }

        // Create index
        let result = store.create_index().await;
        if let Err(ref e) = result {
            eprintln!("Index creation error: {:?}", e);
        }
        assert!(result.is_ok(), "Index creation should succeed");
    }
}
