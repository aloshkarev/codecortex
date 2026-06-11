# A2A Symmetric Hybrid (Option C) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the gaps in [`docs/A2A_COMPLETENESS.md`](../A2A_COMPLETENESS.md) to reach **full symmetric hybrid** — external roles participate in workflows, async bus drives role actors, graph blackboard works on all MCP transports, `PrReviewer` is first-class, and CI proves load/E2E behavior.

**Architecture:** Keep the existing hub + `RoleGateway` + in-process runners; add (1) **lazy graph-backed hub init** for stdio MCP when FalkorDB is reachable, (2) **external dispatch with reply polling** via A2A GetTask/SSE, (3) **bus supervisor** tasks per registered role consuming inboxes, (4) **`pr_review` workflow** with `PrReviewerRunner`, (5) expanded blackboard + task history from hub events. Workflows remain hub-orchestrated but can delegate to external agents symmetrically.

**Tech Stack:** Rust (cortex-a2a, cortex-mcp, cortex-graph, cortex-cli), tokio mpsc/broadcast, reqwest, FalkorDB Cypher, GitHub Actions CI.

**Baseline:** Audit ~65% complete; target **≥90%** on pillars P2–P7 after this plan.

---

## File map (create / modify)

| File | Responsibility |
| --- | --- |
| `crates/cortex-a2a/src/runtime/external.rs` | **Create** — poll external A2A task until reply envelopes decoded |
| `crates/cortex-a2a/src/runtime/supervisor.rs` | **Create** — spawn inbox consumer tasks per role |
| `crates/cortex-a2a/src/runtime/gateway.rs` | External `dispatch_sync` returns collected replies |
| `crates/cortex-a2a/src/hub.rs` | Workflow flags, history, index promotion dispatch, `pr_review` |
| `crates/cortex-a2a/src/session.rs` | Populate `TaskWire.history` from recorded events |
| `crates/cortex-a2a/src/services.rs` | Blackboard mapping for `GraphMutationSignal`, `FinalResult` summary |
| `crates/cortex-a2a/src/runtime/runners.rs` | `PrReviewerRunner`; graph-backed analyzer paths |
| `crates/cortex-mcp/src/handler.rs` | Lazy `build_a2a_hub` when graph connect succeeds |
| `crates/cortex-mcp/src/a2a_services.rs` | Shared helper for stdio + network hub build |
| `crates/cortex-cli/tests/a2a_external_roundtrip.rs` | **Create** — HTTP mock external role |
| `crates/cortex-cli/tests/a2a_workflows_e2e.rs` | **Create** — patch_plan + impact_review + pr_review |
| `crates/cortex-cli/tests/a2a_host_call_budget.rs` | **Create** — documents spawn-only vs spawn+poll |
| `crates/cortex-graph/tests/a2a_blackboard_load.rs` | Assert per-insight ms SLO |
| `.github/workflows/ci.yml` | Optional `a2a-graph` job with `CORTEX_TEST_GRAPH=1` |
| `docs/agents/codecortex-indexer.md` | A2A subscriptions/capabilities |
| `docs/agents/codecortex-pr-reviewer.md` | A2A subscriptions/capabilities |
| `docs/A2A.md` | Symmetric hybrid topology, stdio graph attach |
| `.cursor/rules/codecortex-a2a.mdc` | Cross-link from subagents; note alwaysApply tradeoff |
| `docs/skills/codecortex-workflows/SKILL.md` | `pr_review` workflow + external role notes |

---

## Milestone 1 — Graph-backed hub on stdio MCP

### Task 1: Lazy hub with McpA2aServices when graph is up

**Files:**
- Modify: `crates/cortex-mcp/src/handler.rs` (~827–834)
- Modify: `crates/cortex-mcp/src/a2a_services.rs`
- Test: `crates/cortex-mcp/tests/a2a_hub_stdio_graph.rs` (create)

- [ ] **Step 1: Write failing test**

```rust
// crates/cortex-mcp/tests/a2a_hub_stdio_graph.rs
#[tokio::test]
async fn stdio_handler_uses_graph_hub_when_connect_succeeds() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }
    let mut config = CortexConfig::default();
    config.a2a.enabled = true;
    let hub = crate::a2a_services::try_build_a2a_hub(&config).await;
    assert!(hub.is_some());
    assert!(hub.unwrap().blackboard().is_some());
}
```

- [ ] **Step 2: Run test — expect FAIL**

Run: `CORTEX_TEST_GRAPH=1 cargo test -p cortex-mcp --test a2a_hub_stdio_graph -- --ignored`

- [ ] **Step 3: Implement `try_build_a2a_hub`**

In `a2a_services.rs`, extract from `build_a2a_hub`:

```rust
pub async fn try_build_a2a_hub(config: &CortexConfig) -> Option<Arc<A2aHub>> {
    if !config.a2a.enabled {
        return None;
    }
    let client = GraphClient::connect(config).await.ok()?;
    let writer = Arc::new(BlackboardWriter::new(client.clone(), config.a2a.blackboard.write_batch_size));
    if config.a2a.blackboard.enabled {
        let _ = writer.ensure_schema().await;
    }
    let services = Arc::new(McpA2aServices::new(config.clone()));
    Some(Arc::new(A2aHub::with_services(
        config.a2a.clone(),
        services,
        Some(writer),
        Some(default_repo_path().into()),
    )))
}
```

Change `CortexHandler::new_with_feature_flags` to `try_build_a2a_hub(...).await.unwrap_or_else(|| Arc::new(A2aHub::new(...)))` — requires async constructor path or `block_on` in sync `new` (prefer new `CortexHandler::new_async` used by MCP startup only; keep sync fallback to null hub).

- [ ] **Step 4: Wire MCP startup**

In `handler.rs` `start_with_options` / stdio path (~6171), use `try_build_a2a_hub` same as network server.

- [ ] **Step 5: Run tests**

Run: `CORTEX_TEST_GRAPH=1 cargo test -p cortex-mcp --test a2a_hub_stdio_graph -- --ignored`

- [ ] **Step 6: Commit**

```bash
git add crates/cortex-mcp/src/a2a_services.rs crates/cortex-mcp/src/handler.rs crates/cortex-mcp/tests/a2a_hub_stdio_graph.rs
git commit -m "feat(a2a): attach graph hub and blackboard on stdio when FalkorDB available"
```

---

### Task 2: Task history from hub events

**Files:**
- Modify: `crates/cortex-a2a/src/session.rs`
- Modify: `crates/cortex-a2a/src/hub.rs` (`get_task_wire_with_history`)
- Test: extend `crates/cortex-a2a/tests/proto_contract.rs`

- [ ] **Step 1: Add `events_for_task` on hub**

Store per-task event slices or filter global `events` by `task_id` in `get_task_wire_with_history`.

- [ ] **Step 2: Map events → `A2aMessage` history in `to_wire_with_history`**

```rust
// session.rs — accept optional history messages
pub fn to_wire_with_history(
    &self,
    history_messages: &[A2aMessage],
    history_length: Option<i32>,
) -> TaskWire {
    let mut wire = /* existing fields */;
    wire.history = history_messages.to_vec();
    if let Some(n) = history_length {
        let n = n.max(0) as usize;
        if wire.history.len() > n {
            wire.history = wire.history.split_off(wire.history.len() - n);
        }
    }
    wire
}
```

Use `codec::envelope_to_message` for each recorded envelope in hub.

- [ ] **Step 3: Test history_length truncation**

Add test in `proto_contract.rs` spawning session, recording 3 events, asserting `get_task_wire_with_history(..., Some(2))` returns 2 history entries.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(a2a): populate TaskWire history from hub events"
```

---

## Milestone 2 — External role symmetry

### Task 3: External reply collector

**Files:**
- Create: `crates/cortex-a2a/src/runtime/external.rs`
- Modify: `crates/cortex-a2a/src/runtime/gateway.rs`
- Modify: `crates/cortex-a2a/src/runtime/mod.rs`
- Test: `crates/cortex-cli/tests/a2a_external_roundtrip.rs`

- [ ] **Step 1: Write failing integration test with httpmock**

```rust
// Mock POST message:send → returns task id; GET tasks/{id} → completed task with CodeInsight in artifact
#[tokio::test]
async fn external_dispatch_sync_returns_decoded_replies() {
    let server = MockServer::start_async().await;
    // ... configure patch_planner role to server.uri()
    let gateway = /* RoleGateway with external patch_planner */;
    let replies = gateway.dispatch_sync(envelope, &ctx).await.unwrap();
    assert!(!replies.is_empty());
    assert!(matches!(replies[0].payload, A2aPayload::CodeInsight { .. }));
}
```

- [ ] **Step 2: Implement `ExternalReplyCollector`**

```rust
// external.rs
pub async fn send_and_collect(
    client: &reqwest::Client,
    send_url: &str,
    get_task_url: &str,
    envelope: &A2aEnvelope,
    timeout: Duration,
) -> Result<Vec<A2aEnvelope>> {
    // POST message:send with returnImmediately: true
    // Parse task.id from response
    // Poll GET tasks/{id} until terminal or timeout
    // Decode blackboard extension parts → A2aEnvelope replies
}
```

- [ ] **Step 3: Replace empty return in `dispatch_sync` external branch**

```rust
A2aRoleMode::External => {
    self.collect_external_replies(envelope, ctx).await
}
```

- [ ] **Step 4: Add config `[a2a.roles.*].reply_timeout_secs` default 30**

In `crates/cortex-core/src/a2a_config.rs`.

- [ ] **Step 5: Run test + existing consensus**

Run: `cargo test -p cortex-cli --test a2a_external_roundtrip`
Run: `cargo test -p cortex-cli --test a2a_consensus_deadlock`

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(a2a): collect external role replies in dispatch_sync"
```

---

### Task 4: Respect role mode in hub (remove force_in_process default)

**Files:**
- Modify: `crates/cortex-a2a/src/hub.rs` (`role_ctx`)
- Modify: `crates/cortex-a2a/src/runtime/context.rs`
- Test: `a2a_external_roundtrip` + config fixture

- [ ] **Step 1: Change `force_in_process` default to `false`**

Hub sets `force_in_process: true` only for tests or `[a2a].force_in_process = true` config flag (new, default false).

- [ ] **Step 2: Document in `docs/A2A.md`**

External roles require reachable agent-card URLs; in-process fallback when `mode = in_process`.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(a2a): allow external role modes in production workflows"
```

---

## Milestone 3 — Async bus supervisor

### Task 5: Role inbox consumers

**Files:**
- Create: `crates/cortex-a2a/src/runtime/supervisor.rs`
- Modify: `crates/cortex-a2a/src/hub.rs` (start supervisor on hub init)
- Test: `crates/cortex-a2a/tests/bus_supervisor.rs`

- [ ] **Step 1: Write test — publish to analyzer inbox, consumer handles**

Register `AnalyzerRunner` on bus; publish `GraphMutationSignal`; assert handler invoked and blackboard write attempted (mock services).

- [ ] **Step 2: Implement `BusSupervisor::spawn`**

For each role in `build_runners()`, `bus.register_role(role, buffer)` and spawn:

```rust
tokio::spawn(async move {
    while let Some(env) = rx.recv().await {
        if let Ok(replies) = runner.handle((*env).clone(), &ctx).await {
            for r in replies { bus.publish(r).await; }
        }
    }
});
```

Use shared `RoleContext` per session or session-scoped context map keyed by `task_id`.

- [ ] **Step 3: Wire `notify_index_promotion` to dispatch GraphMutationSignal to Analyzer**

After `record_event`, call `gateway.dispatch` (async) or publish to bus for analyzer inbox.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(a2a): bus supervisor consumes role inboxes and index promotion dispatches"
```

---

## Milestone 4 — PrReviewer + pr_review workflow

### Task 6: PrReviewerRunner

**Files:**
- Modify: `crates/cortex-a2a/src/runtime/runners.rs`
- Modify: `crates/cortex-a2a/src/hub.rs`
- Modify: `crates/cortex-core/src/a2a_config.rs` (workflow template)
- Test: `crates/cortex-cli/tests/a2a_workflows_e2e.rs`

- [ ] **Step 1: Write failing E2E test**

```rust
#[tokio::test]
async fn pr_review_workflow_completes() {
    let hub = test_hub_with_services();
    let resp = hub.spawn_session(SpawnSessionRequest {
        task: "review branch".into(),
        workflow: "pr_review".into(),
        include_paths: vec!["crates/cortex-a2a".into()],
        ..Default::default()
    }).unwrap();
    // poll until completed
    assert!(artifact_contains_delta_context(&task));
}
```

- [ ] **Step 2: Implement `PrReviewerRunner`**

Handle `TaskDelegation` with focus on delta/impact:
- Call `services.get_delta_context` or facade equivalent
- Emit `CodeInsight` with risk + summary
- `Accept` / `Reject` based on blast radius thresholds

Add to `build_runners()`.

- [ ] **Step 3: Add `run_pr_review` in hub**

Roles: `PrReviewer` → optional `Analyzer` for impact confirmation → `FinalResult`.

Register workflow in spawn validation (same pattern as `impact_review`).

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(a2a): add PrReviewerRunner and pr_review workflow"
```

---

### Task 7: Agent docs + MCP tool surface

**Files:**
- Modify: `docs/agents/codecortex-indexer.md`
- Modify: `docs/agents/codecortex-pr-reviewer.md`
- Modify: `docs/skills/codecortex-workflows/SKILL.md`
- Modify: `crates/cortex-mcp/tests/tool_surface_matrix.rs` (if new workflow constant)
- Modify: `docs/A2A.md`

- [ ] **Step 1: Add A2A sections to indexer and pr-reviewer docs** (mirror analyzer format)

Indexer subscriptions: `GraphMutationSignal` outgoing; capabilities: index promotion signals.

Pr-reviewer subscriptions: `TaskDelegation`; capabilities: `CodeInsight`, `FinalResult`.

- [ ] **Step 2: Document `pr_review` in A2A.md workflows table**

- [ ] **Step 3: Run `./plugin/codecortex/scripts/sync-from-docs.sh`**

- [ ] **Step 4: Commit**

```bash
git commit -m "docs(a2a): indexer and pr-reviewer A2A manifests plus pr_review workflow"
```

---

## Milestone 5 — Blackboard completeness

### Task 8: Expand blackboard payload mapping

**Files:**
- Modify: `crates/cortex-a2a/src/services.rs`
- Test: `crates/cortex-graph/tests/a2a_blackboard_payloads.rs` (create)

- [ ] **Step 1: Map `GraphMutationSignal` → `write_mutation_hint`**

- [ ] **Step 2: Map `FinalResult` → compact `AgentInsight` summary (no full patch body)**

- [ ] **Step 3: Integration test writes all payload types, counts nodes**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(a2a): blackboard records mutation hints and final result summaries"
```

---

### Task 9: Blackboard load SLO + CI

**Files:**
- Modify: `crates/cortex-graph/tests/a2a_blackboard_load.rs`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Assert SLO in test**

```rust
assert!(per_insight_ms < 2.0, "SLO violated: {per_insight_ms} ms/insight");
```

- [ ] **Step 2: Add CI job `a2a-graph` (optional service container or self-hosted FalkorDB)**

```yaml
a2a-graph:
  runs-on: ubuntu-latest
  if: github.event_name == 'push' || github.event_name == 'pull_request'
  steps:
    - uses: actions/checkout@v4
    - run: cargo test -p cortex-graph --test a2a_blackboard_load -- --ignored
      env:
        CORTEX_TEST_GRAPH: "1"
```

Document FalkorDB service requirement in job comments if no container.

- [ ] **Step 3: Commit**

```bash
git commit -m "ci(a2a): blackboard load test with 2ms/insight SLO"
```

---

## Milestone 6 — Graph-backed analyzer (reduce fixtures)

### Task 10: Analyzer uses live impact when services present

**Files:**
- Modify: `crates/cortex-a2a/src/runtime/runners.rs` (`AnalyzerRunner`)
- Test: extend `a2a_workflows_e2e.rs` with `CORTEX_TEST_GRAPH=1`

- [ ] **Step 1: Prefer `services.analyze_impact` over spin_lock string heuristic when not NullA2aServices**

Keep heuristic as fallback when graph unavailable.

- [ ] **Step 2: Test impact_review returns non-zero callers on indexed symbol**

Use symbol known in 64-codecortex index (e.g. `A2aHub`).

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(a2a): graph-backed analyzer impact in impact_review workflow"
```

---

## Milestone 7 — Ecosystem rules + host call budget

### Task 11: Rules and subagent cross-links

**Files:**
- Modify: `.cursor/rules/codecortex-subagents.mdc`
- Modify: `.cursor/rules/codecortex-a2a.mdc`
- Modify: `docs/cursor/RULES-INDEX.md`

- [ ] **Step 1: Add to subagents rule:** for PR/kernel/multi-step review → `cortex_a2a_spawn_session` before Task subagent chains.

- [ ] **Step 2: Keep `alwaysApply: false` on a2a rule but add intent-based trigger in `codecortex-workflows` rule frontmatter.**

- [ ] **Step 3: Commit**

```bash
git commit -m "docs(cursor): route multi-step review to A2A spawn_session"
```

---

### Task 12: Host call budget test (documentation enforcement)

**Files:**
- Create: `crates/cortex-cli/tests/a2a_host_call_budget.rs`

- [ ] **Step 1: Test documents contract**

Single spawn with `return_immediately: true` = 1 host call; polling is optional client responsibility. Assert spawn response includes `poll: "get_task"` hint (already present).

- [ ] **Step 2: Add MCP prompt note in `handler_guides.rs` recommending SSE subscribe to reduce polls**

- [ ] **Step 3: Commit**

```bash
git commit -m "test(a2a): document host MCP call budget for spawn_session"
```

---

## Milestone 8 — StrategyProposal (minimal)

### Task 13: Negotiation step in consensus round 0

**Files:**
- Modify: `crates/cortex-a2a/src/hub.rs`
- Modify: `crates/cortex-a2a/src/runtime/runners.rs` (PatchPlanner responds to StrategyProposal)

- [ ] **Step 1: Planner emits `StrategyProposal` before `CodeInsight` on round 0**

- [ ] **Step 2: Gateway or analyzer may `Accept`/`Reject` proposal (optional short-circuit)**

- [ ] **Step 3: Unit test in `crates/cortex-a2a/tests/codec_contract.rs` for StrategyProposal roundtrip**

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(a2a): StrategyProposal negotiation step in consensus_review"
```

---

## Verification gate (definition of done)

Run full matrix:

```bash
cargo test -p cortex-mcp --test tool_surface_matrix --test contract_tests
cargo test -p cortex-a2a -p cortex-cli -p cortex-mcp -p cortex-graph \
  --test proto_contract --test codec_contract --test manifest_registry \
  --test a2a_consensus_deadlock --test a2a_external_roundtrip \
  --test a2a_workflows_e2e --test a2a_host_call_budget \
  --test a2a_http_conformance --test a2a_grpc_contract \
  --test a2a_sse_subscribe
CORTEX_TEST_GRAPH=1 cargo test -p cortex-graph --test a2a_blackboard_load -- --ignored
```

Live MCP (network `multi` transport):

1. `cortex_a2a_spawn_session` × workflows: `consensus_review`, `patch_plan`, `impact_review`, `pr_review`
2. `cortex_a2a_get_task` with `history_length: 5` — non-empty history
3. Cypher: `MATCH (i:AgentInsight) RETURN count(i)` increases after session

Update [`docs/A2A_COMPLETENESS.md`](../A2A_COMPLETENESS.md) scores after implementation.

**Target pillar scores post-C:**

| Pillar | Target |
| --- | --- |
| P2 Symmetric async | ≥85% |
| P3 MCP transport | ≥90% |
| P4 Blackboard | ≥85% |
| P5 Agent ecosystem | ≥90% |
| P6 Meta-tool | ≥90% |
| P7 Tests | ≥85% |

---

## Spec coverage self-review

| Audit gap | Task |
| --- | --- |
| External empty replies | Task 3–4 |
| Bus inboxes unused | Task 5 |
| Stdio NullA2aServices | Task 1 |
| PrReviewer missing | Task 6–7 |
| Task history empty | Task 2 |
| Blackboard partial payloads | Task 8 |
| CI blackboard load | Task 9 |
| patch_plan/impact_review E2E | Task 6, 10 |
| External round-trip test | Task 3 |
| Indexer/pr-reviewer docs | Task 7 |
| StrategyProposal stub | Task 13 |
| Host call count | Task 12 |
| Graph-backed analyzer | Task 10 |

**Deferred (explicit non-goals):** Sled task store, pbjson in proto.rs, push enabled by default, full colon-style axum routes.

---

## Execution handoff

Plan saved to `docs/superpowers/plans/2026-05-31-a2a-symmetric-hybrid.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per milestone, review between tasks  
2. **Inline Execution** — implement milestones sequentially in this session with checkpoints  

Which approach do you want?
