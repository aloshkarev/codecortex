# cortex-core

Core types, error handling, and utilities for the CodeCortex code intelligence system.

## Overview

This crate provides the foundational types and utilities used across all CodeCortex crates.

## Key Types

| Type | Description |
|------|-------------|
| `CodeNode` | Represents a code entity (function, class, etc.) |
| `CodeEdge` | Represents a relationship between code entities |
| `EntityKind` | Enum of entity types (Function, Class, Struct, etc.) |
| `EdgeKind` | Enum of relationship types (Calls, Imports, Inherits, etc.) |
| `Language` | Supported programming languages |
| `CortexConfig` | Application configuration |
| `CortexError` | Error type for all operations |
| `GitOperations` | Git history traversal and blame |

## Features

### Language Detection

```rust
use cortex_core::Language;
use std::path::Path;

let lang = Language::from_path(Path::new("src/main.rs"));
assert_eq!(lang, Some(Language::Rust));
```

### Cyclomatic Complexity

```rust
use cortex_core::compute_cyclomatic_complexity;

let code = "fn test() { if x { a } else { b } }";
let complexity = compute_cyclomatic_complexity(code);
assert_eq!(complexity, 2);
```

### Project State Management

```rust
use cortex_core::{ProjectState, GitInfo, ProjectRef};

let state = ProjectState::new("/path/to/repo");
let branch = state.current_branch();
let is_stale = state.is_current_index_stale();
```

### Git Operations

```rust
use cortex_core::GitOperations;
use std::path::Path;

let git = GitOperations::new(Path::new("/path/to/repo"))?;
let history = git.traverse_history(Some(100))?;
let blame = git.get_blame_info(Path::new("src/main.rs"), 10)?;
let diff = git.compare_branches("main", "feature")?;
```

## Supported Languages

- Rust (`.rs`)
- C (`.c`, `.h`)
- C++ (`.cpp`, `.hpp`, `.cc`, `.cxx`, `.hh`, `.hxx`)
- Python (`.py`)
- Go (`.go`)
- TypeScript (`.ts`, `.tsx`)
- JavaScript (`.js`, `.jsx`, `.mjs`, `.cjs`)
- Java (`.java`)
- PHP (`.php`)
- Ruby (`.rb`)

## Dependencies

- `serde` - Serialization
- `thiserror` - Error handling
- `git2` - Git operations

## Tests

Run tests with:
```bash
cargo test -p cortex-core -- --test-threads=1
```

Current test count: **100 tests**
