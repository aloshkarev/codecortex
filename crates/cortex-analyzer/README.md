# cortex-analyzer

> `cortex-analyzer` provides graph-backed structural analysis and static quality checks for CodeCortex. It powers call graph traversal, code smell detection, refactoring suggestions, navigation queries, and cross-project intelligence, all driven by the indexed graph data in Memgraph/Neo4j.

## What it does

- Traverses the code graph for caller/callee chains, dependency graphs, and type hierarchies
- Detects code quality issues: long functions, deep nesting, duplicated blocks, large classes, and language-specific patterns
- Suggests refactoring opportunities based on smell signals and complexity thresholds
- Provides navigation primitives: `go_to_definition`, `find_usages`, `quick_info`, `branch_structural_diff`
- Supports project-aware analysis via `ProjectAnalysisContext` to reduce cross-file false positives
- Enables cross-project intelligence via `CrossProjectAnalyzer` for similarity, shared-dependency, and API-surface comparisons

## Analysis capabilities

### Relationship queries (`Analyzer`)

| Method | Description |
|--------|-------------|
| `callers(symbol)` | Direct and transitive callers of a symbol |
| `callees(symbol)` | Direct and transitive callees |
| `chain(a, b)` | Call path between two symbols |
| `hierarchy(type)` | Inheritance / trait implementation hierarchy |
| `deps(symbol)` | Dependency subgraph |
| `overrides(method)` | Override and implementation sites |

All relationship query methods accept an optional `AnalyzePathFilters` argument for scoping results to specific files, paths, or glob patterns.

### Quality queries

| Method | Description |
|--------|-------------|
| `dead_code()` | Detect symbols with no incoming `CALLS` edges |
| `complexity(top_n)` | Cyclomatic complexity per symbol, sorted descending |

### `ProjectAnalysisContext`

`ProjectAnalysisContext` loads a project symbol index that enables:

- **Context-aware smell paths** (`*_with_context`) — reduce per-file false positives by checking whether a "large class" or "long function" is genuinely anomalous relative to the project baseline
- **Project-wide duplication** — detect duplicated blocks across all files in the project rather than within a single file
- **Symbol-level scoping** — smell and complexity results include project metadata for better actionability

```rust
use cortex_analyzer::{Analyzer, ProjectAnalysisContext};

let ctx = ProjectAnalysisContext::load_for_project("/path/to/repo").await?;
let smells = analyzer.smells_with_context(&ctx).await?;
for smell in &smells {
    println!("{}: {} at {}:{}", smell.kind, smell.symbol, smell.file, smell.line);
}
```

### `NavigationEngine`

`NavigationEngine` provides IDE-style navigation backed by the graph:

| Method | CLI | MCP tool |
|--------|-----|---------|
| `go_to_definition(symbol)` | `cortex goto` | `go_to_definition` |
| `find_usages(symbol)` | `cortex usages` | `find_all_usages` |
| `quick_info(symbol)` | `cortex info` | `quick_info` |
| `branch_structural_diff(src, tgt)` | `cortex analyze branch-diff` | `branch_structural_diff` |

Navigation queries require that the target repository is indexed and that `MEMBER_OF`, `TYPE_REFERENCE`, and `FIELD_ACCESS` edges have been resolved by the indexer.

### `ReviewAnalyzer`

`ReviewAnalyzer::analyze_with_graph` produces a graph-enriched review report that includes:
- Impact warnings for symbols changed in the diff (based on `CALLS` and `MEMBER_OF` traversal)
- Potential new dead code introduced by the change
- Complexity changes for modified functions

### `CrossProjectAnalyzer`

`CrossProjectAnalyzer` operates across multiple indexed repositories:

| Method | Description |
|--------|-------------|
| `find_similar_across_projects(symbol, projects)` | Find structurally or semantically similar symbols |
| `find_shared_dependencies(projects)` | Detect shared graph-level dependencies |
| `compare_api_surface(projects)` | Compare exported API surfaces across projects |

## Path filters

`AnalyzePathFilters` applies scoping rules to all analysis queries:

```rust
use cortex_analyzer::{AnalyzePathFilters, Analyzer};

let filters = AnalyzePathFilters {
    include_paths: vec!["src/auth".into()],
    include_files: vec![],
    include_globs: vec!["**/*.rs".into()],
    exclude_paths: vec!["src/auth/generated".into()],
    exclude_files: vec![],
    exclude_globs: vec![],
};
filters.validate()?; // validates glob syntax

let rows = analyzer.callers_with_filters("authenticate", Some(&filters)).await?;
```

Filter semantics: includes are additive (OR); excludes are additive (OR); excludes override includes.

## Code smell detection

The smell detector is language-aware and detects:

| Smell | Description |
|-------|-------------|
| Long function | Function body exceeds a line/complexity threshold |
| Large class | Class with too many methods or fields |
| Deep nesting | Excessive branching depth within a function |
| Code duplication | Repeated blocks within or across files |
| Too many parameters | Function with more parameters than the threshold |
| Dead code | Symbols with no reachable callers |

Language-specific handling includes Ruby `def ... end` block boundary detection and TypeScript arrow function patterns.

## Profiling the smell pipeline

```bash
# Profiles per-file smell detection and duplication timing (release build required)
cargo run -p cortex-analyzer --release --bin profile_analyzer -- crates/cortex-analyzer/src

# Bounded cross-file duplication timing
CORTEX_PROFILE_CROSS_DUP=1 \
  cargo run -p cortex-analyzer --release --bin profile_analyzer -- crates/cortex-analyzer/src
```

Set `CORTEX_PROFILE_MAX_LINES=N` to cap file size for timed phases (default 400). Use `perf` or Instruments on the hot phase before adding caches.

## Tests

```bash
cargo test -p cortex-analyzer -- --test-threads=1
```
