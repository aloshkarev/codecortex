# MCP semantic tool tests — implementation plan

**Spec:** [2026-06-02-mcp-semantic-tool-tests-design.md](../specs/2026-06-02-mcp-semantic-tool-tests-design.md)

## Delivered

| Component | Path |
| --- | --- |
| Oracle registry (77 tools) | `tests/mcp_semantic/oracles.json` |
| Semantic audit driver | `scripts/mcp_semantic_audit.py` |
| L1 description contract | `crates/cortex-mcp/tests/mcp_semantic_description.rs` |
| Live Rust gate | `crates/cortex-mcp/tests/mcp_semantic_audit.rs` |
| Makefile | `mcp-semantic-audit`, `mcp-semantic-pr` |
| CI | `.github/workflows/mcp-semantic-audit.yml` |
| Nightly merge | `scripts/nightly-mcp-audit.sh` + `target/mcp-full-audit.json` |
| Smoke alignment | `scripts/mcp_tool_audit.py` (`SEMANTIC_STRICT_TOOLS`) |
| Docs | `docs/MCP_AUDIT_FINDINGS.md` |

## Cluster coverage (subagent batches)

| # | Cluster | Oracle tools |
| --- | --- | --- |
| 1 | Preflight & routing | `check_health`, `index_status`, `recommend_tools`, `get_tool_guidance`, `explain_index_freshness`, `diagnose` |
| 2 | Navigation | `go_to_definition`, `find_all_usages`, `quick_info`, `get_signature`, `get_skeleton` |
| 3 | Search & relationships | `find_code`, `analyze_code_relationships`, `search_logic_flow`, `get_impact_graph`, `explain_result` |
| 4 | Quality | `find_dead_code`, `calculate_cyclomatic_complexity`, `find_patterns`, `analyze_refactoring`, `find_tests` |
| 5 | Agent context | `get_patch_context`, `get_delta_context`, `get_api_contract`, `get_test_context`, `get_context_capsule`, `summarize_module`, `estimate_context_cost` |
| 6 | Branch/review | `branch_structural_diff`, `pr_review` |
| 7 | Vector (nightly) | `vector_*`, `search_across_projects` |
| 8 | Index/admin (nightly) | `add_code_to_graph`, watch, projects, jobs, bundle, memory, destructive dry-run |
| 9 | A2A (nightly) | `cortex_a2a_*` (+ existing `mcp_a2a_workflow.rs`) |
| 10 | Multi/advanced (nightly) | `find_similar_across_projects`, `compare_api_surface`, `execute_cypher_query` |

## Operator commands

```bash
# PR gate (21 oracles)
make mcp-semantic-pr REPO=/path/to/64-codecortex CORTEX_AUDIT_BIN=./target/release/cortex-cli

# Nightly semantic (77 oracles, after vector-index in script)
CORTEX_SEMANTIC_REPO=/path/to/64-codecortex make mcp-nightly-audit

# Rust ignored test
CORTEX_SEMANTIC_AUDIT=1 cargo test -p cortex-mcp --test mcp_semantic_audit -- --ignored
```

## Follow-up

- Tune failing PR oracles against live graph (fix tool or relax `min_length` with evidence).
- Add `scripts/generate_mcp_oracle_hints.py` for suggested counts from `cortex analyze`.
- Optional: native `cortex mcp semantic-audit` subcommand wrapping the Python script.
