# cortex-parser

`cortex-parser` provides multi-language parsing using Tree-sitter.

It extracts symbols and structure used by indexer and analyzer crates.

## Supported languages

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

## What is extracted

Depending on language grammar, extraction includes functions, methods, classes/structs, interfaces/traits, enums, modules/namespaces, and related signature metadata.

## Example

```rust
use cortex_parser::ParserRegistry;
use std::path::Path;

let registry = ParserRegistry::new();
let parser = registry.parser_for_path(Path::new("src/main.rs"))?;
let result = parser.parse("fn main() {}");
```

## Test

```bash
cargo test -p cortex-parser -- --test-threads=1
```
