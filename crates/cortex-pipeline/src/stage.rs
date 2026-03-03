//! Pipeline stages for ECL processing.
//!
//! This module provides the core `Stage` trait and built-in implementations for
//! Extract, Cognify, Embed, and Load stages.

use crate::context::{CognifiedEntity, ExtractedEntity, PipelineContext};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{info, instrument};

/// Result of a stage execution
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Number of items processed
    pub processed_count: usize,
    /// Number of items skipped
    pub skipped_count: usize,
    /// Stage-specific metrics
    pub metrics: HashMap<String, f64>,
    /// Any warnings generated
    pub warnings: Vec<String>,
}

impl StageResult {
    /// Create a successful stage result
    pub fn success(processed: usize, skipped: usize) -> Self {
        Self {
            processed_count: processed,
            skipped_count: skipped,
            metrics: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Add a metric
    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// A processing stage in the ECL pipeline.
///
/// Stages are executed in order and can transform the context
/// between Extract → Cognify → Embed → Load.
#[async_trait]
pub trait Stage: Send + Sync {
    /// Returns the name of this stage
    fn name(&self) -> &str;

    /// Execute this stage
    async fn process(&self, context: &mut PipelineContext) -> Result<StageResult>;
}

/// Extract Stage: Parse source files into entities.
///
/// This stage:
/// - Detects file language
/// - Parses code using tree-sitter
/// - Extracts functions, classes, modules
/// - Builds initial entity graph
pub struct ExtractStage {
    /// Supported file extensions
    extensions: Vec<String>,
    /// Maximum file size to process (bytes)
    max_file_size: usize,
}

impl ExtractStage {
    /// Create a new Extract stage
    pub fn new() -> Self {
        Self {
            extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "go".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "js".to_string(),
                "jsx".to_string(),
                "c".to_string(),
                "h".to_string(),
                "cpp".to_string(),
                "hpp".to_string(),
                "java".to_string(),
                "php".to_string(),
                "rb".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }

    /// Set supported extensions
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Set maximum file size
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }
}

impl Default for ExtractStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for ExtractStage {
    fn name(&self) -> &str {
        "extract"
    }

    #[instrument(skip(self, context), fields(input = ?context.input))]
    async fn process(&self, context: &mut PipelineContext) -> Result<StageResult> {
        let mut result = StageResult::success(0, 0);
        info!("Starting extract stage");

        // Process based on input type
        match &context.input {
            crate::context::PipelineInput::File(path) => {
                let entities = self.extract_file(path)?;
                context.extracted.extend(entities);
                result.processed_count = context.extracted.len();
            }
            crate::context::PipelineInput::Directory(path) => {
                let entities = self.extract_directory(path)?;
                context.extracted.extend(entities);
                result.processed_count = context.extracted.len();
            }
            crate::context::PipelineInput::Content {
                path,
                content,
                language,
            } => {
                let entity = self.extract_content(path, content, language.as_deref())?;
                context.extracted.push(entity);
                result.processed_count = 1;
            }
        }

        info!(
            processed = result.processed_count,
            skipped = result.skipped_count,
            "Extract stage complete"
        );
        Ok(result)
    }
}

impl ExtractStage {
    fn extract_file(&self, path: &std::path::Path) -> Result<Vec<ExtractedEntity>> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > self.max_file_size as u64 {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !self.extensions.iter().any(|e| e == ext) {
            return Ok(vec![]);
        }

        let entity = self.extract_content(&path.display().to_string(), &content, Some(ext))?;
        Ok(vec![entity])
    }

    fn extract_directory(&self, path: &std::path::Path) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();
        for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
            if entry.file_type().is_file()
                && let Ok(file_entities) = self.extract_file(entry.path())
            {
                entities.extend(file_entities);
            }
        }
        Ok(entities)
    }

    fn extract_content(
        &self,
        path: &str,
        content: &str,
        language: Option<&str>,
    ) -> Result<ExtractedEntity> {
        let id = format!("entity:{}", path);
        let entity_type = "module".to_string();
        let name = path.split('/').next_back().unwrap_or("unknown").to_string();

        let mut metadata = HashMap::new();
        if let Some(l) = language {
            metadata.insert("language".to_string(), serde_json::json!(l));
        }

        Ok(ExtractedEntity {
            id,
            entity_type,
            name,
            path: path.to_string(),
            line: Some(1),
            source: Some(content.to_string()),
            docstring: None,
            metadata,
        })
    }
}

/// Cognify Stage: Extract relationships and metrics.
///
/// This stage:
/// - Analyzes code relationships (calls, imports)
/// - Calculates complexity metrics
/// - Identifies code smells
/// - Assigns importance scores
pub struct CognifyStage {
    /// Enable LLM-based summarization
    enable_summarization: bool,
}

impl CognifyStage {
    /// Create a new Cognify stage
    pub fn new() -> Self {
        Self {
            enable_summarization: false,
        }
    }

    /// Enable LLM summarization
    pub fn with_summarization(mut self, enable: bool) -> Self {
        self.enable_summarization = enable;
        self
    }
}

impl Default for CognifyStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for CognifyStage {
    fn name(&self) -> &str {
        "cognify"
    }

    #[instrument(skip(self, context), fields(extracted_count = context.extracted.len()))]
    async fn process(&self, context: &mut PipelineContext) -> Result<StageResult> {
        let mut result = StageResult::success(0, 0);
        info!(count = context.extracted.len(), "Starting cognify stage");

        for entity in &context.extracted {
            let cognified = self.cognify_entity(entity)?;
            context.cognified.push(cognified);
            result.processed_count += 1;
        }

        info!(processed = result.processed_count, "Cognify stage complete");
        Ok(result)
    }
}

impl CognifyStage {
    fn cognify_entity(&self, entity: &ExtractedEntity) -> Result<CognifiedEntity> {
        // Calculate basic metrics
        let metrics = crate::context::EntityMetrics {
            cyclomatic_complexity: self.calculate_complexity(entity),
            lines_of_code: entity
                .source
                .as_ref()
                .map(|s| s.lines().count())
                .unwrap_or(0) as u32,
            parameter_count: 0, // Would extract from source
            nesting_depth: self.calculate_nesting(entity),
        };

        // Calculate importance based on metrics
        let importance = self.calculate_importance(&metrics);

        Ok(CognifiedEntity {
            extracted: entity.clone(),
            relationships: vec![], // Would extract from source
            metrics,
            importance,
            summary: if self.enable_summarization {
                Some(format!("Summary of {}", entity.name))
            } else {
                None
            },
        })
    }

    fn calculate_complexity(&self, entity: &ExtractedEntity) -> u32 {
        entity
            .source
            .as_ref()
            .map(|s| {
                s.matches("if").count()
                    + s.matches("for").count()
                    + s.matches("while").count()
                    + s.matches("match").count()
                    + 1
            })
            .unwrap_or(1) as u32
    }

    fn calculate_nesting(&self, entity: &ExtractedEntity) -> u32 {
        // Simplified nesting calculation
        entity
            .source
            .as_ref()
            .map(|s| {
                let mut max_depth: u32 = 0;
                let mut current_depth: u32 = 0;
                for c in s.chars() {
                    if c == '{' {
                        current_depth += 1;
                        max_depth = max_depth.max(current_depth);
                    } else if c == '}' {
                        current_depth = current_depth.saturating_sub(1);
                    }
                }
                max_depth
            })
            .unwrap_or(0)
    }

    fn calculate_importance(&self, metrics: &crate::context::EntityMetrics) -> f64 {
        // Higher complexity and deeper nesting -> higher importance
        let complexity_factor = metrics.cyclomatic_complexity.min(20) as f64 / 20.0;
        let nesting_factor = metrics.nesting_depth.min(10) as f64 / 10.0;
        let lines_factor = (metrics.lines_of_code as f64 / 100.0).min(1.0);

        (complexity_factor * 0.4 + nesting_factor * 0.3 + lines_factor * 0.3).min(1.0)
    }
}

/// Embed Stage: Generate vector embeddings.
///
/// This stage:
/// - Generates embeddings for entity summaries
/// - Creates vector representations
/// - Stores embedding metadata
pub struct EmbedStage {
    /// Embedding dimension
    dimension: usize,
}

impl EmbedStage {
    /// Create a new Embed stage
    pub fn new() -> Self {
        Self { dimension: 1536 }
    }

    /// Set embedding dimension
    pub fn with_dimension(mut self, dim: usize) -> Self {
        self.dimension = dim;
        self
    }
}

impl Default for EmbedStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for EmbedStage {
    fn name(&self) -> &str {
        "embed"
    }

    #[instrument(skip(self, context), fields(cognified_count = context.cognified.len()))]
    async fn process(&self, context: &mut PipelineContext) -> Result<StageResult> {
        let mut result = StageResult::success(0, 0);
        info!(count = context.cognified.len(), "Starting embed stage");

        for entity in &context.cognified {
            let embedding = self.generate_embedding(entity)?;
            context.embedded.push(crate::context::EmbeddedEntity {
                cognified: entity.clone(),
                embedding,
                embedding_model: "hash-v1".to_string(),
            });
            result.processed_count += 1;
        }

        info!(processed = result.processed_count, "Embed stage complete");
        Ok(result)
    }
}

impl EmbedStage {
    fn generate_embedding(&self, entity: &CognifiedEntity) -> Result<Vec<f32>> {
        // Placeholder: In production, would call embedding provider
        // For now, generate a simple hash-based embedding
        let text = format!(
            "{} {} {}",
            entity.extracted.name,
            entity.extracted.entity_type,
            entity.summary.as_deref().unwrap_or("")
        );

        let mut embedding = vec![0.0f32; self.dimension];
        for (i, c) in text.chars().enumerate() {
            embedding[i % self.dimension] = (c as u32) as f32 / 255.0;
        }

        Ok(embedding)
    }
}

/// Load Stage: Store entities in graph and vector databases.
///
/// This stage:
/// - Persists entities to the graph database
/// - Stores embeddings in vector store
/// - Creates relationships between entities
pub struct LoadStage {
    /// Batch size for loading
    batch_size: usize,
}

impl LoadStage {
    /// Create a new Load stage
    pub fn new() -> Self {
        Self { batch_size: 100 }
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
}

impl Default for LoadStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for LoadStage {
    fn name(&self) -> &str {
        "load"
    }

    #[instrument(skip(self, context), fields(embedded_count = context.embedded.len()))]
    async fn process(&self, context: &mut PipelineContext) -> Result<StageResult> {
        let mut result = StageResult::success(0, 0);
        info!(count = context.embedded.len(), "Starting load stage");

        // Process in batches
        for chunk in context.embedded.chunks(self.batch_size) {
            let loaded = self.load_batch(chunk)?;
            result.processed_count += loaded;
        }

        // Mark pipeline as complete
        context.complete = true;

        info!(processed = result.processed_count, "Load stage complete");
        Ok(result)
    }
}

impl LoadStage {
    fn load_batch(&self, _batch: &[crate::context::EmbeddedEntity]) -> Result<usize> {
        // Placeholder: In production, would:
        // 1. Store entities in graph DB
        // 2. Store embeddings in vector DB
        // 3. Create relationships
        Ok(_batch.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_result_success() {
        let result = StageResult::success(10, 2);
        assert_eq!(result.processed_count, 10);
        assert_eq!(result.skipped_count, 2);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn stage_result_with_metrics() {
        let result = StageResult::success(5, 0).with_metric("latency_ms", 42.0);
        assert_eq!(result.metrics.get("latency_ms"), Some(&42.0));
    }

    #[test]
    fn stage_result_with_warnings() {
        let result = StageResult::success(5, 1)
            .with_warning("Skipped large file".to_string())
            .with_warning("Invalid encoding".to_string());
        assert_eq!(result.warnings.len(), 2);
        assert!(result.warnings[0].contains("Skipped"));
    }

    #[test]
    fn extract_stage_creation() {
        let stage = ExtractStage::new()
            .with_extensions(vec!["rs".to_string()])
            .with_max_file_size(1024);
        assert_eq!(stage.name(), "extract");
        assert_eq!(stage.extensions, vec!["rs"]);
        assert_eq!(stage.max_file_size, 1024);
    }

    #[test]
    fn extract_stage_default_extensions() {
        let stage = ExtractStage::new();
        assert!(stage.extensions.contains(&"rs".to_string()));
        assert!(stage.extensions.contains(&"py".to_string()));
        assert!(stage.extensions.contains(&"go".to_string()));
        assert!(stage.extensions.contains(&"ts".to_string()));
        assert!(stage.extensions.contains(&"js".to_string()));
    }

    #[test]
    fn cognify_stage_creation() {
        let stage = CognifyStage::new().with_summarization(true);
        assert_eq!(stage.name(), "cognify");
        assert!(stage.enable_summarization);
    }

    #[test]
    fn cognify_stage_default_no_summarization() {
        let stage = CognifyStage::new();
        assert!(!stage.enable_summarization);
    }

    #[test]
    fn embed_stage_creation() {
        let stage = EmbedStage::new().with_dimension(768);
        assert_eq!(stage.name(), "embed");
        assert_eq!(stage.dimension, 768);
    }

    #[test]
    fn embed_stage_default_dimension() {
        let stage = EmbedStage::new();
        assert_eq!(stage.dimension, 1536);
    }

    #[test]
    fn load_stage_creation() {
        let stage = LoadStage::new().with_batch_size(50);
        assert_eq!(stage.name(), "load");
        assert_eq!(stage.batch_size, 50);
    }

    #[test]
    fn load_stage_default_batch_size() {
        let stage = LoadStage::new();
        assert_eq!(stage.batch_size, 100);
    }

    #[tokio::test]
    async fn extract_stage_process() {
        let stage = ExtractStage::new();
        let mut context = PipelineContext::from_content(
            "test.rs".to_string(),
            "fn main() {}".to_string(),
            Some("rs".to_string()),
        );
        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 1);
    }

    #[tokio::test]
    async fn extract_stage_language_detection() {
        let stage = ExtractStage::new();

        // Test Rust
        let mut ctx1 = PipelineContext::from_content(
            "test.rs".into(),
            "fn main() {}".into(),
            Some("rs".into()),
        );
        stage.process(&mut ctx1).await.unwrap();
        assert_eq!(
            ctx1.extracted[0].metadata.get("language").unwrap(),
            &serde_json::json!("rs")
        );

        // Test Python
        let mut ctx2 = PipelineContext::from_content(
            "test.py".into(),
            "def main(): pass".into(),
            Some("py".into()),
        );
        stage.process(&mut ctx2).await.unwrap();
        assert_eq!(
            ctx2.extracted[0].metadata.get("language").unwrap(),
            &serde_json::json!("py")
        );
    }

    #[tokio::test]
    async fn cognify_stage_process() {
        let stage = CognifyStage::new();
        let mut context = PipelineContext::default();
        context.extracted.push(ExtractedEntity {
            id: "test".to_string(),
            entity_type: "function".to_string(),
            name: "main".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some("fn main() {}".to_string()),
            docstring: None,
            metadata: HashMap::new(),
        });
        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 1);
        assert!(!context.cognified.is_empty());
    }

    #[tokio::test]
    async fn cognify_stage_complexity_calculation() {
        let stage = CognifyStage::new();
        let mut context = PipelineContext::default();

        // Simple function
        context.extracted.push(ExtractedEntity {
            id: "simple".to_string(),
            entity_type: "function".to_string(),
            name: "simple".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some("fn simple() {}".to_string()),
            docstring: None,
            metadata: HashMap::new(),
        });

        // Complex function with control flow
        context.extracted.push(ExtractedEntity {
            id: "complex".to_string(),
            entity_type: "function".to_string(),
            name: "complex".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some(
                "fn complex() { if x { for i in 0..10 { match y { _ => {} } } } }".to_string(),
            ),
            docstring: None,
            metadata: HashMap::new(),
        });

        stage.process(&mut context).await.unwrap();

        let simple = context
            .cognified
            .iter()
            .find(|e| e.extracted.id == "simple")
            .unwrap();
        let complex = context
            .cognified
            .iter()
            .find(|e| e.extracted.id == "complex")
            .unwrap();

        assert!(complex.metrics.cyclomatic_complexity > simple.metrics.cyclomatic_complexity);
    }

    #[tokio::test]
    async fn cognify_stage_nesting_calculation() {
        let stage = CognifyStage::new();
        let mut context = PipelineContext::default();

        // Deep nesting
        context.extracted.push(ExtractedEntity {
            id: "nested".to_string(),
            entity_type: "function".to_string(),
            name: "nested".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some("fn nested() { { { { {} } } } }".to_string()),
            docstring: None,
            metadata: HashMap::new(),
        });

        stage.process(&mut context).await.unwrap();

        let nested = &context.cognified[0];
        assert!(nested.metrics.nesting_depth >= 4);
    }

    #[tokio::test]
    async fn cognify_stage_importance_calculation() {
        let stage = CognifyStage::new();
        let mut context = PipelineContext::default();

        // Large, complex function
        context.extracted.push(ExtractedEntity {
            id: "important".to_string(),
            entity_type: "function".to_string(),
            name: "important".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some(
                "fn important() {\n    if a { if b { if c { for i in 0..100 { if d {} } } } }\n}"
                    .to_string(),
            ),
            docstring: None,
            metadata: HashMap::new(),
        });

        stage.process(&mut context).await.unwrap();

        let important = &context.cognified[0];
        assert!(important.importance > 0.0);
        assert!(important.importance <= 1.0);
    }

    #[tokio::test]
    async fn cognify_stage_with_summarization() {
        let stage = CognifyStage::new().with_summarization(true);
        let mut context = PipelineContext::default();
        context.extracted.push(ExtractedEntity {
            id: "test".to_string(),
            entity_type: "function".to_string(),
            name: "test_func".to_string(),
            path: "test.rs".to_string(),
            line: Some(1),
            source: Some("fn test_func() {}".to_string()),
            docstring: None,
            metadata: HashMap::new(),
        });

        stage.process(&mut context).await.unwrap();

        assert!(context.cognified[0].summary.is_some());
        assert!(
            context.cognified[0]
                .summary
                .as_ref()
                .unwrap()
                .contains("test_func")
        );
    }

    #[tokio::test]
    async fn embed_stage_process() {
        let stage = EmbedStage::new().with_dimension(128);
        let mut context = PipelineContext::default();
        context.cognified.push(CognifiedEntity {
            extracted: ExtractedEntity {
                id: "test".to_string(),
                entity_type: "function".to_string(),
                name: "main".to_string(),
                path: "test.rs".to_string(),
                line: Some(1),
                source: None,
                docstring: None,
                metadata: HashMap::new(),
            },
            relationships: vec![],
            metrics: crate::context::EntityMetrics {
                cyclomatic_complexity: 1,
                lines_of_code: 5,
                parameter_count: 0,
                nesting_depth: 1,
            },
            importance: 0.5,
            summary: None,
        });
        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 1);
        assert_eq!(context.embedded[0].embedding.len(), 128);
    }

    #[tokio::test]
    async fn embed_stage_embedding_values() {
        let stage = EmbedStage::new().with_dimension(64);
        let mut context = PipelineContext::default();
        context.cognified.push(CognifiedEntity {
            extracted: ExtractedEntity {
                id: "test".to_string(),
                entity_type: "function".to_string(),
                name: "main".to_string(),
                path: "test.rs".to_string(),
                line: Some(1),
                source: None,
                docstring: None,
                metadata: HashMap::new(),
            },
            relationships: vec![],
            metrics: crate::context::EntityMetrics {
                cyclomatic_complexity: 1,
                lines_of_code: 5,
                parameter_count: 0,
                nesting_depth: 1,
            },
            importance: 0.5,
            summary: None,
        });
        stage.process(&mut context).await.unwrap();

        let embedding = &context.embedded[0].embedding;
        // Values should be normalized (0.0 to 1.0)
        for &val in embedding {
            assert!(
                val >= 0.0 && val <= 1.0,
                "Embedding value {} out of range",
                val
            );
        }
    }

    #[tokio::test]
    async fn load_stage_process() {
        let stage = LoadStage::new().with_batch_size(10);
        let mut context = PipelineContext::default();

        // Add some embedded entities
        for i in 0..5 {
            context.embedded.push(crate::context::EmbeddedEntity {
                cognified: CognifiedEntity {
                    extracted: ExtractedEntity {
                        id: format!("test-{}", i),
                        entity_type: "function".to_string(),
                        name: format!("func_{}", i),
                        path: "test.rs".to_string(),
                        line: Some(i),
                        source: None,
                        docstring: None,
                        metadata: HashMap::new(),
                    },
                    relationships: vec![],
                    metrics: crate::context::EntityMetrics {
                        cyclomatic_complexity: 1,
                        lines_of_code: 5,
                        parameter_count: 0,
                        nesting_depth: 1,
                    },
                    importance: 0.5,
                    summary: None,
                },
                embedding: vec![0.0; 64],
                embedding_model: "test".to_string(),
            });
        }

        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 5);
        assert!(context.complete);
    }

    #[tokio::test]
    async fn load_stage_batch_processing() {
        let stage = LoadStage::new().with_batch_size(2);
        let mut context = PipelineContext::default();

        // Add more entities than batch size
        for i in 0..10 {
            context.embedded.push(crate::context::EmbeddedEntity {
                cognified: CognifiedEntity {
                    extracted: ExtractedEntity {
                        id: format!("test-{}", i),
                        entity_type: "function".to_string(),
                        name: format!("func_{}", i),
                        path: "test.rs".to_string(),
                        line: Some(i),
                        source: None,
                        docstring: None,
                        metadata: HashMap::new(),
                    },
                    relationships: vec![],
                    metrics: crate::context::EntityMetrics {
                        cyclomatic_complexity: 1,
                        lines_of_code: 5,
                        parameter_count: 0,
                        nesting_depth: 1,
                    },
                    importance: 0.5,
                    summary: None,
                },
                embedding: vec![0.0; 64],
                embedding_model: "test".to_string(),
            });
        }

        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 10);
    }

    #[tokio::test]
    async fn load_stage_empty_input() {
        let stage = LoadStage::new();
        let mut context = PipelineContext::default();
        // No embedded entities

        let result = stage.process(&mut context).await.unwrap();
        assert_eq!(result.processed_count, 0);
        assert!(context.complete);
    }
}
