# cortex-analyzer

`cortex-analyzer` provides graph-backed analysis and static quality checks.

## Includes

- Relationship queries (`callers`, `callees`, `chain`, `deps`, `hierarchy`, `overrides`)
- Quality queries (`dead_code`, `complexity`)
- Smell detection and refactoring suggestions
- Path filter support used by CLI and MCP

## Recent updates
  - added `ProjectAnalysisContext` and project symbol index for context-aware analysis
  - added context-aware smell paths (`*_with_context`) to reduce per-file false positives
  - added project-wide duplication path support
  - added cross-project analysis surface (`CrossProjectAnalyzer`) for similarity/shared dependency/API comparison flows
  - added `NavigationEngine` with `go_to_definition`, `find_usages`, `quick_info`, `branch_structural_diff`
  - added graph-enriched review path via `ReviewAnalyzer::analyze_with_graph`

## Analyze path filters

`AnalyzePathFilters` supports:

- include paths/files/globs
- exclude paths/files/globs
- validation of glob syntax
- include OR + exclude OR, excludes win

## Profiling (smells + duplication, in-crate)

The `profile_analyzer` binary mirrors the CLI’s **no-graph** smell pipeline (per file) and times `DuplicationDetector` work. Use a **release** build when measuring.

```bash
cargo run -p cortex-analyzer --release --bin profile_analyzer -- crates/cortex-analyzer/src
```

By default each file is truncated to **400 lines** for the timed phases (`CORTEX_PROFILE_MAX_LINES`); set `CORTEX_PROFILE_MAX_LINES=0` for full files (can be very slow on duplication paths).

Optional bounded cross-file duplication timing:

```bash
CORTEX_PROFILE_CROSS_DUP=1 \
  cargo run -p cortex-analyzer --release --bin profile_analyzer -- crates/cortex-analyzer/src
```

All knobs are documented in `src/bin/profile_analyzer.rs`. Use `perf` / Instruments on the hot phase before adding caches in the library.

## Smell detector notes

Smell/refactoring analysis is language-aware and handles extension-specific boundaries, including Ruby `def ... end` patterns.

## Example

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
filters.validate()?;
let _rows = analyzer.callers_with_filters("authenticate", Some(&filters)).await?;
```

## Test

```bash
cargo test -p cortex-analyzer -- --test-threads=1
```
