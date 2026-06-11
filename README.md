# CodeCortex

<!-- BEGIN: Implemented Workspace Variant -->

## Implemented Workspace Variant

This workspace is treated as operational or special-case work, not a generic planned portfolio stub.

| Dimension | Position |
| --- | --- |
| Role | implemented MCP/code-intelligence workspace and agent-enabler reference |
| Documentation rule | State what runs today, exact interfaces, known limits, and reusable patterns. |
| Portfolio boundary | Do not expand the active roadmap unless it produces a narrow evidence artifact or reusable crate capability. |

<!-- END: Implemented Workspace Variant -->

CodeCortex is a Rust-based code intelligence stack for local repositories and AI-assisted workflows.

It combines graph indexing, static analysis, optional vector retrieval, and an MCP server in one runtime.

## Core capabilities

- repository indexing into FalkorDB (sole graph backend; configure via `falkordb_uri`, `falkordb_graph`, `falkordb_password`)
- structural analysis (callers, callees, chains, dependencies, dead code, complexity, smells, refactoring)
- project and branch-aware operations
- navigation (`goto`, `usages`, `info`) on indexed graph data
- cross-project operations (`--all-projects`, `--project`, cross-project analyze/search tools)
- MCP tools for assistant clients
- optional vector indexing and semantic search
- language coverage: Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell

## Quick start

```bash
# Enter reproducible dev environment (preferred)
nix develop

# Build CLI package
nix build .#cortex

# Install locally
mkdir -p ~/.local/bin
cp result/bin/cortex ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex

# Configure runtime (FalkorDB — default, out-of-process)
mkdir -p ~/.cortex
docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest
cat > ~/.cortex/config.toml <<'CFG'
backend_type = "falkordb"
falkordb_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
max_batch_size = 4096
CFG

# Verify and index
cortex doctor
cortex index /path/to/repo --force

# Query and analyze
cortex find name GraphClient
cortex analyze callers authenticate
cortex query "MATCH (n:CodeNode) RETURN count(n) AS c"
```

Cargo fallback (without Nix):

```bash
cargo build --release -p cortex-cli
cp target/release/cortex-cli ~/.local/bin/cortex
chmod +x ~/.local/bin/cortex
```

## Command groups

- repository: `index`, `list`, `delete`, `stats`, `watch`, `unwatch`
- search/query: `find`, `query`, `skeleton`, `signature`, `goto`, `usages`, `info`
- analyze: `callers`, `callees`, `chain`, `hierarchy`, `deps`, `dead-code`, `complexity`, `overrides`, `smells`, `refactoring`, `branch-diff`, `review`, `similar`, `shared-deps`, `compare-api`
- vector: `vector-index`, `search`
- project: `project ...`
- mcp: `mcp start`, `mcp tools`
- operations: `doctor`, `config`, `jobs`, `debug`, `daemon`, `interactive`

## implementation highlights

- context-aware smell detection uses project-wide symbol/call/import context with safe fallback to per-file heuristics (`--no-graph` available).
- CLI scope resolution supports single-project and all-project modes; `find`, `search`, and cross-project analyzer/MCP tools operate across registered repositories.
- graph model supports `MEMBER_OF`, `TYPE_REFERENCE`, `FIELD_ACCESS`, and `qualified_name`; navigation APIs and commands (`goto`, `usages`, `info`) are wired in CLI and MCP; branch structural diff and PR-aware review include graph impact signals.

Use these commands after indexing:

```bash
# Single project navigation
cortex goto "GraphClient"
cortex usages "GraphClient"
cortex info "GraphClient"

# Cross-project mode
cortex find name "main" --all-projects
cortex analyze similar --symbol "parse" --min-repos 2

# Structural branch intelligence
cortex analyze branch-diff feature/nav main --structural
```

## Analyze filter flags

- `--file` (alias to include-file)
- `--folder` (aliases: `--dir`, `--directory`; alias to include-path)
- `--include-path`
- `--include-file`
- `--include-glob`
- `--exclude-path`
- `--exclude-file`
- `--exclude-glob`

Semantics: includes are additive, excludes are additive, excludes win.

Example:

```bash
cortex analyze callers authenticate \
  --include-path src/auth \
  --include-glob "**/*.rs" \
  --exclude-path src/auth/generated
```

## MCP mode

```bash
cortex mcp tools
cortex mcp tools --metadata --format json-pretty
cortex mcp start
```

Transport options:

```bash
# stdio (default)
cortex mcp start
cortex mcp start --enable memory --enable context-capsule --enable index-status

# network transports
cortex mcp start --transport http-sse --listen 127.0.0.1:3001
cortex mcp start --transport websocket --listen 127.0.0.1:3001
cortex mcp start --transport multi --listen 0.0.0.0:3001 --allow-remote --token-env CORTEX_MCP_TOKEN
```

Stdio and network modes route through the same `CortexHandler` tool path for consistent MCP behavior.

CodeCortex advertises MCP `tools`, `resources`, and `prompts`:

- `resources/list` exposes `codecortex://guide/tool-routing`, `codecortex://guide/agent-workflows`, `codecortex://guide/privacy`, `codecortex://tools/catalog`, and `codecortex://schema/context-pack`.
- `prompts/list` exposes reusable workflows for patch planning, branch review, and freshness repair.
- `tools/list` and `cortex mcp tools --metadata` include cost, timeout, index, token, privacy, use-case, precondition, follow-up, and example request guidance.
- `recommend_tools` and `get_tool_guidance` provide cheap, narrow routing help so agents do not need to load the full tool catalog for every task.

Client integration examples:

- `docs/INTEGRATION.md`

### MCP Tool Coverage

- **Indexing and watch**: `add_code_to_graph`, `watch_directory`, `list_watched_paths`, `unwatch_directory`
- **Search and analysis**: `find_code`, `analyze_code_relationships`, `execute_cypher_query`, `find_dead_code`, `go_to_definition`, `find_all_usages`, `quick_info`, `branch_structural_diff`, `pr_review`, `find_similar_across_projects`, `find_shared_dependencies`, `compare_api_surface`, `calculate_cyclomatic_complexity`
- **Vector and hybrid search**: `vector_index_repository`, `vector_index_file`, `vector_search`, `vector_search_hybrid`, `search_across_projects`, `vector_index_status`, `vector_delete_repository`
- **Agent context**: `get_context_capsule`, `get_patch_context`, `get_delta_context`, `get_test_context`, `get_api_contract`, `summarize_module`, `estimate_context_cost`, `recommend_tools`, `tools_search`, `tool_profile`, `get_tool_guidance`, `explain_index_freshness`, `get_impact_graph`, `search_logic_flow`, `get_skeleton`, `get_signature`, `find_tests`, `explain_result`, `index_status`, `ctx_stats`, `ctx_grep`, `ctx_slice`, `ctx_peek`
- **Repository and project operations**: `list_indexed_repositories`, `delete_repository`, `get_repository_stats`, `list_projects`, `add_project`, `remove_project`, `set_current_project`, `get_current_project`, `list_branches`, `refresh_project`, `project_status`, `project_sync`, `project_branch_diff`, `project_queue_status`, `project_metrics`
- **Memory, jobs, bundles, and diagnostics**: `save_observation`, `get_session_context`, `search_memory`, `check_job_status`, `list_jobs`, `load_bundle`, `export_bundle`, `check_health`, `diagnose`, `workspace_setup`, `manage_codecortex`, `cortex_a2a_spawn_session`, `cortex_a2a_get_task`, `cortex_a2a_send_message`, `cortex_a2a_cancel_task`, `cortex_a2a_list_tasks`, `cortex_a2a_subscribe_task`, `cortex_a2a_list_push_configs`, `submit_lsp_edges`, `analyze_refactoring`, `find_patterns`

Agent-facing context tools return bounded metadata including freshness, token budget, estimated tokens, source exposure policy, omitted context, warnings, and suggested next tools. Source snippets are redacted for common secret-bearing lines before being returned.

Use `cortex mcp tools --metadata` or the `codecortex://tools/catalog` MCP resource when configuring agents that need to choose between cheap, bounded, expensive, and background tools safely.

## Real integration tests (12 languages)

Run one language:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_rust -- --ignored --nocapture --test-threads=1
```

Run all languages in strict order:

```bash
CORTEX_INTEGRATION_ENABLE=1 CORTEX_REAL_INTEGRATION=1 \
nix develop -c cargo test -p cortex-cli --test language_matrix_integration integration_all_languages_ordered_one_by_one -- --ignored --nocapture --test-threads=1
```

Note: JSON and Shell are fully supported in parser/indexer/vector/MCP/runtime paths and are currently validated in contract/unit coverage rather than remote-fixture matrix jobs.

Runbook:

- `docs/INTEGRATION_TEST_MATRIX.md`

Indexing notes:

- Graph writes use configured batches (`max_batch_size`, `falkordb_unwind_batch_max`) via FalkorDB `UNWIND` bulk upserts, reducing node/edge persistence from one round trip per entity to one per batch.
- `cortex index --mode incremental-diff` now uses the exact Git changed-file set instead of walking the whole repository.
- If an incremental diff contains deleted source files, CodeCortex falls back to a full forced branch rebuild so stale graph nodes are not left behind.
- MCP `add_code_to_graph` defaults to a forced graph pass; pass `force: false` only when intentionally using the local hash cache.

## Workspace crates

- `crates/cortex-core`
- `crates/cortex-parser`
- `crates/cortex-graph`
- `crates/cortex-indexer`
- `crates/cortex-analyzer`
- `crates/cortex-vector`
- `crates/cortex-watcher`
- `crates/cortex-pipeline`
- `crates/cortex-mcp`
- `crates/cortex-cli`
- `crates/cortex-benches`

## Development

```bash
nix flake check --print-build-logs
nix build .#cortex
```

Cargo fallback:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test --workspace
```

## CI and release automation

- CI checks and package builds run via [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
- Integration matrix smoke and real runs are Nix-based in [`.github/workflows/integration-language-matrix.yml`](.github/workflows/integration-language-matrix.yml).
- GitHub release assets (`linux-x86_64`, `linux-aarch64`, `macos-aarch64`) are produced via [`.github/workflows/release.yml`](.github/workflows/release.yml).

## Agent skills

Canonical agent skills live under `docs/skills/` and are symlinked for Cursor at `.cursor/skills/`:

| Skill | Purpose |
| --- | --- |
| [codecortex](docs/skills/codecortex/SKILL.md) | Code intelligence routing (callers, impact, search, navigation) |
| [codecortex-indexing](docs/skills/codecortex-indexing/SKILL.md) | Graph/vector indexing, watch, freshness repair |
| [codecortex-workflows](docs/skills/codecortex-workflows/SKILL.md) | Patch planning, branch review, triage playbooks |

See [AGENTS.md](AGENTS.md) for the default discover → act → verify loop and MCP resource URIs.

**Subagents** (Task delegation): [docs/agents/](docs/agents/) — `codecortex-indexer`, `codecortex-analyzer`, `codecortex-pr-reviewer`, `codecortex-patch-planner` (symlinked at `.cursor/agents/`).

**Cursor hooks and rules:** [docs/cursor/](docs/cursor/) — advisory hooks + [RULES-INDEX.md](docs/cursor/RULES-INDEX.md); rules in `.cursor/rules/codecortex-*.mdc`.

**Plugin (Claude Code + Cursor):** [plugin/codecortex/](plugin/codecortex/) — skills, agents, hooks, rules, MCP; sync via `plugin/codecortex/scripts/sync-from-docs.sh`.

## Additional docs

- install: `docs/INSTALL.md`
- FalkorDB backend: `docs/FALKORDB.md`
- integrations: `docs/INTEGRATION.md`
- roadmap: `docs/ROADMAP.md`

## License

Apache 2.0
