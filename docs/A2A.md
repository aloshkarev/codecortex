# CodeCortex A2A hybrid architecture

CodeCortex combines **MCP** (host entry) with **A2A v1.0** (agent-to-agent) and a **graph blackboard** (`AgentInsight` nodes in FalkorDB).

## Configure (file only)

Edit `~/.cortex/config.toml`. A2A is **not** configured via environment variables.

```toml
[mcp]
profile = "dev"

[mcp.network]
transport = "multi"
listen = "127.0.0.1:3001"
bearer_token_file = "~/.cortex/mcp.token"

[mcp.tools]
a2a_spawn_session = true

[a2a]
enabled = true
force_in_process = false   # when true, all hub roles run in-process (ignores per-role mode)
max_parallel_roles = 4
consensus_max_rounds = 3
max_negotiation_rounds = 3
task_store = "memory"   # or "sled" with task_store_path = "~/.cortex/a2a/tasks"

[a2a.blackboard]
enabled = true
write_batch_size = 4096

[a2a.roles.analyzer]
mode = "in_process"
subscriptions = ["TaskDelegation", "GraphMutationSignal", "CodeInsight"]
capabilities = ["CodeInsight", "Reject", "Accept"]
skills = ["get_impact_graph", "analyze_code_relationships"]

[a2a.roles.patch_planner]
mode = "external"
agent_card_url = "http://127.0.0.1:3001/.well-known/agents/patch-planner.json"

[a2a.server]
http_enabled = true
base_path = "/a2a/v1"
protocol_version = "1.0"
grpc_enabled = true
grpc_listen = "127.0.0.1:50051"

[a2a.push]
enabled = false   # set true in production via apply_production_profile or [a2a.push].enabled = true

[a2a.host_guard]
max_cypher_rows = 50
```

**Production profile** (enables push + HTTP ingress):

```toml
[mcp]
profile = "production"

# Or manually:
# config.a2a.apply_production_profile() equivalent:
[a2a.push]
enabled = true
[a2a.server]
http_enabled = true
```

Agent markdown under `docs/agents/*.md` is parsed at hub startup (Tier 2); **config.toml wins** on conflicts.

## Host workflow

1. Enable `[a2a]` and `mcp.tools.a2a_spawn_session`.
2. `cortex mcp start` with `mcp.network.transport = "multi"` for graph-backed roles (`McpA2aServices`). Stdio-only MCP uses `A2aHub` with null services unless the network server injects a shared hub.
3. Call MCP tool **`cortex_a2a_spawn_session`** (or `manage_codecortex` with `action = "spawn_a2a_session"`).
4. Subscribe via **`cortex_a2a_subscribe_task`** (SSE), WebSocket **`/a2a/v1/ws`**, or poll **`cortex_a2a_get_task`**. During workflows, the hub emits **`artifactUpdate`** events (`TaskArtifactUpdateEvent`) when intelligence packs or delegations are produced — read `artifact.metadata.mcpToolId` and `task.metadata.suggestedNextTools`, then call those MCP tools on the host. Optional final **`cortex_a2a_get_task`** with `include_artifacts: true` (default) retrieves full spec-shaped artifacts; set `include_artifacts: false` to omit the `artifacts` key per spec §2.3.

### MCP tools (host)

| Tool | Purpose |
| --- | --- |
| `cortex_a2a_spawn_session` | Start `consensus_review`, `patch_plan`, `impact_review`, or `pr_review` |
| `cortex_a2a_get_task` | GetTask; optional spec JSON; `include_artifacts` (default true) |
| `cortex_a2a_send_message` | SendMessage (starts consensus by default) |
| `cortex_a2a_cancel_task` | CancelTask |
| `cortex_a2a_list_tasks` | ListTasks |
| `cortex_a2a_subscribe_task` | Poll + SSE subscribe URL |
| `cortex_a2a_list_push_configs` | List push configs when `[a2a.push].enabled` |

Use MCP prompt **`codecortex_a2a_consensus`** for the full spawn → poll checklist.

### Workflows

| Name | Roles | Use when |
| --- | --- | --- |
| `consensus_review` | planner → analyzer → validator | Deadlock / risky patch review loop |
| `patch_plan` | planner + validator | Capsule + contracts without analyzer reject loop |
| `impact_review` | analyzer | Blast-radius review on `include_paths` |
| `pr_review` | pr_reviewer → optional analyzer | Branch/PR merge review with delta capsule + blast radius |

Disable per workflow under `[a2a.workflows.*]` in config.

## Symmetric hybrid topology

**Symmetric hybrid** means the same hub orchestrates in-process runners and external A2A agents: workflows stay hub-driven, but each role can run locally or over HTTP/gRPC per `[a2a.roles.*].mode`.

| Component | Role |
| --- | --- |
| **`try_build_a2a_hub`** (`cortex-mcp` `a2a_services.rs`) | Lazy graph-backed hub init for **stdio and network** MCP when `[a2a] enabled = true`. Attaches `McpA2aServices` and blackboard when FalkorDB connects; degrades gracefully when the graph is unreachable. |
| **External reply collection** (`cortex-a2a` `runtime/external.rs`) | For `mode = "external"`, `RoleGateway::dispatch_sync` POSTs SendMessage, then polls GetTask / SSE until terminal state and decodes reply envelopes from task history and blackboard extension parts. |
| **Bus supervisor** (`cortex-a2a` `runtime/supervisor.rs`) | One tokio task per role inbox; consumers run registered `RoleRunner`s and republish replies on the bus. Index promotion (`notify_index_promotion`) dispatches `GraphMutationSignal` to analyzer for re-validation. |

Stdio-only MCP previously used a null-services hub; with `try_build_a2a_hub`, stdio gets the same graph-backed services and blackboard as the multi-transport network server when the graph is up.

External roles require reachable `agent_card_url` endpoints. Set `[a2a].force_in_process = true` to ignore per-role `mode` and run all steps in-process (tests or all-local deployments).

## HTTP endpoints

| Route | Purpose |
| --- | --- |
| `POST /a2a/v1/message:send` | A2A SendMessage |
| `POST /a2a/v1/message:stream` | SendStreamingMessage (SSE) |
| `GET /a2a/v1/tasks` | ListTasks (`context_id`, pagination query) |
| `GET /a2a/v1/tasks/{id}` | GetTask |
| `GET /a2a/v1/tasks/{id}/subscribe` | SubscribeToTask (SSE `StreamResponse`) |
| `GET /a2a/v1/ws` | WebSocket A2A task events (`a2a_subscribe` JSON) |
| `POST /a2a/v1/tasks/{id}/cancel` | CancelTask |
| `GET/POST /a2a/v1/tasks/{id}/pushNotificationConfigs` | List / create push configs |
| `GET/DELETE /a2a/v1/tasks/{id}/pushNotificationConfigs/{configId}` | Get / delete push config |
| `GET /.well-known/agent-card.json` | Gateway AgentCard |
| `GET /.well-known/agents/{role}.json` | Per-role AgentCard (served as `.../agents/{role}.json` with `.json` in the `{role_file}` segment) |

All ingress routes honor `A2A-Version` and return spec error JSON (`TaskNotFoundError`, `VersionNotSupportedError`, etc.) on failure.

## Wire model

HTTP/MCP JSON uses camelCase wire types in `cortex-a2a::wire` (aligned with `docs/a2a.proto`). `spec_codec` converts wire ↔ prost for gRPC; pbjson serde is generated at build time for contract drift detection.

## gRPC (spec §10)

When `[a2a.server].grpc_enabled = true`, `A2AService` listens on `grpc_listen` (default `127.0.0.1:50051`) with the same hub as HTTP.

## Push webhooks

Set `[a2a.push].enabled = true` for `TaskPushNotificationConfig` CRUD and outbound task update POSTs when external agents cannot hold SSE/gRPC streams. Signing uses `signing_secret_path` when configured.

## Intelligence backing (consensus)

In-process roles call the **same** graph-backed intelligence module as MCP tools (`crates/cortex-mcp/src/intelligence/` via `McpA2aServices`):

| Role step | MCP equivalent | Notes |
| --- | --- | --- |
| Patch planner | `get_patch_context` | Token-bounded targets, contracts, likely tests; capsule URI in task artifacts |
| Analyzer | `get_impact_graph` | Depth-bounded callers/transitive blast radius; path-scoped freshness on spawn |
| PR reviewer | `get_delta_context` + impact | When `source_branch` / `target_branch` set on spawn |
| Validator | `validate_build` | `cargo check`; optional `[a2a].require_fresh_index = true` gate |

**Spawn scope fields:** `include_paths`, `exclude_paths`, `target_symbol`, `source_branch`, `target_branch`, `mode`. Response includes `freshness`, `suggested_next_tools`, and intelligence capsule artifacts in task history.

**Host cooperation loop:** spawn once → read `suggested_next_tools` → call those MCP tools on the host for bounded context → poll `cortex_a2a_get_task` → read `intelligence_pack` / `tool_delegation` artifacts. External roles (`agent_card_url` set) receive `tool_delegation` artifacts listing MCP tools the host should invoke.

**IntelligencePack:** A2A artifacts and MCP envelopes share the same inner `data` plus metadata (`freshness`, `warnings`, `suggested_next_tools`, `mcp_tool_id`) via `crates/cortex-mcp/src/intelligence/pack.rs`.

**When to spawn A2A vs MCP:** use A2A for multi-role loops (consensus, PR review with analyzer confirmation). Use MCP tools directly for single bounded reads (`get_patch_context`, `get_impact_graph`).

**Deadlock demo:** set `[a2a.workflows.consensus_review].demo_fixture = true` or reference `transport_deadlock` in the task to enable spin-lock negotiation strategies (default is indexed patch planning).

When the graph is empty or unreachable, impact analysis degrades gracefully (lower confidence). Treat spawn `freshness` and MCP `index_status` as authoritative before high-confidence claims.

Hub workflows route through `RoleGateway::dispatch_sync`. By default (`force_in_process = false`), each step uses the role's configured `mode`: `in_process` runs local runners; `external` POSTs to the role's A2A HTTP surface derived from `agent_card_url` (required for external roles — without it dispatch returns no replies). Set `force_in_process = true` to run every role in-process regardless of per-role mode (useful for tests or all-local deployments).

`consensus_review` uses the same dispatch rules: with default config, `patch_planner` and `validator` are external and need reachable `agent_card_url` endpoints; `analyzer` stays in-process. The blackboard caches analyzer insights; `list_insights` skips duplicate impact work per session. Insights older than `[a2a].insight_ttl_secs` are pruned at workflow start.

## External role client

```bash
# SendMessage (returnImmediately)
cargo run -p cortex-a2a --bin a2a-role-client -- patch_planner "Propose patch"

# SSE SubscribeToTask after spawn
cargo run -p cortex-a2a --bin a2a-role-client -- --subscribe patch_planner "Review transport deadlock"
```

## Protocol references

- Normative spec: [docs/a2a.proto](a2a.proto), [docs/specification.md](specification.md)
- **Codegen (HTTP + gRPC):** [`docs/a2a.proto`](a2a.proto) via `prost` / `tonic` (`docs/google/api/` vendored for `google.api.*` imports). Hand JSON types remain in `cortex-a2a::wire` with contract tests against generated prost types.
- Extension URI: `https://codecortex.dev/extensions/blackboard/v1`

## Load / CI testing

Blackboard load test `crates/cortex-graph/tests/a2a_blackboard_load.rs` is `#[ignore]` by default. It asserts **under 2.0 ms per insight** when the graph is up. Run locally with a live graph:

```bash
CORTEX_TEST_GRAPH=1 cargo test -p cortex-graph --test a2a_blackboard_load -- --ignored
```

**CI:** workflow job `a2a-graph` provisions a FalkorDB service container and runs blackboard + network E2E tests with `CORTEX_TEST_GRAPH=1`.

## Crates

| Crate | Role |
| --- | --- |
| `cortex-a2a` | Envelope codec, bus, task store, hub workflows |
| `cortex-graph` | Blackboard schema and writes |
| `cortex-mcp` | Gateway, HTTP routes, MCP meta-tool |
