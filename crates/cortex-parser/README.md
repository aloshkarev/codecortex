# cortex-parser

Multi-language source code parsing using Tree-sitter.

## Overview

This crate provides parsing capabilities for multiple programming languages using Tree-sitter grammars.

## Supported Languages

| Language | Extensions | Entities Extracted |
|----------|------------|-------------------|
| Rust | `.rs` | Functions, structs, enums, traits, impl blocks, type aliases, constants, macros |
| C | `.c`, `.h` | Functions, structs, enums |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | Functions, classes, structs, enums, inheritance |
| Python | `.py` | Functions, classes, methods, decorators |
| Go | `.go` | Functions, structs, interfaces |
| TypeScript | `.ts`, `.tsx` | Functions, classes, interfaces, type aliases, enums |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, classes |
| Java | `.java` | Classes, methods, interfaces, enums, constructors |
| PHP | `.php` | Classes, functions, traits, interfaces |
| Ruby | `.rb` | Classes, methods, modules, singleton methods |

## Usage

### Parser Registry

```rust
use cortex_parser::ParserRegistry;
use std::path::Path;

let registry = ParserRegistry::new();
let parser = registry.parser_for_path(Path::new("src/main.rs"))?;
let result = parser.parse("fn main() {}");
```

### Signature Extraction

```rust
use cortex_parser::{SignatureExtractor, Language};

let extractor = SignatureExtractor::new(Language::Rust);
let signature = extractor.extract("fn add(a: i32, b: i32) -> i32 {}");
// Returns: Signature { name: "add", params: [...], return_type: Some("i32") }
```

## Dependencies

- `tree-sitter` - Parsing framework
- `tree-sitter-rust` - Rust grammar
- `tree-sitter-c` - C grammar
- `tree-sitter-cpp` - C++ grammar
- `tree-sitter-python` - Python grammar
- `tree-sitter-go` - Go grammar
- `tree-sitter-typescript` - TypeScript grammar
- `tree-sitter-javascript` - JavaScript grammar
- `tree-sitter-java` - Java grammar
- `tree-sitter-php` - PHP grammar
- `tree-sitter-ruby` - Ruby grammar

## Tests

Run tests with:
```bash
cargo test -p cortex-parser -- --test-threads=1
```

Current test count: **41 tests**
