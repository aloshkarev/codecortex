# Vector indexing and search

## When to index vectors

Index vectors when agents need:

- Natural-language code discovery (`vector_search`)
- Hybrid graph + semantic retrieval (`vector_search_hybrid`)
- Cross-project semantic search (`search_across_projects`, `find_similar_across_projects`)

Graph-only tasks (callers, dead code, structural diff) do **not** require a vector index.

## Bootstrap

```bash
cortex vector-index /path/to/repo
```

MCP (background): `vector_index_repository` — poll `list_jobs` / `check_job_status`.

Status: `vector_index_status` (MCP) or `cortex stats` (CLI).

## Tool vs index tier

| Tool | Minimum tier |
| --- | --- |
| `vector_search` | `vector` |
| `vector_search_hybrid` | `graph_and_vector` |
| `find_similar_across_projects` | `graph_and_vector` |
| `search_across_projects` | `vector` |

If hybrid search fails or returns empty results, verify both graph and vector indexes on the active project/branch.

## Routing vs graph-only search

| Need | Prefer |
| --- | --- |
| Exact symbol name | `find_code` (graph) |
| NL “where is auth handled?” | `vector_search_hybrid` |
| Semantic only, graph optional | `vector_search` |

After vector hits, narrow with `get_context_capsule` or `get_api_contract` instead of reading whole files.

## Cleanup

MCP: `vector_delete_repository` when removing a repo from the vector store.

CLI: follow `cortex` repository delete commands documented in `README.md`.
