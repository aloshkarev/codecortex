# CodeCortex — Development Plan

<!-- BEGIN: Implemented Workspace Variant -->

## Implemented Workspace Variant

This workspace is treated as operational or special-case work, not a generic planned portfolio stub.

| Dimension | Position |
| --- | --- |
| Role | implemented MCP/code-intelligence workspace and agent-enabler reference |
| Documentation rule | State what runs today, exact interfaces, known limits, and reusable patterns. |
| Portfolio boundary | Do not expand the active roadmap unless it produces a narrow evidence artifact or reusable crate capability. |

<!-- END: Implemented Workspace Variant -->

<!-- BEGIN: Portfolio DEVPLAN Actualization -->

## Portfolio Strategy Alignment

- **Portfolio source**: `ideas.md` entry special, `64-codecortex`.
- **Decision stage**: Build now.
- **Scores**: commercial 86/100; practical MVP 82/100.
- **Cluster**: AI Codebase Intelligence / Developer Tooling.
- **Ideal customer profile**: AI-agent power users, platform engineering teams, codebase owners, internal developer productivity teams.
- **Commercial wedge**: Preserve the production-grade graph/vector/watcher/MCP plan and add portfolio positioning around token-saving codebase context for AI agents.

### Commercial and Practical Score Rationale

| Dimension | Interpretation | Product implication |
| --- | --- | --- |
| Commercial value | 86/100 | Buyer value is strong enough for active packaging. |
| Practical MVP | 82/100 | Implementation is close to current crate leverage. |
| Stage discipline | Build now | Roadmap must follow the stage gates below instead of expanding into a generic platform. |

## Production-Grade MVP Scope

The first production-grade deliverable is: **Preserve the production-grade graph/vector/watcher/MCP plan and add portfolio positioning around token-saving codebase context for AI agents**.

MVP acceptance:

- Ship a local-first indexer and MCP server that returns bounded context capsules with explicit token budgets.
- Validate freshness under frequent file changes with watcher-driven incremental indexing.
- Benchmark context quality against grep-only and naive vector retrieval baselines.
- Package a developer quickstart and production support bundle for private repositories.
- Add freshness, token-budget, and source-attribution contracts to every agent-facing response.
- Benchmark retrieval quality and token savings on active repositories.

Non-goals for the first release:

- Do not build broad platform surface area before the narrow proof artifact is trusted.
- Do not claim unsupported shared-crate capabilities or future enablers as shipped features.
- Do not automate high-risk actions until dry-run evidence, rollback, and operator approval exist.

## Market and Competitor Reality

- **Comparable commercial / OSS products**: Sourcegraph Cody, Cursor indexing, Continue, Codeium, Cody/Code Search, Sourcegraph, OpenGrok, Zoekt, Semgrep, Glean-style internal search.
- **Positioning advantage**: Strong wedge if it reduces agent token waste and stale context during active development rather than acting as another generic code search UI.
- **Competitive risk**: Must prove freshness, relevance, security boundaries, and measurable token savings on real repositories.
- **2026 market rule**: Win by returning fresh, bounded, token-efficient context for AI agents, not by becoming a generic code search interface.

## Cross-Project Integration Plan

- **Integrations from `ideas.md`**: RunForge, ChangeBot, NetLint, IncidentLens, internal agent workflows, and future code-parser enabler work.
- Export outputs as stable files first: JSON, Markdown, HTML/PDF, SARIF, Prometheus/OpenMetrics, or signed evidence bundles as appropriate.
- Keep integration contracts versioned so sibling products can consume reports without linking internal modules.
- Prefer shared reporting, dashboard, config, and fixture patterns before extracting new shared crates.

## Enabler and Shared-Crate Contract

- Use today: ns-core patterns, local Rust services, MCP integration, graph/vector/indexer crates in the project.
- Explicit blocker or caution: freshness, token-budget discipline, incremental indexing, and production MCP ergonomics.
- Keep planned dependencies out of GA promises until they are implemented, tested, and covered by fixtures.
- Keep graph, vector, watcher, and MCP contracts versioned; every context response must include source freshness and token budget metadata.

## Security, Privacy, and Data Governance

- Never send private code to hosted models by default; make local/offline operation and repository exclusions explicit.
- Redact secrets in snippets and support bundles and track source freshness for every answer.
- Version MCP responses, embeddings, graph schema, and cache invalidation behavior.

## Production SLOs and Launch Gates

Minimum SLOs before paid or public launch:

- Index freshness visible within seconds for normal edit bursts.
- Every context capsule declares token budget, source files, and freshness state.
- Regression tests compare retrieval quality against grep/vector baselines.

Stage gates:

- Alpha: prove the narrow artifact on 3-5 realistic fixtures or customer samples.
- Beta: design partner can run the workflow with setup under 30 minutes.
- GA: pricing, support boundary, success metric, and reproducible evidence are ready.

## Step-by-Step Feature Delivery Plan

1. Freeze the first buyer workflow and fixture corpus for this product.
2. Build the smallest CLI/report/API path that proves the workflow end to end.
3. Add deterministic tests, golden fixtures, and competitor/reference comparisons.
4. Package onboarding, diagnostics, redaction, and support bundle generation.
5. Run a design-partner pilot and record objections, false positives, setup time, and willingness to pay.
6. Promote only the validated scope to GA; move everything else to explicit future/enabler sections.

<!-- END: Portfolio DEVPLAN Actualization -->

> Production plan for a fast, project-aware code intelligence platform that keeps pace with frequent code changes and gives AI agents deep repository understanding with minimal token spend.

## Product Goal

CodeCortex should become the local/enterprise code-context layer for AI agents: an always-fresh structural + semantic index that answers codebase questions, packages minimal context, and reduces blind file-reading during code generation.

The platform must optimize for four outcomes:

1. **Fast freshness**: small edits are reflected in graph/vector/MCP results within seconds, not after full re-index.
2. **Low-token context**: agents receive signatures, skeletons, call paths, compact capsules, and evidence references instead of full files by default.
3. **Practical agent workflows**: Cursor, Claude Code, Codex CLI, Gemini CLI, Zed, and Neovim can use the same MCP tools with scoped filters and stable contracts.
4. **Production reliability**: large repositories, branch churn, generated files, multiple projects, and embedding-provider outages degrade gracefully.

## Current Baseline

Available today from the codebase and docs:

| Area | Current capability | Production gap |
| --- | --- | --- |
| Parsing | Tree-sitter based parsing for Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell | Need language-level quality dashboards and incremental parse cache correctness gates |
| Graph backend | **FalkorDB only** (`falkordb_uri`, `falkordb_graph`, `falkordb_write_pool_size`, `falkordb_unwind_batch_max`); Memgraph/Neo4j/Grafeo backends removed | Document tuning matrix and production sizing for large graphs |
| Graph indexing | Repository indexing into FalkorDB with calls, imports, type refs, field access, membership, qualified names | Need stronger partial re-index transactions, tombstones, and freshness proof |
| Vector retrieval | `cortex-vector` with Lance/JSON stores, embedding providers, hybrid search APIs | Need chunk policy, stale-vector eviction, embedding failure fallback, and budgeted reranking |
| Watcher | Smart debouncing, event filters, branch detection, registry, backpressure | Need always-on daemon SLOs and branch/worktree-aware invalidation |
| MCP | Stdio, HTTP+SSE, WebSocket, shared handler, graph/vector/project/watch/memory tools | Need token-budget contracts, tool-cost classes, stricter schemas, and operational observability |
| Agent context | `get_context_capsule`, `get_skeleton`, `get_signature`, navigation, impact, branch diff, PR review | Need context packs optimized for code generation and patch planning, not only search |
| Measurement | Token/time/rework A/B measurement kit | Need automated dashboards, task taxonomy, and CI acceptance thresholds for token savings |
| Testing | Unit, integration, language matrix, contract tests, Nix build/release flows | Need large-repo benchmarks, stale-index tests, and agent workflow regression suites |

## Market and Technical Reality

CodeCortex competes with and complements:

- **Cursor indexing**: semantic + agentic search, automatic sync, editor-native context.
- **Sourcegraph Cody**: large-scale code search and remote repository context.
- **Continue / Codeium / Windsurf-style agent workflows**: editor agents with retrieval, grep, and tool orchestration.
- **GitNexus-style graph systems**: MCP-native structural awareness and dependency tracking.
- **Traditional code intelligence**: LSP, ctags, ripgrep, tree-sitter, OpenGrok, Kythe, SCIP/LSIF.

Key design conclusions:

- Structure-aware chunking beats naive file chunks for code, but graph-only retrieval misses natural-language intent.
- Hybrid retrieval should combine lexical/BM25, semantic vectors, graph neighborhood, and recency/freshness signals.
- Agentic search is still valuable. CodeCortex should not replace grep/file reads; it should make the first query precise and make the follow-up context small.
- Retrieved similar code can waste tokens. For generation, signatures, API contracts, dependencies, call sites, tests, and change impact usually matter more than long bodies of unrelated similar files.
- Freshness is a product feature. If the index may be stale, the MCP response must say so and suggest a repair path.

## Production Architecture Target

```mermaid
flowchart LR
    fs[File Events] --> watch[cortex-watcher]
    git[Git State] --> watch
    watch --> queue[Index Queue]
    queue --> inc[Incremental Indexer]
    inc --> parse[cortex-parser]
    parse --> graph[cortex-graph]
    parse --> chunks[Chunker]
    chunks --> vector[cortex-vector]
    graph --> tools[cortex-mcp Tools]
    vector --> tools
    cache[Hot Cache] --> tools
    tools --> agent[AI Agent]
    tools --> metrics[Telemetry and Measurement]
```

Core invariants:

- Every returned symbol/context item carries `repo`, `branch`, `commit_or_worktree_hash`, `path`, `span`, `language`, `freshness`, and `token_estimate`.
- Index updates are idempotent and tombstone-aware: deleted/renamed files remove graph nodes and vector chunks.
- MCP tools are budget-aware: default responses are concise and expandable.
- Large or risky tools expose `cost_class`, estimated latency, and timeout behavior.
- Agent-facing outputs prefer stable references over pasted source.

## Token-Saving Product Features

### 1. Context Capsule v2

Goal: package enough context for generation or review under a strict token cap.

Inputs:

- `task`: free-text user intent.
- `scope`: paths, files, symbols, branch, project.
- `budget_tokens`: hard cap.
- `mode`: `bugfix`, `feature`, `refactor`, `test`, `review`, `docs`.
- `include`: `signatures`, `skeletons`, `callers`, `callees`, `tests`, `examples`, `config`, `docs`.

Output:

- ranked context items with token estimates,
- short rationale per item,
- symbol signatures and skeletons first,
- only small excerpts when necessary,
- explicit omitted items with why they were omitted,
- freshness and confidence score.

Acceptance:

- p95 capsule generation < 2s on medium repo, < 8s on large repo.
- default capsule < 4k tokens.
- at least 35% token reduction vs agent reading top 5 files manually in measurement kit.

### 2. Agent Patch Plan Context

Purpose: before an AI agent edits code, provide a compact plan pack:

- target symbols and owning modules,
- related interfaces and trait/protocol contracts,
- direct callers/callees,
- likely tests,
- config/schema files,
- risky side effects,
- examples of local style from nearest files.

This should power prompts like: "implement this feature with the least context".

MCP tool:

```json
{
  "tool": "get_patch_context",
  "arguments": {
    "task": "add token refresh to auth client",
    "include_paths": ["src/auth"],
    "budget_tokens": 6000,
    "mode": "feature"
  }
}
```

### 3. Delta Context for Frequent Changes

Purpose: after a branch changes, agents need only what changed and what it affects.

MCP/CLI:

- `branch_structural_diff` remains the base.
- Add `get_delta_context`:
  - changed symbols,
  - removed/renamed symbols,
  - affected callers,
  - tests likely to update,
  - stale index warnings,
  - token-bounded excerpts.

Acceptance:

- branch delta on 500 changed files completes < 15s with graph indexed.
- output remains under default 6k tokens.

### 4. Retrieval Reranker for Code Generation

Scoring formula should combine:

- lexical relevance,
- embedding relevance,
- graph distance from target,
- ownership/module proximity,
- test proximity,
- recent edit/branch delta,
- public API importance,
- generated/vendor penalty,
- token cost penalty.

Default behavior:

- prefer signatures/skeletons over full bodies,
- include implementation body only for top target and direct examples,
- include tests before distant similar code for test/refactor tasks.

### 5. Context Memory With Expiry

Memory should store durable project observations without bloating every response:

- architecture decisions,
- stable invariants,
- common commands,
- project-specific conventions,
- recurring bug patterns.

Rules:

- every memory item has source, confidence, created_at, last_verified_at, and expiry policy;
- stale memory never overrides fresh graph data;
- MCP responses cite memory only when relevant and budget allows.

## Incremental Indexing and Freshness Plan

### Change Event Pipeline

1. `cortex-watcher` receives file events.
2. Event filter drops ignored/generated/vendor files.
3. Debouncer coalesces rapid saves.
4. Index queue groups by repository, branch, and language.
5. Incremental indexer parses changed files only.
6. Graph writer applies file-scoped transaction:
   - delete old file nodes/edges or mark tombstones,
   - insert new symbols/edges,
   - reconcile cross-file placeholders,
   - update repository freshness watermark.
7. Vector updater re-chunks changed files and deletes stale chunks.
8. MCP cache invalidates affected symbols and query keys.

### Freshness States

| State | Meaning | Agent behavior |
| --- | --- | --- |
| `fresh` | index includes current file hash / branch state | normal answer |
| `warming` | changes queued or processing | answer may include warning and offer wait/retry |
| `stale` | index older than worktree or branch | refuse high-confidence impact claims |
| `partial` | graph fresh but vector stale, or reverse | use fresh modality and disclose limitation |
| `unknown` | repo not registered or backend unavailable | run health/index guidance |

### Large Repo Strategy

- File content hash cache to skip unchanged files.
- Per-language parser worker pools with bounded concurrency.
- Commit/worktree snapshot records to avoid full graph scans on every question.
- Optional "index tiers":
  - Tier 0: file tree + symbols + signatures.
  - Tier 1: calls/imports/type refs.
  - Tier 2: complexity/smells/tests.
  - Tier 3: embeddings and summaries.
- MCP tools declare minimum tier. If tier is missing, return a targeted indexing recommendation.

## MCP Tooling Roadmap

### Tool Cost Classes

| Class | Examples | Default response policy |
| --- | --- | --- |
| `cheap` | `get_signature`, `get_skeleton`, `quick_info`, `index_status` | return immediately |
| `bounded` | `get_context_capsule`, `go_to_definition`, `find_all_usages` with filters | enforce token and row limits |
| `expensive` | repo-wide dead code, branch diff, cross-project search | require scope or explicit confirmation in clients |
| `background` | full index, vector index, large PR review | return job ID and progress |

### New / Hardened Tools

- `get_patch_context`: token-bounded context for code generation.
- `get_delta_context`: branch/worktree change context.
- `estimate_context_cost`: estimate tokens/latency before retrieval.
- `explain_index_freshness`: explain stale/missing data and exact repair command.
- `get_test_context`: tests likely affected by task/symbol.
- `get_api_contract`: public signatures, trait/protocol/interface contracts, schema references.
- `summarize_module`: compact architecture summary for a folder/package.

### MCP Response Contract

Every agent-facing context tool should return:

```json
{
  "scope": {},
  "freshness": {},
  "token_budget": 6000,
  "estimated_tokens": 4210,
  "items": [],
  "omitted": [],
  "next_tools": [],
  "warnings": []
}
```

## Step-by-Step Delivery Plan

### Phase 1 — Canonical Baseline and SLOs (Week 1)

- Add this `DEVPLAN.md` as the canonical delivery plan.
- Reconcile docs drift:
  - root `Cargo.toml` comments about extracted crates vs actual in-repo crates,
  - MCP tool counts in docs vs `tools.rs` / contract tests,
  - README, ROADMAP, INTEGRATION references.
- Define SLOs:
  - health check p95 < 500ms,
  - cheap MCP p95 < 300ms,
  - bounded MCP p95 < 2s,
  - incremental update p95 < 5s for 1-file edit,
  - vector update p95 < 10s for 1-file edit,
  - capsule default <= 4k tokens.
- Add SLO table to README and MCP docs.

Definition of done:

- docs agree on capabilities and tool counts;
- SLOs are committed;
- contract test asserts every MCP tool has cost class, timeout, and schema metadata.

### Phase 2 — Freshness and Incremental Index Correctness (Weeks 2-3)

- Implement/finish file hash cache and index watermarks per repo/branch.
- Add tombstone handling for deleted/renamed files in graph and vector stores.
- Add `IndexFreshness` model shared by CLI/MCP.
- Extend `index_status` to report queued, stale, warming, partial, and fresh states.
- Add targeted repair commands in output:
  - `cortex index <repo> --incremental`,
  - `cortex vector-index <repo>`,
  - `cortex watch <repo>`.
- Add tests:
  - modify file updates symbol,
  - delete file removes symbol,
  - rename file preserves/updates references,
  - branch switch marks stale until synced.

Definition of done:

- stale answers are disclosed;
- deleted symbols disappear from navigation/search after incremental update;
- one-file edit reflected in `quick_info` and `get_context_capsule` under 5s on test repo.

### Phase 3 — Hot Cache and Query Latency (Weeks 4-5)

- Add cache keys based on repo, branch, tool, scope, symbol, file hashes, and freshness watermark.
- Cache cheap symbol lookups, skeletons, signatures, and common graph neighborhoods.
- Add bounded result limits and pagination for expensive relationship tools.
- Add query timeout tiers:
  - cheap 1s,
  - bounded 5s,
  - expensive 30s,
  - background job beyond 30s.
- Add telemetry:
  - tool latency,
  - DB latency,
  - cache hit rate,
  - token estimate,
  - result count,
  - timeout reason.

Definition of done:

- hot `quick_info` and `get_signature` p95 < 300ms;
- `get_context_capsule` p95 < 2s on medium repo;
- no expensive MCP tool can accidentally return unbounded results.

### Phase 4 — Context Capsule v2 and Patch Context (Weeks 6-7)

- Redesign `get_context_capsule` ranking around task mode and token budget.
- Add token estimator for:
  - signatures,
  - skeletons,
  - excerpts,
  - graph neighborhoods,
  - test snippets,
  - summaries.
- Implement `get_patch_context`.
- Implement `get_test_context`.
- Implement `get_api_contract`.
- Add "context diet" rules:
  - summarize long files,
  - prefer public signatures,
  - include body only when directly targeted,
  - include tests over distant examples for change tasks.

Definition of done:

- 30-task measurement run shows >= 35% prompt-token reduction vs baseline retrieval workflow without lowering success rate;
- generated context packs cite paths/spans and include omitted-item explanations.

### Phase 5 — Hybrid Retrieval and Reranking (Weeks 8-9)

- Implement unified retrieval pipeline:
  - lexical/BM25 candidate retrieval,
  - vector semantic retrieval,
  - graph neighborhood expansion,
  - recency/freshness scoring,
  - token-cost-aware reranking.
- Add fallback when embeddings unavailable:
  - lexical + graph + signatures,
  - clear warning,
  - no failure for core tools.
- Add chunk policy:
  - AST-based chunks for functions/classes/methods,
  - module summary chunks,
  - public API chunks,
  - test chunks,
  - config/schema chunks.
- Add stale vector eviction for changed/deleted files.

Definition of done:

- vector provider outage does not break MCP;
- hybrid search returns scoped, fresh, token-bounded results;
- stale vectors are removed after file delete/rename tests.

### Phase 6 — Agent Workflow Integration (Weeks 10-11)

- Update `docs/skills/codecortex/SKILL.md` with:
  - patch planning workflow,
  - token-budget presets,
  - freshness preflight,
  - tool-cost class guidance.
- Add examples for Cursor, Claude Code, Codex CLI, Gemini CLI, Zed, Neovim:
  - "debug with 4k context",
  - "implement feature with patch context",
  - "review branch with delta context",
  - "update tests from impacted symbols".
- Add MCP prompt snippets/resources:
  - `codecortex://guide/token-saving`,
  - `codecortex://guide/patch-context`,
  - `codecortex://schema/context-pack`.
- Add `cortex interactive agent-context` command to preview what an agent would receive.

Definition of done:

- an agent can perform a scoped bugfix using only `check_health`, `index_status`, `get_patch_context`, `get_test_context`, and targeted file reads;
- docs include copy-paste client configuration and workflows.

### Phase 7 — Production Operations and Multi-Project Scale (Weeks 12-13)

- Add daemon mode profile:
  - index queue,
  - watch registry,
  - vector update workers,
  - MCP server,
  - metrics endpoint.
- Add `cortex project sync --all` with branch freshness reporting.
- Add cross-project search SLOs and safeguards.
- Add backup/export:
  - graph bundle export,
  - vector metadata export,
  - registry export.
- Add security controls:
  - loopback by default,
  - bearer token for remote MCP,
  - path allowlist,
  - secret redaction,
  - audit log for tool calls.

Definition of done:

- 10 registered repos can be watched and queried;
- remote MCP mode requires explicit `--allow-remote` and token;
- operations docs cover backup/restore and stale index repair.

### Phase 8 — Measurement, Benchmarks, and GA Hardening (Weeks 14-16)

- Extend measurement kit:
  - automatic MCP tool capture,
  - token estimates vs actual provider token import,
  - task category dashboard,
  - rework taxonomy,
  - weekly report export.
- Add benchmark suites:
  - small repo, medium repo, large monorepo,
  - cold index,
  - warm incremental update,
  - graph query latency,
  - vector indexing,
  - capsule generation.
- Add CI gates:
  - MCP tool-surface contract,
  - language integration matrix smoke,
  - stale-index correctness,
  - benchmark regression threshold for critical paths.
- Add release notes and migration docs.

Definition of done:

- 30 baseline + 30 CodeCortex tasks show token savings, time savings, success rate, and rework rate;
- GA release ships with reproducible benchmark report and known-limitations page.

## Production SLOs

| Workflow | Target |
| --- | --- |
| One-file incremental graph update | p95 < 5s |
| One-file vector update | p95 < 10s |
| Cheap MCP tool | p95 < 300ms |
| Bounded MCP tool | p95 < 2s |
| Large branch delta context | p95 < 15s for 500 changed files |
| Default context capsule | <= 4k estimated tokens |
| Patch context | <= 6k estimated tokens |
| Token reduction vs baseline | >= 35% prompt-token reduction over 30 balanced tasks |
| Quality guard | no decrease in success rate; rework rate not worse than baseline |

## Test Strategy

| Layer | Tests |
| --- | --- |
| Parser | per-language fixtures, AST chunk boundaries, syntax-error recovery |
| Incremental index | modify/delete/rename/branch-switch tests, tombstone assertions |
| Graph | schema migration, query timeout, navigation, relationship correctness |
| Vector | chunk lifecycle, stale eviction, provider outage fallback, hybrid reranking |
| MCP | schema contracts, cost classes, token budget, transport parity |
| Watcher | debounce, backpressure, remote FS, generated/vendor filtering |
| Agent workflow | patch context, delta context, test context, capsule quality snapshots |
| Measurement | token import, A/B report, MCP capture, task taxonomy |
| Integration | real language matrix, multi-project registry, branch structural diff |

## Security and Privacy

- Default MCP network bind remains loopback.
- Remote MCP requires `--allow-remote` and bearer token.
- Path allowlists prevent serving arbitrary filesystem context.
- Tool audit log records tool name, repo, branch, scope, latency, result count, but not source bodies by default.
- Memory entries must cite source and be deletable.
- No code leaves the machine unless the configured embedding provider requires it and the user explicitly enables remote embeddings.
- Local embedding mode should be documented as the enterprise-safe default when available.

## Commercial Packaging

| Tier | Capability |
| --- | --- |
| OSS / Local | CLI, graph index, MCP stdio, skeleton/signature/navigation, basic context capsule |
| Pro | vector/hybrid retrieval, patch context, delta context, measurement reports |
| Team | daemon mode, multi-project registry, HTTP/WebSocket MCP with auth, shared policies |
| Enterprise | local embeddings, audit logs, path policies, bundle export/import, SSO/reverse-proxy deployment docs, support for large monorepos |

## Immediate Backlog

1. Add freshness model and expose it in `index_status`, `get_context_capsule`, navigation, and search tools.
2. Add MCP cost classes and token budget metadata to every tool schema.
3. Implement `get_patch_context` on top of existing graph/vector/skeleton/signature primitives.
4. Add stale vector deletion for changed/deleted files.
5. Convert measurement kit output into weekly markdown/JSON report with token/time/rework deltas.
6. Reconcile tool count/doc drift and root workspace comments.
7. Add large-repo benchmark fixtures and p95 regression thresholds.

## Release Framing

- **v1.1 — Fresh and Fast**: freshness model, incremental correctness, latency SLOs, cache, bounded tool behavior.
- **v1.2 — Token-Saving Agent Context**: context capsule v2, patch context, delta context, test context, hybrid reranker.
- **v1.3 — Production Multi-Project Platform**: daemon profile, observability, security, backup/export, enterprise local-embedding guidance.
- **v1.4 — Continuous Agent Feedback Loop**: automated measurement dashboards, context-quality learning, project memory lifecycle, agent workflow regression tests.
