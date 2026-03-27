# cortex-vector

> `cortex-vector` provides vector indexing, semantic search, and embedding generation for CodeCortex. It supports multiple vector backends (LanceDB, Qdrant, JSON) and two embedding providers (OpenAI, Ollama) for hybrid graph+vector retrieval workflows.

## What it does

- Stores code chunk embeddings in a local LanceDB store, a Qdrant cluster, or a simple JSON file store
- Generates embeddings via OpenAI (`text-embedding-3-small`) or locally via Ollama (`nomic-embed-text`, `bge-m3`, and others)
- Powers hybrid search by combining vector similarity results with graph-level constraints
- Exposes cross-repository search paths used by CLI and MCP cross-project tools

## Vector store backends

| Backend | Type | `store_type` value | Use case |
|---------|------|--------------------|---------|
| `LanceStore` | Embedded (default) | `lancedb` | Local development, no external service |
| `JsonStore` | File-based | `json` | Simple persistence, debugging, testing |
| `QdrantStore` | External service | `qdrant` | Production, large-scale, multi-tenant |

All backends implement the `VectorStore` trait:

```rust
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, id: &str, vector: Vec<f32>, metadata: HashMap<String, MetadataValue>) -> anyhow::Result<()>;
    async fn search(&self, query: Vec<f32>, limit: usize) -> anyhow::Result<Vec<SearchResult>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
    async fn count(&self) -> anyhow::Result<usize>;
}
```

## Embedding providers

| Provider | Type | Default model | Dimension | Config key |
|----------|------|--------------|-----------|-----------|
| `OpenAIEmbedder` | Remote API | `text-embedding-3-small` | 1536 | `llm.provider = "openai"` |
| `OllamaEmbedder` | Local service | `nomic-embed-text` | 1536 (padded) | `llm.provider = "ollama"` |

### OpenAI embedder

- Default model: `text-embedding-3-small` (1536 dimensions)
- Supports custom models via `OpenAIEmbedder::with_model`
- Supports Azure/proxy base URL via `OpenAIEmbedder::with_base_url`
- Batch size: up to 100 texts per request
- Reads API key from `OPENAI_API_KEY` via `OpenAIEmbedder::from_env()`

### Ollama embedder

- Default model: `nomic-embed-text`
- Supports any Ollama-hosted model (e.g., `bge-m3`, `mxbai-embed-large`)
- `bge-m3` profile: native 1024 dimensions, automatic query prefix (`"Represent this sentence for searching relevant passages: "`)
- Automatic context-length retry with middle-truncation on long inputs
- Configurable via env vars: `CORTEX_OLLAMA_MAX_INPUT_CHARS`, `CORTEX_OLLAMA_TARGET_DIMENSION`, `CORTEX_OLLAMA_ENABLE_BGE_QUERY_PREFIX`

## Cross-repository search paths

The hybrid layer exposes:

- `search_across_repositories(query, repositories)` — semantic search filtered to a list of repos
- `find_similar_across_projects(symbol, projects)` — similarity search for a symbol across projects
- `search_in_repository_and_branch(query, repo, branch)` — branch-scoped semantic search

These APIs power `search_across_projects` (MCP) and `cortex search` (CLI) in cross-project mode.

## Typical workflow

```
1. Generate embeddings for code chunks via OpenAI or Ollama
2. Upsert vectors with metadata (file, name, language, repo)
3. Search by vector similarity for nearest neighbors
4. Optionally apply graph constraints (repository, branch, entity kind) for hybrid search
```

## LanceDB example

```rust
use cortex_vector::{LanceStore, VectorStore, MetadataValue};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = LanceStore::open("./vectors").await?;

    // Upsert
    let mut meta = HashMap::new();
    meta.insert("name".to_string(), MetadataValue::from("authenticate"));
    meta.insert("file".to_string(), MetadataValue::from("src/auth.rs"));
    store.upsert("node-1", vec![0.1; 1536], meta).await?;

    // Search
    let hits = store.search(vec![0.1; 1536], 10).await?;
    for hit in &hits {
        println!("Score: {:.4}  id: {}", hit.score, hit.id);
    }

    Ok(())
}
```

## OpenAI embedder example

```rust
use cortex_vector::{Embedder, OpenAIEmbedder};

let embedder = OpenAIEmbedder::from_env()?; // reads OPENAI_API_KEY
let embedding = embedder.embed("fn authenticate(user: &str) -> Result<Token>").await?;
println!("Dimension: {}", embedding.len()); // 1536
```

## Ollama embedder example

```rust
use cortex_vector::OllamaEmbedder;

let embedder = OllamaEmbedder::with_model("bge-m3");
let embedding = embedder.embed_query("find authentication logic").await?;
```

## CLI and MCP entry points

| Operation | CLI | MCP tool |
|-----------|-----|---------|
| Index repository | `cortex vector-index <path>` | `vector_index_repository` |
| Index single file | — | `vector_index_file` |
| Semantic search | `cortex search <query>` | `vector_search` |
| Hybrid search | — | `vector_search_hybrid` |
| Cross-project search | — | `search_across_projects` |
| Index status | — | `vector_index_status` |
| Delete index | — | `vector_delete_repository` |

## Configuration

In `~/.cortex/config.toml`:

```toml
[vector]
store_type = "lancedb"          # "lancedb" | "json" | "qdrant"
store_path = "~/.cortex/vectors"
qdrant_uri = "http://127.0.0.1:6333"
embedding_dim = 1536

[llm]
provider = "openai"             # "openai" | "ollama" | "none"
openai_api_key = "sk-..."
openai_embedding_model = "text-embedding-3-small"
ollama_base_url = "http://127.0.0.1:11434"
ollama_embedding_model = "nomic-embed-text"
```

## Tests

```bash
cargo test -p cortex-vector -- --test-threads=1
```
