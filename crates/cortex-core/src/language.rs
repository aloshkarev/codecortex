use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    C,
    Cpp,
    Python,
    Go,
    TypeScript,
    JavaScript,
    Java,
    Php,
    Ruby,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLanguageError;

impl std::fmt::Display for ParseLanguageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown language")
    }
}

impl std::error::Error for ParseLanguageError {}

impl FromStr for Language {
    type Err = ParseLanguageError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "rust" => Ok(Self::Rust),
            "c" => Ok(Self::C),
            "cpp" | "c++" | "cc" => Ok(Self::Cpp),
            "python" | "py" => Ok(Self::Python),
            "go" | "golang" => Ok(Self::Go),
            "typescript" | "ts" => Ok(Self::TypeScript),
            "javascript" | "js" => Ok(Self::JavaScript),
            "java" => Ok(Self::Java),
            "php" => Ok(Self::Php),
            "ruby" | "rb" => Ok(Self::Ruby),
            _ => Err(ParseLanguageError),
        }
    }
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "rs" => Some(Self::Rust),
            "c" => Some(Self::C),
            "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            "py" => Some(Self::Python),
            "go" => Some(Self::Go),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "java" => Some(Self::Java),
            "php" => Some(Self::Php),
            "rb" => Some(Self::Ruby),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Python => "python",
            Self::Go => "go",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Java => "java",
            Self::Php => "php",
            Self::Ruby => "ruby",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_from_str_rust() {
        assert_eq!("rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("RUST".parse::<Language>().unwrap(), Language::Rust);
    }

    #[test]
    fn language_from_str_python() {
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("py".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("PYTHON".parse::<Language>().unwrap(), Language::Python);
    }

    #[test]
    fn language_from_str_go() {
        assert_eq!("go".parse::<Language>().unwrap(), Language::Go);
        assert_eq!("golang".parse::<Language>().unwrap(), Language::Go);
    }

    #[test]
    fn language_from_str_typescript() {
        assert_eq!("typescript".parse::<Language>().unwrap(), Language::TypeScript);
        assert_eq!("ts".parse::<Language>().unwrap(), Language::TypeScript);
    }

    #[test]
    fn language_from_str_javascript() {
        assert_eq!("javascript".parse::<Language>().unwrap(), Language::JavaScript);
        assert_eq!("js".parse::<Language>().unwrap(), Language::JavaScript);
    }

    #[test]
    fn language_from_str_cpp() {
        assert_eq!("cpp".parse::<Language>().unwrap(), Language::Cpp);
        assert_eq!("c++".parse::<Language>().unwrap(), Language::Cpp);
        assert_eq!("cc".parse::<Language>().unwrap(), Language::Cpp);
    }

    #[test]
    fn language_from_str_c() {
        assert_eq!("c".parse::<Language>().unwrap(), Language::C);
    }

    #[test]
    fn language_from_str_java() {
        assert_eq!("java".parse::<Language>().unwrap(), Language::Java);
        assert_eq!("JAVA".parse::<Language>().unwrap(), Language::Java);
    }

    #[test]
    fn language_from_str_php() {
        assert_eq!("php".parse::<Language>().unwrap(), Language::Php);
        assert_eq!("PHP".parse::<Language>().unwrap(), Language::Php);
    }

    #[test]
    fn language_from_str_ruby() {
        assert_eq!("ruby".parse::<Language>().unwrap(), Language::Ruby);
        assert_eq!("RUBY".parse::<Language>().unwrap(), Language::Ruby);
        assert_eq!("rb".parse::<Language>().unwrap(), Language::Ruby);
    }

    #[test]
    fn language_from_str_unknown() {
        assert!("unknown".parse::<Language>().is_err());
        assert!("kotlin".parse::<Language>().is_err());
        assert!("swift".parse::<Language>().is_err());
    }

    #[test]
    fn language_from_path_rust() {
        assert_eq!(Language::from_path(Path::new("src/main.rs")), Some(Language::Rust));
    }

    #[test]
    fn language_from_path_python() {
        assert_eq!(Language::from_path(Path::new("app.py")), Some(Language::Python));
    }

    #[test]
    fn language_from_path_go() {
        assert_eq!(Language::from_path(Path::new("main.go")), Some(Language::Go));
    }

    #[test]
    fn language_from_path_typescript() {
        assert_eq!(Language::from_path(Path::new("app.ts")), Some(Language::TypeScript));
        assert_eq!(Language::from_path(Path::new("component.tsx")), Some(Language::TypeScript));
    }

    #[test]
    fn language_from_path_javascript() {
        assert_eq!(Language::from_path(Path::new("index.js")), Some(Language::JavaScript));
        assert_eq!(Language::from_path(Path::new("component.jsx")), Some(Language::JavaScript));
        assert_eq!(Language::from_path(Path::new("module.mjs")), Some(Language::JavaScript));
        assert_eq!(Language::from_path(Path::new("common.cjs")), Some(Language::JavaScript));
    }

    #[test]
    fn language_from_path_c() {
        assert_eq!(Language::from_path(Path::new("main.c")), Some(Language::C));
    }

    #[test]
    fn language_from_path_cpp() {
        assert_eq!(Language::from_path(Path::new("main.cpp")), Some(Language::Cpp));
        assert_eq!(Language::from_path(Path::new("header.hpp")), Some(Language::Cpp));
        assert_eq!(Language::from_path(Path::new("impl.cc")), Some(Language::Cpp));
        assert_eq!(Language::from_path(Path::new("impl.cxx")), Some(Language::Cpp));
    }

    #[test]
    fn language_from_path_no_extension() {
        assert_eq!(Language::from_path(Path::new("Makefile")), None);
    }

    #[test]
    fn language_from_path_unknown_extension() {
        assert_eq!(Language::from_path(Path::new("file.txt")), None);
        assert_eq!(Language::from_path(Path::new("file.md")), None);
    }

    #[test]
    fn language_from_path_java() {
        assert_eq!(Language::from_path(Path::new("Main.java")), Some(Language::Java));
    }

    #[test]
    fn language_from_path_php() {
        assert_eq!(Language::from_path(Path::new("index.php")), Some(Language::Php));
    }

    #[test]
    fn language_from_path_ruby() {
        assert_eq!(Language::from_path(Path::new("app.rb")), Some(Language::Ruby));
    }

    #[test]
    fn language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Python.as_str(), "python");
        assert_eq!(Language::Go.as_str(), "go");
        assert_eq!(Language::TypeScript.as_str(), "typescript");
        assert_eq!(Language::JavaScript.as_str(), "javascript");
        assert_eq!(Language::C.as_str(), "c");
        assert_eq!(Language::Cpp.as_str(), "cpp");
        assert_eq!(Language::Java.as_str(), "java");
        assert_eq!(Language::Php.as_str(), "php");
        assert_eq!(Language::Ruby.as_str(), "ruby");
    }

    #[test]
    fn language_serialization() {
        let lang = Language::Rust;
        let json = serde_json::to_string(&lang).unwrap();
        assert_eq!(json, "\"rust\"");

        let lang = Language::TypeScript;
        let json = serde_json::to_string(&lang).unwrap();
        assert_eq!(json, "\"typescript\"");
    }

    #[test]
    fn language_deserialization() {
        let lang: Language = serde_json::from_str("\"python\"").unwrap();
        assert_eq!(lang, Language::Python);

        let lang: Language = serde_json::from_str("\"go\"").unwrap();
        assert_eq!(lang, Language::Go);
    }

    #[test]
    fn language_equality() {
        assert_eq!(Language::Rust, Language::Rust);
        assert_ne!(Language::Rust, Language::Python);
    }
}
