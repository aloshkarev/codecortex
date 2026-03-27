# cortex-cli

> `cortex-cli` is the main command-line interface for CodeCortex. It runs indexing, queries, analysis, project management, vector workflows, and MCP server startup through a single `cortex` binary.

## What it does

- Provides the `cortex` binary that drives all CodeCortex operations
- Exposes all 60 MCP tools as CLI commands for scripting and interactive use
- Manages project scope (single-project vs. all-projects) based on working directory context
- Starts the MCP server in any supported transport mode

## Command reference

### Repository

| Command | Description |
|---------|-------------|
| `cortex index <path>` | Index a repository. `--force` re-indexes; `--mode full\|incremental-diff` selects scope; `--base-branch <branch>` sets the git diff base for incremental mode |
| `cortex list` | List all indexed repositories |
| `cortex delete <path>` | Remove a repository from the graph |
| `cortex stats` | Show node/file counts for indexed repositories |
| `cortex watch <path>` | Watch a path for changes and queue re-indexing |
| `cortex unwatch <path>` | Stop watching a path |

### Search and query

| Command | Description |
|---------|-------------|
| `cortex find name <symbol>` | Find by exact or prefix name |
| `cortex find pattern <regex>` | Find by name regex pattern |
| `cortex find type <kind>` | Find by entity kind (function, class, struct, …) |
| `cortex find content <text>` | Full-text search in source content |
| `cortex find decorator <name>` | Find entities decorated with a given decorator/attribute |
| `cortex find argument <name>` | Find functions with a given argument name |
| `cortex query <cypher>` | Execute a raw Cypher query |
| `cortex skeleton <path>` | Structural outline of a file or directory |
| `cortex signature <symbol>` | Get type signature for a symbol |
| `cortex goto <symbol>` | Navigate to the definition of a symbol |
| `cortex usages <symbol>` | Find all usages of a symbol |
| `cortex info <symbol>` | Hover-style info (type, docs, location) |
| `cortex search <query>` | Semantic vector search (requires vector index) |

### Analyze

All `analyze` subcommands support path scoping via filter flags (see below).

| Command | Description |
|---------|-------------|
| `cortex analyze callers <symbol>` | Direct and transitive callers |
| `cortex analyze callees <symbol>` | Direct and transitive callees |
| `cortex analyze chain <a> <b>` | Call path between two symbols |
| `cortex analyze hierarchy <type>` | Inheritance or trait implementation hierarchy |
| `cortex analyze deps <symbol>` | Dependency subgraph |
| `cortex analyze dead-code` | Detect unreachable definitions |
| `cortex analyze complexity` | Cyclomatic complexity per symbol. Use `--top N` for top-N |
| `cortex analyze overrides <method>` | Override and implementation sites |
| `cortex analyze smells` | Detect code smells (long functions, duplication, etc.) |
| `cortex analyze refactoring` | Refactoring suggestions |
| `cortex analyze branch-diff <src> <tgt>` | Structural diff between branches. Add `--structural` for deep node-level diff |
| `cortex analyze review` | Graph-enriched code review (impact warnings, dead code signals) |
| `cortex analyze similar <symbol>` | Cross-project similar symbol search |
| `cortex analyze shared-deps` | Shared dependency analysis across projects |
| `cortex analyze compare-api` | API surface comparison across projects |

### Vector

| Command | Description |
|---------|-------------|
| `cortex vector-index <path>` | Index a repository for semantic vector search |
| `cortex search <query>` | Natural language semantic search (vector) |

### Project

| Command | Description |
|---------|-------------|
| `cortex project list` | List registered projects |
| `cortex project add <path>` | Register a project |
| `cortex project remove <path>` | Unregister a project |
| `cortex project set <path>` | Set the active project |
| `cortex project current` | Show the current active project |
| `cortex project branches` | List branches for the current project |
| `cortex project refresh` | Refresh git state for the current project |
| `cortex project status` | Show project health and indexing state |
| `cortex project sync` | Sync the project graph with disk |
| `cortex project policy show\|set` | Show or set the project indexing policy |
| `cortex project metrics` | Show project-level metrics |

### MCP

| Command | Description |
|---------|-------------|
| `cortex mcp start` | Start the MCP server (stdio by default) |
| `cortex mcp tools` | List all 60 available MCP tools |

MCP serve flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--transport stdio\|http-sse\|websocket\|multi` | `stdio` | Transport mode |
| `--listen <addr:port>` | `127.0.0.1:3001` | Bind address for network transports |
| `--allow-remote` | off | Required for non-loopback bind |
| `--token <value>` | — | Static bearer token |
| `--token-env <ENV_NAME>` | — | Bearer token from environment variable |
| `--max-clients <N>` | — | Maximum concurrent network clients |
| `--idle-timeout-secs <N>` | — | Disconnect idle clients after N seconds |

```bash
cortex mcp start                                          # stdio (default)
cortex mcp start --transport http-sse --listen 127.0.0.1:3001
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token "$CORTEX_MCP_TOKEN"
```

### Operations

| Command | Description |
|---------|-------------|
| `cortex setup` | Interactive first-run setup wizard |
| `cortex doctor` | Verify backend connectivity and config |
| `cortex config show\|set\|reset` | Manage configuration values |
| `cortex jobs list\|status` | Inspect background job queue |
| `cortex daemon start\|stop\|status` | Manage the background daemon process |
| `cortex debug capsule\|cache\|trace\|validate` | Debug internal state |
| `cortex diagnose` | Detailed health diagnostics |
| `cortex memory save\|search\|context\|list\|clear` | Session memory management |
| `cortex capsule <symbol>` | Get a context capsule for a symbol |
| `cortex impact <symbol>` | Blast-radius impact graph for a symbol |
| `cortex refactor <symbol>` | Refactoring analysis for a symbol |
| `cortex patterns` | Detect code patterns in the repository |
| `cortex test <symbol>` | Find tests associated with a symbol |
| `cortex completion` | Generate shell completions (bash, zsh, fish, …) |
| `cortex interactive` | Start an interactive REPL |
| `cortex bundle export\|import` | Export or import a graph bundle |
| `cortex clean` | Remove stale and orphaned graph data |

## Analyze filter flags

| Flag | Alias | Description |
|------|-------|-------------|
| `--file` | `--include-file` | Include a specific file |
| `--folder` | `--dir`, `--directory`, `--include-path` | Include a directory |
| `--include-path` | | Include by path prefix |
| `--include-file` | | Include a specific file |
| `--include-glob` | | Include by glob pattern |
| `--exclude-path` | | Exclude by path prefix |
| `--exclude-file` | | Exclude a specific file |
| `--exclude-glob` | | Exclude by glob pattern |

Includes are additive; excludes are additive; excludes override includes.

## Scope behavior

- **Inside a known project**: defaults to single-project scope using the project registry from `cortex-watcher`.
- **Outside known projects**: `find` and `search` default to all-project scope.
- **Overrides**:
  - `--all-projects` — force cross-project mode
  - `--project <path>` — force explicit single-project mode

Navigation commands (`goto`, `usages`, `info`) always require single-project scope.

## Examples

```bash
cortex index /path/to/repo --force
cortex index /path/to/repo --mode incremental-diff --base-branch main
cortex find name GraphClient
cortex find decorator "#[tokio::test]"
cortex analyze callers authenticate --include-path src/auth --include-glob "**/*.rs"
cortex analyze complexity --top 20
cortex analyze branch-diff feature/nav main --structural
cortex goto "GraphClient::connect"
cortex usages "GraphClient"
cortex mcp tools
CORTEX_FLAG_MCP_MEMORY_READ_ENABLED=true cortex mcp start
```

## Integration tests

Per-language integration tests:

- `crates/cortex-cli/tests/language_matrix_integration.rs`
- `crates/cortex-cli/tests/integration/`

Real remote-fixture matrix runs 14 languages (12 baseline + Kotlin + Swift). JSON and Shell are covered via parser/vector/analyzer unit and contract tests.

```bash
# Run one language
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1

# Run all languages in order
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

## Tests

```bash
cargo test -p cortex-cli -- --test-threads=1
```
