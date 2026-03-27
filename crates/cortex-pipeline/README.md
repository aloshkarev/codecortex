# cortex-pipeline

> `cortex-pipeline` implements an ECL (Extract → Cognify → Embed → Load) pipeline for enriched code processing in CodeCortex. Inspired by the [cognee](https://github.com/topoteretes/cognee) knowledge enrichment approach, it processes code through configurable stages that parse, analyze, embed, and persist entities to both graph and vector stores in a single pass.

## What it does

- Orchestrates multi-stage code processing: parsing → relationship extraction → embedding generation → graph+vector persistence
- Accepts input as a directory, single file, or raw content string
- Supports optional LLM summarization in the Cognify stage for richer embeddings
- Tracks progress and errors per stage with graceful recovery between stages
- Provides a `Stage` trait for custom processing steps

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Pipeline                              │
├────────────────┬───────────────┬──────────────┬─────────────┤
│    Extract     │    Cognify    │    Embed     │    Load     │
│                │               │              │             │
│ • Parse files  │ • Extract rel.│ • Generate   │ • Store in  │
│ • Detect lang  │ • Calc metrics│   embeddings │   graph +   │
│ • Build AST    │ • Importance  │ • Summarize  │   vector DB │
│                │   scoring     │   (optional) │             │
└────────────────┴───────────────┴──────────────┴─────────────┘
```

## Stages

### ExtractStage

Parses source files and produces `ExtractedEntity` records:

- Auto-detects language from file extension using `cortex-core`'s `Language::from_path`
- Supports all 14 languages (Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell)
- Configurable max file size (default unlimited)
- Uses `walkdir` for directory traversal with `.gitignore` awareness

### CognifyStage

Enriches extracted entities and produces `CognifiedEntity` records:

- Extracts relationships between entities (calls, imports, inheritance)
- Calculates cyclomatic complexity and assigns importance scores
- Optional LLM summarization (requires configured `llm.provider` in `CortexConfig`)

### EmbedStage

Generates vector embeddings and produces `EmbeddedEntity` records:

- Creates embeddings from entity summaries or source text
- Configurable embedding dimension (default 1536)
- Supports both OpenAI and Ollama providers via `cortex-vector`

### LoadStage

Persists to both graph and vector databases and returns `LoadResult`:

- Writes graph nodes and edges to Memgraph/Neo4j/Neptune via `cortex-graph`
- Writes embeddings to LanceDB/Qdrant/JSON via `cortex-vector`
- Creates relationships between co-located entities
- Batch-processes writes for efficiency (configurable batch size)

## Stage data types

| Type | Produced by | Description |
|------|------------|-------------|
| `ExtractedEntity` | `ExtractStage` | Parsed code entity with source text |
| `CognifiedEntity` | `CognifyStage` | Entity with relationships, metrics, and importance score |
| `EmbeddedEntity` | `EmbedStage` | Entity with vector embedding |
| `LoadResult` | `LoadStage` | Storage operation result with counts |

## Usage

### Basic pipeline (default ECL stages)

```rust
use cortex_pipeline::{Pipeline, PipelineContext};

let pipeline = Pipeline::with_default_stages();
let context = PipelineContext::from_directory("/path/to/code");
let result = pipeline.run(context).await?;
println!("Loaded {} entities", result.entities_loaded);
```

### Custom pipeline

```rust
use cortex_pipeline::{Pipeline, stage::{ExtractStage, CognifyStage, EmbedStage, LoadStage}};

let pipeline = Pipeline::new()
    .add_stage(ExtractStage::new()
        .with_extensions(vec!["rs".to_string()])
        .with_max_file_size(1024 * 1024))
    .add_stage(CognifyStage::new()
        .with_summarization(true))
    .add_stage(EmbedStage::new()
        .with_dimension(1536))
    .add_stage(LoadStage::new()
        .with_batch_size(50));
```

### Input types

```rust
// From a directory
let ctx = PipelineContext::from_directory("src/");

// From a single file
let ctx = PipelineContext::from_file("src/main.rs");

// From raw content
let ctx = PipelineContext::from_content(
    "test.rs".to_string(),
    "fn main() {}".to_string(),
    Some("rust".to_string()),
);
```

### Custom stage

```rust
use cortex_pipeline::{Stage, PipelineContext, StageResult};
use async_trait::async_trait;

pub struct MyCustomStage;

#[async_trait]
impl Stage for MyCustomStage {
    fn name(&self) -> &str { "custom" }

    async fn process(&self, context: &mut PipelineContext) -> anyhow::Result<StageResult> {
        // Custom processing logic
        Ok(StageResult::success(1, 0))
    }
}
```

## Supported languages

| Language | Extensions |
|----------|-----------|
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
| Kotlin | `.kt`, `.kts` |
| Swift | `.swift` |
| JSON | `.json` |
| Shell | `.sh`, `.bash`, `.zsh` |

## Dependencies

- `cortex-core` — Config, errors, language detection
- `cortex-graph` — Graph database client
- `cortex-parser` — Tree-sitter parsing
- `cortex-vector` — Vector store and embeddings
- `async-trait` — Async trait support for `Stage`
- `walkdir` — Directory traversal
- `tracing` — Logging and instrumentation

## Tests

```bash
cargo test --package cortex-pipeline
cargo test --package cortex-pipeline -- --nocapture
```

Current test count: **33 tests**
