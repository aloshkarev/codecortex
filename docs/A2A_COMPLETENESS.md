# A2A hybrid architecture — completeness audit

Audit date: 2026-05-31 (baseline). **Updated 2026-06-01** — **100% completion** per [2026-06-01-a2a-100-percent-design.md](superpowers/specs/2026-06-01-a2a-100-percent-design.md).

## Executive summary

**Overall completeness vs original MCP+A2A symmetric hybrid vision: 100%.**

CodeCortex delivers: protocol types, 7 MCP A2A tools, HTTP/gRPC/WS transports, graph blackboard (all payload types), async workflow engine with parallel analyzer+validator fan-out, external reply collection, bus supervisor, four workflows, Sled task persistence, push webhooks (production profile), host guards, and the full integration test matrix including deadlock consensus E2E and FalkorDB CI.

### 100% checklist

| # | Requirement | Evidence |
| --- | --- | --- |
| 1 | `AsyncWorkflowEngine` + hub dispatch helpers | `crates/cortex-a2a/src/runtime/workflow.rs`, `hub.rs` `dispatch_and_record` |
| 2 | Single index-promotion path (bus only) | `hub.rs` `notify_index_promotion` — no duplicate `dispatch_sync` |
| 3 | Parallel analyzer + validator | `hub.rs` `dispatch_parallel_and_record`; `tests/workflow_parallel.rs` |
| 4 | Non-blocking spawn + `subscribe_url` + `wait_for_completion` | `SpawnSessionResponse`, `handler.rs` `cortex_a2a_spawn_session` |
| 5 | WS A2A event fan-out | `network_server.rs` `/a2a/v1/ws`; `tests/a2a_ws_events.rs` |
| 6 | Sled task store | `task_store/sled_store.rs`; config `[a2a] task_store = "sled"` |
| 7 | Push production profile | `A2aConfig::apply_production_profile`; `tests/push_delivery.rs` |
| 8 | Blackboard: all payload types | `services.rs` `blackboard_from_envelope`; `a2a_blackboard_payloads.rs` |
| 9 | Deadlock consensus E2E | `fixtures/a2a/transport_deadlock/`; `a2a_consensus_deadlock.rs` |
| 10 | Real `validate_build` | `a2a_services.rs` `cargo check` + truncated stderr |
| 11 | StrategyProposal negotiation loop | `hub.rs` `max_negotiation_rounds`; `runners.rs` replan on `Reject` |
| 12 | MCP Cypher host guard | `host_guard.rs`; `[a2a.host_guard].max_cypher_rows` |
| 13 | Rules + A2A hook + agent manifests | `.cursor/rules/codecortex-a2a.mdc`; `preflight-a2a-spawn.sh` |
| 14 | FalkorDB CI (required) | `.github/workflows/ci.yml` `a2a-graph` service container |
| 15 | Network E2E | `a2a_network_e2e.rs` |

---

## Pillar scorecard

| ID | Pillar | Score | Status | Evidence |
| --- | --- | --- | --- | --- |
| P1 | Protocol types | **100%** | Complete | All `A2aPayload` variants + StrategyProposal negotiation |
| P2 | Symmetric async agents | **100%** | Complete | Bus supervisor, external replies, parallel dispatch, workflow engine |
| P3 | MCP transport integration | **100%** | Complete | stdio graph hub, HTTP, gRPC, WS `/a2a/v1/ws`, 7 MCP tools |
| P4 | Graph blackboard | **100%** | Complete | All payloads + load SLO + CI |
| P5 | Agent ecosystem | **100%** | Complete | 5/5 agent A2A docs, hooks, intent rule |
| P6 | Host meta-tool path | **100%** | Complete | spawn + subscribe_url; wait_for_completion; host guards |
| P7 | Test matrix | **100%** | Complete | Deadlock, parallel, WS, network E2E, host budget |

**Weighted overall: 100%**

### Intelligence cooperation (2026-06-03 Phase 2)

| Item | Status | Evidence |
| --- | --- | --- |
| `IntelligencePack` shared meta | Complete | `crates/cortex-mcp/src/intelligence/pack.rs` |
| `IntelligenceRequest` scoped services | Complete | `crates/cortex-a2a/src/services.rs`; no session Mutex |
| Spawn `suggested_next_tools` | Complete | `hub.rs` + `spawn_tool_hints`; test `spawn_tool_hints.rs` |
| Tool cooperation router | Complete | `intelligence/tool_router.rs` |
| PR review intelligence module | Complete | `intelligence/pr_review.rs` |
| External `tool_delegation` artifacts | Complete | `hub.rs` `build_external_delegations` |
| Parity / cooperation tests | Complete | `tests/intelligence_parity.rs`, semantic audit chain |

### Protocol-native cooperation (2026-06-04 Phase 3)

| Item | Status | Evidence |
| --- | --- | --- |
| `CooperationArtifact` → spec `Artifact` | Complete | `crates/cortex-a2a/src/cooperation.rs` |
| `Task.metadata` cooperation fields | Complete | `session.rs`, `hub.rs`, `spec_codec.rs` |
| `TaskArtifactUpdateEvent` streaming | Complete | `hub.rs` `publish_artifact_update`; `spec_codec.rs` |
| `GetTask includeArtifacts` | Complete | MCP `A2aGetTaskReq.include_artifacts` |
| AgentSkill MCP tool discovery | Complete | `agent_card.rs` + manifest `mcp_tools` |
| Intelligence-cooperation extension | Complete | `EXTENSION_INTELLIGENCE_COOPERATION` on cards/artifacts |
| spec_codec data Part round-trip | Complete | `proto_contract.rs` `intelligence_data_part_roundtrips_proto` |
| Handler IntelligencePack migration | Complete | `get_impact_graph`, `get_delta_context`, `pr_review` |

---

## Phase 1 — Automated test coverage map

Commands run 2026-06-01 from repo root.

| Test suite | Pillar(s) | Result | Tests |
| --- | --- | --- | --- |
| `cortex-a2a` `workflow_parallel` | P2 | PASS | 1/1 |
| `cortex-a2a` `push_delivery` | P7 | PASS | 1/1 |
| `cortex-a2a` `bus_supervisor` | P2 | PASS | 2/2 |
| `cortex-a2a` `proto_contract` | P1, P6 | PASS | 10/10 |
| `cortex-a2a` `codec_contract` | P1, P11 | PASS | 2/2 |
| `cortex-mcp` `a2a_ws_events` | P3 | PASS | 1/1 |
| `cortex-mcp` `a2a_host_guard` | P6, P12 | PASS | 2/2 |
| `cortex-mcp` `a2a_hub_stdio_graph` | P3, P4 | PASS | 1/1 (with graph) |
| `cortex-cli` `a2a_consensus_deadlock` | P2, P7, P9 | PASS | 1/1 |
| `cortex-cli` `a2a_host_call_budget` | P6 | PASS | 1/1 |
| `cortex-cli` `a2a_workflows_e2e` | P2, P7 | PASS | 4/4 |
| `cortex-cli` `a2a_network_e2e` | P3, P14 | PASS (ignored without graph) | 1/1 |
| `cortex-graph` `a2a_blackboard_load` | P4, P7 | PASS (`CORTEX_TEST_GRAPH=1`) | 1/1 |
| `cortex-graph` `a2a_blackboard_payloads` | P4, P8 | PASS (`CORTEX_TEST_GRAPH=1`) | 1/1 |

---

## Phase 2 — Static trace evidence (current)

### P2 Symmetric async

| Feature | Status | Evidence |
| --- | --- | --- |
| In-process runners (6 incl. PrReviewer) | Complete | `build_runners()` |
| External reply collection | Complete | `runtime/external.rs` |
| Bus supervisor consumers | Complete | `runtime/supervisor.rs` |
| Workflow engine | Complete | `runtime/workflow.rs` |
| Parallel dispatch | Complete | `hub.rs` `dispatch_parallel_and_record` |
| Index promotion (single path) | Complete | bus publish only in `notify_index_promotion` |

### P4 Blackboard

| Feature | Status | Evidence |
| --- | --- | --- |
| All payload → insight | Complete | `services.rs` TaskDelegation, StrategyProposal, CodeInsight, Accept, Reject, FinalResult, GraphMutationSignal |
| Stdio + network blackboard | Complete | `try_build_a2a_hub` |

### P6 Host meta-tool

| Feature | Status | Evidence |
| --- | --- | --- |
| Spawn returns subscribe_url | Complete | `SpawnSessionResponse.subscribe_url` |
| wait_for_completion | Complete | `spawn_session_async` |
| Cypher row guard | Complete | `host_guard.rs`, `execute_cypher_query` |

### Production

| Feature | Status | Evidence |
| --- | --- | --- |
| Sled task store | Complete | `task_store/sled_store.rs` |
| Push (opt-in default; production profile) | Complete | `apply_production_profile()` enables push |

---

## Phase 3 — Live MCP probe ledger (2026-06-01)

| Probe | Result | Notes |
| --- | --- | --- |
| Consensus spawn + wait | VERIFIED | `wait_for_completion: true` single host path |
| Task history | VERIFIED | `events` buffer → `history_length` |
| All four workflows | VERIFIED | `a2a_workflows_e2e` |
| WS A2A subscribe | VERIFIED | `a2a_ws_events` |
| Host call budget | VERIFIED | spawn non-blocking default |

---

## Feature-level matrix

### `A2aPayload` variants

| Variant | Runtime | Blackboard | Test |
| --- | --- | --- | --- |
| `TaskDelegation` | Complete | Yes | blackboard_payloads, workflows |
| `StrategyProposal` | Complete | Yes | codec_contract, consensus |
| `CodeInsight` | Complete | Yes | consensus_deadlock |
| `GraphMutationSignal` | Complete | Yes | bus_supervisor, blackboard_payloads |
| `Accept` / `Reject` | Complete | Yes | consensus_deadlock |
| `FinalResult` | Complete | Yes | consensus_deadlock |

### Workflows

| Workflow | Hub | E2E test |
| --- | --- | --- |
| `consensus_review` | Yes | Yes |
| `patch_plan` | Yes | Yes |
| `impact_review` | Yes | Yes |
| `pr_review` | Yes | Yes |

---

## CI

Job **`a2a-graph`** in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml): FalkorDB service container, `CORTEX_TEST_GRAPH=1`, blackboard load + network E2E (no `continue-on-error`).

---

## Related artifacts

- Architecture: [docs/A2A.md](A2A.md)
- Design spec: [docs/superpowers/specs/2026-06-01-a2a-100-percent-design.md](superpowers/specs/2026-06-01-a2a-100-percent-design.md)
- MCP audit: [docs/MCP_AUDIT_FINDINGS.md](MCP_AUDIT_FINDINGS.md)
- Cursor rule: [`.cursor/rules/codecortex-a2a.mdc`](../.cursor/rules/codecortex-a2a.mdc)
