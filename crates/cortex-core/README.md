# cortex-core

`cortex-core` contains shared contracts and utilities used across the workspace.

## Main components

- Core graph types (`CodeNode`, `CodeEdge`, entity and edge kinds)
- Language detection (`Language`)
- Shared config (`CortexConfig`)
- Shared error type (`CortexError`)
- Complexity helpers (cyclomatic and cognitive)
- Git helpers used by project and indexing flows

## Example

```rust
use cortex_core::Language;
use std::path::Path;

let lang = Language::from_path(Path::new("src/main.rs"));
assert_eq!(lang, Some(Language::Rust));
```

```rust
use cortex_core::compute_cyclomatic_complexity;

let c = compute_cyclomatic_complexity("fn f() { if ok { a() } else { b() } }");
assert_eq!(c, 2);
```

## Supported language detection

- Rust
- Python
- Go
- TypeScript
- JavaScript
- C
- C++
- Java
- PHP
- Ruby
- Kotlin
- Swift
- JSON
- Shell

## Test

```bash
cargo test -p cortex-core -- --test-threads=1
```
