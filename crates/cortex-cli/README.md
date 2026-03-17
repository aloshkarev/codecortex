# cortex-cli

`cortex-cli` is the main command-line interface for CodeCortex.

It runs indexing, queries, analysis, project operations, vector workflows, and MCP server startup.

## Command areas

- repository: `index`, `list`, `delete`, `stats`
- search/query: `find`, `query`, `skeleton`, `signature`, `goto`, `usages`, `info`
- analyze: callers/callees/chain/hierarchy/deps/dead-code/complexity/overrides/smells/refactoring/branch-diff/review/similar/shared-deps/compare-api
- vector: `vector-index`, `search`
- project: `project ...`
- mcp: `mcp start`, `mcp tools`
- ops: `doctor`, `config`, `jobs`, `debug`, `daemon`, `interactive`

## Scope behavior

- Inside a known project: defaults to single-project scope.
- Outside known projects: `find`/`search` default to all-project scope.
- Overrides:
  - `--all-projects` to force multi-project mode
  - `--project <path>` to force explicit single-project mode

Navigation commands (`goto`, `usages`, `info`) require single-project scope.

## Navigation and review updates

- Added CLI navigation commands:
  - `cortex goto <symbol>`
  - `cortex usages <symbol>`
  - `cortex info <symbol>`
- Enhanced branch diff with structural mode:
  - `cortex analyze branch-diff <source> <target> --structural`
- Review flow can use graph intelligence when context is available:
  - impact warnings for changed symbols
  - potential new dead code signals

## MCP serve modes

`cortex mcp start` keeps backward-compatible stdio defaults.
Network transports and stdio share the same underlying `CortexHandler` tool routing path.

Network serve flags:

- `--transport stdio|http-sse|websocket|multi`
- `--listen 127.0.0.1:3001` (default loopback)
- `--allow-remote` (required for non-loopback bind)
- `--token <value>` or `--token-env <ENV_NAME>`
- `--max-clients <N>`
- `--idle-timeout-secs <N>`

Examples:

```bash
# Existing stdio behavior
cortex mcp start

# HTTP+SSE on localhost
cortex mcp start --transport http-sse --listen 127.0.0.1:3001

# WebSocket + HTTP+SSE with bearer token
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token "$CORTEX_MCP_TOKEN"
```

## Examples

```bash
cortex index /path/to/repo --force
cortex find name GraphClient
cortex analyze callers authenticate
cortex analyze complexity --top 20
cortex mcp tools
```

## Analyze filter flags

- `--file` (alias to `--include-file`)
- `--folder` (aliases: `--dir`, `--directory`; alias to `--include-path`)
- `--include-path`
- `--include-file`
- `--include-glob`
- `--exclude-path`
- `--exclude-file`
- `--exclude-glob`

## Integration tests

Per-language integration tests:

- `crates/cortex-cli/tests/language_matrix_integration.rs`
- `crates/cortex-cli/tests/integration/`

Real remote-fixture matrix currently runs 12 languages (10 baseline + Kotlin + Swift). JSON and Shell runtime support is covered via parser/vector/analyzer unit and contract tests.

Run one language:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1
```

Run all languages in order:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

## Test

```bash
cargo test -p cortex-cli -- --test-threads=1
```
