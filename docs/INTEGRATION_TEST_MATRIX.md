# Multi-Language Integration Tests

This runbook documents full-scale integration tests for CodeCortex across the 12 primary parser/indexer languages.

## Scope

The suite validates:

1. indexing correctness per language fixture
2. `cortex analyze` command behavior (`callers`, `callees`, `chain`, `hierarchy`, `deps`, `dead-code`, `complexity`, `overrides`, `smells`, `refactoring`, `branch-diff`, `review` when applicable)
3. MCP tool functionality through real JSON-RPC calls on indexed repositories

## Fixture Matrix (Pinned)

Defined in `crates/cortex-cli/tests/integration/projects.rs` as `PROJECT_FIXTURES`.

| Language | Repository | Commit |
| --- | --- | --- |
| Rust | `serde-rs/serde` | `fa7da4a93567ed347ad0735c28e439fca688ef26` |
| Python | `pallets/click` | `cdab890e57a30a9f437b88ce9652f7bfce980c1f` |
| Go | `spf13/cobra` | `61968e893eee2f27696c2fbc8e34fa5c4afaf7c4` |
| TypeScript | `axios/axios` | `ebc6056adc341b1bcc7c940262391c2b4c7223b6` |
| JavaScript | `expressjs/express` | `6c4249feec8ab40631817c8e7001baf2ed022224` |
| C | `libcheck/check` | `455005dc29dc6727de7ee36fee4b49a13b39f73f` |
| C++ | `fmtlib/fmt` | `ae6fd83e2ee09ac260f30bbd33f2071e99f972de` |
| Java | `google/gson` | `b7d59549188867deb42e46073fb38735a5beda1c` |
| PHP | `Seldaek/monolog` | `6db20ca029219dd8de378cea8e32ee149399ef1b` |
| Ruby | `sinatra/sinatra` | `f891dd2b6f4911e356600efe6c3b82af97d262c6` |
| Kotlin | `InsertKoinIO/koin` | `461b5684684bb1b17411f27c35a955cdc90f299b` |
| Swift | `apple/swift-argument-parser` | `1e77425a27b864b97501c78511382bd0a0500520` |

## Test Files

- `crates/cortex-cli/tests/integration/projects.rs`
- `crates/cortex-cli/tests/integration/harness.rs`
- `crates/cortex-cli/tests/integration/flow.rs`
- `crates/cortex-cli/tests/language_matrix_integration.rs`
- `crates/cortex-mcp/tests/tool_surface_matrix.rs`

## Execution (One-by-One)

Requires:

- reachable graph backend (Memgraph/Neo4j-compatible Bolt endpoint)
- network access to clone fixture repositories

Environment:

- `CORTEX_INTEGRATION_ENABLE=1`
- `CORTEX_REAL_INTEGRATION=1`
- `CORTEX_TEST_BOLT_URI=bolt://127.0.0.1:7687` (or your backend URI)

Run all languages in strict one-by-one order:

```bash
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

Run one language independently:

```bash
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1
```

Available per-language tests:

- `integration_rust`
- `integration_python`
- `integration_go`
- `integration_typescript`
- `integration_javascript`
- `integration_c`
- `integration_cpp`
- `integration_java`
- `integration_php`
- `integration_ruby`
- `integration_kotlin`
- `integration_swift`

JSON and Shell are implemented with full parser/indexing/vector/MCP runtime support. Their contract coverage currently runs in parser/vector/unit tests, and they are intentionally not yet part of the real remote-fixture matrix.

Run explicit ordered full pass:

```bash
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

## Failure Triage

Classify each failure before fixing:

- parser/language support regression
- indexer graph write regression
- analyze command adapter/filter regression
- MCP tool-surface drift or documentation mismatch

For every defect:

1. create a minimal reproducible assertion in test code
2. apply smallest fix in owning crate
3. rerun the failing test plus at least one cross-language control test

## CI

Workflow: `.github/workflows/integration-language-matrix.yml`

- `smoke` runs on PRs and pushes and executes Nix-defined smoke checks (`mcpToolSurfaceGuard`, `integrationFixtureGuard`)
- `real-matrix` runs only on manual dispatch with `run_real_matrix=true` inside `nix develop`
- `target_language` input chooses either `all` or one explicit language test
- real-matrix uploads log artifacts for debugging
