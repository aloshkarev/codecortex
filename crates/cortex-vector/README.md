# cortex-vector

Vector storage and embedding providers for CodeCortex hybrid search.

## Overview

This crate provides:
- `VectorStore` trait for abstracting vector database operations
- `LanceStore` implementation using LanceDB (embedded, serverless)
- `JsonStore` implementation using JSON storage (simple, for small projects)
- `Embedder` trait for generating embeddings from text
- OpenAI and Ollama embedding provider implementations
- `HybridSearch` for combining graph and vector search

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HybridSearch                           │
│  (Combines GraphClient + VectorStore for semantic search)   │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┴───────────────────┐
          ▼                                       ▼
┌─────────────────────┐               ┌─────────────────────┐
│    VectorStore      │               │    Embedder         │
│    (LanceStore)     │               │ (OpenAI / Ollama)   │
└─────────────────────┘               └─────────────────────┘
```

## Storage Options

| Store Type | Description | Best For |
|------------|-------------|----------|
| `lancedb` | Embedded LanceDB (default) | All use cases, scalable, no server needed |
| `json` | Local JSON file storage | Development, small projects (< 1k docs) |
| `qdrant` | Production vector database | Large deployments, cloud |

**Note:** LanceDB is the recommended storage backend. It's an embedded, serverless vector database that:
- Requires no additional installation or server
- Uses Apache Arrow format for efficient storage
- Supports vector indexing for fast similarity search
- Scales to millions of documents

## Usage

### Basic Vector Store with LanceDB

```rust
use cortex_vector::{LanceStore, VectorStore, MetadataValue};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open or create vector store
    let store = LanceStore::open("./vectors").await?;

    // Insert document with embedding
    let embedding = vec![0.1; 1536]; // 1536 dimensions for OpenAI
    let mut metadata = HashMap::new();
    metadata.insert("name".to_string(), MetadataValue::from("authenticate"));
    metadata.insert("file".to_string(), MetadataValue::from("auth.rs"));

    store.upsert("func-1", embedding, metadata).await?;

    // Search for similar vectors
    let query = vec![0.1; 1536];
    let results = store.search(query, 10).await?;

    for result in results {
        println!("{}: {:.2}", result.id, result.score);
    }

    Ok(())
}
```

### Create Vector Index

For better performance with large datasets, create a vector index:

```rust
// After inserting documents, create an index for faster search
store.create_index().await?;
```

### Embedding Generation

```rust
use cortex_vector::{OpenAIEmbedder, OllamaEmbedder, Embedder};

// OpenAI embeddings
let embedder = OpenAIEmbedder::new("sk-...".to_string());
let embedding = embedder.embed("fn authenticate(user: &str) -> Result<Token>").await?;

// Ollama embeddings (local)
let embedder = OllamaEmbedder::new("http://localhost:11434", "nomic-embed-text");
let embedding = embedder.embed("authentication function").await?;
```

### Hybrid Search

```rust
use cortex_vector::{HybridSearch, LanceStore, OpenAIEmbedder};
use cortex_graph::GraphClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let graph = GraphClient::connect(&config).await?;
    let store = LanceStore::open("./vectors").await?;
    let embedder = OpenAIEmbedder::new(api_key);

    let hybrid = HybridSearch::new(graph, store, embedder);

    // Semantic search with graph context
    let results = hybrid
        .search_with_context("error handling code", 10)
        .await?;

    for result in results {
        println!("{} (score: {:.2})", result.id, result.score);
        println!("  Graph context: {} callers", result.callers.len());
    }

    Ok(())
}
```

## Key Types

### VectorStore Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, id: &str, embedding: Vec<f32>, metadata: HashMap<String, MetadataValue>) -> Result<(), VectorError>;
    async fn search(&self, query: Vec<f32>, k: usize) -> Result<Vec<SearchResult>, VectorError>;
    async fn delete(&self, id: &str) -> Result<bool, VectorError>;
    async fn count(&self) -> Result<usize, VectorError>;
}
```

### Embedder Trait

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    fn dimension(&self) -> usize;
}
```

### Search Types

```rust
pub enum SearchType {
    Vector,     // Pure vector similarity
    Graph,      // Graph traversal only
    Hybrid,     // Combined vector + graph
}
```

## Embedding Providers

| Provider | Model | Dimension | Priority |
|----------|-------|-----------|----------|
| OpenAI | text-embedding-3-small | 1536 | High |
| Ollama | nomic-embed-text | 768 | High (local) |
| Cohere | embed-v3 | 1024 | Medium |

## Performance

### LanceStore (default)
- Search: < 50ms for 100k documents with index
- Embedding generation: ~50ms per text (OpenAI)
- Local embedding (Ollama): ~20ms per text
- Hybrid search: < 200ms end-to-end

### JsonStore
- Search: < 100ms for 10k documents
- No indexing (O(n) similarity)
- Good for small datasets

## Configuration

```bash
# OpenAI API key
export OPENAI_API_KEY=sk-...

# Ollama base URL (default: http://localhost:11434)
export OLLAMA_BASE_URL=http://localhost:11434
```

## Tests

Run tests with:
```bash
cargo test -p cortex-vector -- --test-threads=1
```

Current test count: **22 tests**
