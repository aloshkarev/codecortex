# cortex-vector

`cortex-vector` provides vector indexing/search and embedder integration for CodeCortex.

It is used for semantic retrieval and hybrid graph+vector workflows.

## Core pieces

- `VectorStore` trait
- `LanceStore` (default embedded vector backend)
- `JsonStore` (simple local backend)
- Embedder trait and providers
- Hybrid search integration with graph context

## Recent updates

- Hybrid search layer includes cross-repository query paths:
  - `search_across_repositories`
  - `find_similar_across_projects`
  - `search_in_repository_and_branch`
- These APIs power CLI and MCP cross-project search behavior.

## Typical flow

1. Generate embeddings for code chunks.
2. Upsert vectors with metadata.
3. Search by vector similarity.
4. Optionally combine with graph constraints for hybrid search.

## Example

```rust
use cortex_vector::{LanceStore, VectorStore, MetadataValue};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = LanceStore::open("./vectors").await?;
    let mut meta = HashMap::new();
    meta.insert("name".to_string(), MetadataValue::from("authenticate"));
    store.upsert("node-1", vec![0.1; 1536], meta).await?;
    let _hits = store.search(vec![0.1; 1536], 10).await?;
    Ok(())
}
```

## CLI entrypoints that use this crate

- `cortex vector-index`
- `cortex search`
- MCP vector tools (`vector_index_repository`, `vector_search`, etc.)

## Test

```bash
cargo test -p cortex-vector -- --test-threads=1
```
