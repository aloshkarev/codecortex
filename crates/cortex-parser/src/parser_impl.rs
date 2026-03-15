use crate::languages;
use cortex_core::{CodeEdge, CodeNode, CortexError, Language, Result};
use std::path::Path;
use tree_sitter::Parser as TsParser;

#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub nodes: Vec<CodeNode>,
    pub edges: Vec<CodeEdge>,
    /// Raw import strings (informational; edges are in `edges`)
    pub imports: Vec<String>,
    /// Raw call strings (informational; edges are in `edges`)
    pub calls: Vec<String>,
}

pub trait Parser: Send + Sync {
    fn parse(&self, source: &str, path: &Path) -> Result<ParseResult>;
}

#[derive(Clone)]
pub struct TreeSitterParser {
    language: Language,
}

impl TreeSitterParser {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    fn ts_language(&self) -> tree_sitter::Language {
        match self.language {
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
}

impl Parser for TreeSitterParser {
    fn parse(&self, source: &str, path: &Path) -> Result<ParseResult> {
        let mut ts_parser = TsParser::new();
        ts_parser
            .set_language(&self.ts_language())
            .map_err(|e| CortexError::Parse {
                path: path.display().to_string(),
                message: e.to_string(),
            })?;

        let tree = ts_parser
            .parse(source, None)
            .ok_or_else(|| CortexError::Parse {
                path: path.display().to_string(),
                message: "tree-sitter produced no tree".into(),
            })?;

        let result = match self.language {
            Language::Rust => languages::rust::extract(source, path, &tree),
            Language::C => languages::c::extract(source, path, &tree),
            Language::Cpp => languages::cpp::extract(source, path, &tree),
            Language::Python => languages::python::extract(source, path, &tree),
            Language::Go => languages::go::extract(source, path, &tree),
            Language::TypeScript => languages::typescript::extract(source, path, &tree),
            Language::JavaScript => languages::javascript::extract(source, path, &tree),
            Language::Java => languages::java::extract(source, path, &tree),
            Language::Php => languages::php::extract(source, path, &tree),
            Language::Ruby => languages::ruby::extract(source, path, &tree),
            Language::Kotlin => languages::kotlin::extract(source, path, &tree),
            Language::Swift => languages::swift::extract(source, path, &tree),
            Language::Json => languages::json::extract(source, path, &tree),
            Language::Shell => languages::shell::extract(source, path, &tree),
        };

        Ok(result)
    }
}
