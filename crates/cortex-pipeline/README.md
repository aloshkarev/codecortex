# cortex-pipeline

ECL (Extract → Cognify → Load) Pipeline for structured code processing.

## Overview

This crate provides a flexible pipeline architecture inspired by [cognee](https://github.com/topoteretes/cognee) for processing code through multiple enrichment stages.

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        Pipeline                                    │
├────────────────┬────────────────┬────────────────┬───────────────┤
│    Extract     │    Cognify     │     Embed      │     Load      │
│                │                │                │               │
│ • Parse files  │ • Extract rel. │ • Generate     │ • Store in    │
│ • Detect lang  │ • Calc metrics │   embeddings   │   graph +     │
│ • Build AST    │ • Identify sm. │ • Summarize    │   vector DB   │
└────────────────┴────────────────┴────────────────┴───────────────┘
```

## Usage

### Basic Pipeline

```rust
use cortex_pipeline::{Pipeline, PipelineContext};

// Create pipeline with default ECL stages
let pipeline = Pipeline::with_default_stages();

// Create context from a directory
let context = PipelineContext::from_directory("/path/to/code");

// Run the pipeline
let result = pipeline.run(context).await?;
```

### Custom Pipeline

```rust
use cortex_pipeline::{Pipeline, stage::{ExtractStage, CognifyStage, EmbedStage, LoadStage}};

let pipeline = Pipeline::new()
    .add_stage(ExtractStage::new()
        .with_extensions(vec!["rs".to_string()])
        .with_max_file_size(1024 * 1024))
    .add_stage(CognifyStage::new()
        .with_summarization(true))
    .add_stage(EmbedStage::new()
        .with_dimension(768))
    .add_stage(LoadStage::new()
        .with_batch_size(50));
```

### Input Types

```rust
// From a single file
let ctx = PipelineContext::from_file("src/main.rs");

// From a directory
let ctx = PipelineContext::from_directory("src/");

// From raw content
let ctx = PipelineContext::from_content(
    "test.rs".to_string(),
    "fn main() {}".to_string(),
    Some("rust".to_string()),
);
```

## Stages

### ExtractStage

Parses source files and extracts code entities:
- Detects file language from extension
- Supports 14 file extensions (rs, py, go, ts, js, etc.)
- Configurable max file size

### CognifyStage

Analyzes code and enriches entities:
- Extracts relationships between entities
- Calculates complexity metrics
- Assigns importance scores
- Optional LLM summarization

### EmbedStage

Generates vector embeddings:
- Creates embeddings from entity summaries
- Configurable embedding dimension
- Supports multiple embedding providers

### LoadStage

Stores entities in databases:
- Persists to graph database (Memgraph/Neo4j)
- Stores embeddings in vector store
- Creates relationships between entities
- Batch processing support

## Context Types

| Type | Description |
|------|-------------|
| `ExtractedEntity` | Parsed code entity with source |
| `CognifiedEntity` | Entity with relationships and metrics |
| `EmbeddedEntity` | Entity with vector embedding |
| `LoadResult` | Result of storage operations |

## Features

- **Async Processing**: All stages are async for non-blocking operation
- **Progress Tracking**: Real-time pipeline state updates
- **Error Handling**: Graceful error recovery between stages
- **Extensibility**: Implement `Stage` trait for custom stages

## Custom Stages

```rust
use cortex_pipeline::{Stage, PipelineContext, StageResult};
use async_trait::async_trait;

pub struct MyCustomStage;

#[async_trait]
impl Stage for MyCustomStage {
    fn name(&self) -> &str {
        "custom"
    }

    async fn process(&self, context: &mut PipelineContext) -> anyhow::Result<StageResult> {
        // Custom processing logic
        Ok(StageResult::success(1, 0))
    }
}
```

## Supported Languages

| Language | Extensions |
|----------|------------|
| Rust | `.rs` |
| Python | `.py` |
| Go | `.go` |
| TypeScript | `.ts`, `.tsx` |
| JavaScript | `.js`, `.jsx` |
| C | `.c`, `.h` |
| C++ | `.cpp`, `.hpp` |
| Java | `.java` |
| PHP | `.php` |
| Ruby | `.rb` |

## Tests

```bash
# Run tests
cargo test --package cortex-pipeline

# Run with output
cargo test --package cortex-pipeline -- --nocapture
```

Current test count: **33 tests**

## Dependencies

- `cortex-core` - Core types and error handling
- `cortex-graph` - Graph database client
- `cortex-parser` - Tree-sitter parsing
- `cortex-vector` - Vector store and embeddings
- `async-trait` - Async trait support
- `tracing` - Logging and instrumentation
- `walkdir` - Directory traversal
