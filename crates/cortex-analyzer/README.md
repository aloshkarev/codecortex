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
