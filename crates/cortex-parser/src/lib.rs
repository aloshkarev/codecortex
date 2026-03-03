//! # CodeCortex Parser Library
//!
//! Multi-language source code parsing using Tree-sitter.
//!
//! ## Overview
//!
//! This crate provides parsing capabilities for multiple programming languages:
//!
//! - **Language Support**: Rust, C, C++, Python, Go, TypeScript, JavaScript, Java, PHP, Ruby
//! - **Parser Registry**: [`ParserRegistry`] for automatic language detection
//! - **Signature Extraction**: [`SignatureExtractor`] for function/method signatures
//! - **Parse Results**: [`ParseResult`] containing extracted code entities
//!
//! ## Supported Languages
//!
//! | Language | Extensions | Tree-sitter Grammar |
//! |----------|------------|---------------------|
//! | Rust | `.rs` | tree-sitter-rust |
//! | C | `.c`, `.h` | tree-sitter-c |
//! | C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` | tree-sitter-cpp |
//! | Python | `.py` | tree-sitter-python |
//! | Go | `.go` | tree-sitter-go |
//! | TypeScript | `.ts`, `.tsx` | tree-sitter-typescript |
//! | JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | tree-sitter-javascript |
//! | Java | `.java` | tree-sitter-java |
//! | PHP | `.php` | tree-sitter-php |
//! | Ruby | `.rb` | tree-sitter-ruby |
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_parser::ParserRegistry;
//! use std::path::Path;
//!
//! let registry = ParserRegistry::new();
//! let parser = registry.parser_for_path(Path::new("src/main.rs")).unwrap();
//! let result = parser.parse("fn main() {}", Path::new("src/main.rs"));
//! ```
//!
//! ## Feature Flags
//!
//! This crate has no feature flags - all languages are always available.

pub mod languages;
mod parser_impl;
mod registry;
pub mod signature;

pub use parser_impl::{ParseResult, Parser};
pub use registry::ParserRegistry;
pub use signature::{Parameter, SelfType, Signature, SignatureExtractor, Visibility};
