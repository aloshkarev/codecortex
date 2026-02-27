# CodeCortex

CodeCortex is a Rust code-graph service and AI productivity toolkit featuring:

- **`cortex` CLI** for indexing, search, and analysis
- **MCP server mode** for AI clients (Claude, Cursor, VSCode)
- **Memgraph backend** (Bolt protocol) with Neo4j support
- **Multi-language parsing** via tree-sitter (Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby)

## Installation

### Quick Install

```bash
# One-line install
curl -fsSL https://raw.githubusercontent.com/aloshkarev/codecortex/main/quickstart.sh | bash
```

### Using Make

```bash
# Clone and setup
git clone https://github.com/aloshkarev/codecortex.git
cd codecortex

# Full setup (build, install, start Memgraph)
make setup

# Or step by step
make release        # Build release binary
make install        # Install to ~/.local/bin
make run-memgraph   # Start Memgraph with Docker
```

### Manual Install

See [docs/INSTALL.md](docs/INSTALL.md) for comprehensive installation guide covering:
- macOS (Intel and Apple Silicon)
- Ubuntu/Debian Linux
- Memgraph setup (Docker and native)
- MCP service configuration
- IDE integration

## Architecture

| Crate | Tests | Description |
|-------|-------|-------------|
| `cortex-core` | 100 | Core data models, language detection, complexity analysis |
| `cortex-parser` | 26 | Tree-sitter parsing for 10 languages, signature extraction |
| `cortex-graph` | 53 | Graph database client (Memgraph/Neo4j), node/edge operations |
| `cortex-indexer` | 56 | File indexing, parallel processing, incremental indexing |
| `cortex-analyzer` | 61 | Code analysis queries, code smells, coupling, cohesion |
| `cortex-watcher` | 60 | File watching with filtering and performance tuning |
| `cortex-mcp` | 137 | MCP server with 41 AI productivity tools |
| `cortex-cli` | - | Command-line interface with interactive mode |
| `cortex-benches` | - | Performance benchmarks |

## Prerequisites

- Rust (stable toolchain)
- Docker / Docker Compose
- Memgraph reachable at Bolt URI (default `bolt://127.0.0.1:7687`)

## Quickstart

1. Start Memgraph locally:

```bash
docker compose up -d
# Or use make
make run-memgraph
```

2. Build and install:

```bash
make setup
```

3. Verify installation:

```bash
cortex --version
cortex doctor
```

4. Index a repository:

```bash
cortex index /path/to/your/code
```

5. Run example queries:

```bash
cortex find name GraphClient
cortex analyze callers index_path
cortex analyze complexity --top 10
cortex stats
```

## MCP Mode

Start the MCP server over stdio:

```bash
./target/release/cortex-cli mcp start
```

The setup wizard can generate an `mcp.json` snippet for Cursor/VSCode.

### MCP Tools (41 total)

**Indexing & Repository Management:**
- `add_code_to_graph` - Index a directory or file
- `watch_directory` / `unwatch_directory` - File watching
- `index_status` - Index health check
- `list_indexed_repositories` / `delete_repository`
- `list_watched_paths`
- `load_bundle` / `export_bundle` - Bundle import/export
- `get_repository_stats`

**Code Search & Retrieval:**
- `get_context_capsule` - Hybrid retrieval (TF-IDF + graph centrality)
- `find_code` - Symbol search
- `get_skeleton` - Code skeleton/compressed view
- `get_signature` - Symbol signature lookup
- `execute_cypher_query` - Direct graph queries

**Impact & Dependency Analysis:**
- `get_impact_graph` - Blast radius analysis
- `search_logic_flow` - Multi-path finding
- `analyze_code_relationships` - Relationship exploration
- `find_dead_code` - Unused code detection
- `calculate_cyclomatic_complexity` - Complexity metrics

**AI Productivity:**
- `find_tests` - Find related tests
- `explain_result` - Query explanation
- `analyze_refactoring` - Refactoring guidance
- `diagnose` - System diagnostics
- `find_patterns` - Architectural pattern detection

**Project Management:**
- `list_projects` - List registered projects
- `add_project` / `remove_project` - Project registry
- `set_current_project` / `get_current_project` - Active project context
- `list_branches` - Git branch listing
- `refresh_project` - Refresh Git info

**Memory & Session:**
- `save_observation` / `search_memory` / `get_session_context`
- `workspace_setup` - Workspace bootstrap

**Advanced:**
- `submit_lsp_edges` - LSP edge ingestion
- `check_health` / `check_job_status` / `list_jobs`

## Testing

Run unit tests:

```bash
cargo test --workspace
```

Live graph tests (requires running Memgraph):

```bash
cargo test -p cortex-graph --test live_test -- --nocapture
```

Docker testcontainers integration tests:

```bash
cargo test -p cortex-graph --test integration_test -- --ignored --nocapture
```

## Benchmarks

```bash
cargo bench
```

Benchmark categories:
- **TF-IDF scoring** - Document tokenization and scoring
- **Impact graph** - Blast radius computation
- **Context capsule** - Hybrid retrieval performance
- **Indexing throughput** - File processing speed

## Supported Languages

| Language | Extensions | Features |
|----------|------------|----------|
| Rust | `.rs` | Full support |
| Python | `.py` | Full support |
| Go | `.go` | Full support |
| TypeScript | `.ts`, `.tsx` | Full support |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | Full support |
| C | `.c`, `.h` | Full support + compile_commands.json |
| C++ | `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx` | Full support + compile_commands.json |
| Java | `.java` | Full support |
| PHP | `.php` | Full support |
| Ruby | `.rb` | Full support |

## Graph Schema

**Nodes:**
- `Repository`, `Directory`, `File`
- `Function`, `Struct`, `Enum`, `Trait`
- `Class`, `Interface`
- `Parameter`, `Variable`, `Module`

**Edges:**
- `CONTAINS` - Hierarchical containment
- `CALLS` - Function calls
- `IMPORTS` - Module imports
- `INHERITS` / `IMPLEMENTS` - Type relationships
- `HAS_PARAMETER` - Function parameters
- `DEFINED_IN` - Definition location

## Performance SLOs

| Tool | p50 | p95 | Timeout |
|------|-----|-----|---------|
| `get_context_capsule` | 600ms | 2500ms | 8s |
| `get_impact_graph` | 500ms | 2200ms | 8s |
| `search_logic_flow` | 700ms | 3000ms | 10s |
| `get_skeleton` | 50ms | 200ms | 2s |
| `search_memory` | 350ms | 1500ms | 6s |

## Quality Gates

- **Retrieval**: Recall@20 >= 0.85, nDCG@20 >= 0.78
- **Graph**: PathValidity = 1.0, PathCompleteness >= 0.8
- **Memory**: Staleness accuracy >= 0.9, Link resolution >= 0.95
- **Skeleton**: Signature retention >= 0.98, Compression >= 0.7

## Development

```bash
# Format code
cargo fmt --all

# Run lints
cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
cargo test --workspace

# Build documentation
cargo doc --no-deps --open
```

## License

MIT
