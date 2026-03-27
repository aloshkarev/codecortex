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
    Kotlin,
    Swift,
    Json,
    Shell,
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
        match () {
            () if s.eq_ignore_ascii_case("rust") => Ok(Self::Rust),
            () if s.eq_ignore_ascii_case("c") => Ok(Self::C),
            () if s.eq_ignore_ascii_case("cpp")
                || s.eq_ignore_ascii_case("c++")
                || s.eq_ignore_ascii_case("cc") =>
            {
                Ok(Self::Cpp)
            }
            () if s.eq_ignore_ascii_case("python") || s.eq_ignore_ascii_case("py") => {
                Ok(Self::Python)
            }
            () if s.eq_ignore_ascii_case("go") || s.eq_ignore_ascii_case("golang") => Ok(Self::Go),
            () if s.eq_ignore_ascii_case("typescript") || s.eq_ignore_ascii_case("ts") => {
                Ok(Self::TypeScript)
            }
            () if s.eq_ignore_ascii_case("javascript") || s.eq_ignore_ascii_case("js") => {
                Ok(Self::JavaScript)
            }
            () if s.eq_ignore_ascii_case("java") => Ok(Self::Java),
            () if s.eq_ignore_ascii_case("php") => Ok(Self::Php),
            () if s.eq_ignore_ascii_case("ruby") || s.eq_ignore_ascii_case("rb") => Ok(Self::Ruby),
            () if s.eq_ignore_ascii_case("kotlin")
                || s.eq_ignore_ascii_case("kt")
                || s.eq_ignore_ascii_case("kts") =>
            {
                Ok(Self::Kotlin)
            }
            () if s.eq_ignore_ascii_case("swift") => Ok(Self::Swift),
            () if s.eq_ignore_ascii_case("json") => Ok(Self::Json),
            () if s.eq_ignore_ascii_case("shell")
                || s.eq_ignore_ascii_case("sh")
                || s.eq_ignore_ascii_case("bash")
                || s.eq_ignore_ascii_case("zsh") =>
            {
                Ok(Self::Shell)
            }
            _ => Err(ParseLanguageError),
        }
    }
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match () {
            () if ext.eq_ignore_ascii_case("rs") => Some(Self::Rust),
            () if ext.eq_ignore_ascii_case("c") || ext.eq_ignore_ascii_case("h") => Some(Self::C),
            () if ext.eq_ignore_ascii_case("cc")
                || ext.eq_ignore_ascii_case("cpp")
                || ext.eq_ignore_ascii_case("cxx")
                || ext.eq_ignore_ascii_case("hpp")
                || ext.eq_ignore_ascii_case("hh")
                || ext.eq_ignore_ascii_case("hxx") =>
            {
                Some(Self::Cpp)
            }
            () if ext.eq_ignore_ascii_case("py") => Some(Self::Python),
            () if ext.eq_ignore_ascii_case("go") => Some(Self::Go),
            () if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") => {
                Some(Self::TypeScript)
            }
            () if ext.eq_ignore_ascii_case("js")
                || ext.eq_ignore_ascii_case("jsx")
                || ext.eq_ignore_ascii_case("mjs")
                || ext.eq_ignore_ascii_case("cjs") =>
            {
                Some(Self::JavaScript)
            }
            () if ext.eq_ignore_ascii_case("java") => Some(Self::Java),
            () if ext.eq_ignore_ascii_case("php") => Some(Self::Php),
            () if ext.eq_ignore_ascii_case("rb") => Some(Self::Ruby),
            () if ext.eq_ignore_ascii_case("kt") || ext.eq_ignore_ascii_case("kts") => {
                Some(Self::Kotlin)
            }
            () if ext.eq_ignore_ascii_case("swift") => Some(Self::Swift),
            () if ext.eq_ignore_ascii_case("json") => Some(Self::Json),
            () if ext.eq_ignore_ascii_case("sh")
                || ext.eq_ignore_ascii_case("bash")
                || ext.eq_ignore_ascii_case("zsh") =>
            {
                Some(Self::Shell)
            }
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
            Self::Kotlin => "kotlin",
            Self::Swift => "swift",
            Self::Json => "json",
            Self::Shell => "shell",
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
        assert_eq!(
            "typescript".parse::<Language>().unwrap(),
            Language::TypeScript
        );
        assert_eq!("ts".parse::<Language>().unwrap(), Language::TypeScript);
    }

    #[test]
    fn language_from_str_javascript() {
        assert_eq!(
            "javascript".parse::<Language>().unwrap(),
            Language::JavaScript
        );
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
    fn language_from_str_kotlin() {
        assert_eq!("kotlin".parse::<Language>().unwrap(), Language::Kotlin);
        assert_eq!("kt".parse::<Language>().unwrap(), Language::Kotlin);
        assert_eq!("kts".parse::<Language>().unwrap(), Language::Kotlin);
    }

    #[test]
    fn language_from_str_swift() {
        assert_eq!("swift".parse::<Language>().unwrap(), Language::Swift);
        assert_eq!("SWIFT".parse::<Language>().unwrap(), Language::Swift);
    }

    #[test]
    fn language_from_str_json() {
        assert_eq!("json".parse::<Language>().unwrap(), Language::Json);
        assert_eq!("JSON".parse::<Language>().unwrap(), Language::Json);
    }

    #[test]
    fn language_from_str_shell() {
        assert_eq!("shell".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("sh".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("bash".parse::<Language>().unwrap(), Language::Shell);
        assert_eq!("zsh".parse::<Language>().unwrap(), Language::Shell);
    }

    #[test]
    fn language_from_str_unknown() {
        assert!("unknown".parse::<Language>().is_err());
        assert!("lua".parse::<Language>().is_err());
        assert!("yaml".parse::<Language>().is_err());
    }

    #[test]
    fn language_from_path_rust() {
        assert_eq!(
            Language::from_path(Path::new("src/main.rs")),
            Some(Language::Rust)
        );
    }

    #[test]
    fn language_from_path_python() {
        assert_eq!(
            Language::from_path(Path::new("app.py")),
            Some(Language::Python)
        );
    }

    #[test]
    fn language_from_path_go() {
        assert_eq!(
            Language::from_path(Path::new("main.go")),
            Some(Language::Go)
        );
    }

    #[test]
    fn language_from_path_typescript() {
        assert_eq!(
            Language::from_path(Path::new("app.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("component.tsx")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn language_from_path_javascript() {
        assert_eq!(
            Language::from_path(Path::new("index.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("component.jsx")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("module.mjs")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("common.cjs")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn language_from_path_c() {
        assert_eq!(Language::from_path(Path::new("main.c")), Some(Language::C));
        assert_eq!(
            Language::from_path(Path::new("header.h")),
            Some(Language::C)
        );
    }

    #[test]
    fn language_from_path_cpp() {
        assert_eq!(
            Language::from_path(Path::new("main.cpp")),
            Some(Language::Cpp)
        );
        assert_eq!(
            Language::from_path(Path::new("header.hpp")),
            Some(Language::Cpp)
        );
        assert_eq!(
            Language::from_path(Path::new("impl.cc")),
            Some(Language::Cpp)
        );
        assert_eq!(
            Language::from_path(Path::new("impl.cxx")),
            Some(Language::Cpp)
        );
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
        assert_eq!(
            Language::from_path(Path::new("Main.java")),
            Some(Language::Java)
        );
    }

    #[test]
    fn language_from_path_php() {
        assert_eq!(
            Language::from_path(Path::new("index.php")),
            Some(Language::Php)
        );
    }

    #[test]
    fn language_from_path_ruby() {
        assert_eq!(
            Language::from_path(Path::new("app.rb")),
            Some(Language::Ruby)
        );
    }

    #[test]
    fn language_from_path_kotlin() {
        assert_eq!(
            Language::from_path(Path::new("src/Main.kt")),
            Some(Language::Kotlin)
        );
        assert_eq!(
            Language::from_path(Path::new("build.gradle.kts")),
            Some(Language::Kotlin)
        );
    }

    #[test]
    fn language_from_path_swift() {
        assert_eq!(
            Language::from_path(Path::new("Sources/App.swift")),
            Some(Language::Swift)
        );
    }

    #[test]
    fn language_from_path_json() {
        assert_eq!(
            Language::from_path(Path::new("package.json")),
            Some(Language::Json)
        );
    }

    #[test]
    fn language_from_path_shell() {
        assert_eq!(
            Language::from_path(Path::new("scripts/build.sh")),
            Some(Language::Shell)
        );
        assert_eq!(
            Language::from_path(Path::new("scripts/run.bash")),
            Some(Language::Shell)
        );
        assert_eq!(
            Language::from_path(Path::new("scripts/dev.zsh")),
            Some(Language::Shell)
        );
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
        assert_eq!(Language::Kotlin.as_str(), "kotlin");
        assert_eq!(Language::Swift.as_str(), "swift");
        assert_eq!(Language::Json.as_str(), "json");
        assert_eq!(Language::Shell.as_str(), "shell");
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

        let lang: Language = serde_json::from_str("\"kotlin\"").unwrap();
        assert_eq!(lang, Language::Kotlin);
    }

    #[test]
    fn language_equality() {
        assert_eq!(Language::Rust, Language::Rust);
        assert_ne!(Language::Rust, Language::Python);
    }
}
