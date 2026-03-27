# CodeCortex

[![CI](https://github.com/aloshkarev/codecortex/actions/workflows/ci.yml/badge.svg)](https://github.com/aloshkarev/codecortex/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> CodeCortex is a Rust-based code intelligence stack that indexes local repositories into a graph database, exposes **60 MCP tools** to AI clients, and combines structural graph analysis with optional vector search for semantic and hybrid code retrieval.

## What it does

- **Graph indexing** — Parse and store code structure into Memgraph, Neo4j, or AWS Neptune using parameterized Cypher
- **Structural analysis** — Callers, callees, call chains, dependency graphs, dead code detection, cyclomatic complexity, code smells, and refactoring suggestions
- **Navigation** — `goto`, `usages`, `info` backed by graph-indexed `qualified_name` and `TYPE_REFERENCE` edges
- **MCP server** — 60 tools for AI clients (Claude, Cursor, Codex CLI, Gemini, Zed, Neovim) over stdio, HTTP-SSE, WebSocket, or all at once
- **Vector search** — Optional semantic indexing (OpenAI or Ollama embedders) and hybrid graph+vector retrieval
- **Cross-project** — Similarity search, shared-dependency detection, and API surface comparison across multiple repositories
- **14 languages** — Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell

## Table of contents

- [CodeCortex](#codecortex)
  - [What it does](#what-it-does)
  - [Table of contents](#table-of-contents)
  - [Quick start](#quick-start)
    - [Prerequisites](#prerequisites)
    - [Build and install](#build-and-install)
    - [Run](#run)
  - [Command reference](#command-reference)
    - [Repository](#repository)
    - [Search and query](#search-and-query)
    - [Analyze](#analyze)
    - [Vector](#vector)
    - [Project](#project)
    - [MCP](#mcp)
    - [Operations](#operations)
    - [Analyze filter flags](#analyze-filter-flags)
  - [MCP mode](#mcp-mode)
    - [Transports](#transports)
    - [MCP tool catalog (60 tools)](#mcp-tool-catalog-60-tools)
  - [Feature flags](#feature-flags)
  - [Comparison with alternatives](#comparison-with-alternatives)
  - [Development](#development)
    - [Integration tests (14 languages)](#integration-tests-14-languages)
  - [Workspace crates](#workspace-crates)
  - [CI and release](#ci-and-release)
  - [Documentation](#documentation)
  - [Contributing](#contributing)
  - [License](#license)

## Quick start

### Prerequisites

- [Nix](https://nixos.org/) (recommended) or Rust stable
- [Docker](https://www.docker.com/) (for Memgraph backend)

### Build and install

```bash
# Enter reproducible dev environment (recommended)
nix develop

# Build CLI
nix build .#cortex

# Install locally
mkdir -p ~/.local/bin
cp result/bin/cortex ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

**Cargo fallback** (without Nix):

```bash
cargo build --release -p cortex-cli
cp target/release/cortex-cli ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

For a guided install with dependency checks: `./install.sh` or `./quickstart.sh`.

### Run

```bash
# Start graph backend
docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph-mage:3.8.1

# Configure
mkdir -p ~/.cortex
cat > ~/.cortex/config.toml <<'CFG'
memgraph_uri = "memgraph://127.0.0.1:7687"
memgraph_user = ""
memgraph_password = ""
backend_type = "memgraph"
CFG

# Verify and index
cortex doctor
cortex index /path/to/repo --force

# Query and analyze
cortex find name GraphClient
cortex analyze callers authenticate
cortex query "MATCH (n:CodeNode) RETURN count(n) AS c"
```

## Command reference

### Repository

| Command | Description |
|---------|-------------|
| `index <path>` | Index a repository into the graph. `--force` re-indexes, `--mode full\|incremental-diff` controls scope, `--base-branch <branch>` sets git diff base |
| `list` | List indexed repositories |
| `delete <path>` | Remove a repository from the graph |
| `stats` | Show indexed file and node counts |
| `watch <path>` | Start watching a path for changes |
| `unwatch <path>` | Stop watching a path |

### Search and query

| Command | Description |
|---------|-------------|
| `find name <symbol>` | Find by exact or prefix name |
| `find pattern <regex>` | Find by name regex |
| `find type <kind>` | Find by entity kind (function, class, …) |
| `find content <text>` | Full-text search in source |
| `find decorator <name>` | Find by decorator/attribute |
| `find argument <name>` | Find by function argument name |
| `query <cypher>` | Execute raw Cypher query |
| `skeleton <path>` | Structural outline of a file or directory |
| `signature <symbol>` | Get function/type signature |
| `goto <symbol>` | Navigate to definition |
| `usages <symbol>` | Find all usages |
| `info <symbol>` | Hover-style type and doc info |
| `search <query>` | Semantic vector search (requires vector index) |

### Analyze

| Command | Description |
|---------|-------------|
| `analyze callers <symbol>` | Direct and transitive callers |
| `analyze callees <symbol>` | Direct and transitive callees |
| `analyze chain <a> <b>` | Call path between two symbols |
| `analyze hierarchy <type>` | Inheritance/trait hierarchy |
| `analyze deps <symbol>` | Dependency subgraph |
| `analyze dead-code` | Detect unreachable definitions |
| `analyze complexity` | Cyclomatic complexity (use `--top N`) |
| `analyze overrides <method>` | Override and implementation sites |
| `analyze smells` | Code smell detection |
| `analyze refactoring` | Refactoring suggestions |
| `analyze branch-diff <src> <tgt>` | Structural diff between branches. `--structural` for deep node-level diff |
| `analyze review` | Graph-enriched code review |
| `analyze similar <symbol>` | Cross-project similar symbol search |
| `analyze shared-deps` | Shared dependency analysis |
| `analyze compare-api` | Compare API surfaces across projects |

### Vector

| Command | Description |
|---------|-------------|
| `vector-index <path>` | Index repository for semantic search |
| `search <query>` | Natural language semantic search |

### Project

| Command | Description |
|---------|-------------|
| `project list` | List registered projects |
| `project add <path>` | Register a project |
| `project remove <path>` | Unregister a project |
| `project set <path>` | Set active project |
| `project current` | Show current project |
| `project branches` | List branches for current project |
| `project refresh` | Refresh git state |
| `project status` | Show project health |
| `project sync` | Sync project graph with disk |
| `project policy show\|set` | Show or set project indexing policy |
| `project metrics` | Show project-level metrics |

### MCP

| Command | Description |
|---------|-------------|
| `mcp start` | Start MCP server (stdio by default) |
| `mcp tools` | List all available MCP tools |

### Operations

| Command | Description |
|---------|-------------|
| `setup` | Interactive first-run setup wizard |
| `doctor` | Verify backend connectivity and config |
| `config show\|set\|reset` | Manage configuration |
| `jobs list\|status` | Inspect background job queue |
| `daemon start\|stop\|status` | Manage background daemon |
| `debug capsule\|cache\|trace\|validate` | Debug internals |
| `diagnose` | Detailed health diagnostics |
| `memory save\|search\|context\|list\|clear` | Session memory management |
| `capsule <symbol>` | Get context capsule for a symbol |
| `impact <symbol>` | Blast-radius impact graph |
| `refactor <symbol>` | Refactoring analysis |
| `patterns` | Detect code patterns |
| `test <symbol>` | Find tests for a symbol |
| `completion` | Generate shell completions |
| `interactive` | Start interactive REPL |
| `bundle export\|import` | Export/import graph bundles |
| `clean` | Remove stale graph data |

### Analyze filter flags

All `analyze` commands support path scoping:

```
--file / --include-file
--folder / --dir / --directory / --include-path
--include-glob
--exclude-path
--exclude-file
--exclude-glob
```

Includes are additive; excludes are additive; excludes override includes.

```bash
cortex analyze callers authenticate \
  --include-path src/auth \
  --include-glob "**/*.rs" \
  --exclude-path src/auth/generated
```

## MCP mode

```bash
cortex mcp tools    # list all 60 tools
cortex mcp start    # stdio (default, backward-compatible)
```

### Transports

```bash
# HTTP+SSE on localhost
cortex mcp start --transport http-sse --listen 127.0.0.1:3001

# WebSocket + HTTP+SSE with bearer token
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token-env CORTEX_MCP_TOKEN
```

Transport flags: `--transport stdio|http-sse|websocket|multi`, `--listen <addr:port>`, `--allow-remote`, `--token <value>`, `--token-env <ENV>`, `--max-clients <N>`, `--idle-timeout-secs <N>`.

All transports route through the same `CortexHandler`, so tool behavior and schemas stay consistent across clients.

### MCP tool catalog (60 tools)

| Area | Tools |
|------|-------|
| Indexing | `add_code_to_graph`, `index_status` |
| Watch | `watch_directory`, `list_watched_paths`, `unwatch_directory` |
| Search | `find_code` |
| Analyze | `analyze_code_relationships`, `find_dead_code`, `calculate_cyclomatic_complexity`, `analyze_refactoring`, `find_patterns` |
| Query | `execute_cypher_query` |
| Navigation | `go_to_definition`, `find_all_usages`, `quick_info` |
| Review | `branch_structural_diff`, `pr_review` |
| Cross-project | `find_similar_across_projects`, `find_shared_dependencies`, `compare_api_surface`, `search_across_projects` |
| Vector | `vector_index_repository`, `vector_index_file`, `vector_search`, `vector_search_hybrid`, `vector_index_status`, `vector_delete_repository` |
| Context | `get_context_capsule`, `get_impact_graph`, `search_logic_flow`, `get_skeleton`, `get_signature` |
| Workspace | `workspace_setup` |
| LSP | `submit_lsp_edges` |
| Memory | `save_observation`, `get_session_context`, `search_memory` |
| Repository | `list_indexed_repositories`, `delete_repository`, `get_repository_stats` |
| Jobs | `check_job_status`, `list_jobs` |
| Bundle | `load_bundle`, `export_bundle` |
| Health | `check_health`, `diagnose` |
| Tests | `find_tests` |
| Explain | `explain_result` |
| Project | `list_projects`, `add_project`, `remove_project`, `set_current_project`, `get_current_project`, `list_branches`, `refresh_project`, `project_status`, `project_sync`, `project_branch_diff`, `project_queue_status`, `project_metrics` |

MCP also exposes a **prompt** (`codecortex_route_tools`) and a **resource** (`codecortex://guide/tool-routing`) that provide routing guidance to AI agents.

See [docs/INTEGRATION.md](docs/INTEGRATION.md) for client setup (Cursor, Claude Code, Codex CLI, Gemini CLI, Zed, Neovim).

## Feature flags

Several MCP tools are disabled by default because they are resource-intensive, have side effects, or require additional setup. Enable them with `--enable` args on `cortex mcp start`, or with environment variables. Both sources are combined — either can activate a tool.

### `--enable` flags (recommended)

```bash
cortex mcp start --enable memory --enable context-capsule
```

| `--enable` value | Default | Tool(s) controlled |
|-----------------|---------|-------------------|
| `context-capsule` | off | `get_context_capsule` |
| `impact-graph` | off | `get_impact_graph`, `analyze_refactoring` |
| `logic-flow` | off | `search_logic_flow` |
| `index-status` | off | `index_status` |
| `skeleton` | off | `get_skeleton`, `get_signature`, `find_tests`, `explain_result`, `find_patterns` |
| `workspace-setup` | off | `workspace_setup` |
| `lsp-ingest` | off | `submit_lsp_edges` |
| `memory` | off | `save_observation`, `get_session_context`, `search_memory` |
| `memory-write` | off | `save_observation` only |
| `memory-read` | off | `get_session_context`, `search_memory` only |

Repeat `--enable` to activate multiple tools:

```bash
cortex mcp start \
  --enable memory \
  --enable context-capsule \
  --enable impact-graph
```

### Environment variable overrides

All tools can also be toggled with environment variables. These are combined with `--enable` — either source can activate a tool.

| Environment variable | Default | Tool(s) controlled |
|---------------------|---------|-------------------|
| `CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED` | `false` | `get_context_capsule` |
| `CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED` | `false` | `get_impact_graph` |
| `CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED` | `false` | `search_logic_flow` |
| `CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED` | `false` | `index_status` |
| `CORTEX_FLAG_MCP_SKELETON_ENABLED` | `false` | `get_skeleton` |
| `CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED` | `false` | `workspace_setup` |
| `CORTEX_FLAG_MCP_LSP_INGEST_ENABLED` | `false` | `submit_lsp_edges` |
| `CORTEX_FLAG_MCP_MEMORY_READ_ENABLED` | `false` | `get_session_context`, `search_memory` |
| `CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED` | `false` | `save_observation` |
| `CORTEX_FLAG_MCP_VECTOR_READ_ENABLED` | `true` | `vector_search`, `vector_search_hybrid`, `search_across_projects` |
| `CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED` | `true` | `vector_index_repository`, `vector_index_file` |
| `CORTEX_FLAG_MCP_CACHE_ENABLED` | `true` | Query result caching |
| `CORTEX_FLAG_MCP_TELEMETRY_ENABLED` | `true` | Telemetry collection |
| `CORTEX_FLAG_MCP_TFIDF_SCORING_ENABLED` | `true` | TF-IDF reranking |
| `CORTEX_FLAG_MCP_CENTRALITY_SCORING_ENABLED` | `true` | Graph centrality scoring |

Accepted env values: `1`, `true`, `yes`, `on` to enable; `0`, `false`, `no`, `off` to disable.

## Comparison with alternatives

| Feature | **CodeCortex** | Octocode | codebase-memory-mcp | Coraline | code-graph-mcp |
|---------|:--------------:|:--------:|:-------------------:|:--------:|:--------------:|
| Language | Rust | Rust | C/Go | Rust | TypeScript |
| License | Apache 2.0 | Apache 2.0 | MIT | MIT | MIT |
| MCP tools | **60** | ~10 | 14 | MCP | ~15 |
| Parsed languages | 14 | 12+ | 66 | 28+ | 15+ |
| Graph backend | Memgraph/Neo4j/Neptune | Built-in | SQLite | SQLite | Neo4j/SQLite |
| Vector/hybrid search | Yes (LanceDB/Qdrant) | Yes | No | Yes | Yes (LanceDB) |
| Cross-project analysis | Yes | No | No | No | Limited |
| Branch structural diff | Yes | No | No | No | No |
| PR review w/ graph | Yes | Code review | No | No | No |
| MCP transports | stdio + HTTP-SSE + WS | stdio | stdio | stdio | stdio |
| Bearer token auth | Yes | No | No | No | No |
| File watching | Yes | Yes | No | Yes | No |
| Session memory | Yes | Yes | No | No | No |
| ECL pipeline | Yes | No | No | No | No |
| Feature flag matrix | Yes | No | No | No | No |
| Nix flake | Yes | No | No | No | No |
| Cloud graph (Neptune) | Yes | No | No | No | No |

CodeCortex's differentiators: largest MCP tool surface (60 tools), enterprise-grade multi-transport with bearer auth, branch-aware structural diff and PR review, cross-project intelligence, and a feature flag matrix for selective tool enablement.

## Development

```bash
nix flake check --print-build-logs
nix build .#cortex
```

**Cargo fallback:**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace
```

### Integration tests (14 languages)

```bash
# One language
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1

# All languages (strict order)
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

Runbook: [docs/INTEGRATION_TEST_MATRIX.md](docs/INTEGRATION_TEST_MATRIX.md)

## Workspace crates

| Crate | Responsibility |
|-------|---------------|
| `cortex-core` | Config, errors, shared models, EdgeKind |
| `cortex-parser` | Multi-language parsing (tree-sitter, 14 languages) |
| `cortex-graph` | Memgraph/Neo4j/Neptune client, Cypher engine, bundle store |
| `cortex-indexer` | Repository scanning, symbol extraction, incremental indexing |
| `cortex-analyzer` | Call graphs, smells, navigation, cross-project analysis |
| `cortex-vector` | LanceDB/Qdrant/JSON vector stores, OpenAI/Ollama embedders |
| `cortex-watcher` | File watching, smart debouncing, project registry |
| `cortex-pipeline` | ECL pipeline (Extract→Cognify→Embed→Load) |
| `cortex-mcp` | MCP server, 60 tools, feature flags, transport routing |
| `cortex-cli` | CLI entrypoint, all commands and flags |
| `cortex-benches` | Benchmarks |

## CI and release

- **CI** — [`.github/workflows/ci.yml`](.github/workflows/ci.yml) (Nix checks + build)
- **Integration** — [`.github/workflows/integration-language-matrix.yml`](.github/workflows/integration-language-matrix.yml)
- **Releases** — [`.github/workflows/release.yml`](.github/workflows/release.yml) (linux-x86_64, linux-aarch64, macos-aarch64)

## Documentation

- [Install guide](docs/INSTALL.md)
- [Integration guide](docs/INTEGRATION.md)
- [Roadmap](docs/ROADMAP.md)
- [LLM-readable summary](llms.txt)

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) first.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
