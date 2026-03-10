# CodeCortex

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/aloshkarev/codecortex)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)

CodeCortex is a Rust-native code intelligence platform for local repositories and AI-agent workflows.

It combines:
- Graph indexing (symbols + relationships)
- Code analysis (call graph, complexity, smells, refactoring guidance)
- Hybrid retrieval (graph + vector)
- MCP server mode for AI assistants
- CLI workflows for developers and CI

## What You Get

- `cortex` CLI for indexing, querying, diagnostics, and interactive exploration
- MCP server with 46 tools for code-aware AI workflows
- Memgraph/Neo4j graph backend support
- Tree-sitter parsing for 10 languages
- Optional vector search with LanceDB/Qdrant
- Memory subsystem for persistent engineering observations

## Who This Is For

- Teams that want local/self-hosted code intelligence
- AI-heavy development workflows where symbol-level context matters
- Codebases where plain text search is not enough
- Engineers who want both CLI and MCP access to the same indexed data

## High-Level Architecture

```mermaid
flowchart LR
    A[Source Code Repos] --> B[cortex-indexer]
    B --> C[(Memgraph / Neo4j)]
    B --> D[(Vector Store)]

    E[cortex-cli] --> C
    E --> D

    F[cortex-mcp] --> C
    F --> D

    C --> G[cortex-analyzer]
    D --> G
```

## End-to-End Workflow

1. Start graph backend (usually Memgraph)
2. Build/install CodeCortex
3. Configure connection in `~/.cortex/config.toml`
4. Index repository with `cortex index <path>`
5. Query via CLI (`find`, `analyze`, `query`, `search`)
6. Expose the same indexed context to AI assistants via `cortex mcp start`

## Installation

### Fast Path

```bash
curl -fsSL https://raw.githubusercontent.com/aloshkarev/codecortex/main/quickstart.sh | bash
```

### Build from Source

```bash
git clone https://github.com/aloshkarev/codecortex.git
cd codecortex

cargo build --release -p cortex-cli
# binary: target/release/cortex-cli
```

### Make-Based Setup

```bash
make setup
```

### Full Install Docs

See [docs/INSTALL.md](docs/INSTALL.md).

## Prerequisites

- Rust stable toolchain
- Memgraph 3.x (recommended) or Neo4j-compatible backend
- Docker (recommended for local Memgraph)

## Quick Start (Memgraph + CLI)

```bash
# 1) Start Memgraph
docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph-mage:3.8.1

# 2) Configure connection (example)
cat > ~/.cortex/config.toml <<'CFG'
memgraph_uri = "memgraph://127.0.0.1:7687"
memgraph_user = ""
memgraph_password = ""
backend_type = "memgraph"
CFG

# 3) Verify runtime
cortex doctor

# 4) Index a repo
cortex index /path/to/repo --force

# 5) Query
cortex find name GraphClient
cortex analyze callers index_path
cortex query "MATCH (n:CodeNode) RETURN count(n) AS c"
```

Note: If your binary is not symlinked as `cortex`, run `target/release/cortex-cli`.

## Configuration Essentials

Typical keys in `~/.cortex/config.toml`:

```toml
memgraph_uri = "memgraph://127.0.0.1:7687"
memgraph_user = ""
memgraph_password = ""
backend_type = "memgraph" # or "neo4j"

max_batch_size = 500
indexer_timeout_secs = 300
indexer_max_files = 0
```

## CLI Usage Guide

Run full help:

```bash
cortex --help
```

Primary command families:

- **Indexing/Repo:** `index`, `list`, `delete`, `stats`, `watch`, `unwatch`
- **Search/Analysis:** `find`, `analyze`, `query`, `skeleton`, `signature`, `patterns`
- **AI Context:** `capsule`, `impact`, `refactor`, `test`, `diagnose`, `memory`
- **Vector:** `vector-index`, `search` (semantic/hybrid code search)
- **Project:** `project` (list, add, set-current, branches, refresh, status, sync, queue-status, metrics)
- **MCP:** `mcp start`, `mcp tools`
- **Ops:** `doctor`, `config`, `jobs`, `debug`, `daemon`, `completion`, `interactive`

### Interactive REPL

```bash
cortex interactive
```

Use `help` inside REPL to list supported interactive commands.

## MCP Mode (AI Assistant Integration)

Start MCP server over stdio:

```bash
cortex mcp start
```

List available tools:

```bash
cortex mcp tools
```

### Example MCP client config (Cursor/VSCode style)

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start"]
    }
  }
}
```

For Cursor, Claude, Zed, and other clients see [docs/INTEGRATION.md](docs/INTEGRATION.md).

### MCP Tool Coverage (46 tools)

- **Index/repository:** `add_code_to_graph`, `list_indexed_repositories`, `delete_repository`, `get_repository_stats`
- **Vector:** `vector_index_repository`, `vector_index_file`, `vector_search`, `vector_search_hybrid`, `vector_index_status`, `vector_delete_repository`
- **Search/analysis:** `find_code`, `get_skeleton`, `get_signature`, `analyze_code_relationships`, `find_dead_code`, `calculate_cyclomatic_complexity`, `analyze_refactoring`, `find_patterns`, `find_tests`
- **Context/impact:** `get_context_capsule`, `get_impact_graph`, `search_logic_flow`
- **Health/diagnostics:** `check_health`, `index_status`, `diagnose`, `explain_result`
- **Project management:** `list_projects`, `add_project`, `remove_project`, `set_current_project`, `get_current_project`, `list_branches`, `refresh_project`, `project_status`, `project_sync`, `project_branch_diff`, `project_queue_status`, `project_metrics`
- **Watch/jobs:** `watch_directory`, `unwatch_directory`, `list_watched_paths`, `check_job_status`, `list_jobs`
- **Memory:** `save_observation`, `get_session_context`, `search_memory`
- **Bundles/LSP:** `load_bundle`, `export_bundle`, `submit_lsp_edges`, `workspace_setup`
- **Advanced:** `execute_cypher_query`

## Crate-by-Crate Documentation

The workspace contains 11 crates. This section explains role, key APIs, and how each crate is used in practice.

| Crate | Purpose | Typically Used By |
|---|---|---|
| `cortex-core` | Shared types, config, errors, language/complexity helpers | All crates |
| `cortex-parser` | Tree-sitter parsing and signature extraction | `cortex-indexer`, `cortex-analyzer`, `cortex-mcp` |
| `cortex-graph` | Graph client, schema/migrations, query helpers, bundle I/O | `cortex-indexer`, `cortex-cli`, `cortex-mcp`, `cortex-analyzer` |
| `cortex-indexer` | Repository scanning, entity/edge extraction, graph write pipeline | `cortex-cli`, `cortex-mcp` |
| `cortex-analyzer` | Analysis queries, smell detection, coupling/cohesion, refactoring suggestions | `cortex-cli`, `cortex-mcp` |
| `cortex-watcher` | FS watching, debounce/filtering, project registry | `cortex-cli`, `cortex-mcp` |
| `cortex-vector` | Vector store abstraction + embedding providers + hybrid search | `cortex-cli`, `cortex-mcp` |
| `cortex-pipeline` | ECL pipeline (Extract → Cognify → Embed → Load) | Advanced programmatic flows |
| `cortex-mcp` | MCP server/tool router over CodeCortex primitives | AI assistants |
| `cortex-cli` | User-facing command runner and output formatting | Developers/CI |
| `cortex-benches` | Criterion benchmarks for retrieval/cache/impact workloads | Performance validation |

### `cortex-core`

- Defines `CodeNode`, `CodeEdge`, `Language`, `CortexConfig`, `CortexError`
- Provides language detection and complexity utilities
- Acts as stable contract layer between crates

### `cortex-parser`

- Parses Rust, Python, Go, TS/JS, C/C++, Java, PHP, Ruby
- Produces entities/signatures consumed by indexer/analyzer
- Tree-sitter based for fast, structured extraction

### `cortex-graph`

- Connects to Memgraph/Neo4j via Bolt
- Manages schema/indexes and query execution
- Supports graph bundle export/import (`.ccx` style)

### `cortex-indexer`

- Walks repository files and build metadata
- Extracts symbols/edges and writes to graph
- Supports force/incremental style flows and reports indexing metrics

### `cortex-analyzer`

- Query helpers: callers/callees/call chains/dependencies/dead code
- Smell detection modules: bloaters, couplers, dispensables, etc.
- Refactoring engine maps smells → recommended techniques with priority

### `cortex-watcher`

- Watches directories recursively
- Debounces noisy FS event streams
- Integrates project/branch context for re-index triggers

### `cortex-vector`

- `VectorStore` abstraction (`lancedb`, `json`, optional `qdrant` path)
- Embedders (OpenAI/Ollama)
- Hybrid graph+vector search entry points

### `cortex-pipeline`

- Structured processing pipeline:
  - Extract (parse)
  - Cognify (relationships/metrics)
  - Embed (vectorization)
  - Load (graph/vector persistence)

### `cortex-mcp`

- Exposes 46 MCP tools (see [crates/cortex-mcp/src/lib.rs](crates/cortex-mcp/src/lib.rs) for categories)
- Uses `cortex-analyzer`, `cortex-indexer`, `cortex-graph`, `cortex-vector`, `cortex-watcher`
- Tool descriptions and parameter schemas guide AI agents; see [docs/INTEGRATION.md](docs/INTEGRATION.md) for best practices
- Intended integration point for Cursor, Claude, Zed, and other MCP clients

### `cortex-cli`

- Orchestrates all workspace crates; entrypoint is `cortex` (binary `cortex-cli`)
- Subcommands: `index`, `find`, `analyze`, `query`, `capsule`, `impact`, `vector-index`, `search`, `project`, `mcp start` / `mcp tools`, `doctor`, `config`, `jobs`, `daemon`, `debug`, `completion`, `interactive`, and more
- Supports JSON/YAML/table output for scripts and CI
- Includes `doctor` and `debug` for operations and troubleshooting

### `cortex-benches`

- Benchmarks capsule retrieval, impact graph, cache, TF-IDF/hybrid paths
- Useful for regression/perf budgets before release

## Graph Model (Conceptual)

Common nodes:

- `Repository`, `Directory`, `File`
- `Function`, `Struct`, `Enum`, `Trait`, `Class`, `Interface`, `Module`

Common edges:

- `CONTAINS`, `CALLS`, `IMPORTS`
- `INHERITS`, `IMPLEMENTS`
- parameter/definition relationships

## Language Support

- Rust
- Python
- Go
- TypeScript
- JavaScript
- C
- C++
- Java
- PHP
- Ruby

## Operational Playbook

### Re-index after major refactor

```bash
cortex index /path/to/repo --force
```

### Verify graph health

```bash
cortex doctor
cortex query "MATCH (n:CodeNode) RETURN count(n) AS c"
```

### Validate MCP surface

```bash
cortex mcp tools
```

### Vector workflow

```bash
cortex vector-index /path/to/repo --force
cortex search "how auth middleware handles token refresh"
```

### Measure ROI in daily work

Use the ready-to-run measurement kit to track token/time/quality impact of baseline vs Cortex workflows:

- [docs/MEASUREMENT_KIT.md](docs/MEASUREMENT_KIT.md)
- `make measure-init`
- `make measure-session-start MODE=baseline|cortex`
- `make measure-mcp-capture SESSION=<id>`
- `make measure-report`

## Comparison: CodeCortex vs `codegraphcontext` and Other OSS Solutions

This is a practical positioning comparison, not a benchmark shootout.

Comparison references (public docs):
- [CodeGraphContext docs](https://codegraphcontext.github.io/)
- [CodeGraphContext GitHub](https://github.com/Shashankss1205/CodeGraphContext)
- [Sourcegraph Cody docs](https://sourcegraph.com/docs/cody/core-concepts/code-graph)
- [OpenGrok](https://github.com/oracle/opengrok)
- [Glean](https://github.com/facebookincubator/Glean)

| Solution | Primary Focus | Data Model | AI Agent Integration | Strengths | Trade-offs |
|---|---|---|---|---|---|
| **CodeCortex** | Local/self-hosted code intelligence + MCP + CLI | Property graph + optional vectors | Native MCP server (`cortex mcp start`) | Unified indexing/analysis/MCP pipeline in one workspace; strong Rust modularity | Requires operating graph DB and indexing lifecycle |
| **CodeGraphContext** | MCP-first code graph context for assistants | Code graph DB | MCP server + CLI | Fast path to graph-backed assistant context and code relationship queries | Different scope/architecture choices; evaluate tool depth for your workflows |
| **Sourcegraph Cody** | Enterprise IDE assistant on Sourcegraph context stack | Search + code graph indexes in Sourcegraph platform | Cody clients/extensions (not MCP-first) | Mature IDE integrations and large-repo context workflows | Typically platform-centric deployment model |
| **OpenGrok** | Source browsing/search/cross-reference | Text/xref indexes (ctags + servlet stack) | No MCP-native flow | Battle-tested code search and navigation | Not designed as an MCP tool backend for agentic workflows |
| **Glean** | Large-scale language fact database/indexing | Rich fact store, language-specific schemas | Not MCP-native | Powerful semantic fact platform for large organizations | Heavier setup and steeper integration surface |

### How to Choose

Choose **CodeCortex** when you need:
- One local stack for CLI engineers and AI agents
- Graph + analysis + MCP in the same runtime
- Direct control of indexing and storage

Choose **CodeGraphContext** when you want:
- A focused MCP code graph context engine with its own setup/UX

Choose **OpenGrok/Glean/Sourcegraph stack** when your primary goal is:
- Existing enterprise search ecosystem (Sourcegraph/OpenGrok)
- Organization-wide fact platform at scale (Glean)

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features

# Tests
cargo test --workspace

# Docs
cargo doc --workspace --no-deps
```

## Benchmarks

```bash
cargo bench
```

## Troubleshooting

### `Failed to connect to Memgraph`

- Check endpoint and protocol in config (`memgraph://` or `bolt://`)
- Verify container/service is running and port is reachable
- Run `cortex doctor`

### `Index finished but queries return little/no data`

- Re-run with force: `cortex index <path> --force`
- Confirm repository path used in filters matches indexed path
- Validate node count via `cortex query`

### `MCP client cannot see tools`

- Verify server starts: `cortex mcp start`
- Check client config command/args and environment
- Verify with `cortex mcp tools`

## Roadmap

See [docs/ROADMAP.md](docs/ROADMAP.md).

## License

MIT
