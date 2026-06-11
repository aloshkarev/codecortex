//! Thread-local reuse of `tree_sitter::Parser` instances.
//!
//! Creating and `set_language` on every file parse dominated CPU on large repos.
//! Rayon workers each get one parser per [`cortex_core::Language`].

use cortex_core::{CortexError, Language, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language as TsLanguage, Parser as TsParser, Tree};

fn ts_language(lang: Language) -> TsLanguage {
    match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
        Language::Swift => tree_sitter_swift::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Shell => tree_sitter_bash::LANGUAGE.into(),
    }
}

thread_local! {
    static PARSERS: RefCell<HashMap<Language, TsParser>> = RefCell::new(HashMap::new());
}

/// Parse `source` with a thread-local parser for `language`.
pub fn parse_tree(language: Language, source: &str, path: &Path) -> Result<Tree> {
    PARSERS.with(|cell| {
        let mut map = cell.borrow_mut();
        let entry = map.entry(language).or_insert_with(|| {
            let mut parser = TsParser::new();
            parser
                .set_language(&ts_language(language))
                .expect("tree-sitter language setup is static");
            parser
        });
        entry.parse(source, None).ok_or_else(|| CortexError::Parse {
            path: path.display().to_string(),
            message: "tree-sitter produced no tree".into(),
        })
    })
}
