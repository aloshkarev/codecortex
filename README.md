# CodeCortex

CodeCortex is a Rust-based code intelligence stack for local repositories and AI-assisted workflows.

It combines graph indexing, static analysis, optional vector retrieval, and an MCP server in one runtime.

## Core capabilities

- repository indexing into Memgraph/Neo4j-compatible graph backends
- structural analysis (callers, callees, chains, dependencies, dead code, complexity, smells, refactoring)
- project and branch-aware operations
- MCP tools for assistant clients
- optional vector indexing and semantic search
- language coverage: Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell

## Quick start

```bash
# Start graph backend
docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph-mage:3.8.1

# Configure runtime
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

## Command groups

- repository: `index`, `list`, `delete`, `stats`, `watch`, `unwatch`
- search/query: `find`, `query`, `skeleton`, `signature`
- analyze: `callers`, `callees`, `chain`, `hierarchy`, `deps`, `dead-code`, `complexity`, `overrides`, `smells`, `refactoring`, `branch-diff`, `review`
- vector: `vector-index`, `search`
- project: `project ...`
- mcp: `mcp start`, `mcp tools`
- operations: `doctor`, `config`, `jobs`, `debug`, `daemon`, `interactive`

## Analyze filter flags

- `--file` (alias to include-file)
- `--folder` (aliases: `--dir`, `--directory`; alias to include-path)
- `--include-path`
- `--include-file`
- `--include-glob`
- `--exclude-path`
- `--exclude-file`
- `--exclude-glob`

Semantics: includes are additive, excludes are additive, excludes win.

Example:

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

Transport options:

```bash
# stdio (default)
cortex mcp start

# network transports
cortex mcp start --transport http-sse --listen 127.0.0.1:3001
cortex mcp start --transport websocket --listen 127.0.0.1:3001
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token-env CORTEX_MCP_TOKEN
```

Stdio and network modes route through the same `CortexHandler` tool path for consistent MCP behavior.

Client integration examples:

- `docs/INTEGRATION.md`

## Real integration tests (12 languages)

Run one language:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1
```

Run all languages in strict order:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

Note: JSON and Shell are fully supported in parser/indexer/vector/MCP/runtime paths and are currently validated in contract/unit coverage rather than remote-fixture matrix jobs.

Runbook:

- `docs/INTEGRATION_TEST_MATRIX.md`

## Workspace crates

- `crates/cortex-core`
- `crates/cortex-parser`
- `crates/cortex-graph`
- `crates/cortex-indexer`
- `crates/cortex-analyzer`
- `crates/cortex-vector`
- `crates/cortex-watcher`
- `crates/cortex-pipeline`
- `crates/cortex-mcp`
- `crates/cortex-cli`
- `crates/cortex-benches`

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace
```

## Additional docs

- install: `docs/INSTALL.md`
- integrations: `docs/INTEGRATION.md`
- roadmap: `docs/ROADMAP.md`

## License

MIT
