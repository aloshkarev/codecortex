//! Schema definitions for vector documents

use crate::{EMBEDDING_DIMENSION, MetadataValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A document with its embedding and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    /// Unique identifier for this document
    pub id: String,

    /// The embedding vector
    pub embedding: Vec<f32>,

    /// Document content (source code, documentation, etc.)
    pub content: String,

    /// Metadata associated with this document
    pub metadata: VectorMetadata,
}

impl VectorDocument {
    /// Create a new document with embedding
    pub fn new(id: String, embedding: Vec<f32>, content: String) -> Self {
        Self {
            id,
            embedding,
            content,
            metadata: VectorMetadata::default(),
        }
    }

    /// Create a document with metadata
    pub fn with_metadata(
        id: String,
        embedding: Vec<f32>,
        content: String,
        metadata: VectorMetadata,
    ) -> Self {
        Self {
            id,
            embedding,
            content,
            metadata,
        }
    }

    /// Validate the document
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Document ID cannot be empty".to_string());
        }

        if self.embedding.len() != EMBEDDING_DIMENSION {
            return Err(format!(
                "Invalid embedding dimension: expected {}, got {}",
                EMBEDDING_DIMENSION,
                self.embedding.len()
            ));
        }

        Ok(())
    }

    /// Convert metadata to a HashMap
    pub fn metadata_to_map(&self) -> HashMap<String, MetadataValue> {
        let mut map = HashMap::new();

        if let Some(ref path) = self.metadata.path {
            map.insert("path".to_string(), MetadataValue::String(path.clone()));
        }

        if let Some(ref name) = self.metadata.name {
            map.insert("name".to_string(), MetadataValue::String(name.clone()));
        }

        if let Some(ref kind) = self.metadata.kind {
            map.insert("kind".to_string(), MetadataValue::String(kind.clone()));
        }

        if let Some(ref lang) = self.metadata.language {
            map.insert("language".to_string(), MetadataValue::String(lang.clone()));
        }

        if let Some(ref repo) = self.metadata.repository {
            map.insert(
                "repository".to_string(),
                MetadataValue::String(repo.clone()),
            );
        }

        if let Some(ref branch) = self.metadata.branch {
            map.insert("branch".to_string(), MetadataValue::String(branch.clone()));
        }

        if let Some(line) = self.metadata.line_number {
            map.insert(
                "line_number".to_string(),
                MetadataValue::Integer(line as i64),
            );
        }

        if let Some(ref doc_type) = self.metadata.doc_type {
            map.insert(
                "doc_type".to_string(),
                MetadataValue::String(doc_type.clone()),
            );
        }

        map
    }
}

/// Metadata for code documents in the vector store
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorMetadata {
    /// File path relative to repository root
    pub path: Option<String>,

    /// Symbol name (for functions, classes, etc.)
    pub name: Option<String>,

    /// Entity kind (function, class, struct, etc.)
    pub kind: Option<String>,

    /// Programming language
    pub language: Option<String>,

    /// Repository path
    pub repository: Option<String>,

    /// Git branch
    pub branch: Option<String>,

    /// Line number in file
    pub line_number: Option<usize>,

    /// Document type (code, doc, test, config, etc.)
    pub doc_type: Option<String>,

    /// Additional custom metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl VectorMetadata {
    /// Create metadata for a code symbol
    pub fn code_symbol(
        path: impl Into<String>,
        name: impl Into<String>,
        kind: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            path: Some(path.into()),
            name: Some(name.into()),
            kind: Some(kind.into()),
            language: Some(language.into()),
            doc_type: Some("code".to_string()),
            ..Default::default()
        }
    }

    /// Create metadata for documentation
    pub fn documentation(path: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            path: Some(path.into()),
            language: Some(language.into()),
            doc_type: Some("doc".to_string()),
            ..Default::default()
        }
    }

    /// Set repository information
    pub fn with_repository(mut self, repo: impl Into<String>, branch: impl Into<String>) -> Self {
        self.repository = Some(repo.into());
        self.branch = Some(branch.into());
        self
    }

    /// Set line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }

    /// Add extra metadata
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_document_creation() {
        let doc = VectorDocument::new(
            "test-1".to_string(),
            vec![0.0; EMBEDDING_DIMENSION],
            "fn test() {}".to_string(),
        );

        assert_eq!(doc.id, "test-1");
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_vector_document_invalid_dimension() {
        let doc = VectorDocument::new(
            "test-1".to_string(),
            vec![0.0; 100], // Wrong dimension
            "fn test() {}".to_string(),
        );

        assert!(doc.validate().is_err());
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = VectorMetadata::code_symbol("src/main.rs", "main", "function", "rust")
            .with_repository("/home/user/project", "main")
            .with_line(10);

        assert_eq!(metadata.path, Some("src/main.rs".to_string()));
        assert_eq!(metadata.name, Some("main".to_string()));
        assert_eq!(metadata.kind, Some("function".to_string()));
        assert_eq!(metadata.language, Some("rust".to_string()));
        assert_eq!(metadata.repository, Some("/home/user/project".to_string()));
        assert_eq!(metadata.branch, Some("main".to_string()));
        assert_eq!(metadata.line_number, Some(10));
        assert_eq!(metadata.doc_type, Some("code".to_string()));
    }

    #[test]
    fn test_metadata_to_map() {
        let metadata =
            VectorMetadata::code_symbol("src/main.rs", "main", "function", "rust").with_line(10);

        let doc = VectorDocument::with_metadata(
            "test-1".to_string(),
            vec![0.0; EMBEDDING_DIMENSION],
            "fn main() {}".to_string(),
            metadata,
        );

        let map = doc.metadata_to_map();

        assert!(map.contains_key("path"));
        assert!(map.contains_key("name"));
        assert!(map.contains_key("line_number"));
    }
}
