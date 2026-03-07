use crate::parser_impl::{Parser, TreeSitterParser};
use cortex_core::{CortexError, Language, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct ParserRegistry {
    parsers: HashMap<Language, Arc<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut parsers: HashMap<Language, Arc<dyn Parser>> = HashMap::new();
        for lang in [
            Language::Rust,
            Language::C,
            Language::Cpp,
            Language::Python,
            Language::Go,
            Language::TypeScript,
            Language::JavaScript,
            Language::Java,
            Language::Php,
            Language::Ruby,
        ] {
            parsers.insert(lang, Arc::new(TreeSitterParser::new(lang)));
        }
        Self { parsers }
    }

    pub fn parser_for_path(&self, path: &Path) -> Result<Arc<dyn Parser>> {
        let language = Language::from_path(path)
            .ok_or_else(|| CortexError::UnsupportedLanguage(path.display().to_string()))?;
        self.parsers
            .get(&language)
            .cloned()
            .ok_or_else(|| CortexError::UnsupportedLanguage(path.display().to_string()))
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parser_registry_new() {
        let registry = ParserRegistry::new();
        // Registry should have parsers for all supported languages
        assert!(!registry.parsers.is_empty());
    }

    #[test]
    fn parser_registry_default() {
        let registry = ParserRegistry::default();
        assert_eq!(registry.parsers.len(), ParserRegistry::new().parsers.len());
    }

    #[test]
    fn parser_for_path_rust() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("src/main.rs");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_python() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("app.py");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_go() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("main.go");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_typescript() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("index.ts");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());

        let path = PathBuf::from("component.tsx");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_javascript() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("index.js");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());

        let path = PathBuf::from("module.mjs");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_c() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("main.c");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_cpp() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("main.cpp");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());

        let path = PathBuf::from("header.hpp");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn parser_for_path_unsupported() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("README.md");
        let result = registry.parser_for_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn parser_for_path_no_extension() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("Makefile");
        let result = registry.parser_for_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn parser_registry_clone() {
        let registry = ParserRegistry::new();
        let cloned = registry.clone();
        // Both should have the same number of parsers
        assert_eq!(registry.parsers.len(), cloned.parsers.len());
    }

    #[test]
    fn parser_for_path_with_complex_path() {
        let registry = ParserRegistry::new();
        let path = PathBuf::from("/home/user/projects/myapp/src/components/Button.tsx");
        let result = registry.parser_for_path(&path);
        assert!(result.is_ok());
    }
}
