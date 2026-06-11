# Project Management Execution Checklist

## Phase 0: Baseline and Migration Safety
- [ ] Define migration strategy for daemon/runtime DB (`~/.cortex/daemon/daemon.db`).
- [ ] Add versioned schema migrations and rollback notes.
- [ ] Add feature flags for daemon-backed workflows.

## Phase 1: Persistent Daemon Foundation
- [x] Add daemon runtime paths + lifecycle (`start`, `stop`, `status`, `run`).
- [x] Add persistent SQLite runtime DB (WAL mode).
- [x] Add daemon heartbeat persistence.
- [x] Add persistent index job queue table.
- [x] Add queue dedupe by `(repo, branch, commit, mode)` key.
- [x] Expose CLI commands: `cortex daemon start|stop|status`.
- [ ] Add socket RPC contract and request routing.

## Phase 2: Daemon-Backed Watch/Index
- [x] Route `cortex watch` to daemon queue API.
- [x] Route `cortex index` to daemon queue API.
- [x] Recover watch sessions and queued jobs on daemon restart.
- [x] Add job retry/backoff policy and stale job recovery.

## Phase 3: Branch Health and Project Status
- [x] Persist per-branch health (`indexed_commit`, `current_commit`, `is_stale`, `last_indexed_at`).
- [ ] Sync branch health with Memgraph `BranchIndex` records.
- [x] Add `cortex project status` with freshness/queue/error health.

## Phase 4: Branch Policy Controls
- [x] Extend project policy with `index_only`.
- [x] Extend project policy with `exclude_patterns`.
- [x] Extend project policy with `max_parallel_index_jobs`.
- [x] Add CLI `project policy show|set`.
- [x] Enforce policy in daemon scheduler.

## Phase 5: Incremental Branch-Diff Indexing
- [x] Add index mode selector `full|incremental-diff`.
- [x] Compute merge-base + changed file plan.
- [x] Fall back to full index when merge-base unavailable/unreliable.
- [x] Add planner stats to job output for observability.

## Phase 6: Explicit Project Sync Workflow
- [x] Add `cortex project sync`.
- [x] Implement flow: refresh -> detect switch -> enqueue index -> cleanup old branches.
- [x] Return stage-by-stage status in command output.

## Phase 7: Observability
- [x] Add metrics: watch latency, index duration, queue wait, dropped events, branch switch count.
- [ ] Persist counters and rolling windows in daemon DB.
- [x] Add `cortex project metrics`.
- [ ] Add tracing correlation IDs per project/job.

## Phase 8: Regression Test Matrix
- [ ] Temp git repo scenarios: branch switch with pending jobs.
- [ ] Watcher simulation: burst edits + dedupe + branch switch.
- [ ] Daemon restart recovery scenarios.
- [ ] Incremental-diff correctness vs full-index baseline.

## Cortex-MCP Integration Checklist
- [x] Add MCP tool: `project_status`.
- [x] Add MCP tool: `project_sync`.
- [x] Add MCP tool: `project_branch_diff`.
- [x] Add MCP tool: `project_queue_status`.
- [x] Add MCP tool: `project_metrics`.
- [ ] Add freshness metadata (`branch`, `commit`, `indexed_commit`, `is_stale`) in all retrieval tools.
- [ ] Add stale-data guardrails: auto-sync or explicit stale warning.
- [ ] Add token-saving retrieval defaults: structured summary first, lazy source expansion second.
