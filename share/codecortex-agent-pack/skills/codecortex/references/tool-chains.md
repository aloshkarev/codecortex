# Tool Chains

Use these chains when the task is ambiguous or high impact.

## Preflight

1. `check_health`
2. `index_status`
3. Optional `diagnose` if either step indicates instability

## Bug Impact

1. `find_code` for symbol localization
2. `analyze_code_relationships` with `find_callers` and `find_callees`
3. `get_impact_graph` for blast radius
4. `get_context_capsule` for implementation context

## Refactor Planning

1. `analyze_refactoring`
2. `calculate_cyclomatic_complexity`
3. `find_dead_code`
4. `find_patterns`

## API Path Tracing

1. `find_code`
2. `search_logic_flow`
3. `get_signature`
4. `get_skeleton`

## Repository Readiness

1. `index_status`
2. `get_repository_stats`
3. `analyze_code_relationships` with `module_deps`
