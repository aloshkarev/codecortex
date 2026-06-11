use crate::languages;
use crate::parse_pool;
use cortex_core::{CodeEdge, CodeNode, Language, Result};
use std::path::Path;

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
}

impl Parser for TreeSitterParser {
    fn parse(&self, source: &str, path: &Path) -> Result<ParseResult> {
        let tree = parse_pool::parse_tree(self.language, source, path)?;

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
