//! Build System Detection for CodeCortex Indexer
//!
//! Detects build systems and configurations to improve indexing accuracy:
//! - Cargo (Rust): Cargo.toml
//! - CMake (C/C++): CMakeLists.txt
//! - Make: Makefile, makefile
//! - Go Modules: go.mod
//! - Python: pyproject.toml, setup.py, requirements.txt
//!
//! For C/C++ projects, also parses compile_commands.json for:
//! - Include paths
//! - Preprocessor definitions
//! - Compiler flags

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Detected build system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildSystem {
    /// Rust with Cargo
    Cargo,
    /// C/C++ with CMake
    CMake,
    /// C/C++ with Make
    Make,
    /// Go modules
    GoModules,
    /// Python with setuptools/pip
    Python,
    /// Node.js/npm
    Npm,
    /// Node.js/yarn
    Yarn,
    /// Node.js/pnpm
    Pnpm,
    /// Mixed/multiple build systems
    Mixed,
}

impl BuildSystem {
    /// Get the primary language for this build system
    pub fn primary_languages(&self) -> Vec<&'static str> {
        match self {
            BuildSystem::Cargo => vec!["rust"],
            BuildSystem::CMake => vec!["c", "cpp"],
            BuildSystem::Make => vec!["c", "cpp"],
            BuildSystem::GoModules => vec!["go"],
            BuildSystem::Python => vec!["python"],
            BuildSystem::Npm | BuildSystem::Yarn | BuildSystem::Pnpm => {
                vec!["javascript", "typescript"]
            }
            BuildSystem::Mixed => vec![],
        }
    }

    /// Get the config file names for this build system
    pub fn config_files(&self) -> Vec<&'static str> {
        match self {
            BuildSystem::Cargo => vec!["Cargo.toml", "Cargo.lock"],
            BuildSystem::CMake => vec!["CMakeLists.txt", "CMakeCache.txt"],
            BuildSystem::Make => vec!["Makefile", "makefile", "GNUmakefile"],
            BuildSystem::GoModules => vec!["go.mod", "go.sum"],
            BuildSystem::Python => vec![
                "pyproject.toml",
                "setup.py",
                "setup.cfg",
                "requirements.txt",
            ],
            BuildSystem::Npm => vec!["package.json", "package-lock.json"],
            BuildSystem::Yarn => vec!["package.json", "yarn.lock"],
            BuildSystem::Pnpm => vec!["package.json", "pnpm-lock.yaml"],
            BuildSystem::Mixed => vec![],
        }
    }
}

impl std::fmt::Display for BuildSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildSystem::Cargo => write!(f, "cargo"),
            BuildSystem::CMake => write!(f, "cmake"),
            BuildSystem::Make => write!(f, "make"),
            BuildSystem::GoModules => write!(f, "go_modules"),
            BuildSystem::Python => write!(f, "python"),
            BuildSystem::Npm => write!(f, "npm"),
            BuildSystem::Yarn => write!(f, "yarn"),
            BuildSystem::Pnpm => write!(f, "pnpm"),
            BuildSystem::Mixed => write!(f, "mixed"),
        }
    }
}

/// Detected project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Root path of the project
    pub root_path: PathBuf,
    /// Detected build systems (may be multiple in monorepos)
    pub build_systems: Vec<BuildSystem>,
    /// Project name (from build config if available)
    pub name: Option<String>,
    /// Project version (from build config if available)
    pub version: Option<String>,
    /// Include paths for C/C++ projects
    pub include_paths: Vec<PathBuf>,
    /// Preprocessor definitions for C/C++
    pub defines: HashMap<String, Option<String>>,
    /// Source directories
    pub source_dirs: Vec<PathBuf>,
    /// Exclude patterns
    pub exclude_patterns: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<Dependency>,
    /// Compile commands if available
    pub compile_commands: Vec<CompileCommand>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::new(),
            build_systems: Vec::new(),
            name: None,
            version: None,
            include_paths: Vec::new(),
            defines: HashMap::new(),
            source_dirs: Vec::new(),
            exclude_patterns: Vec::new(),
            dependencies: Vec::new(),
            compile_commands: Vec::new(),
        }
    }
}

/// A project dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Production dependency
    Production,
    /// Development dependency
    Development,
    /// Build dependency
    Build,
}

/// A compile command from compile_commands.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileCommand {
    /// The source file being compiled
    pub file: PathBuf,
    /// The compile command as a string
    pub command: Option<String>,
    /// The working directory
    pub directory: PathBuf,
    /// Parsed arguments (alternative to command)
    pub arguments: Option<Vec<String>>,
    /// The output file
    pub output: Option<PathBuf>,
}

impl CompileCommand {
    /// Resolve the source file path relative to the compile command directory.
    pub fn resolved_file_path(&self) -> PathBuf {
        if self.file.is_absolute() {
            self.file.clone()
        } else {
            self.directory.join(&self.file)
        }
    }

    /// Extract include paths from this compile command
    pub fn include_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Use arguments if available, otherwise parse command string
        let args: Vec<String> = if let Some(ref args) = self.arguments {
            args.clone()
        } else if let Some(ref cmd) = self.command {
            shlex::split(cmd).unwrap_or_default()
        } else {
            return paths;
        };

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg == "-I" || arg == "-iquote" {
                if let Some(path) = iter.next() {
                    paths.push(PathBuf::from(path));
                }
            } else if let Some(stripped) = arg.strip_prefix("-I") {
                paths.push(PathBuf::from(stripped));
            } else if let Some(stripped) = arg.strip_prefix("-isystem") {
                if !stripped.is_empty() {
                    paths.push(PathBuf::from(stripped));
                } else if let Some(path) = iter.next() {
                    paths.push(PathBuf::from(path));
                }
            }
        }
        paths
    }

    /// Extract include paths resolved relative to the compile command directory.
    pub fn resolved_include_paths(&self) -> Vec<PathBuf> {
        self.include_paths()
            .into_iter()
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    self.directory.join(path)
                }
            })
            .collect()
    }

    /// Extract preprocessor definitions from this compile command
    pub fn defines(&self) -> HashMap<String, Option<String>> {
        let mut defines = HashMap::new();

        // Use arguments if available, otherwise parse command string
        let args: Vec<String> = if let Some(ref args) = self.arguments {
            args.clone()
        } else if let Some(ref cmd) = self.command {
            shlex::split(cmd).unwrap_or_default()
        } else {
            return defines;
        };

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg == "-D" {
                if let Some(def) = iter.next() {
                    let (key, value) = parse_define(def);
                    defines.insert(key, value);
                }
            } else if let Some(stripped) = arg.strip_prefix("-D") {
                let (key, value) = parse_define(stripped);
                defines.insert(key, value);
            }
        }
        defines
    }
}

/// Parse a preprocessor definition like "NAME" or "NAME=value"
fn parse_define(s: &str) -> (String, Option<String>) {
    match s.split_once('=') {
        Some((key, value)) => (key.to_string(), Some(value.to_string())),
        None => (s.to_string(), None),
    }
}

/// Build system detector
#[derive(Debug, Clone)]
pub struct BuildDetector {
    /// Root path to detect from
    root: PathBuf,
}

impl BuildDetector {
    /// Create a new detector for the given root path
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Detect all build systems in the project
    pub fn detect(&self) -> ProjectConfig {
        let mut config = ProjectConfig {
            root_path: self.root.clone(),
            ..Default::default()
        };

        // Check for each build system
        let mut detected = Vec::new();

        if self.has_cargo() {
            detected.push(BuildSystem::Cargo);
            self.enrich_cargo_config(&mut config);
        }

        if self.has_cmake() {
            detected.push(BuildSystem::CMake);
            self.enrich_cmake_config(&mut config);
        }

        if self.has_make() {
            detected.push(BuildSystem::Make);
        }

        if self.has_go_modules() {
            detected.push(BuildSystem::GoModules);
            self.enrich_go_config(&mut config);
        }

        if self.has_python() {
            detected.push(BuildSystem::Python);
            self.enrich_python_config(&mut config);
        }

        if self.has_npm() {
            detected.push(BuildSystem::Npm);
            self.enrich_npm_config(&mut config);
        }

        if self.has_yarn() {
            detected.push(BuildSystem::Yarn);
        }

        if self.has_pnpm() {
            detected.push(BuildSystem::Pnpm);
            self.enrich_pnpm_config(&mut config);
        }

        // Additional language-aware hints for projects that may not use
        // currently-modeled build systems in this enum.
        if self.has_gradle_kotlin() {
            config.source_dirs.push(self.root.join("src/main/kotlin"));
            config.source_dirs.push(self.root.join("src/test/kotlin"));
            config.source_dirs.push(self.root.join("src/main/java"));
            config.source_dirs.push(self.root.join("src/test/java"));
        }
        if self.has_swift_package() {
            config.source_dirs.push(self.root.join("Sources"));
            config.source_dirs.push(self.root.join("Tests"));
        }

        // Load compile_commands.json if present (for C/C++ projects)
        if detected.contains(&BuildSystem::CMake) || detected.contains(&BuildSystem::Make) {
            self.load_compile_commands(&mut config);
        }

        // Deduplicate build systems
        config.build_systems = detected
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Mark as mixed if multiple distinct systems
        if config.build_systems.len() > 1 {
            // If we have both C/C++ and non-C/C++, or multiple unrelated systems
            let lang_groups = count_language_groups(&config.build_systems);
            if lang_groups > 1 {
                config.build_systems.push(BuildSystem::Mixed);
            }
        }

        config
    }

    /// Check if Cargo.toml exists
    fn has_cargo(&self) -> bool {
        self.root.join("Cargo.toml").exists()
    }

    /// Check for CMakeLists.txt
    fn has_cmake(&self) -> bool {
        self.root.join("CMakeLists.txt").exists()
    }

    /// Check for Makefile variants
    fn has_make(&self) -> bool {
        ["Makefile", "makefile", "GNUmakefile"]
            .iter()
            .any(|name| self.root.join(name).exists())
    }

    /// Check for go.mod
    fn has_go_modules(&self) -> bool {
        self.root.join("go.mod").exists()
    }

    /// Check for Python project files
    fn has_python(&self) -> bool {
        [
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "requirements.txt",
        ]
        .iter()
        .any(|name| self.root.join(name).exists())
    }

    /// Check for package.json with npm lock
    fn has_npm(&self) -> bool {
        self.root.join("package.json").exists() && self.root.join("package-lock.json").exists()
    }

    /// Check for yarn.lock
    fn has_yarn(&self) -> bool {
        self.root.join("package.json").exists() && self.root.join("yarn.lock").exists()
    }

    /// Check for pnpm-lock.yaml
    fn has_pnpm(&self) -> bool {
        self.root.join("package.json").exists() && self.root.join("pnpm-lock.yaml").exists()
    }

    /// Check for Kotlin Gradle build files
    fn has_gradle_kotlin(&self) -> bool {
        ["build.gradle.kts", "settings.gradle.kts", "gradle.properties"]
            .iter()
            .any(|name| self.root.join(name).exists())
    }

    /// Check for Swift Package Manager manifests
    fn has_swift_package(&self) -> bool {
        self.root.join("Package.swift").exists()
    }

    /// Enrich config with Cargo information
    fn enrich_cargo_config(&self, config: &mut ProjectConfig) {
        let cargo_toml = self.root.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            // Simple TOML parsing for name and version
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("name = ") {
                    config.name = Some(extract_toml_string(line));
                } else if line.starts_with("version = ") {
                    config.version = Some(extract_toml_string(line));
                }
            }
        }

        // Add standard Rust source directories
        config.source_dirs.push(self.root.join("src"));
        config.source_dirs.push(self.root.join("tests"));
        config.source_dirs.push(self.root.join("benches"));
        config.source_dirs.push(self.root.join("examples"));

        // Exclude target directory
        config.exclude_patterns.push("target/**".to_string());
    }

    /// Enrich config with CMake information
    fn enrich_cmake_config(&self, config: &mut ProjectConfig) {
        // Add common C/C++ source directories
        for dir in &["src", "include", "lib", "inc", "source"] {
            let path = self.root.join(dir);
            if path.exists() {
                config.source_dirs.push(path);
            }
        }

        // Add common include paths
        for dir in &["include", "inc", "src"] {
            let path = self.root.join(dir);
            if path.exists() {
                config.include_paths.push(path);
            }
        }

        // Standard defines for CMake projects
        config.defines.insert("NDEBUG".to_string(), None);
    }

    /// Enrich config with Go module information
    fn enrich_go_config(&self, config: &mut ProjectConfig) {
        let go_mod = self.root.join("go.mod");
        if let Ok(content) = std::fs::read_to_string(&go_mod) {
            for line in content.lines() {
                let line = line.trim();
                if let Some(name) = line.strip_prefix("module ") {
                    config.name = Some(name.to_string());
                    break;
                }
            }
        }

        // Add standard Go source directories
        for dir in &["cmd", "pkg", "internal", "api"] {
            let path = self.root.join(dir);
            if path.exists() {
                config.source_dirs.push(path);
            }
        }

        // Exclude vendor directory
        config.exclude_patterns.push("vendor/**".to_string());
    }

    /// Enrich config with Python project information
    fn enrich_python_config(&self, config: &mut ProjectConfig) {
        let pyproject = self.root.join("pyproject.toml");
        if let Ok(content) = std::fs::read_to_string(&pyproject) {
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("name = ") {
                    config.name = Some(extract_toml_string(line));
                } else if line.starts_with("version = ") {
                    config.version = Some(extract_toml_string(line));
                }
            }
        }

        // Exclude common Python cache directories
        config.exclude_patterns.push("__pycache__/**".to_string());
        config.exclude_patterns.push("*.pyc".to_string());
        config.exclude_patterns.push(".venv/**".to_string());
        config.exclude_patterns.push("venv/**".to_string());
    }

    /// Enrich config with npm package information
    fn enrich_npm_config(&self, config: &mut ProjectConfig) {
        let package_json = self.root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&package_json)
            && let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content)
        {
            config.name = pkg["name"].as_str().map(|s| s.to_string());
            config.version = pkg["version"].as_str().map(|s| s.to_string());

            // Add dependencies
            if let Some(deps) = pkg["dependencies"].as_object() {
                for (name, value) in deps {
                    config.dependencies.push(Dependency {
                        name: name.clone(),
                        version: value.as_str().map(|s| s.to_string()),
                        path: None,
                        dep_type: DependencyType::Production,
                    });
                }
            }
            if let Some(deps) = pkg["devDependencies"].as_object() {
                for (name, value) in deps {
                    config.dependencies.push(Dependency {
                        name: name.clone(),
                        version: value.as_str().map(|s| s.to_string()),
                        path: None,
                        dep_type: DependencyType::Development,
                    });
                }
            }
        }

        // Exclude node_modules
        config.exclude_patterns.push("node_modules/**".to_string());
    }

    /// Enrich config with pnpm package information
    fn enrich_pnpm_config(&self, config: &mut ProjectConfig) {
        let package_json = self.root.join("package.json");
        if let Ok(content) = std::fs::read_to_string(&package_json)
            && let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content)
        {
            if config.name.is_none() {
                config.name = pkg["name"].as_str().map(|s| s.to_string());
            }
            if config.version.is_none() {
                config.version = pkg["version"].as_str().map(|s| s.to_string());
            }

            // Add dependencies if not already present
            if let Some(deps) = pkg["dependencies"].as_object() {
                for (name, value) in deps {
                    // Avoid duplicates
                    if !config.dependencies.iter().any(|d| &d.name == name) {
                        config.dependencies.push(Dependency {
                            name: name.clone(),
                            version: value.as_str().map(|s| s.to_string()),
                            path: None,
                            dep_type: DependencyType::Production,
                        });
                    }
                }
            }
            if let Some(deps) = pkg["devDependencies"].as_object() {
                for (name, value) in deps {
                    if !config.dependencies.iter().any(|d| &d.name == name) {
                        config.dependencies.push(Dependency {
                            name: name.clone(),
                            version: value.as_str().map(|s| s.to_string()),
                            path: None,
                            dep_type: DependencyType::Development,
                        });
                    }
                }
            }
        }

        // Exclude node_modules (same as npm)
        if !config
            .exclude_patterns
            .iter()
            .any(|p| p == "node_modules/**")
        {
            config.exclude_patterns.push("node_modules/**".to_string());
        }
    }

    /// Load compile_commands.json for C/C++ projects
    fn load_compile_commands(&self, config: &mut ProjectConfig) {
        // Check common locations for compile_commands.json
        let locations = [
            self.root.join("compile_commands.json"),
            self.root.join("build/compile_commands.json"),
            self.root.join("cmake-build-debug/compile_commands.json"),
            self.root.join("out/build/compile_commands.json"),
        ];

        for path in locations {
            if path.exists()
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(commands) = serde_json::from_str::<Vec<CompileCommand>>(&content)
            {
                // Extract include paths and defines from all commands
                let mut all_includes = HashSet::new();
                let mut all_defines = HashMap::new();

                let mut normalized_commands = commands;

                for cmd in &mut normalized_commands {
                    cmd.file = cmd.resolved_file_path();
                    if let Some(output) = &cmd.output
                        && !output.is_absolute()
                    {
                        cmd.output = Some(cmd.directory.join(output));
                    }
                }

                for cmd in &normalized_commands {
                    for inc in cmd.resolved_include_paths() {
                        all_includes.insert(inc);
                    }
                    all_defines.extend(cmd.defines());
                }

                config.include_paths.extend(all_includes);
                config.defines.extend(all_defines);
                config.compile_commands = normalized_commands;

                tracing::info!(
                    "Loaded {} compile commands from {:?}",
                    config.compile_commands.len(),
                    path
                );
                break;
            }
        }
    }

    /// Get compile command for a specific file
    pub fn get_compile_command_for_file<'a>(
        config: &'a ProjectConfig,
        file: &Path,
    ) -> Option<&'a CompileCommand> {
        let normalized_file = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
        config.compile_commands.iter().find(|cmd| {
            let normalized_cmd = cmd
                .file
                .canonicalize()
                .unwrap_or_else(|_| cmd.resolved_file_path());
            normalized_cmd == normalized_file
        })
    }
}

/// Extract a quoted string from a TOML line like `name = "value"`
fn extract_toml_string(line: &str) -> String {
    if let Some(start) = line.find('"') {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find('"') {
            return rest[..end].to_string();
        }
    }
    line.split('=')
        .nth(1)
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_default()
}

/// Count distinct language groups in build systems
fn count_language_groups(systems: &[BuildSystem]) -> usize {
    let mut groups = HashSet::new();
    for system in systems {
        let group = match system {
            BuildSystem::Cargo => "rust",
            BuildSystem::CMake | BuildSystem::Make => "cpp",
            BuildSystem::GoModules => "go",
            BuildSystem::Python => "python",
            BuildSystem::Npm | BuildSystem::Yarn | BuildSystem::Pnpm => "js",
            BuildSystem::Mixed => continue,
        };
        groups.insert(group);
    }
    groups.len()
}

/// Simple shell-style argument splitter (minimal implementation)
mod shlex {
    pub fn split(s: &str) -> Option<Vec<String>> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;
        let mut escape_next = false;
        let quote_char = if s.contains('"') { '"' } else { '\'' };

        for ch in s.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
            } else if ch == '\\' {
                escape_next = true;
            } else if in_quote {
                if ch == quote_char {
                    in_quote = false;
                } else {
                    current.push(ch);
                }
            } else if ch == quote_char {
                in_quote = true;
            } else if ch.is_whitespace() {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            result.push(current);
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_toml_string() {
        assert_eq!(extract_toml_string("name = \"myproject\""), "myproject");
        assert_eq!(extract_toml_string("version = \"1.0.0\""), "1.0.0");
    }

    #[test]
    fn test_parse_define() {
        assert_eq!(parse_define("DEBUG"), ("DEBUG".to_string(), None));
        assert_eq!(
            parse_define("VERSION=2"),
            ("VERSION".to_string(), Some("2".to_string()))
        );
        assert_eq!(
            parse_define("PATH=/usr/bin"),
            ("PATH".to_string(), Some("/usr/bin".to_string()))
        );
    }

    #[test]
    fn test_build_system_primary_languages() {
        assert_eq!(BuildSystem::Cargo.primary_languages(), vec!["rust"]);
        assert_eq!(BuildSystem::CMake.primary_languages(), vec!["c", "cpp"]);
        assert_eq!(BuildSystem::GoModules.primary_languages(), vec!["go"]);
    }

    #[test]
    fn test_compile_command_include_paths() {
        let cmd = CompileCommand {
            file: PathBuf::from("test.c"),
            command: Some("gcc -I/usr/include -I./include -DDEBUG test.c".to_string()),
            directory: PathBuf::from("/project"),
            arguments: None,
            output: None,
        };

        let includes = cmd.include_paths();
        assert!(includes.contains(&PathBuf::from("/usr/include")));
        assert!(includes.contains(&PathBuf::from("./include")));
    }

    #[test]
    fn test_compile_command_resolved_paths() {
        let cmd = CompileCommand {
            file: PathBuf::from("src/test.c"),
            command: Some("gcc -I./include -I/usr/include test.c".to_string()),
            directory: PathBuf::from("/project"),
            arguments: None,
            output: Some(PathBuf::from("build/test.o")),
        };

        assert_eq!(
            cmd.resolved_file_path(),
            PathBuf::from("/project/src/test.c")
        );
        assert_eq!(
            cmd.resolved_include_paths(),
            vec![
                PathBuf::from("/project/include"),
                PathBuf::from("/usr/include")
            ]
        );
    }

    #[test]
    fn test_compile_command_defines() {
        let cmd = CompileCommand {
            file: PathBuf::from("test.c"),
            command: Some("gcc -DDEBUG -DVERSION=2 test.c".to_string()),
            directory: PathBuf::from("/project"),
            arguments: None,
            output: None,
        };

        let defines = cmd.defines();
        assert_eq!(defines.get("DEBUG"), Some(&None));
        assert_eq!(defines.get("VERSION"), Some(&Some("2".to_string())));
    }

    #[test]
    fn test_detect_cargo_project() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "test-project"
version = "0.1.0"
"#,
        )
        .unwrap();
        fs::create_dir(root.join("src")).unwrap();

        let detector = BuildDetector::new(root);
        let config = detector.detect();

        assert!(config.build_systems.contains(&BuildSystem::Cargo));
        assert_eq!(config.name, Some("test-project".to_string()));
        assert_eq!(config.version, Some("0.1.0".to_string()));
    }

    #[test]
    fn test_detect_go_project() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("go.mod"),
            "module example.com/myproject\n\ngo 1.21\n",
        )
        .unwrap();

        let detector = BuildDetector::new(root);
        let config = detector.detect();

        assert!(config.build_systems.contains(&BuildSystem::GoModules));
        assert_eq!(config.name, Some("example.com/myproject".to_string()));
    }

    #[test]
    fn test_shlex_split() {
        let result = shlex::split("gcc -I/usr/include -DNAME=value test.c").unwrap();
        assert_eq!(
            result,
            vec!["gcc", "-I/usr/include", "-DNAME=value", "test.c"]
        );

        let result = shlex::split("gcc -I\"path with spaces\" test.c").unwrap();
        assert_eq!(result, vec!["gcc", "-Ipath with spaces", "test.c"]);
    }

    #[test]
    fn test_detect_pnpm_project() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(
            root.join("package.json"),
            r#"{"name": "pnpm-project", "version": "1.0.0"}"#,
        )
        .unwrap();
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'\n").unwrap();

        let detector = BuildDetector::new(root);
        let config = detector.detect();

        assert!(config.build_systems.contains(&BuildSystem::Pnpm));
        assert_eq!(config.name, Some("pnpm-project".to_string()));
    }

    #[test]
    fn test_get_compile_command_for_relative_file() {
        let command = CompileCommand {
            file: PathBuf::from("src/test.c"),
            command: None,
            directory: PathBuf::from("/project"),
            arguments: None,
            output: None,
        };
        let config = ProjectConfig {
            compile_commands: vec![command],
            ..Default::default()
        };

        let found =
            BuildDetector::get_compile_command_for_file(&config, Path::new("/project/src/test.c"));
        assert!(found.is_some());
    }

    #[test]
    fn test_build_system_pnpm_config_files() {
        assert!(BuildSystem::Pnpm.config_files().contains(&"pnpm-lock.yaml"));
        assert!(
            BuildSystem::Pnpm
                .primary_languages()
                .contains(&"javascript")
        );
    }

    #[test]
    fn test_detect_kotlin_gradle_hints() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(root.join("build.gradle.kts"), "plugins {}").unwrap();

        let detector = BuildDetector::new(root);
        let config = detector.detect();

        assert!(config.source_dirs.contains(&root.join("src/main/kotlin")));
        assert!(config.source_dirs.contains(&root.join("src/test/kotlin")));
    }

    #[test]
    fn test_detect_swift_package_hints() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        fs::write(root.join("Package.swift"), "// swift-tools-version:5.9").unwrap();

        let detector = BuildDetector::new(root);
        let config = detector.detect();

        assert!(config.source_dirs.contains(&root.join("Sources")));
        assert!(config.source_dirs.contains(&root.join("Tests")));
    }
}
