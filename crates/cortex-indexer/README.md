# cortex-indexer

`cortex-indexer` scans repositories, parses source files, and writes graph data.

## Responsibilities

- File discovery and language routing
- Symbol/relationship extraction via parser crate
- Graph persistence through `cortex-graph`
- Build-system detection for repository metadata
- Indexing reports with counts and timing

## Example

```rust
use cortex_indexer::{Indexer, IndexConfig};
use cortex_graph::GraphClient;

async fn run(client: GraphClient) -> anyhow::Result<()> {
    let indexer = Indexer::with_config(
        client,
        IndexConfig {
            timeout_secs: 300,
            batch_size: 500,
            max_files: 0,
            ..Default::default()
        },
    )?;
    let report = indexer.index_path("/path/to/repo").await?;
    println!("Indexed {} files", report.indexed_files);
    Ok(())
}
```

## CLI usage

```bash
cortex index /path/to/repo --force
cortex list
cortex stats
```

## Test

```bash
cargo test -p cortex-indexer -- --test-threads=1
```
