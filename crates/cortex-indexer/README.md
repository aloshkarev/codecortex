# cortex-indexer

Source code indexing and skeleton generation for efficient code retrieval.

## Overview

This crate provides indexing functionality for parsing source files and storing them in the graph database.

## Features

- **Multi-language Indexing**: Parse and index code from 10 languages
- **Skeleton Generation**: Compressed code views for LLM context efficiency
- **Build Detection**: Identify project build systems and dependencies
- **Parallel Processing**: Multi-threaded indexing with configurable thread pools
- **Incremental Indexing**: Hash-based change detection for efficient re-indexing
- **Git-aware Indexing**: Track changes by Git revision
- **Timeout Support**: Configurable timeouts for large repositories
- **Progress Tracking**: Real-time indexing progress callbacks

## Supported Build Systems

| Build System | Detection File |
|--------------|----------------|
| Cargo | `Cargo.toml` |
| npm | `package.json` |
| pnpm | `pnpm-lock.yaml` |
| yarn | `yarn.lock` |
| pip | `requirements.txt` |
| poetry | `pyproject.toml` |
| pipenv | `Pipfile` |
| Go modules | `go.mod` |
| CMake | `CMakeLists.txt` |
| Compile Commands | `compile_commands.json` |

## Usage

### Indexing a Repository

```rust
use cortex_indexer::{Indexer, IndexConfig};
use cortex_graph::GraphClient;

async fn index_example(client: GraphClient) -> Result<(), Box<dyn std::error::Error>> {
    let config = IndexConfig {
        timeout_secs: 60,
        batch_size: 500,
        max_files: 10000,
        ..Default::default()
    };
    let indexer = Indexer::with_config(client, config)?;
    let report = indexer.index_path("/path/to/repo").await?;
    println!("Indexed {} files in {:.2}s", report.indexed_files, report.duration_secs);
    Ok(())
}
```

### Skeleton Generation

```rust
use cortex_indexer::build_skeleton;

let code = r#"
/// A user repository for database operations.
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    /// Find a user by ID.
    pub fn find(&self, id: u64) -> Option<User> {
        self.db.query(id)
    }
}
"#;

let skeleton = build_skeleton(code);
// Returns compressed version with signatures and docstrings
```

### Parallel Processing

```rust
use cortex_indexer::{ParallelProcessor, ParallelConfig};

let config = ParallelConfig {
    num_threads: 4,
    min_batch_size: 10,
    ..Default::default()
};
let processor = ParallelProcessor::with_config(config);
// Use processor.process_parallel() for parallel file processing
```

### Incremental Indexing

```rust
use cortex_indexer::IncrementalIndexer;
use std::path::Path;

let mut indexer = IncrementalIndexer::new();
indexer.set_revision("abc123");

// Check if file changed
if indexer.has_file_changed(Path::new("src/main.rs"), &content) {
    // Re-index the file
    indexer.record_file(Path::new("src/main.rs"), &new_content);
}
```

### Git-aware Incremental Indexing

```rust
use cortex_indexer::GitAwareIncremental;
use std::path::Path;

let indexer = GitAwareIncremental::new(Path::new("/path/to/repo"));
let changed = indexer.get_uncommitted_changes(Path::new("/path/to/repo"));
for path in changed {
    // Re-index changed files
}
```

## Dependencies

- `tree-sitter` - Parsing
- `ignore` - .gitignore handling
- `rayon` - Parallel processing
- `sha2` - Content hashing
- `blake3` - Fast file hashing

## Tests

Run tests with:
```bash
cargo test -p cortex-indexer -- --test-threads=1
```

Current test count: **69 tests**

## Hash Functions

Two hash functions are available:

| Function | Algorithm | Use Case |
|----------|-----------|----------|
| `file_hash` | SHA-256 | Default, cryptographically secure |
| `file_hash_fast` | BLAKE3 | Fast hashing for large files |

```rust
use cortex_indexer::{file_hash, file_hash_fast};

let sha256_hash = file_hash("content");     // 64 hex chars
let blake3_hash = file_hash_fast("content"); // 64 hex chars, faster
```
