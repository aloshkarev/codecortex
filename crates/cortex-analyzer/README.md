# cortex-analyzer

Code analysis queries for call graphs, dependencies, and code metrics.

## Overview

This crate provides analysis functionality for querying code relationships and metrics.

## Query Types

| Query | Description | Example |
|-------|-------------|---------|
| `find_code` | Search symbols | `find_code("UserRepo", Name)` |
| `callers` | Find callers | `callers("main")` |
| `callees` | Find callees | `callees("process")` |
| `call_chain` | Find path | `call_chain("main", "db_query")` |
| `dead_code` | Find unreachable | `dead_code()` |
| `complexity` | Calculate complexity | `complexity("process_file")` |
| `hierarchy` | Class hierarchy | `hierarchy("BaseHandler")` |
| `overrides` | Method overrides | `overrides("handle")` |

## Code Quality Analysis

### Code Smell Detection

```rust
use cortex_analyzer::{SmellDetector, SmellConfig, SmellType};

let detector = SmellDetector::new();
let smells = detector.detect("fn long_fn() { /* ... */ }", "test.rs");
for smell in smells {
    println!("{:?}: {} at line {}", smell.smell_type, smell.message, smell.line);
}
```

### Coupling Analysis

```rust
use cortex_analyzer::CouplingAnalyzer;

let mut analyzer = CouplingAnalyzer::new();
analyzer.add_dependency("module_a", "module_b");
analyzer.add_dependency("module_b", "module_c");

let metrics = analyzer.analyze_coupling("module_a");
println!("Afferent coupling: {}", metrics.afferent);
println!("Efferent coupling: {}", metrics.efferent);
println!("Instability: {}", metrics.instability());
```

### Cohesion Metrics

```rust
use cortex_analyzer::CohesionMetrics;

let metrics = CohesionMetrics::from_methods(&methods);
println!("LCOM: {}", metrics.lcom);  // Lack of Cohesion of Methods
```

### Duplication Detection

```rust
use cortex_analyzer::DuplicationDetector;

let detector = DuplicationDetector::new();
let sources = vec![
    ("a.rs".to_string(), "duplicate code...".to_string()),
    ("b.rs".to_string(), "duplicate code...".to_string()),
];
let duplicates = detector.find_duplicates(&sources);
for dup in duplicates {
    println!("Duplicate in {} files", dup.locations.len());
}
```

## Usage

```rust
use cortex_analyzer::Analyzer;
use cortex_core::SearchKind;

let analyzer = Analyzer::new();

// Build a query to find code by name
let query = analyzer.build_find_code_query(
    "UserRepository",
    SearchKind::Name,
    None,  // No path filter
);
// Returns Cypher query string

// Build callers query
let query = analyzer.build_callers_query("main", Some(3));  // Depth 3
```

## Search Kinds

- `Name` - Exact name match
- `Pattern` - Regex pattern match
- `Type` - Entity kind match (Function, Class, etc.)
- `Content` - Search in source code

## Dependencies

- `cortex-core` - Core types
- `regex` - Pattern matching

## Tests

Run tests with:
```bash
cargo test -p cortex-analyzer -- --test-threads=1
```

Current test count: **61 tests**
