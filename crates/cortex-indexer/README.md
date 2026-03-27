# cortex-indexer

> `cortex-indexer` scans repositories, parses source files using `cortex-parser`, and writes graph nodes and edges to the database. It supports full and incremental-diff indexing modes, content-addressed change detection, and build-system metadata extraction.

## What it does

- Discovers files in a repository tree using language routing and configurable ignore rules
- Parses each file via the `ParserRegistry` and writes `CodeNode` / `CodeEdge` records through `cortex-graph`
- Performs a post-indexing reconciliation pass to link `TYPE_REFERENCE` and `FIELD_ACCESS` edges to concrete graph nodes
- Tracks file content hashes (SHA-256) in a persistent cache for incremental re-indexing
- Detects build system metadata (Cargo.toml, package.json, go.mod, etc.) and attaches it as repository properties
- Reports indexing results with file counts, timing, and any errors

## Indexing modes

| Mode | Flag | Description |
|------|------|-------------|
| `full` | `--mode full` | Re-parse and re-index all files regardless of prior state. Use with `--force` |
| `incremental-diff` | `--mode incremental-diff` | Re-index only files changed (added/modified/deleted) since the last indexed git commit |

For `incremental-diff`, supply `--base-branch <branch>` to set the git ref to diff against. If omitted, the default branch is used.

```bash
# Full re-index
cortex index /path/to/repo --force

# Incremental re-index against main
cortex index /path/to/repo --mode incremental-diff --base-branch main
```

## Content-addressed change detection

The incremental indexer maintains a hash cache (SQLite via `sled`) at `~/.cortex/hash-cache/`:

- Each file entry stores: SHA-256 content hash, file size, modification timestamp, and the repository git revision when it was last indexed.
- On re-index, files are skipped when content hash matches — mtime optimization avoids hashing unmodified files.
- The cache is keyed by canonical file path and repository revision, so switching branches invalidates the correct entries.

The hash algorithm is **SHA-256** (via the `sha2` crate). `blake3` is available as a fast alternative for internal hashing operations.

## Type-reference reconciliation

After the main parse-and-write pass, the indexer runs a reconciliation pass:

1. Collects all `TYPE_REFERENCE` and `FIELD_ACCESS` placeholder edges emitted by parsers
2. Resolves each target name against indexed graph nodes using `qualified_name` lookups
3. Writes concrete edges, improving precision for downstream navigation queries (`go_to_definition`, `find_all_usages`, structural diff)

## Build system detection

The indexer detects and records build-system metadata from:

| File | System |
|------|--------|
| `Cargo.toml` | Rust / Cargo |
| `package.json` | Node.js / npm |
| `go.mod` | Go modules |
| `pom.xml` | Java / Maven |
| `build.gradle` | Java / Gradle |
| `pyproject.toml`, `setup.py` | Python |
| `CMakeLists.txt` | C/C++ / CMake |

## Usage

```rust
use cortex_indexer::{Indexer, IndexConfig};
use cortex_graph::GraphClient;

async fn run(client: GraphClient) -> anyhow::Result<()> {
    let indexer = Indexer::with_config(
        client,
        IndexConfig {
            timeout_secs: 300,
            batch_size: 500,
            max_files: 0,          // 0 = unlimited
            ..Default::default()
        },
    )?;
    let report = indexer.index_path("/path/to/repo").await?;
    println!("Indexed {} files in {:.2?}", report.indexed_files, report.elapsed);
    Ok(())
}
```

## CLI usage

```bash
cortex index /path/to/repo --force
cortex index /path/to/repo --mode incremental-diff --base-branch main
cortex list
cortex stats
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `CORTEX_INDEXER_BATCH_SIZE` | `500` | Graph write batch size |
| `CORTEX_INDEXER_TIMEOUT_SECS` | `300` | Indexing timeout in seconds |
| `CORTEX_INDEXER_MAX_FILES` | `0` (unlimited) | Max files per indexing run |

## Dependencies

- `cortex-parser` — Tree-sitter parsing for 14 languages
- `cortex-graph` — Graph database client
- `cortex-core` — Config and shared models
- `sha2` — SHA-256 content hashing
- `blake3` — Fast hashing for internal operations
- `sled` — Embedded hash cache storage
- `rayon` — Parallel file processing
- `ignore` — `.gitignore`-aware file discovery

## Tests

```bash
cargo test -p cortex-indexer -- --test-threads=1
```
