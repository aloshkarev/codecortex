# TWAG integration tests

Real-repo accuracy, token-efficiency, and A2A E2E tests against the indexed TWAG corpus at `/run/media/alex/artefacts/projects/work/twag` (override with `CORTEX_TWAG_REPO`).

## Environment gates

| Variable | Required by | Purpose |
| --- | --- | --- |
| `CORTEX_TEST_TWAG=1` | TWAG accuracy / token benchmark / A2A E2E | Opt-in to external TWAG checkout |
| `CORTEX_TEST_GRAPH=1` | Graph-backed tests | FalkorDB available; TWAG indexed |
| `CORTEX_TWAG_REPO` | Optional | TWAG path (default above) |
| `CORTEX_BIN` | MCP stdio tests | `cortex` binary (default: `cortex`) |

### Tests without graph

| Test | Command |
| --- | --- |
| Validator layout (CMake / Cargo) | `cargo test -p cortex-mcp --test a2a_validate_build` |
| Validator build failure (`fix_build`) | `cargo test -p cortex-mcp --test a2a_validator_build_failure` |
| Token report JSON shape | `cargo test -p cortex-benches --bin token_efficiency_twag` |

## Fixtures

Golden oracles: `crates/cortex-mcp/tests/fixtures/twag_goldens/` (see README there).

Optional live MCP snapshots: `scripts/twag_golden_refresh.py` (writes `*.live.json`).

## Index TWAG (graph scope)

```bash
export TWAG=/run/media/alex/artefacts/projects/work/twag
export CORTEX_BIN=./target/release/cortex-cli

$CORTEX_BIN index "$TWAG" --force --graph-repository-path "$TWAG" \
  --include-file components/cp/src/orchestrator.cpp \
  --include-file components/cp/include/wmg/cp/orchestrator.hpp

$CORTEX_BIN index "$TWAG" --force --graph-repository-path "$TWAG" \
  --include-file third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/node.rs \
  --include-file third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/lib.rs \
  --include-file third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/dispatch.rs \
  --include-file third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/relay.rs \
  --include-file third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/peer.rs
```

## Run graph-gated tests

```bash
export CORTEX_TEST_TWAG=1 CORTEX_TEST_GRAPH=1

# Corpus accuracy (all manifest cases)
cargo test -p cortex-mcp --test twag_corpus_accuracy -- --ignored --nocapture

# Token-efficiency JSON → target/token-efficiency-twag.json
CORTEX_TEST_TWAG=1 cargo run -p cortex-benches --bin token_efficiency_twag

# A2A E2E (consensus_review + pr_review, sled task_store; validate overridden to `/bin/true`)
cargo test -p cortex-mcp --test a2a_twag_e2e -- --ignored --nocapture --test-threads=1
```

Single accuracy case:

```bash
cargo test -p cortex-mcp --test twag_corpus_accuracy twag_find_callers_apply_config_snapshot -- --ignored --nocapture
```

## File map

| Path | Role |
| --- | --- |
| `crates/cortex-mcp/tests/twag_common.rs` | MCP session + golden assertions |
| `crates/cortex-mcp/tests/twag_corpus_accuracy.rs` | TWAG accuracy suite |
| `crates/cortex-mcp/tests/fixtures/twag_goldens/*` | Golden JSON oracles |
| `crates/cortex-mcp/tests/a2a_twag_e2e.rs` | A2A consensus + PR on TWAG |
| `crates/cortex-mcp/tests/a2a_validator_build_failure.rs` | Validator build failure mode |
| `crates/cortex-mcp/tests/a2a_validate_build.rs` | TWAG / rdiameter validate layout |
| `crates/cortex-benches/src/bin/token_efficiency_twag.rs` | Token savings benchmark |
| `scripts/twag_golden_refresh.py` | Optional live snapshot refresh |
| `docs/TWAG_INTEGRATION.md` | This runbook |
