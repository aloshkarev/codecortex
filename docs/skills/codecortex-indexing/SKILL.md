---
name: codecortex-indexing
description: Guides graph and vector indexing, incremental updates, directory watch, background jobs, and freshness repair for CodeCortex. Use when the user mentions index, reindex, stale index, freshness, watch, vector-index, FalkorDB, Memgraph, Grafeo, add_code_to_graph, incremental indexing, index_status, or background jobs. Prefer MCP tools for agent-driven repair; use cortex CLI for operator bootstrap.
---

# CodeCortex Indexing Skill

*Canonical path: `docs/skills/codecortex-indexing/`; symlinked for Cursor at `.cursor/skills/codecortex-indexing`.*

## Overview

Keeps the graph and optional vector indexes trustworthy so analysis and context tools return accurate `freshness`. Pair with [codecortex](../codecortex/SKILL.md) for queries after index is healthy.

**Delegate to subagent:** multi-step repair or bootstrap → [codecortex-indexer](../../agents/codecortex-indexer.md) ([agents README](../../agents/README.md)).

**Discover first:** `resources/read` → `codecortex://guide/agent-workflows`; for freshness semantics use `explain_index_freshness`.

## Bootstrap (operator)

```bash
cortex doctor
cortex index /path/to/repo --force
cortex vector-index /path/to/repo   # optional, for semantic/hybrid search
cortex mcp start
```

MCP equivalent preflight: `workspace_setup`, `check_health`, then `index_status` / `vector_index_status`.

## Core workflow

| Step | CLI | MCP |
| --- | --- | --- |
| Verify health | `cortex doctor` | `check_health` |
| Graph index | `cortex index <repo> [--force] [--profile conservative]` | `add_code_to_graph` (default `force: true`) |
| Vector index | `cortex vector-index <repo>` | `vector_index_repository` (background) |
| Status | `cortex stats`, `cortex list` | `index_status`, `vector_index_status` |
| Explain freshness | — | `explain_index_freshness` |
| Incremental | `cortex index --mode incremental-diff` | `add_code_to_graph` with `force: false` only when using hash cache intentionally |
| Watch | `cortex watch <path>` | `watch_directory` / `unwatch_directory` / `list_watched_paths` |
| Background work | `cortex jobs` | `list_jobs`, `check_job_status` |
| Repair stale | `cortex index <repo> --force` | prompt `codecortex_freshness_repair`; `diagnose` |

## Incremental rules

- `incremental-diff` uses the Git changed-file set, not a full tree walk.
- If the diff includes **deleted** source files, CodeCortex falls back to a **full forced** rebuild so stale graph nodes are removed.
- MCP `add_code_to_graph`: default is a forced graph pass; pass `force: false` only when intentionally relying on the local hash cache.

## `.cortexignore`

| File | Purpose |
| --- | --- |
| `~/.cortex/cortexignore` | Global ignores for all repos (`global_cortexignore_path` in config overrides) |
| `<repo>/.cortexignore` | Repo-local ignores (nested files supported; gitignore semantics including `!`) |

Discovery order: global → `.gitignore` → nested `.cortexignore` → build-detected excludes → `index_exclude_patterns` / project policy / CLI `--exclude-pattern`.

**Roots:** `repo_root` loads the full ignore stack (git root when available). `scan_root` limits which files are emitted (e.g. monorepo subfolder index). `is_ignored()` and `collect_files()` share one `WalkBuilder` configuration for consistent hierarchical behavior.

After editing ignore files, run `cortex index --force` and `cortex vector-index` so graph and vector tiers stay aligned.

Watch: use `SmartWatchConfig::for_project_path(path)` or `daemon::smart_watch_config_for_project(path)` so `repo_root`, `scan_root`, and policy excludes are wired. Set `use_cortexignore: true` (default).

## After indexing

Before graph or hybrid tools, confirm:

1. `index_status` shows graph ready for the target repo/branch.
2. For `vector_search` / `vector_search_hybrid`, `vector_index_status` is ready.
3. Context tool responses report acceptable `freshness` (not `stale` / `partial` / `unknown` for impact-heavy work).

## Freshness repair chain

1. `explain_index_freshness`
2. `diagnose`
3. `index_status` and `vector_index_status`
4. Reindex: CLI `cortex index --force` and/or MCP `add_code_to_graph` with `force: true`
5. Poll `list_jobs` / `check_job_status` for background vector or `project_sync`
6. Re-verify `check_health` and `index_status`

Use MCP prompt `codecortex_freshness_repair` when the client supports prompts.

## Privacy

- Prefer local MCP stdio and local embeddings for private repositories.
- Remote embeddings or remote MCP are explicit opt-in only.

## Project context and reindex

- **FalkorDB** is the default graph backend (`backend_type = "falkordb"`). Graph keys use the **project scan path** (`graph_repository_path_for_index`), not only the git root.
- **CLI** `cortex project set` defaults to `--auto-index` with `index_on_switch: true`; indexing is skipped when the registry already has the current branch/commit.
- **MCP** `set_current_project` only switches registry context (no implicit index). Use `add_code_to_graph` or `project_sync` when you need a refresh.
- Daemon jobs must use the same `repository_path` as indexing. If the daemon is stopped with pending jobs, run `cortex daemon clear-queue` then `cortex daemon start`.

## Indexing profiles

| Profile | When | How |
| --- | --- | --- |
| `highspeed` | Default on dev hosts (≥8 GiB RAM) | Built into `CortexConfig::default()` / `cortex index` |
| `conservative` | Laptops, memory pressure | `cortex index --profile conservative`, `CORTEX_INDEX_PROFILE=conservative`, or TOML `indexing_profile = "conservative"` |

`cortex doctor` prints the active profile, CPU count, write pool size, and points to `cortex index-report analyze`.

Benchmarks and pass gates: [audit/index-perf/README.md](../../../audit/index-perf/README.md).

## Force + branch: deferred node replay

On a **git branch**, `cortex index --force` spills parsed nodes to disk and replays them in **`phase_deferred_node_write`** after branch delete. This is often the slowest phase on large repos (e.g. linux kernel).

| Mitigation | Command / config |
| --- | --- |
| Prefer incremental | `cortex index <repo>` or `--mode incremental-diff` |
| Skip deferred replay | `cortex index --force --wipe-branch-first` or `index_force_delete_branch_before_parse = true` in `~/.cortex/config.toml` |
| Smaller tree | `index_include_files`, `--include-file`, `index_exclude_patterns` |

Analyze: `cortex index-report analyze --file report.json` — check `deferred_*` fields and deferred fraction in heuristics.

## Guardrails

- Do not run expensive graph analysis when `index_status` or `freshness` indicates stale data; repair first.
- Background tools (`vector_index_repository`, `project_sync`) may take minutes; poll jobs instead of assuming completion.
- Watch mode keeps indexes warm; unwatch when tearing down dev environments.
- On large multi-repo FalkorDB instances, set the current project (or `include_paths`) so analysis queries filter by `repository_path`.

## Progressive disclosure

- [references/backends-and-config.md](references/backends-and-config.md)
- [references/vector.md](references/vector.md)
- Analysis routing after index is healthy: [codecortex](../codecortex/SKILL.md)
