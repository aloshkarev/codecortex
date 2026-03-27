# cortex-parser

> `cortex-parser` provides multi-language AST parsing using Tree-sitter grammars for all 14 CodeCortex-supported languages. It extracts symbols, relationships, and qualified names that feed the indexer, analyzer, and navigation layers.

## What it does

- Parses source files into structured `CodeNode` and `CodeEdge` records via language-specific grammar handlers
- Emits `qualified_name` on every extracted entity for accurate cross-file symbol disambiguation
- Emits navigation edges (`MEMBER_OF`, `TYPE_REFERENCE`, `FIELD_ACCESS`) for supported languages
- Exposes a `ParserRegistry` that selects the correct parser from a file path or language enum

## Supported languages

| Language | Extensions | Parser |
|----------|-----------|--------|
| Rust | `.rs` | `tree-sitter-rust` |
| Python | `.py` | `tree-sitter-python` |
| Go | `.go` | `tree-sitter-go` |
| TypeScript | `.ts`, `.tsx` | `tree-sitter-typescript` |
| JavaScript | `.js`, `.jsx` | `tree-sitter-javascript` |
| C | `.c`, `.h` | `tree-sitter-c` |
| C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | `tree-sitter-cpp` |
| Java | `.java` | `tree-sitter-java` |
| PHP | `.php` | `tree-sitter-php` |
| Ruby | `.rb` | `tree-sitter-ruby` |
| Kotlin | `.kt`, `.kts` | `tree-sitter-kotlin` |
| Swift | `.swift` | `tree-sitter-swift` |
| JSON | `.json` | `tree-sitter-json` |
| Shell | `.sh`, `.bash`, `.zsh` | `tree-sitter-bash` |

## What is extracted

Extraction scope depends on language grammar coverage:

| Entity type | Rust | Python | Go | TS/JS | C/C++ | Java | PHP | Ruby | Kotlin | Swift | JSON | Shell |
|------------|:----:|:------:|:--:|:-----:|:-----:|:----:|:---:|:----:|:------:|:-----:|:----:|:-----:|
| Functions / free functions | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | ✓ |
| Methods | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | — |
| Classes / structs / enums | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | — |
| Traits / interfaces | ✓ | — | ✓ | ✓ | — | ✓ | ✓ | ✓ | ✓ | ✓ | — | — |
| Modules / namespaces | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | — | — |
| Signatures / visibility | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | partial |
| `qualified_name` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — | partial |

## Navigation edge emission

For languages with sufficient grammar support, the parser emits additional edges used by `go_to_definition`, `find_all_usages`, `quick_info`, and structural diff:

| Edge | Rust | Python | TypeScript | Go | Java |
|------|:----:|:------:|:----------:|:--:|:----:|
| `MEMBER_OF` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `TYPE_REFERENCE` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `FIELD_ACCESS` | ✓ | ✓ | ✓ | ✓ | ✓ |

These edges are resolved and linked to concrete graph nodes by `cortex-indexer` during the reconciliation pass.

## Usage

```rust
use cortex_parser::ParserRegistry;
use std::path::Path;

let registry = ParserRegistry::new();

// Auto-select parser from file path
let parser = registry.parser_for_path(Path::new("src/main.rs"))?;
let result = parser.parse("fn main() {}");
println!("Extracted {} nodes", result.nodes.len());

// Access qualified names
for node in &result.nodes {
    println!("{} ({})", node.qualified_name, node.kind);
}
```

## Tests

```bash
cargo test -p cortex-parser -- --test-threads=1
```
