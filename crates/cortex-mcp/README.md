# cortex-mcp

Model Context Protocol server implementation with 41 production-ready tools.

## Overview

This crate implements the MCP server for CodeCortex, providing AI assistants with powerful code intelligence capabilities.

## Tool Categories (41 Tools)

### Code Retrieval (4 tools)
| Tool | Description | p95 Latency |
|------|-------------|-------------|
| `get_context_capsule` | Context-aware code retrieval | 2500ms |
| `find_code` | Search by name/pattern/type | 500ms |
| `get_skeleton` | Compressed code view | 200ms |
| `get_signature` | Function signature lookup | 100ms |

### Impact Analysis (3 tools)
| Tool | Description | p95 Latency |
|------|-------------|-------------|
| `get_impact_graph` | Blast radius analysis | 2200ms |
| `search_logic_flow` | Multi-path finding | 3000ms |
| `find_dead_code` | Unreachable code detection | 1000ms |

### Code Quality (4 tools)
| Tool | Description |
|------|-------------|
| `calculate_cyclomatic_complexity` | Complexity metrics |
| `find_tests` | Test discovery |
| `analyze_refactoring` | Refactoring suggestions |
| `find_patterns` | 15 design patterns |

### Diagnostics (4 tools)
| Tool | Description |
|------|-------------|
| `diagnose` | System diagnostics |
| `check_health` | Health check |
| `index_status` | Indexing status |
| `explain_result` | Result explanation |

### Memory System (3 tools)
| Tool | Description |
|------|-------------|
| `save_observation` | Persist observations |
| `get_session_context` | Session context |
| `search_memory` | Search stored memories |

### Project Management (7 tools)
| Tool | Description |
|------|-------------|
| `list_projects` | List all projects |
| `add_project` | Add new project |
| `remove_project` | Remove project |
| `set_current_project` | Set active project |
| `get_current_project` | Get active project |
| `list_branches` | List Git branches |
| `refresh_project` | Refresh Git state |

### Other Tools (16 tools)
- LSP Integration: `submit_lsp_edges`, `workspace_setup`
- Repository: `add_code_to_graph`, `list_indexed_repositories`, `delete_repository`, `get_repository_stats`
- Bundle: `load_bundle`, `export_bundle`
- Watch: `watch_directory`, `unwatch_directory`, `list_watched_paths`
- Advanced: `execute_cypher_query`, `analyze_code_relationships`, `check_job_status`, `list_jobs`

## Quality Metrics

```rust
use cortex_mcp::{QualityRegistry, QualityTimer, QualityHealthStatus};

let registry = QualityRegistry::with_defaults();

// Time a tool invocation
let timer = QualityTimer::new(&registry, "get_context_capsule");
// ... execute tool ...
// Timer automatically records on drop

// Get metrics
let metrics = registry.get_metrics("get_context_capsule");
println!("Error rate: {:.2}%", metrics.unwrap().error_rate * 100.0);

// Get system health
let health = registry.health_status();
assert!(matches!(health, QualityHealthStatus::Excellent | QualityHealthStatus::Good));
```

## Usage

```rust
use cortex_mcp::CortexHandler;

// Create handler with graph client
let handler = CortexHandler::new(graph_client);

// Get all tool names
let tools = cortex_mcp::tool_names();
assert_eq!(tools.len(), 41);
```

## Feature Flags

Tools can be enabled/disabled via environment variables:

```bash
# Disable specific tools
CORTEX_FLAG_MCP_MEMORY_READ_ENABLED=0

# Enable all tools (default)
# All tools enabled by default
```

## Performance SLOs

| Metric | Target |
|--------|--------|
| p50 latency (capsule) | 600ms |
| p95 latency (capsule) | 2500ms |
| Cache hit rate | > 80% |
| Quality: Recall@20 | >= 0.85 |
| Quality: nDCG@20 | >= 0.78 |

## Dependencies

- `rmcp` - MCP protocol
- `cortex-core`, `cortex-graph`, `cortex-indexer`, etc.
- `dashmap` - Concurrent cache
- `tokio` - Async runtime

## Tests

Run tests with:
```bash
cargo test -p cortex-mcp -- --test-threads=1
```

Current test count: **137 tests**
