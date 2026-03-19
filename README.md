# CodeCortex

[![CI](https://github.com/aloshkarev/codecortex/actions/workflows/ci.yml/badge.svg)](https://github.com/aloshkarev/codecortex/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

**CodeCortex** is a Rust-based code intelligence stack for local repositories and AI-assisted workflows. It combines graph indexing, static analysis, optional vector retrieval, and an [MCP](https://modelcontextprotocol.io/) server in one runtime.

## Table of contents

- [Features](#features)
- [Quick start](#quick-start)
- [Command reference](#command-reference)
- [MCP mode](#mcp-mode)
- [Development](#development)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Features

- **Graph indexing** — Index repositories into Memgraph/Neo4j-compatible backends
- **Structural analysis** — Callers, callees, chains, dependencies, dead code, complexity, smells, refactoring
- **Project-aware** — Multi-project and branch-aware operations
- **Navigation** — `goto`, `usages`, `info` on indexed graph data
- **Cross-project** — `--all-projects`, `--project`, cross-project analyze/search tools
- **MCP server** — 60+ tools for AI clients (Claude, Cursor, VSCode)
- **Vector search** — Optional semantic indexing and hybrid search
- **Multi-language** — Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell

## Quick start

### Prerequisites

- [Nix](https://nixos.org/) (recommended) or Rust
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

For guided install with dependency checks: `./install.sh` or `./quickstart.sh`.

## Command reference

| Group | Commands |
|-------|----------|
| Repository | `index`, `list`, `delete`, `stats`, `watch`, `unwatch` |
| Search/query | `find`, `query`, `skeleton`, `signature`, `goto`, `usages`, `info` |
| Analyze | `callers`, `callees`, `chain`, `hierarchy`, `deps`, `dead-code`, `complexity`, `overrides`, `smells`, `refactoring`, `branch-diff`, `review`, `similar`, `shared-deps`, `compare-api` |
| Vector | `vector-index`, `search` |
| Project | `project ...` |
| MCP | `mcp start`, `mcp tools` |
| Operations | `doctor`, `config`, `jobs`, `debug`, `daemon`, `interactive` |

### Analyze filter flags

- `--file` (alias: `include-file`)
- `--folder`, `--dir`, `--directory` (alias: `include-path`)
- `--include-path`, `--include-file`, `--include-glob`
- `--exclude-path`, `--exclude-file`, `--exclude-glob`

Includes are additive; excludes are additive; excludes override includes.

```bash
cortex analyze callers authenticate \
  --include-path src/auth \
  --include-glob "**/*.rs" \
  --exclude-path src/auth/generated
```

## MCP mode

```bash
cortex mcp tools
cortex mcp start
```

**Transports:**

```bash
# stdio (default)
cortex mcp start

# network
cortex mcp start --transport http-sse --listen 127.0.0.1:3001
cortex mcp start --transport websocket --listen 127.0.0.1:3001
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token-env CORTEX_MCP_TOKEN
```

Stdio and network modes use the same `CortexHandler` tool path. See [docs/INTEGRATION.md](docs/INTEGRATION.md) for client setup.

### MCP Tool Coverage

- **Indexing** `add_code_to_graph`
- **Watch** `watch_directory`
- **Watch** `list_watched_paths`
- **Watch** `unwatch_directory`
- **Search** `find_code`
- **Analyze** `analyze_code_relationships`
- **Query** `execute_cypher_query`
- **Analyze** `find_dead_code`
- **Navigation** `go_to_definition`
- **Navigation** `find_all_usages`
- **Navigation** `quick_info`
- **Review** `branch_structural_diff`
- **Review** `pr_review`
- **CrossProject** `find_similar_across_projects`
- **CrossProject** `find_shared_dependencies`
- **CrossProject** `compare_api_surface`
- **Analyze** `calculate_cyclomatic_complexity`
- **Vector** `vector_index_repository`
- **Vector** `vector_index_file`
- **Vector** `vector_search`
- **Vector** `vector_search_hybrid`
- **Vector** `search_across_projects`
- **Vector** `vector_index_status`
- **Vector** `vector_delete_repository`
- **Context** `get_context_capsule`
- **Impact** `get_impact_graph`
- **LogicFlow** `search_logic_flow`
- **Structure** `get_skeleton`
- **Indexing** `index_status`
- **Workspace** `workspace_setup`
- **LSP** `submit_lsp_edges`
- **Memory** `save_observation`
- **Memory** `get_session_context`
- **Memory** `search_memory`
- **Repository** `list_indexed_repositories`
- **Repository** `delete_repository`
- **Repository** `get_repository_stats`
- **Jobs** `check_job_status`
- **Jobs** `list_jobs`
- **Bundle** `load_bundle`
- **Bundle** `export_bundle`
- **Health** `check_health`
- **Health** `diagnose`
- **Signature** `get_signature`
- **Tests** `find_tests`
- **Explain** `explain_result`
- **Refactoring** `analyze_refactoring`
- **Patterns** `find_patterns`
- **Project** `list_projects`
- **Project** `add_project`
- **Project** `remove_project`
- **Project** `set_current_project`
- **Project** `get_current_project`
- **Project** `list_branches`
- **Project** `refresh_project`
- **Project** `project_status`
- **Project** `project_sync`
- **Project** `project_branch_diff`
- **Project** `project_queue_status`
- **Project** `project_metrics`

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

### Integration tests (12 languages)

```bash
# One language
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1

# All languages (strict order)
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

Runbook: [docs/INTEGRATION_TEST_MATRIX.md](docs/INTEGRATION_TEST_MATRIX.md)

### Workspace crates

- `cortex-core` — Config, errors, models
- `cortex-parser` — Multi-language parsing (tree-sitter)
- `cortex-graph` — Memgraph/Neo4j client
- `cortex-indexer` — Graph indexing pipeline
- `cortex-analyzer` — Call graphs, smells, refactoring
- `cortex-vector` — Embeddings and semantic search
- `cortex-watcher` — File watching and jobs
- `cortex-pipeline` — Indexing stages
- `cortex-mcp` — MCP server and tools
- `cortex-cli` — CLI entrypoint
- `cortex-benches` — Benchmarks

## CI and release

- **CI** — [`.github/workflows/ci.yml`](.github/workflows/ci.yml) (Nix checks + build)
- **Integration** — [`.github/workflows/integration-language-matrix.yml`](.github/workflows/integration-language-matrix.yml)
- **Releases** — [`.github/workflows/release.yml`](.github/workflows/release.yml) (linux-x86_64, linux-aarch64, macos-aarch64)

## Documentation

- [Install guide](docs/INSTALL.md)
- [Integration guide](docs/INTEGRATION.md)
- [Roadmap](docs/ROADMAP.md)

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) first.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
