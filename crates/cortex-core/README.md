# cortex-core

> `cortex-core` provides the shared contracts, configuration, and data models used by every other crate in the CodeCortex workspace — including `CortexConfig`, `CortexError`, `CodeNode`, `CodeEdge`, `EdgeKind`, language detection, and complexity helpers.

## What it does

- Defines the canonical graph node and edge types (`CodeNode`, `CodeEdge`) that all crates exchange
- Provides a unified `CortexConfig` loaded from `~/.cortex/config.toml` with environment variable overrides
- Exposes a single `CortexError` / `Result` type used across the workspace
- Computes cyclomatic and cognitive complexity from source text
- Detects language from file extension via `Language::from_path`

## Node and edge model

### Node labels

| Label | Description |
|-------|-------------|
| `Repository` | Root repository node |
| `Directory` | Directory in the repository tree |
| `File` | Source file |
| `Function` | Function or free function |
| `Method` | Associated or member method |
| `Class` | Class definition |
| `Struct` | Struct definition |
| `Enum` | Enum definition |
| `Trait` | Trait or interface definition |
| `Module` | Module or namespace |

### Edge kinds (`EdgeKind`)

| Variant | Cypher label | Meaning |
|---------|-------------|---------|
| `Contains` | `CONTAINS` | Hierarchical containment (repo→dir→file→symbol) |
| `Calls` | `CALLS` | Function/method call |
| `Imports` | `IMPORTS` | Import or `use` statement |
| `Inherits` | `INHERITS` | Class or type inheritance |
| `Implements` | `IMPLEMENTS` | Trait or interface implementation |
| `MemberOf` | `MEMBER_OF` | Member-to-parent type relationship |
| `TypeReference` | `TYPE_REFERENCE` | Type used in a type-position (parameter, return, field) |
| `FieldAccess` | `FIELD_ACCESS` | Field access expression |

`MemberOf`, `TypeReference`, and `FieldAccess` are emitted by the parser for languages that have sufficient grammar coverage (Rust, Python, TypeScript, Go, Java) and are used by navigation flows (`go_to_definition`, `find_all_usages`, structural diff).

## Configuration

`CortexConfig` is loaded from `~/.cortex/config.toml`. Environment variables override file values at runtime.

### Core fields

| Field | Env override | Default | Description |
|-------|-------------|---------|-------------|
| `memgraph_uri` | `CORTEX_MEMGRAPH_URI` | `bolt://127.0.0.1:7687` | Graph database URI |
| `memgraph_user` | `CORTEX_MEMGRAPH_USER` | `memgraph` | Graph database user |
| `memgraph_password` | `CORTEX_MEMGRAPH_PASSWORD` | `memgraph` | Graph database password |
| `backend_type` | `CORTEX_BACKEND_TYPE` | `memgraph` | Backend: `memgraph` or `neo4j` |
| `max_batch_size` | `CORTEX_INDEXER_BATCH_SIZE` | `500` | Graph write batch size |
| `indexer_timeout_secs` | `CORTEX_INDEXER_TIMEOUT_SECS` | `300` | Indexer timeout (seconds) |
| `indexer_max_files` | `CORTEX_INDEXER_MAX_FILES` | `0` (unlimited) | Max files per indexing run |
| `analyzer_query_limit` | `CORTEX_ANALYZER_QUERY_LIMIT` | `1000` | Max rows per analyzer query |
| `analyzer_cache_ttl_secs` | `CORTEX_ANALYZER_CACHE_TTL_SECS` | `300` | Analyzer cache TTL (seconds) |
| `watcher_debounce_secs` | `CORTEX_WATCHER_DEBOUNCE_SECS` | `2` | File event debounce delay |
| `watcher_max_events` | `CORTEX_WATCHER_MAX_EVENTS` | `128` | Max queued file events |
| `pool_max_connections` | `CORTEX_POOL_MAX_CONNECTIONS` | `10` | DB connection pool size |
| `pool_min_idle` | `CORTEX_POOL_MIN_IDLE` | `2` | Minimum idle connections |
| `pool_connection_timeout_secs` | `CORTEX_POOL_TIMEOUT_SECS` | `30` | Connection acquire timeout |

### Vector sub-config (`vector.*`)

| Field | Default | Description |
|-------|---------|-------------|
| `vector.store_type` | `lancedb` | `lancedb`, `json`, or `qdrant` |
| `vector.store_path` | `~/.cortex/vectors` | Local vector storage path |
| `vector.qdrant_uri` | `http://127.0.0.1:6333` | Qdrant server URI |
| `vector.qdrant_api_key` | — | Qdrant API key (optional) |
| `vector.embedding_dim` | `1536` | Embedding dimension |

### LLM sub-config (`llm.*`)

| Field | Default | Description |
|-------|---------|-------------|
| `llm.provider` | `none` | `openai`, `ollama`, or `none` |
| `llm.openai_api_key` | — | OpenAI API key |
| `llm.openai_embedding_model` | `text-embedding-3-small` | OpenAI embedding model |
| `llm.ollama_base_url` | `http://127.0.0.1:11434` | Ollama server URL |
| `llm.ollama_embedding_model` | `nomic-embed-text` | Ollama embedding model |

### Minimal config.toml example

```toml
memgraph_uri = "memgraph://127.0.0.1:7687"
memgraph_user = ""
memgraph_password = ""
backend_type = "memgraph"
max_batch_size = 500

[llm]
provider = "openai"
openai_api_key = "sk-..."
```

## Supported languages

`Language::from_path` detects: Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell (14 total).

## Complexity helpers

```rust
use cortex_core::{compute_cyclomatic_complexity, Language};
use std::path::Path;

// Language detection
let lang = Language::from_path(Path::new("src/main.rs"));
assert_eq!(lang, Some(Language::Rust));

// Cyclomatic complexity
let c = compute_cyclomatic_complexity("fn f() { if ok { a() } else { b() } }");
assert_eq!(c, 2);
```

Complexity counts branching constructs: `if`, `else if`, `for`, `while`, `loop`, `match` arms, `&&`, `||`, `?`, `catch`.

## Tests

```bash
cargo test -p cortex-core -- --test-threads=1
```
