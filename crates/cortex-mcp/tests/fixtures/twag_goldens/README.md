# TWAG corpus goldens

Expectation-based oracles for MCP navigation tools on the TWAG monorepo
(default: `/run/media/alex/artefacts/projects/work/twag`).

Each case is a JSON file (`<case_id>.json`) loaded by `twag_corpus_accuracy.rs`.
`manifest.json` lists all case ids for the manifest smoke test.

## Environment gates

| Variable | Required | Purpose |
| --- | --- | --- |
| `CORTEX_TEST_TWAG` | `1` | Enable TWAG integration tests |
| `CORTEX_TEST_GRAPH` | `1` | FalkorDB graph backend available |
| `CORTEX_TWAG_REPO` | — | Override TWAG checkout path |
| `CORTEX_BIN` | — | Path to `cortex` / `cortex-cli` binary |

## Index TWAG slice (once per machine / after graph wipe)

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

Use `--graph-repository-path "$TWAG"` so `repository_path` matches MCP `set_current_project` scope.

## Run tests

```bash
# Accuracy (all manifest cases)
CORTEX_TEST_TWAG=1 CORTEX_TEST_GRAPH=1 \
  cargo test -p cortex-mcp --test twag_corpus_accuracy -- --ignored --nocapture

# Token-efficiency JSON report → target/twag-token-efficiency.json
CORTEX_TEST_TWAG=1 CORTEX_TEST_GRAPH=1 \
  cargo test -p cortex-mcp --test twag_token_efficiency -- --ignored --nocapture

# A2A on TWAG (consensus_review + pr_review, sled task_store)
CORTEX_TEST_TWAG=1 CORTEX_TEST_GRAPH=1 \
  cargo test -p cortex-cli --test a2a_twag_e2e -- --ignored --nocapture
```

## Cases

| Case | Tool | Focus |
| --- | --- | --- |
| `find_callers_orchestrator_snapshot` | `analyze_code_relationships` | C++ `snapshot` callers in orchestrator |
| `find_callers_apply_config_snapshot` | `analyze_code_relationships` | C++ `apply_config_snapshot` callers |
| `go_to_definition_orchestrator_snapshot` | `go_to_definition` | C++ `Orchestrator::snapshot` |
| `go_to_definition_should_relay` | `go_to_definition` | Rust `should_relay` in rdiameter-core |
| `find_all_usages_should_relay` | `find_all_usages` | Rust usages (may be empty / partial) |
| `find_all_usages_peer_state` | `find_all_usages` | Rust type usages (may be empty / partial) |

Optional `*.live.json` files capture full MCP payloads for debugging; tests use the expectation `*.json` files only.
