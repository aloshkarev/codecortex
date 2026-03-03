//! Pipeline context for passing data between stages.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Input source for the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineInput {
    /// Process a single file
    File(PathBuf),
    /// Process a directory
    Directory(PathBuf),
    /// Process raw content
    Content {
        path: String,
        content: String,
        language: Option<String>,
    },
}

/// Extracted entity from the Extract stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Unique identifier
    pub id: String,
    /// Entity type (function, class, module, etc.)
    pub entity_type: String,
    /// Entity name
    pub name: String,
    /// Source file path
    pub path: String,
    /// Line number
    pub line: Option<u32>,
    /// Source code
    pub source: Option<String>,
    /// Documentation
    pub docstring: Option<String>,
    /// Extracted metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Cognified entity with relationships and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognifiedEntity {
    /// Base extracted entity
    pub extracted: ExtractedEntity,
    /// Relationships to other entities
    pub relationships: Vec<EntityRelationship>,
    /// Code metrics
    pub metrics: EntityMetrics,
    /// Importance score (0.0 - 1.0)
    pub importance: f64,
    /// Semantic summary (optional, LLM-generated)
    pub summary: Option<String>,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    /// Relationship type (calls, imports, inherits, etc.)
    pub rel_type: String,
    /// Target entity ID
    pub target_id: String,
    /// Relationship metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Code metrics for an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic_complexity: u32,
    /// Lines of code
    pub lines_of_code: u32,
    /// Number of parameters
    pub parameter_count: u32,
    /// Nesting depth
    pub nesting_depth: u32,
}

/// Embedded entity with vector representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedEntity {
    /// Base cognified entity
    pub cognified: CognifiedEntity,
    /// Vector embedding
    pub embedding: Vec<f32>,
    /// Embedding model used
    pub embedding_model: String,
}

/// Load result from storing entities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadResult {
    /// Entities stored in graph
    pub graph_entities: usize,
    /// Entities stored in vector store
    pub vector_entities: usize,
    /// Relationships created
    pub relationships: usize,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Pipeline context passed between stages
#[derive(Debug)]
pub struct PipelineContext {
    /// Input source
    pub input: PipelineInput,
    /// Extracted entities
    pub extracted: Vec<ExtractedEntity>,
    /// Cognified entities
    pub cognified: Vec<CognifiedEntity>,
    /// Embedded entities
    pub embedded: Vec<EmbeddedEntity>,
    /// Load result
    pub loaded: LoadResult,
    /// Stage outputs (for custom data)
    pub stage_outputs: HashMap<String, serde_json::Value>,
    /// Processing start time
    pub started_at: std::time::Instant,
    /// Whether pipeline is complete
    pub complete: bool,
}

impl PipelineContext {
    /// Create context from a file path
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            input: PipelineInput::File(path.into()),
            extracted: Vec::new(),
            cognified: Vec::new(),
            embedded: Vec::new(),
            loaded: LoadResult::default(),
            stage_outputs: HashMap::new(),
            started_at: std::time::Instant::now(),
            complete: false,
        }
    }

    /// Create context from a directory path
    pub fn from_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            input: PipelineInput::Directory(path.into()),
            extracted: Vec::new(),
            cognified: Vec::new(),
            embedded: Vec::new(),
            loaded: LoadResult::default(),
            stage_outputs: HashMap::new(),
            started_at: std::time::Instant::now(),
            complete: false,
        }
    }

    /// Create context from raw content
    pub fn from_content(path: String, content: String, language: Option<String>) -> Self {
        Self {
            input: PipelineInput::Content {
                path,
                content,
                language,
            },
            extracted: Vec::new(),
            cognified: Vec::new(),
            embedded: Vec::new(),
            loaded: LoadResult::default(),
            stage_outputs: HashMap::new(),
            started_at: std::time::Instant::now(),
            complete: false,
        }
    }

    /// Get processing duration
    pub fn duration(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Store stage output
    pub fn set_output(&mut self, stage_name: &str, value: serde_json::Value) {
        self.stage_outputs.insert(stage_name.to_string(), value);
    }

    /// Get stage output
    pub fn get_output(&self, stage_name: &str) -> Option<&serde_json::Value> {
        self.stage_outputs.get(stage_name)
    }
}

impl Default for PipelineContext {
    fn default() -> Self {
        Self {
            input: PipelineInput::Directory(PathBuf::from(".")),
            extracted: Vec::new(),
            cognified: Vec::new(),
            embedded: Vec::new(),
            loaded: LoadResult::default(),
            stage_outputs: HashMap::new(),
            started_at: std::time::Instant::now(),
            complete: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_from_file() {
        let ctx = PipelineContext::from_file("/path/to/file.rs");
        assert!(matches!(ctx.input, PipelineInput::File(_)));
    }

    #[test]
    fn context_from_directory() {
        let ctx = PipelineContext::from_directory("/path/to/repo");
        assert!(matches!(ctx.input, PipelineInput::Directory(_)));
    }

    #[test]
    fn context_from_content() {
        let ctx = PipelineContext::from_content(
            "test.rs".to_string(),
            "fn main() {}".to_string(),
            Some("rust".to_string()),
        );
        assert!(matches!(ctx.input, PipelineInput::Content { .. }));
    }

    #[test]
    fn stage_outputs() {
        let mut ctx = PipelineContext::default();
        ctx.set_output("test_stage", serde_json::json!({"key": "value"}));
        assert!(ctx.get_output("test_stage").is_some());
        assert!(ctx.get_output("nonexistent").is_none());
    }

    #[test]
    fn extracted_entity_serialization() {
        let entity = ExtractedEntity {
            id: "func:main".to_string(),
            entity_type: "function".to_string(),
            name: "main".to_string(),
            path: "src/main.rs".to_string(),
            line: Some(1),
            source: Some("fn main() {}".to_string()),
            docstring: None,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("func:main"));
    }

    #[test]
    fn cognified_entity_with_metrics() {
        let entity = CognifiedEntity {
            extracted: ExtractedEntity {
                id: "func:test".to_string(),
                entity_type: "function".to_string(),
                name: "test".to_string(),
                path: "test.rs".to_string(),
                line: Some(1),
                source: None,
                docstring: None,
                metadata: HashMap::new(),
            },
            relationships: vec![],
            metrics: EntityMetrics {
                cyclomatic_complexity: 3,
                lines_of_code: 10,
                parameter_count: 2,
                nesting_depth: 1,
            },
            importance: 0.8,
            summary: Some("Test function".to_string()),
        };
        assert_eq!(entity.metrics.cyclomatic_complexity, 3);
    }
}
