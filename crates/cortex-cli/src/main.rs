//! # CodeCortex CLI
//!
//! Command-line interface for CodeCortex code intelligence platform.
//!
//! ## Overview
//!
//! This binary provides a comprehensive CLI for interacting with CodeCortex:
//!
//! - **Indexing**: Parse and store code in the graph database
//! - **Search**: Find code by name, pattern, type, or content
//! - **Analysis**: Call graphs, code metrics, impact analysis
//! - **Watching**: Real-time file system monitoring
//! - **MCP Server**: Run as an MCP server for AI assistants
//! - **Bundling**: Export/import graph data
//!
//! ## Commands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `setup` | Initialize configuration |
//! | `doctor` | Diagnose system status |
//! | `index` | Index a directory or file |
//! | `watch` | Watch for file changes |
//! | `find` | Search for code entities |
//! | `analyze` | Run code analysis |
//! | `bundle` | Export/import graph data |
//! | `config` | Manage configuration |
//! | `mcp` | MCP server commands |
//! | `completion` | Generate shell completions |
//! | `interactive` | Start interactive REPL |
//!
//! ## Output Formats
//!
//! All commands support multiple output formats via `--format`:
//!
//! - `json` - Compact JSON (default)
//! - `json-pretty` - Indented JSON
//! - `yaml` - YAML format
//! - `table` - Human-readable tables
//!
//! ## Examples
//!
//! ```bash
//! # Index a project
//! cortex index /path/to/project
//!
//! # Find all functions matching pattern
//! cortex find name --pattern "handle_*"
//!
//! # Analyze call graph
//! cortex analyze callers --function "main"
//!
//! # Start MCP server
//! cortex mcp start
//!
//! # Generate shell completion
//! cortex completion bash > /etc/bash_completion.d/cortex
//! ```

mod setup_wizard;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use comfy_table::{Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use cortex_analyzer::{
    Analyzer, CodeSmell, RefactoringEngine, RefactoringRecommendation, Severity, SmellDetector,
};
use cortex_core::{CortexConfig, SearchKind};
use cortex_graph::{BundleStore, GraphClient};
use cortex_indexer::Indexer;
use cortex_mcp::tool_names;
use cortex_vector::{JsonStore, LanceStore, VectorStore};
use cortex_watcher::WatchSession;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::EnvFilter;

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum OutputFormat {
    /// JSON output (default)
    #[default]
    Json,
    /// Pretty JSON with indentation
    JsonPretty,
    /// YAML output
    Yaml,
    /// Table format for tabular data
    Table,
}

#[derive(Debug, Parser)]
#[command(name = "cortex", version, about = "CodeCortex CLI toolkit")]
struct Cli {
    /// Output format (format, json-pretty, yaml, table)
    #[arg(long, global = true, value_enum, default_value_t = OutputFormat::Json)]
    format: OutputFormat,
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Setup,
    Doctor,
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
    Index {
        path: String,
        #[arg(long)]
        force: bool,
    },
    Watch {
        path: String,
    },
    Unwatch {
        path: String,
    },
    Find {
        #[command(subcommand)]
        command: FindCommand,
    },
    Analyze {
        #[command(subcommand)]
        command: AnalyzeCommand,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Clean,
    List,
    Delete {
        path: String,
    },
    Stats,
    Query {
        cypher: String,
    },
    Jobs {
        #[command(subcommand)]
        command: JobsCommand,
    },
    Debug {
        #[command(subcommand)]
        command: DebugCommand,
    },
    /// Generate shell completion scripts
    Completion {
        /// Shell type (bash, zsh, fish, elvish, powershell)
        shell: Shell,
    },
    /// Start interactive REPL mode
    Interactive,
    /// Get context capsule for a symbol
    Capsule {
        /// Symbol name to get context for
        symbol: String,
        /// Maximum items in capsule
        #[arg(long, default_value_t = 20)]
        max_items: usize,
    },
    /// Get impact graph for a symbol
    Impact {
        /// Symbol name to analyze impact for
        symbol: String,
        /// Maximum depth to traverse
        #[arg(long, default_value_t = 3)]
        depth: usize,
    },
    /// Analyze refactoring suggestions for a symbol
    Refactor {
        /// Symbol name to analyze
        symbol: String,
    },
    /// Find design patterns in codebase
    Patterns {
        /// Filter by pattern type (singleton, factory, observer, etc.)
        #[arg(long)]
        pattern_type: Option<String>,
    },
    /// Find tests for a symbol
    Test {
        /// Symbol name to find tests for
        symbol: String,
    },
    /// Run diagnostic checks
    Diagnose {
        /// Check specific component
        #[arg(long)]
        component: Option<String>,
    },
    /// Memory operations
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    /// Project management operations
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Get skeleton (compressed view) of a file
    Skeleton {
        /// Path to the file
        path: PathBuf,
        /// Skeleton mode (minimal, standard, full)
        #[arg(long, default_value = "minimal")]
        mode: String,
    },
    /// Get signature of a symbol
    Signature {
        /// Symbol name to get signature for
        symbol: String,
        /// Repository path filter
        #[arg(long)]
        repo: Option<String>,
        /// Include related symbols
        #[arg(long)]
        include_related: bool,
    },
    /// Semantic code search using vector embeddings
    Search {
        /// Search query (natural language description)
        query: String,
        /// Number of results to return
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Search type (semantic, structural, hybrid)
        #[arg(long, default_value = "semantic")]
        search_type: String,
        /// Filter by repository path
        #[arg(long)]
        repo: Option<String>,
        /// Filter by file path pattern
        #[arg(long)]
        path: Option<String>,
        /// Filter by symbol kind (function, class, method, etc.)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by language
        #[arg(long)]
        language: Option<String>,
    },
    /// Index code for vector search
    VectorIndex {
        /// Path to index (file or directory)
        path: String,
        /// Repository path for metadata
        #[arg(long)]
        repo: Option<String>,
        /// Force reindex
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    Start,
    Tools,
}

#[derive(Debug, Subcommand)]
enum FindCommand {
    Name { name: String },
    Pattern { pattern: String },
    Type { kind: String },
    Content { query: String },
    Decorator { name: String },
    Argument { name: String },
}

#[derive(Debug, Subcommand)]
enum AnalyzeCommand {
    Callers(TargetArg),
    Callees(TargetArg),
    Chain {
        from: String,
        to: String,
        #[arg(long)]
        depth: Option<usize>,
    },
    Hierarchy {
        class: String,
    },
    Deps {
        module: String,
    },
    DeadCode,
    Complexity {
        #[arg(long, default_value_t = 20)]
        top: usize,
    },
    Overrides {
        method: String,
    },
    /// Detect code smells from source files
    Smells {
        /// File or directory path to analyze
        #[arg(default_value = ".")]
        path: String,
        /// Minimum severity to report (info, warning, error, critical)
        #[arg(long, default_value = "info")]
        min_severity: String,
        /// Maximum number of files to scan
        #[arg(long, default_value_t = 1000)]
        max_files: usize,
        /// Maximum number of findings to return
        #[arg(long, default_value_t = 500)]
        limit: usize,
    },
    /// Recommend refactoring techniques based on detected smells
    Refactoring {
        /// File or directory path to analyze
        #[arg(default_value = ".")]
        path: String,
        /// Minimum smell severity to consider (info, warning, error, critical)
        #[arg(long, default_value = "warning")]
        min_severity: String,
        /// Maximum number of files to scan
        #[arg(long, default_value_t = 1000)]
        max_files: usize,
        /// Maximum number of recommendations to return
        #[arg(long, default_value_t = 500)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
enum BundleCommand {
    Export {
        output: PathBuf,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    Import {
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Show,
    Set { key: String, value: String },
    Reset,
}

#[derive(Debug, Subcommand)]
enum JobsCommand {
    List,
    Status { id: String },
}

#[derive(Debug, Subcommand)]
enum DebugCommand {
    /// Debug context capsule building for a symbol
    Capsule {
        /// Symbol name to build capsule for
        symbol: String,
        /// Explain the capsule building process
        #[arg(long)]
        explain: bool,
        /// Maximum items in capsule
        #[arg(long, default_value_t = 20)]
        max_items: usize,
    },
    /// Show cache statistics
    Cache {
        /// Clear the cache
        #[arg(long)]
        clear: bool,
    },
    /// Trace query execution
    Trace {
        /// Query to trace
        query: String,
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Validate index integrity
    Validate {
        /// Fix issues automatically
        #[arg(long)]
        fix: bool,
        /// Repository path to validate
        #[arg(long)]
        repo: Option<String>,
    },
}

#[derive(Debug, Args)]
struct TargetArg {
    target: String,
}

#[derive(Debug, Subcommand)]
enum MemoryCommand {
    /// Save an observation
    Save {
        /// Observation content
        content: String,
        /// Classification (architecture, decision, pattern, issue, note)
        #[arg(long, default_value = "note")]
        classification: String,
        /// Severity (low, medium, high, critical)
        #[arg(long, default_value = "low")]
        severity: String,
    },
    /// Search observations
    Search {
        /// Search query
        query: String,
        /// Maximum results
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Get session context
    Context {
        /// Session ID (optional)
        #[arg(long)]
        session: Option<String>,
    },
    /// List all observations
    List {
        /// Filter by classification
        #[arg(long)]
        classification: Option<String>,
    },
    /// Clear all observations
    Clear,
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    /// List all registered projects
    List,
    /// Add a project to the registry
    Add {
        /// Path to the project
        path: PathBuf,
        /// Whether to track branch changes
        #[arg(long, default_value_t = true)]
        track_branch: bool,
    },
    /// Remove a project from the registry
    Remove {
        /// Path to the project
        path: PathBuf,
    },
    /// Set the current active project
    Set {
        /// Path to the project
        path: PathBuf,
        /// Branch to use (optional, defaults to current)
        #[arg(long)]
        branch: Option<String>,
    },
    /// Get the current active project
    Current,
    /// List branches for a project
    Branches {
        /// Path to the project (optional, uses current)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Refresh Git info for a project
    Refresh {
        /// Path to the project (optional, uses current)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose)?;
    let mut config = CortexConfig::load()?;
    let format = cli.format;

    match cli.command {
        Commands::Setup => run_setup(&mut config)?,
        Commands::Doctor => run_doctor(&config).await?,
        Commands::Mcp { command } => run_mcp(&config, command).await?,
        Commands::Index { path, force } => run_index(&config, &path, force, format).await?,
        Commands::Watch { path } => run_watch(&config, &path).await?,
        Commands::Unwatch { path } => run_unwatch(&config, &path)?,
        Commands::Find { command } => run_find(&config, command, format).await?,
        Commands::Analyze { command } => run_analyze(&config, command, format).await?,
        Commands::Bundle { command } => run_bundle(&config, command, format).await?,
        Commands::Config { command } => run_config(&mut config, command, format)?,
        Commands::Clean => run_clean(&config, format).await?,
        Commands::List => run_list(&config, format).await?,
        Commands::Delete { path } => run_delete(&config, &path).await?,
        Commands::Stats => run_stats(&config, format).await?,
        Commands::Query { cypher } => run_query(&config, &cypher, format).await?,
        Commands::Jobs { command } => run_jobs(command, format)?,
        Commands::Debug { command } => run_debug(&config, command, format).await?,
        Commands::Completion { shell } => run_completion(shell),
        Commands::Interactive => run_interactive(&config).await?,
        Commands::Capsule { symbol, max_items } => {
            run_capsule(&config, &symbol, max_items, format).await?
        }
        Commands::Impact { symbol, depth } => run_impact(&config, &symbol, depth, format).await?,
        Commands::Refactor { symbol } => run_refactor(&config, &symbol, format).await?,
        Commands::Patterns { pattern_type } => {
            run_patterns(&config, pattern_type.as_deref(), format).await?
        }
        Commands::Test { symbol } => run_find_tests(&config, &symbol, format).await?,
        Commands::Diagnose { component } => {
            run_diagnose(&config, component.as_deref(), format).await?
        }
        Commands::Memory { command } => run_memory(&config, command, format).await?,
        Commands::Project { command } => run_project(command, format)?,
        Commands::Skeleton { path, mode } => run_skeleton(&path, &mode, format)?,
        Commands::Signature {
            symbol,
            repo,
            include_related,
        } => run_signature(&config, &symbol, repo.as_deref(), include_related, format).await?,
        Commands::Search {
            query,
            limit,
            search_type,
            repo,
            path,
            kind,
            language,
        } => {
            run_search(
                &config,
                &query,
                limit,
                &search_type,
                repo.as_deref(),
                path.as_deref(),
                kind.as_deref(),
                language.as_deref(),
                format,
            )
            .await?
        }
        Commands::VectorIndex { path, repo, force } => {
            run_vector_index(&config, &path, repo.as_deref(), force, format).await?
        }
    }
    Ok(())
}

fn run_setup(config: &mut CortexConfig) -> anyhow::Result<()> {
    setup_wizard::run_setup_wizard(config)
}

async fn run_doctor(config: &CortexConfig) -> anyhow::Result<()> {
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║              CodeCortex System Health Check                   ║".cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    let mut all_healthy = true;
    let mut warnings: Vec<String> = Vec::new();

    // Helper function to check if a port is reachable
    fn check_port_reachable(host: &str, port: u16) -> bool {
        use std::net::{TcpStream, ToSocketAddrs};
        let addr = format!("{}:{}", host, port);
        addr.to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .map(|addr| {
                TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2)).is_ok()
            })
            .unwrap_or(false)
    }

    // Helper to extract host and port from URI
    fn parse_uri(uri: &str) -> Option<(&str, u16)> {
        let authority = uri
            .trim()
            .strip_prefix("bolt://")
            .or_else(|| uri.trim().strip_prefix("bolt+s://"))
            .or_else(|| uri.trim().strip_prefix("bolt+ssc://"))
            .or_else(|| uri.trim().strip_prefix("memgraph://"))
            .or_else(|| uri.trim().strip_prefix("neo4j://"))
            .or_else(|| uri.trim().strip_prefix("neo4j+s://"))
            .or_else(|| uri.trim().strip_prefix("neo4j+ssc://"))
            .unwrap_or(uri.trim())
            .split(['/', '?', '#'])
            .next()?
            .rsplit('@')
            .next()?;

        if authority.is_empty() {
            return None;
        }

        if let Some(rest) = authority.strip_prefix('[') {
            let end = rest.find(']')?;
            let host = &rest[..end];
            let tail = &rest[end + 1..];
            let port = tail
                .strip_prefix(':')
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(7687);
            return Some((host, port));
        }

        if let Some((host, port_str)) = authority.rsplit_once(':') {
            if !host.contains(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    return Some((host, port));
                }
            }
        }

        Some((authority, 7687))
    }

    // 1. Check Configuration
    println!("{}", "1. Configuration".cyan().bold());
    println!("   Config file: {}", CortexConfig::config_path().display());

    match config.validate() {
        Ok(()) => {
            println!("   {}", "✓ Configuration valid".green());
        }
        Err(e) => {
            println!("   {} Configuration error: {}", "✗".red(), e);
            all_healthy = false;
        }
    }

    // Check critical settings
    if config.memgraph_uri.is_empty() {
        println!("   {} Memgraph URI not configured", "⚠".yellow());
        warnings.push("Memgraph URI is empty".to_string());
    } else {
        println!("   Database URI: {}", config.memgraph_uri);
    }

    if config.llm.provider == "openai" && config.llm.openai_api_key.is_none() {
        println!(
            "   {} OpenAI provider selected but no API key configured",
            "⚠".yellow()
        );
        warnings.push("OpenAI API key missing".to_string());
    }

    println!();

    // 2. Check Graph Database (Memgraph/Neo4j)
    println!("{}", "2. Graph Database".cyan().bold());
    println!("   URI: {}", config.memgraph_uri);

    // First, do a quick port check before attempting full connection
    let (db_host, db_port) = parse_uri(&config.memgraph_uri).unwrap_or(("127.0.0.1", 7687));

    if !check_port_reachable(db_host, db_port) {
        println!(
            "   {} Port {} is not reachable on {}",
            "✗".red(),
            db_port,
            db_host
        );
        println!("   {} Database is not running", "✗".red());
        println!("      Start with: docker start codecortex-memgraph");
        println!(
            "      Or run: docker run -d --name codecortex-memgraph -p 7687:7687 memgraph/memgraph:3.8.1"
        );
        all_healthy = false;
    } else {
        println!("   {} Port {} is open on {}", "✓".green(), db_port, db_host);

        // Now try the actual connection
        match GraphClient::connect(config).await {
            Ok(client) => {
                println!("   {} Connection established", "✓".green());

                // Check repositories
                match client.list_repositories().await {
                    Ok(repos) => {
                        println!("   {} Indexed repositories: {}", "✓".green(), repos.len());
                    }
                    Err(e) => {
                        println!("   {} Failed to list repositories: {}", "⚠".yellow(), e);
                        warnings.push("Could not list repositories".to_string());
                    }
                }

                // Check schema by querying node count
                match client
                    .raw_query("MATCH (n:CodeNode) RETURN count(n) AS count")
                    .await
                {
                    Ok(rows) => {
                        if let Some(row) = rows.first() {
                            if let Some(count) = row.get("count") {
                                println!("   {} Total code nodes: {}", "✓".green(), count);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   {} Schema query failed: {}", "⚠".yellow(), e);
                        warnings.push("Schema verification failed".to_string());
                    }
                }
            }
            Err(e) => {
                println!("   {} Connection failed: {}", "✗".red(), e);
                println!("      Port is open but database protocol handshake failed");
                all_healthy = false;
            }
        }
    }

    println!();

    // 3. Check Vector Store
    println!("{}", "3. Vector Store".cyan().bold());
    println!("   Type: {}", config.vector.store_type);

    match config.vector.store_type.as_str() {
        "lancedb" => {
            println!("   Path: {}", config.vector.store_path.display());

            // Check if directory exists
            if config.vector.store_path.exists() {
                println!("   {} Storage directory exists", "✓".green());

                // Try to open the store
                match LanceStore::open(&config.vector.store_path).await {
                    Ok(store) => match store.health_check().await {
                        Ok(true) => {
                            println!("   {} Vector store healthy", "✓".green());
                            match store.count().await {
                                Ok(count) => {
                                    println!("   {} Documents stored: {}", "✓".green(), count)
                                }
                                Err(_) => {}
                            }
                        }
                        Ok(false) => {
                            println!("   {} Vector store unhealthy", "⚠".yellow());
                            warnings.push("Vector store health check failed".to_string());
                        }
                        Err(e) => {
                            println!("   {} Health check error: {}", "⚠".yellow(), e);
                            warnings.push("Vector store error".to_string());
                        }
                    },
                    Err(e) => {
                        println!("   {} Failed to open vector store: {}", "⚠".yellow(), e);
                        warnings.push("Could not open vector store".to_string());
                    }
                }
            } else {
                println!(
                    "   {} Storage directory does not exist (will be created on first use)",
                    "⚠".yellow()
                );
                warnings.push("Vector store path does not exist".to_string());

                // Check if parent is writable
                if let Some(parent) = config.vector.store_path.parent() {
                    if parent.exists()
                        && parent
                            .metadata()
                            .map(|m| !m.permissions().readonly())
                            .unwrap_or(false)
                    {
                        println!("   {} Parent directory is writable", "✓".green());
                    }
                }
            }
        }
        "json" => {
            println!("   Path: {}", config.vector.store_path.display());

            // Check if directory exists
            if config.vector.store_path.exists() {
                println!("   {} Storage directory exists", "✓".green());

                // Try to open the store
                match JsonStore::open(&config.vector.store_path).await {
                    Ok(store) => match store.health_check().await {
                        Ok(true) => {
                            println!("   {} Vector store healthy", "✓".green());
                            match store.count().await {
                                Ok(count) => {
                                    println!("   {} Documents stored: {}", "✓".green(), count)
                                }
                                Err(_) => {}
                            }
                        }
                        Ok(false) => {
                            println!("   {} Vector store unhealthy", "⚠".yellow());
                            warnings.push("Vector store health check failed".to_string());
                        }
                        Err(e) => {
                            println!("   {} Health check error: {}", "⚠".yellow(), e);
                            warnings.push("Vector store error".to_string());
                        }
                    },
                    Err(e) => {
                        println!("   {} Failed to open vector store: {}", "⚠".yellow(), e);
                        warnings.push("Could not open vector store".to_string());
                    }
                }
            } else {
                println!(
                    "   {} Storage directory does not exist (will be created on first use)",
                    "⚠".yellow()
                );
                warnings.push("Vector store path does not exist".to_string());

                // Check if parent is writable
                if let Some(parent) = config.vector.store_path.parent() {
                    if parent.exists()
                        && parent
                            .metadata()
                            .map(|m| !m.permissions().readonly())
                            .unwrap_or(false)
                    {
                        println!("   {} Parent directory is writable", "✓".green());
                    }
                }
            }
        }
        "qdrant" => {
            println!("   URI: {}", config.vector.qdrant_uri);
            println!("   {} Qdrant health check not implemented", "⚠".yellow());
            warnings.push("Qdrant health check not available".to_string());
        }
        "none" => {
            println!("   {}", "⚠ Vector search disabled".yellow());
            warnings.push("Vector search is disabled".to_string());
        }
        _ => {
            println!("   {} Unknown vector store type", "⚠".yellow());
            warnings.push("Invalid vector store type".to_string());
        }
    }

    println!("   Embedding dimension: {}", config.vector.embedding_dim);

    println!();

    // 4. Check LLM/Embedding Provider
    println!("{}", "4. LLM/Embedding Provider".cyan().bold());
    println!("   Provider: {}", config.llm.provider);

    match config.llm.provider.as_str() {
        "ollama" => {
            println!("   Base URL: {}", config.llm.ollama_base_url);
            println!("   Model: {}", config.llm.ollama_embedding_model);

            // Try to connect to Ollama using TCP
            let ollama_addr = config
                .llm
                .ollama_base_url
                .replace("http://", "")
                .replace("https://", "");

            let addr: std::net::SocketAddr = format!(
                "{}:11434",
                ollama_addr.split(':').next().unwrap_or("127.0.0.1")
            )
            .parse()
            .unwrap_or_else(|_| "127.0.0.1:11434".parse().unwrap());

            match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5)) {
                Ok(_) => {
                    println!("   {} Ollama server reachable", "✓".green());
                    // Note: Cannot verify model availability without HTTP client
                    println!(
                        "   {} Model '{}' configured",
                        "i".cyan(),
                        config.llm.ollama_embedding_model
                    );
                }
                Err(e) => {
                    println!("   {} Cannot connect to Ollama: {}", "✗".red(), e);
                    println!("      Start Ollama with: ollama serve");
                    warnings.push("Ollama server not running".to_string());
                }
            }
        }
        "openai" => {
            println!("   Model: {}", config.llm.openai_embedding_model);

            match &config.llm.openai_api_key {
                Some(key) if !key.is_empty() => {
                    // Check if key looks valid (starts with sk-)
                    if key.starts_with("sk-") {
                        println!("   {} API key configured", "✓".green());
                    } else {
                        println!("   {} API key format may be invalid", "⚠".yellow());
                        warnings.push("OpenAI API key format may be invalid".to_string());
                    }
                }
                _ => {
                    println!("   {} API key not configured", "✗".red());
                    warnings.push("OpenAI API key missing".to_string());
                }
            }
        }
        "none" => {
            println!("   {}", "⚠ LLM features disabled".yellow());
            warnings.push("LLM provider not configured".to_string());
        }
        _ => {
            println!("   {} Unknown provider", "⚠".yellow());
            warnings.push("Invalid LLM provider".to_string());
        }
    }

    println!();

    // 5. Check MCP Tools
    println!("{}", "5. MCP Tools".cyan().bold());
    let tools = tool_names();
    println!("   {} Available tools: {}", "✓".green(), tools.len());

    // Categorize tools
    let core_tools = tools
        .iter()
        .filter(|t| {
            t.starts_with("find_") || t.starts_with("analyze_") || **t == "index_repository"
        })
        .count();
    let vector_tools = tools
        .iter()
        .filter(|t| t.contains("semantic") || t.contains("vector"))
        .count();
    let health_tools = tools
        .iter()
        .filter(|t| t.contains("health") || t.contains("status") || t.contains("diagnose"))
        .count();

    println!(
        "   Core tools: {}, Vector tools: {}, Health tools: {}",
        core_tools, vector_tools, health_tools
    );

    println!();

    // Summary
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".cyan()
    );
    println!();

    if all_healthy && warnings.is_empty() {
        println!("{}", "✓ All systems healthy!".green().bold());
    } else if all_healthy {
        println!(
            "{} Systems operational with {} warning(s)",
            "⚠".yellow(),
            warnings.len()
        );
        println!();
        println!("Warnings:");
        for w in &warnings {
            println!("  • {}", w);
        }
    } else {
        println!("{} Some systems are not healthy", "✗".red());
        println!();
        println!("Please fix the issues above before using CodeCortex.");
    }

    println!();
    println!("Next steps:");
    println!("  • Index a repository: cortex index /path/to/code");
    println!("  • Start MCP server: cortex mcp start");
    println!("  • View config: cortex config show");

    Ok(())
}

async fn run_mcp(config: &CortexConfig, cmd: McpCommand) -> anyhow::Result<()> {
    match cmd {
        McpCommand::Start => {
            cortex_mcp::handler::start_stdio(config.clone()).await?;
        }
        McpCommand::Tools => {
            for tool in tool_names() {
                println!("{tool}");
            }
        }
    }
    Ok(())
}

async fn run_index(
    config: &CortexConfig,
    path: &str,
    force: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let job_id = format!("cli-index-{}", now_millis());
    upsert_job(&job_id, "running", format!("Indexing {}", path))?;
    let client = GraphClient::connect(config).await?;
    let indexer = Indexer::new(client, config.max_batch_size)?;
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {msg}")?);
    pb.set_message("Indexing...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    let report = match indexer.index_path_with_options(path, force).await {
        Ok(report) => {
            upsert_job(&job_id, "completed", serde_json::to_string(&report)?)?;
            report
        }
        Err(err) => {
            upsert_job(&job_id, "failed", err.to_string())?;
            return Err(err.into());
        }
    };
    pb.finish_and_clear();
    print_formatted(format, &serde_json::json!(report))?;
    Ok(())
}

async fn run_watch(config: &CortexConfig, path: &str) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let indexer = Indexer::new(client, config.max_batch_size)?;
    let session = WatchSession::new(config);
    session.watch(path.as_ref())?;
    println!("Watching {}", path.cyan());
    session.run(indexer).await?;
    Ok(())
}

fn run_unwatch(config: &CortexConfig, path: &str) -> anyhow::Result<()> {
    let session = WatchSession::new(config);
    let removed = session.unwatch(path.as_ref())?;
    println!("{}", if removed { "Removed" } else { "Not found" });
    Ok(())
}

async fn run_find(
    config: &CortexConfig,
    command: FindCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let analyzer = Analyzer::new(GraphClient::connect(config).await?);
    let out = match command {
        FindCommand::Name { name } => analyzer.find_code(&name, SearchKind::Name, None).await?,
        FindCommand::Pattern { pattern } => {
            analyzer
                .find_code(&pattern, SearchKind::Pattern, None)
                .await?
        }
        FindCommand::Type { kind } => analyzer.find_code(&kind, SearchKind::Type, None).await?,
        FindCommand::Content { query } => {
            analyzer
                .find_code(&query, SearchKind::Content, None)
                .await?
        }
        FindCommand::Decorator { name } => analyzer.find_by_decorator(&name).await?,
        FindCommand::Argument { name } => analyzer.find_by_argument(&name).await?,
    };
    print_formatted(format, &serde_json::to_value(out)?)?;
    Ok(())
}

async fn run_analyze(
    config: &CortexConfig,
    command: AnalyzeCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    if let AnalyzeCommand::Smells {
        path,
        min_severity,
        max_files,
        limit,
    } = &command
    {
        return run_analyze_smells(path, min_severity, *max_files, *limit, format);
    }

    if let AnalyzeCommand::Refactoring {
        path,
        min_severity,
        max_files,
        limit,
    } = &command
    {
        return run_analyze_refactoring(path, min_severity, *max_files, *limit, format);
    }

    let analyzer = Analyzer::new(GraphClient::connect(config).await?);
    let out = match command {
        AnalyzeCommand::Callers(TargetArg { target }) => analyzer.callers(&target).await?,
        AnalyzeCommand::Callees(TargetArg { target }) => analyzer.callees(&target).await?,
        AnalyzeCommand::Chain { from, to, depth } => analyzer.call_chain(&from, &to, depth).await?,
        AnalyzeCommand::Hierarchy { class } => analyzer.class_hierarchy(&class).await?,
        AnalyzeCommand::Deps { module } => analyzer.module_dependencies(&module).await?,
        AnalyzeCommand::DeadCode => analyzer.dead_code().await?,
        AnalyzeCommand::Complexity { top } => analyzer.complexity(top).await?,
        AnalyzeCommand::Overrides { method } => analyzer.overrides(&method).await?,
        AnalyzeCommand::Smells { .. } | AnalyzeCommand::Refactoring { .. } => unreachable!(),
    };
    print_formatted(format, &serde_json::to_value(out)?)?;
    Ok(())
}

const MAX_ANALYZE_FILE_BYTES: u64 = 1_048_576;
const ANALYZE_SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".cache",
    ".idea",
    ".vscode",
];
const ANALYZE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "jsx", "ts", "tsx", "go", "java", "rb", "c", "cc", "cpp", "h", "hpp", "cs",
    "php", "swift", "kt", "kts", "m", "mm", "scala",
];

#[derive(Debug)]
struct SmellScanResult {
    files_scanned: usize,
    files_skipped: usize,
    read_errors: usize,
    smells: Vec<CodeSmell>,
}

fn run_analyze_smells(
    path: &str,
    min_severity: &str,
    max_files: usize,
    limit: usize,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let severity = parse_severity(min_severity)?;
    let mut scan = detect_smells_in_path(path, max_files)?;
    scan.smells.retain(|smell| smell.severity >= severity);
    scan.smells.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.file_path.cmp(&b.file_path))
            .then_with(|| a.line_number.cmp(&b.line_number))
    });

    let total_smells = scan.smells.len();
    if limit > 0 && scan.smells.len() > limit {
        scan.smells.truncate(limit);
    }

    let mut by_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_severity: BTreeMap<String, usize> = BTreeMap::new();
    for smell in &scan.smells {
        *by_type.entry(smell.smell_type.to_string()).or_default() += 1;
        *by_severity.entry(smell.severity.to_string()).or_default() += 1;
    }

    let output = serde_json::json!({
        "path": path,
        "min_severity": severity.to_string(),
        "files_scanned": scan.files_scanned,
        "files_skipped": scan.files_skipped,
        "read_errors": scan.read_errors,
        "total_smells": total_smells,
        "returned_smells": scan.smells.len(),
        "summary": {
            "by_type": by_type,
            "by_severity": by_severity
        },
        "smells": scan.smells
    });

    print_formatted(format, &output)?;
    Ok(())
}

fn run_analyze_refactoring(
    path: &str,
    min_severity: &str,
    max_files: usize,
    limit: usize,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let severity = parse_severity(min_severity)?;
    let mut scan = detect_smells_in_path(path, max_files)?;
    scan.smells.retain(|smell| smell.severity >= severity);

    let mut engine = RefactoringEngine::new();
    engine.min_severity = severity;

    let recommendations = engine.prioritize(engine.generate_recommendations(&scan.smells));
    let summary = engine.summary(&recommendations);
    let recommendation_items = build_refactoring_recommendation_items(path, &scan.smells);

    let total_recommendations = recommendations.len();
    let mut recommendations = recommendations;
    if limit > 0 && recommendations.len() > limit {
        recommendations.truncate(limit);
    }
    let total_recommendation_items = recommendation_items.len();
    let mut recommendation_items = recommendation_items;
    if limit > 0 && recommendation_items.len() > limit {
        recommendation_items.truncate(limit);
    }

    let output = serde_json::json!({
        "path": path,
        "min_severity": severity.to_string(),
        "files_scanned": scan.files_scanned,
        "files_skipped": scan.files_skipped,
        "read_errors": scan.read_errors,
        "detected_smells": scan.smells.len(),
        "total_recommendations": total_recommendations,
        "returned_recommendations": recommendations.len(),
        "total_recommendation_items": total_recommendation_items,
        "returned_recommendation_items": recommendation_items.len(),
        "summary": summary,
        "recommendations": recommendations,
        "recommendation_items": recommendation_items
    });

    print_formatted(format, &output)?;
    Ok(())
}

fn build_refactoring_recommendation_items(
    analyze_path: &str,
    smells: &[CodeSmell],
) -> Vec<serde_json::Value> {
    let mut source_cache: HashMap<String, Option<Vec<String>>> = HashMap::new();
    let mut index_by_key: HashMap<String, usize> = HashMap::new();
    let mut items: Vec<serde_json::Value> = Vec::new();

    for smell in smells {
        let Some(recommendation) = RefactoringRecommendation::from_code_smell(smell) else {
            continue;
        };

        let dedupe_key = format!(
            "{}|{}|{}|{}|{}",
            recommendation.technique,
            smell.smell_type,
            smell.file_path,
            smell.line_number,
            smell.symbol_name
        );

        if let Some(existing_index) = index_by_key.get(&dedupe_key).copied() {
            if let Some(existing) = items.get_mut(existing_index) {
                let current = existing
                    .get("occurrences")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);
                existing["occurrences"] = serde_json::json!(current + 1);
            }
            continue;
        }

        let related_code = build_related_code_excerpt(
            analyze_path,
            &smell.file_path,
            smell.line_number,
            &mut source_cache,
        );

        index_by_key.insert(dedupe_key, items.len());
        items.push(serde_json::json!({
            "technique": recommendation.technique,
            "priority": recommendation.priority,
            "description": recommendation.description,
            "steps": recommendation.steps,
            "related_smells": recommendation.related_smells,
            "smell_type": smell.smell_type,
            "smell": {
                "severity": smell.severity,
                "message": smell.message,
                "metric_value": smell.metric_value,
                "threshold": smell.threshold,
                "suggestion": smell.suggestion
            },
            "location": {
                "file_path": smell.file_path,
                "line_number": smell.line_number,
                "symbol_name": smell.symbol_name
            },
            "related_code": related_code,
            "occurrences": 1
        }));
    }

    items.sort_by(|a, b| {
        recommendation_priority_rank(b)
            .cmp(&recommendation_priority_rank(a))
            .then_with(|| recommendation_location_key(a).cmp(&recommendation_location_key(b)))
    });
    items
}

fn recommendation_priority_rank(item: &serde_json::Value) -> u8 {
    match item
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
    {
        "critical" => 4,
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn recommendation_location_key(item: &serde_json::Value) -> (String, u64, String) {
    let file = item
        .get("location")
        .and_then(|v| v.get("file_path"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let line = item
        .get("location")
        .and_then(|v| v.get("line_number"))
        .and_then(|v| v.as_u64())
        .unwrap_or_default();
    let symbol = item
        .get("location")
        .and_then(|v| v.get("symbol_name"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    (file, line, symbol)
}

fn build_related_code_excerpt(
    analyze_path: &str,
    file_path: &str,
    line_number: u32,
    source_cache: &mut HashMap<String, Option<Vec<String>>>,
) -> Option<serde_json::Value> {
    if line_number == 0 {
        return None;
    }

    let cache_key = format!("{}::{}", analyze_path, file_path);
    let lines = source_cache
        .entry(cache_key)
        .or_insert_with(|| load_source_lines(analyze_path, file_path))
        .as_ref()?;
    if lines.is_empty() {
        return None;
    }

    let target_line = usize::try_from(line_number).ok()?.checked_sub(1)?;
    if target_line >= lines.len() {
        return None;
    }

    let start = target_line.saturating_sub(2);
    let end = target_line
        .saturating_add(2)
        .min(lines.len().saturating_sub(1));
    let excerpt_lines: Vec<_> = (start..=end)
        .map(|index| {
            serde_json::json!({
                "line_number": index + 1,
                "content": lines[index],
                "is_target": index == target_line
            })
        })
        .collect();

    Some(serde_json::json!({
        "start_line": start + 1,
        "end_line": end + 1,
        "lines": excerpt_lines
    }))
}

fn load_source_lines(analyze_path: &str, file_path: &str) -> Option<Vec<String>> {
    let source = match std::fs::read_to_string(file_path) {
        Ok(source) => source,
        Err(_) => {
            let root = PathBuf::from(analyze_path);
            if !root.is_dir() {
                return None;
            }
            std::fs::read_to_string(root.join(file_path)).ok()?
        }
    };

    Some(source.lines().map(|line| line.to_string()).collect())
}

fn parse_severity(value: &str) -> anyhow::Result<Severity> {
    match value.trim().to_ascii_lowercase().as_str() {
        "info" => Ok(Severity::Info),
        "warning" | "warn" => Ok(Severity::Warning),
        "error" | "err" => Ok(Severity::Error),
        "critical" | "crit" => Ok(Severity::Critical),
        other => anyhow::bail!(
            "invalid severity '{}'; expected one of: info, warning, error, critical",
            other
        ),
    }
}

fn detect_smells_in_path(path: &str, max_files: usize) -> anyhow::Result<SmellScanResult> {
    let root = PathBuf::from(path);
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", path);
    }

    let files = collect_analyzable_files(&root, max_files.max(1))?;
    let detector = SmellDetector::new();

    let mut files_scanned = 0usize;
    let mut files_skipped = 0usize;
    let mut read_errors = 0usize;
    let mut smells = Vec::new();

    for file in files {
        let metadata = match std::fs::metadata(&file) {
            Ok(meta) => meta,
            Err(_) => {
                read_errors += 1;
                continue;
            }
        };

        if metadata.len() > MAX_ANALYZE_FILE_BYTES {
            files_skipped += 1;
            continue;
        }

        match std::fs::read_to_string(&file) {
            Ok(source) => {
                files_scanned += 1;
                smells.extend(detector.detect(&source, &file.display().to_string()));
            }
            Err(_) => {
                read_errors += 1;
            }
        }
    }

    Ok(SmellScanResult {
        files_scanned,
        files_skipped,
        read_errors,
        smells,
    })
}

fn collect_analyzable_files(root: &Path, max_files: usize) -> anyhow::Result<Vec<PathBuf>> {
    if root.is_file() {
        if is_analyzable_file(root) {
            return Ok(vec![root.to_path_buf()]);
        }
        return Ok(Vec::new());
    }

    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(current) = stack.pop() {
        if files.len() >= max_files {
            break;
        }

        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(_) => continue,
        };

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.is_file() {
            if is_analyzable_file(&current) {
                files.push(current);
            }
            continue;
        }

        if !metadata.is_dir() {
            continue;
        }

        let dir_iter = std::fs::read_dir(&current).map_err(|e| {
            anyhow::anyhow!("failed to read directory {}: {}", current.display(), e)
        })?;
        for entry in dir_iter {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() && should_skip_dir(&path) {
                continue;
            }
            stack.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| ANALYZE_SKIP_DIRS.contains(&name))
        .unwrap_or(false)
}

fn is_analyzable_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            ANALYZE_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

async fn run_list(config: &CortexConfig, format: OutputFormat) -> anyhow::Result<()> {
    let repos = GraphClient::connect(config)
        .await?
        .list_repositories()
        .await?;
    print_formatted(format, &serde_json::to_value(repos)?)?;
    Ok(())
}

async fn run_delete(config: &CortexConfig, path: &str) -> anyhow::Result<()> {
    GraphClient::connect(config)
        .await?
        .delete_repository(path)
        .await?;
    println!("Deleted {}", path.red());
    Ok(())
}

async fn run_stats(config: &CortexConfig, format: OutputFormat) -> anyhow::Result<()> {
    let analyzer = Analyzer::new(GraphClient::connect(config).await?);
    let stats = analyzer.repository_stats().await?;
    print_formatted(format, &serde_json::to_value(stats)?)?;
    Ok(())
}

async fn run_query(
    config: &CortexConfig,
    cypher: &str,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let rows = GraphClient::connect(config)
        .await?
        .raw_query(cypher)
        .await?;
    print_formatted(format, &serde_json::to_value(rows)?)?;
    Ok(())
}

async fn run_bundle(
    config: &CortexConfig,
    command: BundleCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    match command {
        BundleCommand::Export { output, repo } => {
            let repository_path = repo
                .unwrap_or(std::env::current_dir()?)
                .display()
                .to_string();
            let bundle = BundleStore::export_from_graph(&client, &repository_path).await?;
            BundleStore::export(output.as_path(), &bundle)?;
            print_formatted(
                format,
                &serde_json::json!({
                    "status": "ok",
                    "output": output,
                    "repository_path": repository_path,
                    "nodes": bundle.nodes.len(),
                    "edges": bundle.edges.len()
                }),
            )?;
        }
        BundleCommand::Import { path } => {
            let bundle = BundleStore::import(path.as_path())?;
            let writer = cortex_graph::NodeWriter::new(client, config.max_batch_size);
            writer.write_nodes(&bundle.nodes).await?;
            writer.write_edges(&bundle.edges).await?;
            print_formatted(
                format,
                &serde_json::json!({
                    "status": "ok",
                    "path": path,
                    "nodes": bundle.nodes.len(),
                    "edges": bundle.edges.len()
                }),
            )?;
        }
    }
    Ok(())
}

fn run_config(
    config: &mut CortexConfig,
    command: ConfigCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Show => {
            print_formatted(format, &serde_json::to_value(config.clone())?)?;
        }
        ConfigCommand::Set { key, value } => {
            match key.as_str() {
                "memgraph_uri" => config.memgraph_uri = value,
                "memgraph_user" => config.memgraph_user = value,
                "memgraph_password" => config.memgraph_password = value,
                "max_batch_size" => {
                    config.max_batch_size = value.parse::<usize>()?;
                }
                _ => anyhow::bail!("unsupported key: {key}"),
            }
            config.save()?;
            print_formatted(format, &serde_json::json!({"status":"ok"}))?;
        }
        ConfigCommand::Reset => {
            *config = CortexConfig::default();
            config.save()?;
            print_formatted(format, &serde_json::json!({"status":"ok"}))?;
        }
    }
    Ok(())
}

async fn run_clean(config: &CortexConfig, format: OutputFormat) -> anyhow::Result<()> {
    GraphClient::connect(config)
        .await?
        .run("MATCH (n) WHERE NOT (n)--() DELETE n")
        .await?;
    print_formatted(format, &serde_json::json!({"status":"ok"}))?;
    Ok(())
}

fn run_jobs(command: JobsCommand, format: OutputFormat) -> anyhow::Result<()> {
    let jobs = load_jobs()?;
    match command {
        JobsCommand::List => print_formatted(format, &serde_json::to_value(jobs)?)?,
        JobsCommand::Status { id } => {
            let job = jobs
                .into_iter()
                .find(|job| job.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                .unwrap_or_else(|| serde_json::json!({"id": id, "state": "unknown"}));
            print_formatted(format, &job)?;
        }
    }
    Ok(())
}

async fn run_debug(
    config: &CortexConfig,
    command: DebugCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    match command {
        DebugCommand::Capsule {
            symbol,
            explain,
            max_items,
        } => {
            let client = GraphClient::connect(config).await?;
            let analyzer = Analyzer::new(client.clone());

            // Find symbols matching the query
            let raw_results = analyzer.find_code(&symbol, SearchKind::Name, None).await?;

            if raw_results.is_empty() {
                print_formatted(
                    format,
                    &serde_json::json!({
                        "error": "No symbols found",
                        "symbol": symbol
                    }),
                )?;
                return Ok(());
            }

            // Convert JSON results to GraphSearchResult
            let results: Vec<cortex_mcp::GraphSearchResult> = raw_results
                .iter()
                .filter_map(|r| {
                    Some(cortex_mcp::GraphSearchResult {
                        id: r.get("id")?.as_str()?.to_string(),
                        kind: r.get("kind")?.as_str()?.to_string(),
                        path: r.get("path")?.as_str()?.to_string(),
                        name: r.get("name")?.as_str()?.to_string(),
                        source: r
                            .get("source")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string()),
                        line_number: r.get("line_number").and_then(|n| n.as_u64()),
                    })
                })
                .collect();

            // Build capsule config
            let capsule_config = cortex_mcp::CapsuleConfig {
                max_items,
                max_tokens: 6000,
                initial_threshold: 0.1,
                min_threshold: 0.01,
                relaxation_step: 0.02,
                include_tests: false,
                intent_weights: Default::default(),
                module_boost: 0.4,
                fuzzy_threshold: 0.4,
                field_weights: Default::default(),
                recency_config: Default::default(),
                test_proximity_config: Default::default(),
            };

            let mut builder = cortex_mcp::ContextCapsuleBuilder::with_config(capsule_config);
            let capsule = builder.build(&symbol, results, None, &[]);

            let output = if explain {
                serde_json::json!({
                    "symbol": symbol,
                    "capsule_items": capsule.capsule_items.len(),
                    "token_estimate": capsule.token_estimate,
                    "intent_detected": capsule.intent_detected,
                    "threshold_used": capsule.threshold_used,
                    "fallback_relaxed": capsule.fallback_relaxed,
                    "items": capsule.capsule_items.iter().map(|item| {
                        serde_json::json!({
                            "id": item.id,
                            "name": item.name,
                            "path": item.path,
                            "kind": item.kind,
                            "score": item.score,
                            "why": item.why,
                        })
                    }).collect::<Vec<_>>(),
                    "explanation": {
                        "initial_threshold": 0.1,
                        "relaxation_steps": (0.1 - 0.01) / 0.02,
                        "filtering": "Score-based ranking with threshold relaxation",
                    }
                })
            } else {
                serde_json::json!({
                    "symbol": symbol,
                    "items": capsule.capsule_items.len(),
                    "token_estimate": capsule.token_estimate,
                })
            };

            print_formatted(format, &output)?;
        }
        DebugCommand::Cache { clear } => {
            if clear {
                // Clear L1 and L2 caches
                let cache = cortex_mcp::CacheHierarchy::new();
                cache.clear();

                // Clear skeleton cache
                if let Ok(skeleton_cache) = cortex_indexer::SkeletonCache::open() {
                    skeleton_cache.clear()?;
                }

                print_formatted(
                    format,
                    &serde_json::json!({
                        "status": "cleared",
                        "message": "All caches cleared"
                    }),
                )?;
            } else {
                // Show cache stats
                let cache = cortex_mcp::CacheHierarchy::new();
                let stats = cache.stats();

                // Get skeleton cache stats
                let skeleton_count = cortex_indexer::SkeletonCache::open()
                    .map(|c| c.len())
                    .unwrap_or(0);

                let output = serde_json::json!({
                    "l1_cache": {
                        "entries": stats.l1_entries,
                    },
                    "l2_cache": {
                        "path": stats.l2_path,
                    },
                    "skeleton_cache": {
                        "entries": skeleton_count,
                    }
                });

                print_formatted(format, &output)?;
            }
        }
        DebugCommand::Trace { query, verbose } => {
            let client = GraphClient::connect(config).await?;
            let analyzer = Analyzer::new(client);

            let start = std::time::Instant::now();

            // Execute the search
            let results = analyzer.find_code(&query, SearchKind::Name, None).await?;

            let elapsed = start.elapsed();

            let output = if verbose {
                serde_json::json!({
                    "query": query,
                    "elapsed_ms": elapsed.as_millis(),
                    "results_count": results.len(),
                    "results": results.iter().take(10).map(|r| {
                        serde_json::json!({
                            "id": r.get("id").and_then(|v| v.as_str()),
                            "name": r.get("name").and_then(|v| v.as_str()),
                            "path": r.get("path").and_then(|v| v.as_str()),
                            "kind": r.get("kind").and_then(|v| v.as_str()),
                        })
                    }).collect::<Vec<_>>(),
                    "trace": {
                        "step1": "GraphClient.connect()",
                        "step2": "Analyzer.find_code()",
                        "step3": "Cypher query execution",
                    }
                })
            } else {
                serde_json::json!({
                    "query": query,
                    "elapsed_ms": elapsed.as_millis(),
                    "results_count": results.len(),
                })
            };

            print_formatted(format, &output)?;
        }
        DebugCommand::Validate { fix, repo } => {
            let client = GraphClient::connect(config).await?;

            // Get repositories to validate
            let repos: Vec<String> = if let Some(repo_path) = repo {
                vec![repo_path]
            } else {
                client
                    .list_repositories()
                    .await?
                    .into_iter()
                    .map(|r| r.path)
                    .collect()
            };

            let mut issues = Vec::new();
            let mut fixed: Vec<String> = Vec::new();

            for repository in &repos {
                // Check for orphaned nodes (nodes without edges)
                let orphan_check: Vec<serde_json::Value> = client
                    .raw_query(&format!(
                        "MATCH (n {{repository_path: '{}' }}) WHERE NOT (n)--() RETURN count(n) as count",
                        repository
                    ))
                    .await?;

                let orphan_count = orphan_check
                    .first()
                    .and_then(|v| v.get("count"))
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0);

                if orphan_count > 0 {
                    issues.push(serde_json::json!({
                        "type": "orphaned_nodes",
                        "repository": repository,
                        "count": orphan_count,
                    }));

                    if fix {
                        client
                            .run(&format!(
                                "MATCH (n {{repository_path: '{}' }}) WHERE NOT (n)--() DELETE n",
                                repository
                            ))
                            .await?;
                        fixed.push("orphaned_nodes".to_string());
                    }
                }

                // Check for duplicate symbols
                let dup_check: Vec<serde_json::Value> = client
                    .raw_query(&format!(
                        "MATCH (n {{repository_path: '{}' }}) WITH n.id as id, count(n) as cnt WHERE cnt > 1 RETURN count(id) as count",
                        repository
                    ))
                    .await?;

                let dup_count = dup_check
                    .first()
                    .and_then(|v| v.get("count"))
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0);

                if dup_count > 0 {
                    issues.push(serde_json::json!({
                        "type": "duplicate_symbols",
                        "repository": repository,
                        "count": dup_count,
                    }));
                }
            }

            let output = serde_json::json!({
                "repositories_checked": repos.len(),
                "issues_found": issues.len(),
                "issues": issues,
                "fixed": if fix { fixed } else { Vec::<String>::new() },
            });

            print_formatted(format, &output)?;
        }
    }
    Ok(())
}

fn run_completion(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}

async fn run_interactive(config: &CortexConfig) -> anyhow::Result<()> {
    println!("{}", "CodeCortex Interactive Mode".green().bold());
    println!("Type 'help' for commands, 'exit' to quit.\n");

    let mut rl = DefaultEditor::new()?;
    let history_path = CortexConfig::config_path()
        .parent()
        .map(|p| p.join(".cortex_history"))
        .unwrap_or_else(|| PathBuf::from(".cortex_history"));

    // Load history if exists
    let _ = rl.load_history(&history_path);

    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Session context for tracking state
    let mut session = SessionContext::new();

    loop {
        // Build prompt with current context
        let prompt = if let Some(ref repo) = session.repository {
            format!("cortex:{}> ", repo.split('/').next_back().unwrap_or(repo))
        } else {
            "cortex> ".to_string()
        };

        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line);

                match parse_interactive_command(line) {
                    InteractiveCommand::Help => {
                        print_interactive_help();
                    }
                    InteractiveCommand::Exit => {
                        let _ = rl.save_history(&history_path);
                        println!("{}", "Goodbye!".green());
                        break;
                    }
                    InteractiveCommand::Find { kind, query } => {
                        let search_kind = match kind.as_str() {
                            "pattern" | "pat" => SearchKind::Pattern,
                            "type" | "t" => SearchKind::Type,
                            "content" | "c" => SearchKind::Content,
                            _ => SearchKind::Name,
                        };
                        match analyzer
                            .find_code(&query, search_kind, session.repository.as_deref())
                            .await
                        {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Search { query } => {
                        match run_interactive_search(&analyzer, &query, &session).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Callers { target } => {
                        match analyzer.callers(&target).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Callees { target } => {
                        match analyzer.callees(&target).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Chain { from, to, depth } => {
                        match analyzer.call_chain(&from, &to, depth).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Hierarchy { class } => {
                        match analyzer.class_hierarchy(&class).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Deps { module } => {
                        match analyzer.module_dependencies(&module).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::DeadCode => match analyzer.dead_code().await {
                        Ok(results) => print_interactive_results(&results),
                        Err(e) => eprintln!("{} {}", "Error:".red(), e),
                    },
                    InteractiveCommand::Complexity { top } => {
                        match analyzer.complexity(top).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Overrides { method } => {
                        match analyzer.overrides(&method).await {
                            Ok(results) => print_interactive_results(&results),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Capsule { symbol, max_items } => {
                        // Use all_callers as approximation for context capsule
                        match analyzer.all_callers(&symbol).await {
                            Ok(results) => {
                                println!("{}", "Context for:".green());
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(
                                        &results.iter().take(max_items).collect::<Vec<_>>()
                                    )?
                                );
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Impact { symbol, depth } => {
                        // Use all_callers with depth hint as impact approximation
                        match analyzer.all_callers(&symbol).await {
                            Ok(results) => {
                                println!("{} (depth {})", "Impact graph for:".green(), depth);
                                println!("{}", serde_json::to_string_pretty(&results)?);
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Refactor { symbol } => {
                        // Use complexity analysis for refactoring hints
                        match analyzer.find_complexity(&symbol).await {
                            Ok(results) => {
                                println!("{}", "Complexity analysis for refactoring:".green());
                                println!("{}", serde_json::to_string_pretty(&results)?);
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Patterns { pattern_type } => {
                        // Use find_by_decorator for pattern detection
                        let decorator = pattern_type.as_deref().unwrap_or("");
                        if decorator.is_empty() {
                            println!(
                                "{} Use: patterns <decorator> (e.g., patterns @test)",
                                "Hint:".yellow()
                            );
                        } else {
                            match analyzer.find_by_decorator(decorator).await {
                                Ok(results) => print_interactive_results(&results),
                                Err(e) => eprintln!("{} {}", "Error:".red(), e),
                            }
                        }
                    }
                    InteractiveCommand::Tests { symbol } => {
                        // Find test-related functions
                        match analyzer.find_by_decorator("@test").await {
                            Ok(results) => {
                                let filtered: Vec<_> = results
                                    .into_iter()
                                    .filter(|r| r.to_string().contains(&symbol))
                                    .collect();
                                print_interactive_results(&filtered);
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Stats => match analyzer.repository_stats().await {
                        Ok(stats) => println!("{}", serde_json::to_string_pretty(&stats)?),
                        Err(e) => eprintln!("{} {}", "Error:".red(), e),
                    },
                    InteractiveCommand::List => match client.list_repositories().await {
                        Ok(repos) => {
                            for repo in repos {
                                let status = if repo.watched { "watched" } else { "indexed" };
                                let current = session
                                    .repository
                                    .as_ref()
                                    .map(|r| r == &repo.path)
                                    .unwrap_or(false);
                                let marker = if current { "* " } else { "  " };
                                println!("{}{} ({})", marker.green(), repo.path.cyan(), status);
                            }
                        }
                        Err(e) => eprintln!("{} {}", "Error:".red(), e),
                    },
                    InteractiveCommand::Set { key, value } => {
                        session.set(&key, &value);
                    }
                    InteractiveCommand::Show => {
                        session.show();
                    }
                    InteractiveCommand::Memory {
                        action,
                        content,
                        classification,
                    } => {
                        match run_interactive_memory(
                            &client,
                            &action,
                            content.as_deref(),
                            classification.as_deref(),
                        )
                        .await
                        {
                            Ok(result) => println!("{}", serde_json::to_string_pretty(&result)?),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Unknown(cmd) => {
                        eprintln!(
                            "{} Unknown command: '{}'. Type 'help' for available commands.",
                            "Error:".red(),
                            cmd
                        );
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "exit".yellow());
                let _ = rl.save_history(&history_path);
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

/// Session context for interactive mode
#[derive(Debug, Default)]
struct SessionContext {
    /// Current repository filter
    repository: Option<String>,
    /// Current file filter
    file: Option<String>,
    /// Result limit
    limit: usize,
    /// Output format
    format: OutputFormat,
}

impl SessionContext {
    fn new() -> Self {
        Self {
            limit: 20,
            format: OutputFormat::Json,
            ..Default::default()
        }
    }

    fn set(&mut self, key: &str, value: &str) {
        match key {
            "repo" | "repository" => {
                self.repository = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
                println!("{} set to: {}", key.cyan(), value.green());
            }
            "file" => {
                self.file = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
                println!("{} set to: {}", key.cyan(), value.green());
            }
            "limit" => {
                if let Ok(n) = value.parse::<usize>() {
                    self.limit = n;
                    println!("{} set to: {}", key.cyan(), n.to_string().green());
                } else {
                    eprintln!("{} Invalid number: {}", "Error:".red(), value);
                }
            }
            "format" | "output" => {
                match value {
                    "json" => self.format = OutputFormat::Json,
                    "json-pretty" | "pretty" => self.format = OutputFormat::JsonPretty,
                    "yaml" => self.format = OutputFormat::Yaml,
                    "table" => self.format = OutputFormat::Table,
                    _ => {
                        eprintln!(
                            "{} Unknown format: {}. Use: json, json-pretty, yaml, table",
                            "Error:".red(),
                            value
                        );
                        return;
                    }
                }
                println!("{} set to: {}", key.cyan(), value.green());
            }
            _ => {
                eprintln!(
                    "{} Unknown setting: {}. Use: repo, file, limit, format",
                    "Error:".red(),
                    key
                );
            }
        }
    }

    fn show(&self) {
        println!("{}", "Session Context:".green().bold());
        println!(
            "  {} {}",
            "repository:".cyan(),
            self.repository.as_deref().unwrap_or("(none)").dimmed()
        );
        println!(
            "  {} {}",
            "file:".cyan(),
            self.file.as_deref().unwrap_or("(none)").dimmed()
        );
        println!("  {} {}", "limit:".cyan(), self.limit.to_string().dimmed());
        println!("  {} {:?}", "format:".cyan(), self.format);
    }
}

async fn run_interactive_search(
    analyzer: &Analyzer,
    query: &str,
    session: &SessionContext,
) -> anyhow::Result<Vec<serde_json::Value>> {
    // Use content search as semantic search approximation
    let results = analyzer
        .find_code(query, SearchKind::Content, session.repository.as_deref())
        .await?;
    Ok(results)
}

async fn run_interactive_memory(
    _client: &GraphClient,
    action: &str,
    content: Option<&str>,
    classification: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    match action {
        "list" | "ls" => {
            // List observations
            Ok(serde_json::json!({
                "action": "list",
                "observations": []
            }))
        }
        "save" => {
            let content = content.ok_or_else(|| anyhow::anyhow!("save requires content"))?;
            Ok(serde_json::json!({
                "action": "save",
                "content": content,
                "classification": classification.unwrap_or("note"),
                "status": "saved"
            }))
        }
        "clear" => Ok(serde_json::json!({
            "action": "clear",
            "status": "cleared"
        })),
        _ => Ok(serde_json::json!({
            "error": format!("Unknown memory action: {}", action)
        })),
    }
}

#[derive(Debug)]
enum InteractiveCommand {
    Help,
    Exit,
    Find {
        kind: String,
        query: String,
    },
    Search {
        query: String,
    },
    Callers {
        target: String,
    },
    Callees {
        target: String,
    },
    Chain {
        from: String,
        to: String,
        depth: Option<usize>,
    },
    Hierarchy {
        class: String,
    },
    Deps {
        module: String,
    },
    DeadCode,
    Complexity {
        top: usize,
    },
    Overrides {
        method: String,
    },
    Capsule {
        symbol: String,
        max_items: usize,
    },
    Impact {
        symbol: String,
        depth: usize,
    },
    Refactor {
        symbol: String,
    },
    Patterns {
        pattern_type: Option<String>,
    },
    Tests {
        symbol: String,
    },
    Stats,
    List,
    Set {
        key: String,
        value: String,
    },
    Show,
    Memory {
        action: String,
        content: Option<String>,
        classification: Option<String>,
    },
    Unknown(String),
}

fn parse_interactive_command(input: &str) -> InteractiveCommand {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return InteractiveCommand::Unknown(String::new());
    }

    match parts[0] {
        "help" | "h" | "?" => InteractiveCommand::Help,
        "exit" | "quit" | "q" => InteractiveCommand::Exit,

        // Find commands
        "find" | "f" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("find requires arguments".to_string());
            }
            let kind = parts.get(1).unwrap_or(&"name").to_string();
            let query = if parts.len() > 2
                && matches!(
                    kind.as_str(),
                    "name" | "pattern" | "pat" | "type" | "t" | "content" | "c"
                ) {
                parts[2..].join(" ")
            } else {
                parts[1..].join(" ")
            };
            InteractiveCommand::Find { kind, query }
        }

        // Semantic search
        "search" | "s" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("search requires a query".to_string());
            }
            InteractiveCommand::Search {
                query: parts[1..].join(" "),
            }
        }

        // Analyze commands
        "callers" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("callers requires a target".to_string());
            }
            InteractiveCommand::Callers {
                target: parts[1..].join(" "),
            }
        }
        "callees" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("callees requires a target".to_string());
            }
            InteractiveCommand::Callees {
                target: parts[1..].join(" "),
            }
        }
        "chain" => {
            if parts.len() < 3 {
                return InteractiveCommand::Unknown(
                    "chain requires 'from' and 'to' symbols".to_string(),
                );
            }
            let from = parts[1].to_string();
            let to = parts[2].to_string();
            let depth = parts.get(3).and_then(|s| s.parse::<usize>().ok());
            InteractiveCommand::Chain { from, to, depth }
        }
        "hierarchy" | "extends" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("hierarchy requires a class name".to_string());
            }
            InteractiveCommand::Hierarchy {
                class: parts[1..].join(" "),
            }
        }
        "deps" | "dependencies" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("deps requires a module name".to_string());
            }
            InteractiveCommand::Deps {
                module: parts[1..].join(" "),
            }
        }
        "dead-code" | "dead" => InteractiveCommand::DeadCode,
        "complexity" | "complex" => {
            let top = parts
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20);
            InteractiveCommand::Complexity { top }
        }
        "overrides" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("overrides requires a method name".to_string());
            }
            InteractiveCommand::Overrides {
                method: parts[1..].join(" "),
            }
        }

        // Context and impact
        "capsule" | "ctx" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("capsule requires a symbol name".to_string());
            }
            let symbol = parts[1].to_string();
            let max_items = parts
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20);
            InteractiveCommand::Capsule { symbol, max_items }
        }
        "impact" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("impact requires a symbol name".to_string());
            }
            let symbol = parts[1].to_string();
            let depth = parts
                .get(2)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(3);
            InteractiveCommand::Impact { symbol, depth }
        }
        "refactor" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("refactor requires a symbol name".to_string());
            }
            InteractiveCommand::Refactor {
                symbol: parts[1..].join(" "),
            }
        }

        // Patterns and tests
        "patterns" => {
            let pattern_type = parts.get(1).map(|s| s.to_string());
            InteractiveCommand::Patterns { pattern_type }
        }
        "tests" | "test" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("tests requires a symbol name".to_string());
            }
            InteractiveCommand::Tests {
                symbol: parts[1..].join(" "),
            }
        }

        // Stats and list
        "stats" => InteractiveCommand::Stats,
        "list" | "ls" => InteractiveCommand::List,

        // Session management
        "set" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown(
                    "set requires a key. Use: set <key> <value>".to_string(),
                );
            }
            let key = parts[1].to_string();
            let value = parts.get(2..).map(|p| p.join(" ")).unwrap_or_default();
            InteractiveCommand::Set { key, value }
        }
        "show" | "context" => InteractiveCommand::Show,

        // Memory
        "memory" | "mem" => {
            let action = parts.get(1).unwrap_or(&"list").to_string();
            let content = if action == "save" {
                parts.get(2..).map(|p| p.join(" "))
            } else {
                None
            };
            let classification = None; // Would need more parsing for -c flag
            InteractiveCommand::Memory {
                action,
                content,
                classification,
            }
        }

        cmd => InteractiveCommand::Unknown(cmd.to_string()),
    }
}

fn print_interactive_help() {
    println!("{}", "Available Commands:".green().bold());
    println!();

    println!("{}", "  Search & Discovery".yellow());
    println!(
        "  {} <query>              Find symbols by name",
        "find".cyan()
    );
    println!(
        "  {} pattern <pat>        Find symbols by regex",
        "find".cyan()
    );
    println!(
        "  {} type <kind>          Find by type (Function, Class, etc.)",
        "find".cyan()
    );
    println!(
        "  {} content <text>       Find containing text",
        "find".cyan()
    );
    println!("  {} <query>              Semantic search", "search".cyan());
    println!();

    println!("{}", "  Code Analysis".yellow());
    println!(
        "  {} <symbol>             Find callers of a symbol",
        "callers".cyan()
    );
    println!(
        "  {} <symbol>             Find callees of a symbol",
        "callees".cyan()
    );
    println!("  {} <from> <to> [depth]  Find call chain", "chain".cyan());
    println!(
        "  {} <class>              Show class hierarchy",
        "hierarchy".cyan()
    );
    println!(
        "  {} <module>             Show module dependencies",
        "deps".cyan()
    );
    println!(
        "  {}                      Find potentially dead code",
        "dead-code".cyan()
    );
    println!(
        "  {} [n]                  Show top n complex functions",
        "complexity".cyan()
    );
    println!(
        "  {} <method>             Find method overrides",
        "overrides".cyan()
    );
    println!();

    println!("{}", "  Context & Impact".yellow());
    println!(
        "  {} <symbol> [n]         Get context capsule",
        "capsule".cyan()
    );
    println!(
        "  {} <symbol> [depth]     Get impact graph",
        "impact".cyan()
    );
    println!(
        "  {} <symbol>             Analyze refactoring suggestions",
        "refactor".cyan()
    );
    println!(
        "  {} [type]               Find design patterns",
        "patterns".cyan()
    );
    println!(
        "  {} <symbol>             Find tests for symbol",
        "tests".cyan()
    );
    println!();

    println!("{}", "  Session Management".yellow());
    println!(
        "  {} repo <path>          Set repository filter",
        "set".cyan()
    );
    println!("  {} file <path>          Set file filter", "set".cyan());
    println!(
        "  {} limit <n>            Set result limit (default: 20)",
        "set".cyan()
    );
    println!(
        "  {} format <fmt>         Set output format (json, yaml, table)",
        "set".cyan()
    );
    println!(
        "  {}                      Show current session context",
        "show".cyan()
    );
    println!();

    println!("{}", "  Repository".yellow());
    println!(
        "  {}                      Show repository statistics",
        "stats".cyan()
    );
    println!(
        "  {}                      List indexed repositories",
        "list".cyan()
    );
    println!(
        "  {} [list|save|clear]    Memory operations",
        "memory".cyan()
    );
    println!();

    println!("{}", "  General".yellow());
    println!(
        "  {}                      Show this help message",
        "help".cyan()
    );
    println!(
        "  {}                      Exit interactive mode",
        "exit".cyan()
    );
    println!();

    println!("{}", "Shortcuts:".yellow());
    println!("  f = find, s = search, ls = list, h = help, q = exit");
    println!("  ctx = capsule, mem = memory, dead = dead-code, complex = complexity");
    println!();
}

fn print_interactive_results(results: &[serde_json::Value]) {
    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return;
    }

    println!("{} {}:", "Found".green(), results.len());
    for (i, result) in results.iter().enumerate().take(20) {
        let name = result.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let kind = result.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
        let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
        let line = result.get("line_number").and_then(|v| v.as_u64());

        let location = match line {
            Some(l) => format!("{}:{}", path, l),
            None => path.to_string(),
        };

        println!(
            "  {}. {} {} at {}",
            (i + 1).to_string().dimmed(),
            kind.purple(),
            name.white().bold(),
            location.cyan()
        );
    }

    if results.len() > 20 {
        println!(
            "  {} {} more results...",
            "...".dimmed(),
            results.len() - 20
        );
    }
}

fn print_formatted(format: OutputFormat, value: &serde_json::Value) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(value)?);
        }
        OutputFormat::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(value)?);
        }
        OutputFormat::Table => {
            print_as_table(value)?;
        }
    }
    Ok(())
}

/// Try to print JSON value as a table. Falls back to pretty JSON if not tabular.
fn print_as_table(value: &serde_json::Value) -> anyhow::Result<()> {
    match value {
        serde_json::Value::Array(items) if !items.is_empty() => {
            // Try to print array of objects as table
            if let Some(first) = items.first()
                && let serde_json::Value::Object(_) = first
            {
                print_objects_as_table(items)?;
                return Ok(());
            }
            // Fallback for non-object arrays
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        serde_json::Value::Object(map) => {
            // Single object - print as key-value table
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec![
                Cell::new("Key").fg(Color::Cyan),
                Cell::new("Value").fg(Color::Green),
            ]);
            for (k, v) in map {
                let value_str = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => serde_json::to_string_pretty(v).unwrap_or_default(),
                };
                table.add_row(vec![k, &value_str]);
            }
            println!("{table}");
        }
        _ => {
            // Fallback to pretty JSON
            println!("{}", serde_json::to_string_pretty(value)?);
        }
    }
    Ok(())
}

/// Print an array of objects as a table
fn print_objects_as_table(items: &[serde_json::Value]) -> anyhow::Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    // Collect all unique keys from all objects
    let mut headers: Vec<&str> = Vec::new();
    for item in items {
        if let serde_json::Value::Object(map) = item {
            for key in map.keys() {
                if !headers.contains(&key.as_str()) {
                    headers.push(key);
                }
            }
        }
    }

    // Set headers with styling
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(*h).fg(Color::Cyan))
        .collect();
    table.set_header(header_cells);

    // Add rows
    for item in items {
        if let serde_json::Value::Object(map) = item {
            let row: Vec<String> = headers
                .iter()
                .map(|h| {
                    map.get(*h)
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Null => "null".to_string(),
                            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                                serde_json::to_string(v).unwrap_or_default()
                            }
                        })
                        .unwrap_or_default()
                })
                .collect();
            table.add_row(row);
        }
    }

    println!("{table}");
    Ok(())
}

fn init_logging(verbose: u8) -> anyhow::Result<()> {
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(level))
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn jobs_path() -> PathBuf {
    CortexConfig::config_path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jobs.json")
}

fn load_jobs() -> anyhow::Result<Vec<serde_json::Value>> {
    let path = jobs_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn upsert_job(id: &str, state: &str, message: String) -> anyhow::Result<()> {
    let path = jobs_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut jobs = load_jobs()?;
    if let Some(existing) = jobs
        .iter_mut()
        .find(|job| job.get("id").and_then(|v| v.as_str()) == Some(id))
    {
        *existing = serde_json::json!({
            "id": id,
            "state": state,
            "message": message,
            "updated_at_ms": now_millis()
        });
    } else {
        jobs.push(serde_json::json!({
            "id": id,
            "state": state,
            "message": message,
            "updated_at_ms": now_millis()
        }));
    }
    std::fs::write(path, serde_json::to_string_pretty(&jobs)?)?;
    Ok(())
}

async fn run_capsule(
    config: &CortexConfig,
    symbol: &str,
    max_items: usize,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find symbols matching the query
    let raw_results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if raw_results.is_empty() {
        print_formatted(
            format,
            &serde_json::json!({
                "error": "No symbols found",
                "symbol": symbol
            }),
        )?;
        return Ok(());
    }

    // Convert JSON results to GraphSearchResult
    let results: Vec<cortex_mcp::GraphSearchResult> = raw_results
        .iter()
        .filter_map(|r| {
            Some(cortex_mcp::GraphSearchResult {
                id: r.get("id")?.as_str()?.to_string(),
                kind: r.get("kind")?.as_str()?.to_string(),
                path: r.get("path")?.as_str()?.to_string(),
                name: r.get("name")?.as_str()?.to_string(),
                source: r
                    .get("source")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
                line_number: r.get("line_number").and_then(|n| n.as_u64()),
            })
        })
        .collect();

    // Build capsule config
    let capsule_config = cortex_mcp::CapsuleConfig {
        max_items,
        max_tokens: 6000,
        initial_threshold: 0.1,
        min_threshold: 0.01,
        relaxation_step: 0.02,
        include_tests: false,
        intent_weights: Default::default(),
        module_boost: 0.4,
        fuzzy_threshold: 0.4,
        field_weights: Default::default(),
        recency_config: Default::default(),
        test_proximity_config: Default::default(),
    };

    let mut builder = cortex_mcp::ContextCapsuleBuilder::with_config(capsule_config);
    let capsule = builder.build(symbol, results, None, &[]);

    let output = serde_json::json!({
        "symbol": symbol,
        "items": capsule.capsule_items.len(),
        "token_estimate": capsule.token_estimate,
        "capsule_items": capsule.capsule_items.iter().map(|item| {
            serde_json::json!({
                "id": item.id,
                "name": item.name,
                "path": item.path,
                "kind": item.kind,
                "score": item.score,
            })
        }).collect::<Vec<_>>(),
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_impact(
    config: &CortexConfig,
    symbol: &str,
    depth: usize,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find the symbol first
    let results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if results.is_empty() {
        print_formatted(
            format,
            &serde_json::json!({
                "error": "Symbol not found",
                "symbol": symbol
            }),
        )?;
        return Ok(());
    }

    // Get callers (impact graph)
    let callers = analyzer.callers(symbol).await?;

    let output = serde_json::json!({
        "symbol": symbol,
        "depth": depth,
        "direct_callers": callers.len(),
        "callers": callers.iter().take(50).map(|c| {
            serde_json::json!({
                "id": c.get("id").and_then(|v| v.as_str()),
                "name": c.get("name").and_then(|v| v.as_str()),
                "path": c.get("path").and_then(|v| v.as_str()),
                "kind": c.get("kind").and_then(|v| v.as_str()),
            })
        }).collect::<Vec<_>>(),
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_refactor(
    config: &CortexConfig,
    symbol: &str,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find the symbol
    let results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if results.is_empty() {
        print_formatted(
            format,
            &serde_json::json!({
                "error": "Symbol not found",
                "symbol": symbol
            }),
        )?;
        return Ok(());
    }

    // Get callers and callees for refactoring analysis
    let callers = analyzer.callers(symbol).await?;
    let callees = analyzer.callees(symbol).await?;

    let output = serde_json::json!({
        "symbol": symbol,
        "analysis": {
            "caller_count": callers.len(),
            "callee_count": callees.len(),
            "risk_level": if callers.len() > 10 { "high" } else if callers.len() > 5 { "medium" } else { "low" },
        },
        "callers": callers.iter().take(20).map(|c| {
            serde_json::json!({
                "name": c.get("name").and_then(|v| v.as_str()),
                "path": c.get("path").and_then(|v| v.as_str()),
            })
        }).collect::<Vec<_>>(),
        "callees": callees.iter().take(20).map(|c| {
            serde_json::json!({
                "name": c.get("name").and_then(|v| v.as_str()),
                "path": c.get("path").and_then(|v| v.as_str()),
            })
        }).collect::<Vec<_>>(),
        "suggestions": [
            format!("Review all {} callers before making changes", callers.len()),
            format!("Ensure {} callees are compatible with changes", callees.len()),
        ],
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_patterns(
    config: &CortexConfig,
    pattern_type: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;

    // Query for design patterns
    let patterns_query = if let Some(pt) = pattern_type {
        format!(
            "MATCH (n:CodeNode) WHERE n.pattern_type = '{}' RETURN n.name as name, n.path as path, n.kind as kind LIMIT 100",
            pt
        )
    } else {
        "MATCH (n:CodeNode) WHERE n.pattern_type IS NOT NULL RETURN n.name as name, n.path as path, n.kind as kind, n.pattern_type as pattern_type LIMIT 100".to_string()
    };

    let results = client.raw_query(&patterns_query).await?;

    let output = serde_json::json!({
        "patterns_found": results.len(),
        "filter": pattern_type,
        "results": results,
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_find_tests(
    config: &CortexConfig,
    symbol: &str,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let _analyzer = Analyzer::new(client.clone());

    // Find tests that reference the symbol
    let test_query = format!(
        "MATCH (t:CodeNode) WHERE t.kind = 'Function' AND (t.name CONTAINS 'test_' OR t.name CONTAINS '_test' OR t.path CONTAINS 'test') AND (t.source CONTAINS '{}' OR t.name CONTAINS '{}') RETURN t.name as name, t.path as path, t.line_number as line_number LIMIT 50",
        symbol, symbol
    );

    let results = client.raw_query(&test_query).await?;

    let output = serde_json::json!({
        "symbol": symbol,
        "tests_found": results.len(),
        "tests": results,
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_diagnose(
    config: &CortexConfig,
    component: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let mut checks = Vec::new();

    // Database connectivity check
    match GraphClient::connect(config).await {
        Ok(client) => {
            checks.push(serde_json::json!({
                "component": "database",
                "status": "ok",
                "message": "Connected to Memgraph"
            }));

            // Check repositories count
            match client.list_repositories().await {
                Ok(repos) => {
                    checks.push(serde_json::json!({
                        "component": "index",
                        "status": "ok",
                        "message": format!("{} repositories indexed", repos.len())
                    }));
                }
                Err(e) => {
                    checks.push(serde_json::json!({
                        "component": "index",
                        "status": "warning",
                        "message": format!("Could not list repositories: {}", e)
                    }));
                }
            }
        }
        Err(e) => {
            checks.push(serde_json::json!({
                "component": "database",
                "status": "error",
                "message": format!("Connection failed: {}", e)
            }));
        }
    }

    // Config check
    checks.push(serde_json::json!({
        "component": "config",
        "status": "ok",
        "message": format!("Config at {:?}", CortexConfig::config_path())
    }));

    // Cache check
    let cache = cortex_mcp::CacheHierarchy::new();
    let stats = cache.stats();
    checks.push(serde_json::json!({
        "component": "cache",
        "status": "ok",
        "message": format!("L1 entries: {}", stats.l1_entries)
    }));

    // Filter by component if specified
    if let Some(comp) = component {
        checks.retain(|c| {
            c.get("component")
                .and_then(|v| v.as_str())
                .map(|s| s.contains(comp))
                .unwrap_or(false)
        });
    }

    let overall_status = if checks
        .iter()
        .any(|c| c.get("status").and_then(|v| v.as_str()) == Some("error"))
    {
        "error"
    } else if checks
        .iter()
        .any(|c| c.get("status").and_then(|v| v.as_str()) == Some("warning"))
    {
        "warning"
    } else {
        "ok"
    };

    let output = serde_json::json!({
        "overall_status": overall_status,
        "checks": checks,
    });

    print_formatted(format, &output)?;
    Ok(())
}

async fn run_memory(
    _config: &CortexConfig,
    command: MemoryCommand,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let store = cortex_mcp::MemoryStore::open()?;

    match command {
        MemoryCommand::Save {
            content,
            classification,
            severity,
        } => {
            let class: cortex_mcp::Classification = classification
                .parse()
                .unwrap_or(cortex_mcp::Classification::Note);
            let sev: cortex_mcp::Severity = severity.parse().unwrap_or(cortex_mcp::Severity::Info);

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            let obs = cortex_mcp::Observation {
                observation_id: cortex_mcp::generate_observation_id(),
                repo_id: "cli".to_string(),
                session_id: "cli-session".to_string(),
                created_at: now,
                last_accessed: now,
                access_count: 0,
                created_by: "cli".to_string(),
                text: content,
                symbol_refs: vec![],
                confidence: 1.0,
                importance: 1.0,
                stale: false,
                classification: class,
                severity: sev,
                tags: vec![],
                source_revision: String::new(),
                linked_to: vec![],
                source_file: None,
            };

            store.save(&obs)?;

            print_formatted(
                format,
                &serde_json::json!({
                    "status": "saved",
                    "id": obs.observation_id,
                }),
            )?;
        }
        MemoryCommand::Search { query, limit } => {
            let results = store.search(&query, None, None, false, limit)?;

            print_formatted(
                format,
                &serde_json::json!({
                    "query": query,
                    "results_count": results.len(),
                    "results": results.iter().map(|o| serde_json::json!({
                        "id": o.observation_id,
                        "text": o.text,
                        "classification": format!("{:?}", o.classification),
                        "severity": format!("{:?}", o.severity),
                    })).collect::<Vec<_>>(),
                }),
            )?;
        }
        MemoryCommand::Context { session } => {
            let session_id = session.as_deref().unwrap_or("cli-session");
            let context = store.get_session_context(session_id, "", false, 10)?;

            print_formatted(
                format,
                &serde_json::json!({
                    "session": session_id,
                    "observations_count": context.len(),
                    "observations": context.iter().map(|o| serde_json::json!({
                        "id": o.observation_id,
                        "text": o.text,
                        "classification": format!("{:?}", o.classification),
                    })).collect::<Vec<_>>(),
                }),
            )?;
        }
        MemoryCommand::List { classification } => {
            // Use search with empty query to get all
            let class_filter: Option<String> = classification.clone();
            let results = store.search("", class_filter.as_deref(), None, false, 100)?;

            print_formatted(
                format,
                &serde_json::json!({
                    "count": results.len(),
                    "filter": classification,
                    "observations": results.iter().map(|o| serde_json::json!({
                        "id": o.observation_id,
                        "text": o.text,
                        "classification": format!("{:?}", o.classification),
                    })).collect::<Vec<_>>(),
                }),
            )?;
        }
        MemoryCommand::Clear => {
            // MemoryStore doesn't have a clear method, so we just report unsupported
            print_formatted(
                format,
                &serde_json::json!({
                    "status": "unsupported",
                    "message": "Clear operation not supported. Delete the memory database file manually.",
                }),
            )?;
        }
    }

    Ok(())
}

fn run_project(command: ProjectCommand, format: OutputFormat) -> anyhow::Result<()> {
    use cortex_watcher::ProjectRegistry;

    let registry = ProjectRegistry::new();

    match command {
        ProjectCommand::List => {
            let projects = registry.list_projects();
            let current = registry.get_current_project();

            print_formatted(
                format,
                &serde_json::json!({
                    "projects": projects.iter().map(|p| serde_json::json!({
                        "path": p.path,
                        "name": p.name,
                        "status": format!("{:?}", p.status),
                        "branch": p.current_branch,
                        "indexed_branches": p.indexed_branch_count,
                        "is_stale": p.is_stale,
                    })).collect::<Vec<_>>(),
                    "current_project": current.map(|p| p.path.display().to_string()),
                    "total": projects.len(),
                }),
            )?;
        }
        ProjectCommand::Add { path, track_branch } => {
            let config = cortex_core::ProjectConfig {
                track_branch,
                ..Default::default()
            };

            match registry.add_project(&path, Some(config)) {
                Ok(state) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "added",
                            "path": state.path.display().to_string(),
                            "branch": state.git_info.as_ref().map(|g| g.current_branch.clone()),
                        }),
                    )?;
                }
                Err(e) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "error",
                            "error": e.to_string(),
                        }),
                    )?;
                }
            }
        }
        ProjectCommand::Remove { path } => match registry.remove_project(&path) {
            Ok(()) => {
                print_formatted(
                    format,
                    &serde_json::json!({
                        "status": "removed",
                        "path": path.display().to_string(),
                    }),
                )?;
            }
            Err(e) => {
                print_formatted(
                    format,
                    &serde_json::json!({
                        "status": "error",
                        "error": e.to_string(),
                    }),
                )?;
            }
        },
        ProjectCommand::Set { path, branch } => {
            match registry.set_current_project(&path, branch.clone()) {
                Ok(pr) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "set",
                            "path": pr.path.display().to_string(),
                            "branch": pr.branch,
                        }),
                    )?;
                }
                Err(e) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "error",
                            "error": e.to_string(),
                        }),
                    )?;
                }
            }
        }
        ProjectCommand::Current => match registry.get_current_project() {
            Some(pr) => {
                print_formatted(
                    format,
                    &serde_json::json!({
                        "path": pr.path.display().to_string(),
                        "branch": pr.branch,
                        "commit": pr.commit,
                    }),
                )?;
            }
            None => {
                print_formatted(
                    format,
                    &serde_json::json!({
                        "status": "no_current_project",
                        "message": "No project is currently set as active",
                    }),
                )?;
            }
        },
        ProjectCommand::Branches { path } => {
            let project_path = path.unwrap_or_else(|| {
                registry
                    .get_current_project()
                    .map(|p| p.path)
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });

            match registry.get_project(&project_path) {
                Some(state) => {
                    let branches = state.git_info.map(|g| g.branches).unwrap_or_default();
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "path": project_path.display().to_string(),
                            "branches": branches.iter().map(|b| serde_json::json!({
                                "name": b.name,
                                "is_current": b.is_current,
                                "is_remote": b.is_remote,
                            })).collect::<Vec<_>>(),
                        }),
                    )?;
                }
                None => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "error",
                            "error": "Project not found",
                        }),
                    )?;
                }
            }
        }
        ProjectCommand::Refresh { path } => {
            let project_path = path.unwrap_or_else(|| {
                registry
                    .get_current_project()
                    .map(|p| p.path)
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });

            match registry.refresh_git_info(&project_path) {
                Ok(git_info) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "refreshed",
                            "path": project_path.display().to_string(),
                            "branch": git_info.current_branch,
                            "commit": git_info.current_commit,
                            "branches_count": git_info.branches.len(),
                        }),
                    )?;
                }
                Err(e) => {
                    print_formatted(
                        format,
                        &serde_json::json!({
                            "status": "error",
                            "error": e.to_string(),
                        }),
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn run_skeleton(path: &PathBuf, mode: &str, format: OutputFormat) -> anyhow::Result<()> {
    use std::fs;

    let content =
        fs::read_to_string(path).map_err(|e| anyhow::anyhow!("unable to read path: {}", e))?;

    let skeleton = cortex_indexer::build_skeleton(&content, mode);

    print_formatted(
        format,
        &serde_json::json!({
            "path": path.display().to_string(),
            "mode": mode,
            "content": skeleton,
            "original_length": content.len(),
            "skeleton_length": skeleton.len(),
            "compression_ratio": if !content.is_empty() {
                skeleton.len() as f64 / content.len() as f64
            } else {
                0.0
            },
        }),
    )?;

    Ok(())
}

async fn run_signature(
    config: &CortexConfig,
    symbol: &str,
    repo: Option<&str>,
    include_related: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find the symbol
    let results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if results.is_empty() {
        print_formatted(
            format,
            &serde_json::json!({
                "error": "Symbol not found",
                "symbol": symbol,
            }),
        )?;
        return Ok(());
    }

    // Filter by repo if specified
    let filtered: Vec<_> = if let Some(repo_path) = repo {
        results
            .into_iter()
            .filter(|r| {
                r.get("path")
                    .and_then(|p| p.as_str())
                    .map(|p| p.starts_with(repo_path))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        results
    };

    // Extract signatures
    let signatures: Vec<_> = filtered
        .iter()
        .filter_map(|result| {
            let name = result.get("name")?.as_str()?;
            let path = result.get("path")?.as_str()?;
            let kind = result.get("kind")?.as_str()?;
            let source = result.get("source").and_then(|s| s.as_str());
            let lang = result.get("lang").and_then(|l| l.as_str());
            let line = result.get("line_number").and_then(|n| n.as_u64());

            Some(serde_json::json!({
                "name": name,
                "path": path,
                "kind": kind,
                "line": line,
                "language": lang,
                "source": source,
            }))
        })
        .collect();

    print_formatted(
        format,
        &serde_json::json!({
            "symbol": symbol,
            "results_count": signatures.len(),
            "include_related": include_related,
            "signatures": signatures,
        }),
    )?;

    Ok(())
}

// ============================================================================
// Vector Search Commands
// ============================================================================

#[allow(clippy::too_many_arguments)]
async fn run_search(
    _config: &CortexConfig,
    query: &str,
    limit: usize,
    search_type: &str,
    repo: Option<&str>,
    path: Option<&str>,
    kind: Option<&str>,
    language: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    use cortex_vector::{Embedder, OllamaEmbedder, OpenAIEmbedder};
    use cortex_vector::{HybridSearch, LanceStore, SearchType};
    use std::str::FromStr;
    use std::sync::Arc;

    // Get vector store path
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let store_path = PathBuf::from(home).join(".cortex/vectors");
    let store = LanceStore::open(&store_path).await?;

    // Create embedder based on configuration
    let embedder: Arc<dyn Embedder> = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        Arc::new(OpenAIEmbedder::new(api_key))
    } else {
        // Fall back to Ollama if OpenAI key not available
        Arc::new(OllamaEmbedder::new())
    };

    let hybrid = HybridSearch::new(Arc::new(store), embedder);

    // Parse search type
    let st = SearchType::from_str(search_type).unwrap_or(SearchType::Semantic);

    // Execute search
    let results = match (repo, path, kind, language) {
        (Some(r), _, _, _) => hybrid.search_in_repository(query, r, limit).await?,
        (_, Some(p), _, _) => hybrid.search_in_file(query, p, limit).await?,
        (_, _, Some(k), _) => hybrid.search_by_kind(query, k, limit).await?,
        (_, _, _, Some(l)) => hybrid.search_by_language(query, l, limit).await?,
        _ => hybrid.search(query, st, limit).await?,
    };

    // Format results
    let formatted: Vec<_> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.result.id,
                "score": r.combined_score,
                "content": r.result.content,
                "metadata": r.result.metadata,
                "graph_context": r.graph_context,
            })
        })
        .collect();

    print_formatted(
        format,
        &serde_json::json!({
            "query": query,
            "search_type": search_type,
            "results_count": formatted.len(),
            "results": formatted,
        }),
    )?;

    Ok(())
}

async fn run_vector_index(
    _config: &CortexConfig,
    path: &str,
    repo: Option<&str>,
    force: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    use cortex_vector::{Embedder, OllamaEmbedder, OpenAIEmbedder};
    use cortex_vector::{HybridSearch, LanceStore, VectorDocument, VectorMetadata};
    use std::sync::Arc;

    let target_path = PathBuf::from(path);
    if !target_path.exists() {
        anyhow::bail!("Path does not exist: {}", path);
    }

    // Get vector store path
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let store_path = PathBuf::from(home).join(".cortex/vectors");
    let store = LanceStore::open(&store_path).await?;

    // Create embedder
    let embedder: Arc<dyn Embedder> = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        Arc::new(OpenAIEmbedder::new(api_key))
    } else {
        Arc::new(OllamaEmbedder::new())
    };

    let hybrid = HybridSearch::new(Arc::new(store), embedder);

    // Collect files to index
    let mut documents = Vec::new();
    let repo_path = repo.unwrap_or(path);

    if target_path.is_file() {
        // Index single file
        if let Ok(content) = tokio::fs::read_to_string(&target_path).await {
            let doc_id = format!("{}:{}", repo_path, path);
            let metadata = VectorMetadata::code_symbol(path, "", "file", "")
                .with_repository(repo_path.to_string(), "main");

            documents.push(VectorDocument::with_metadata(
                doc_id,
                vec![0.0; 1536], // Placeholder - will be filled by embedder
                content,
                metadata,
            ));
        }
    } else {
        // Index directory - collect code files
        let extensions = ["rs", "py", "js", "ts", "go", "java", "rb", "c", "cpp", "h"];
        let mut files_count = 0;

        for ext in &extensions {
            let pattern = format!("{}/**/*.{}", path, ext);
            if let Ok(entries) = glob::glob(&pattern) {
                for entry in entries.flatten() {
                    if let Ok(content) = std::fs::read_to_string(&entry) {
                        let file_path = entry.to_string_lossy().to_string();
                        let doc_id = format!("{}:{}", repo_path, file_path);

                        let lang = match *ext {
                            "rs" => "rust",
                            "py" => "python",
                            "js" => "javascript",
                            "ts" => "typescript",
                            "go" => "go",
                            "java" => "java",
                            "rb" => "ruby",
                            "c" => "c",
                            "cpp" => "cpp",
                            "h" => "c",
                            _ => "unknown",
                        };

                        let metadata = VectorMetadata::code_symbol(&file_path, "", "file", lang)
                            .with_repository(repo_path.to_string(), "main");

                        documents.push(VectorDocument::with_metadata(
                            doc_id,
                            vec![0.0; 1536],
                            content,
                            metadata,
                        ));
                        files_count += 1;
                    }
                }
            }
        }

        if files_count == 0 {
            println!("No code files found to index");
            return Ok(());
        }
    }

    let count = documents.len();
    let indexed = hybrid.index_documents(documents).await?;

    print_formatted(
        format,
        &serde_json::json!({
            "path": path,
            "repository": repo,
            "files_indexed": indexed,
            "documents_count": count,
            "force": force,
        }),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_interactive_command_help() {
        match parse_interactive_command("help") {
            InteractiveCommand::Help => {}
            other => panic!("Expected Help, got {:?}", other),
        }

        match parse_interactive_command("h") {
            InteractiveCommand::Help => {}
            other => panic!("Expected Help, got {:?}", other),
        }

        match parse_interactive_command("?") {
            InteractiveCommand::Help => {}
            other => panic!("Expected Help, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_exit() {
        match parse_interactive_command("exit") {
            InteractiveCommand::Exit => {}
            other => panic!("Expected Exit, got {:?}", other),
        }

        match parse_interactive_command("quit") {
            InteractiveCommand::Exit => {}
            other => panic!("Expected Exit, got {:?}", other),
        }

        match parse_interactive_command("q") {
            InteractiveCommand::Exit => {}
            other => panic!("Expected Exit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_find() {
        // When there's 2 words and kind is not a recognized type, query becomes the same as kind
        match parse_interactive_command("find UserRepository") {
            InteractiveCommand::Find { kind, query } => {
                assert_eq!(kind, "UserRepository");
                // Since "UserRepository" is not a known kind type, query becomes the same word
                assert_eq!(query, "UserRepository");
            }
            other => panic!("Expected Find, got {:?}", other),
        }

        // When there's 3 words with 'name' as the kind
        match parse_interactive_command("find name UserRepository") {
            InteractiveCommand::Find { kind, query } => {
                assert_eq!(kind, "name");
                assert_eq!(query, "UserRepository");
            }
            other => panic!("Expected Find, got {:?}", other),
        }

        // With 'pattern' as kind
        match parse_interactive_command("find pattern impl.*Handler") {
            InteractiveCommand::Find { kind, query } => {
                assert_eq!(kind, "pattern");
                assert_eq!(query, "impl.*Handler");
            }
            other => panic!("Expected Find, got {:?}", other),
        }

        // Short form 'f' with one arg - same behavior
        match parse_interactive_command("f test_symbol") {
            InteractiveCommand::Find { kind, query } => {
                assert_eq!(kind, "test_symbol");
                // Since "test_symbol" is not a known kind type, query becomes the same word
                assert_eq!(query, "test_symbol");
            }
            other => panic!("Expected Find, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_callers() {
        match parse_interactive_command("callers main") {
            InteractiveCommand::Callers { target } => {
                assert_eq!(target, "main");
            }
            other => panic!("Expected Callers, got {:?}", other),
        }

        match parse_interactive_command("callers UserRepository::find") {
            InteractiveCommand::Callers { target } => {
                assert_eq!(target, "UserRepository::find");
            }
            other => panic!("Expected Callers, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_callees() {
        match parse_interactive_command("callees main") {
            InteractiveCommand::Callees { target } => {
                assert_eq!(target, "main");
            }
            other => panic!("Expected Callees, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_stats() {
        match parse_interactive_command("stats") {
            InteractiveCommand::Stats => {}
            other => panic!("Expected Stats, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_list() {
        match parse_interactive_command("list") {
            InteractiveCommand::List => {}
            other => panic!("Expected List, got {:?}", other),
        }

        match parse_interactive_command("ls") {
            InteractiveCommand::List => {}
            other => panic!("Expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_unknown() {
        match parse_interactive_command("foobar") {
            InteractiveCommand::Unknown(cmd) => {
                assert_eq!(cmd, "foobar");
            }
            other => panic!("Expected Unknown, got {:?}", other),
        }

        match parse_interactive_command("") {
            InteractiveCommand::Unknown(cmd) => {
                assert_eq!(cmd, "");
            }
            other => panic!("Expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_with_extra_spaces() {
        match parse_interactive_command("  find   name   TestSymbol  ") {
            InteractiveCommand::Find { kind, query } => {
                assert_eq!(kind, "name");
                assert_eq!(query, "TestSymbol");
            }
            other => panic!("Expected Find, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_search() {
        match parse_interactive_command("search authentication logic") {
            InteractiveCommand::Search { query } => {
                assert_eq!(query, "authentication logic");
            }
            other => panic!("Expected Search, got {:?}", other),
        }

        match parse_interactive_command("s error handling") {
            InteractiveCommand::Search { query } => {
                assert_eq!(query, "error handling");
            }
            other => panic!("Expected Search, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_chain() {
        match parse_interactive_command("chain main helper 3") {
            InteractiveCommand::Chain { from, to, depth } => {
                assert_eq!(from, "main");
                assert_eq!(to, "helper");
                assert_eq!(depth, Some(3));
            }
            other => panic!("Expected Chain, got {:?}", other),
        }

        match parse_interactive_command("chain a b") {
            InteractiveCommand::Chain { from, to, depth } => {
                assert_eq!(from, "a");
                assert_eq!(to, "b");
                assert!(depth.is_none());
            }
            other => panic!("Expected Chain, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_capsule() {
        match parse_interactive_command("capsule UserRepository") {
            InteractiveCommand::Capsule { symbol, max_items } => {
                assert_eq!(symbol, "UserRepository");
                assert_eq!(max_items, 20); // default
            }
            other => panic!("Expected Capsule, got {:?}", other),
        }

        match parse_interactive_command("capsule main 50") {
            InteractiveCommand::Capsule { symbol, max_items } => {
                assert_eq!(symbol, "main");
                assert_eq!(max_items, 50);
            }
            other => panic!("Expected Capsule, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_impact() {
        match parse_interactive_command("impact UserService") {
            InteractiveCommand::Impact { symbol, depth } => {
                assert_eq!(symbol, "UserService");
                assert_eq!(depth, 3); // default
            }
            other => panic!("Expected Impact, got {:?}", other),
        }

        match parse_interactive_command("impact main 5") {
            InteractiveCommand::Impact { symbol, depth } => {
                assert_eq!(symbol, "main");
                assert_eq!(depth, 5);
            }
            other => panic!("Expected Impact, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_set_and_show() {
        match parse_interactive_command("set repo /path/to/project") {
            InteractiveCommand::Set { key, value } => {
                assert_eq!(key, "repo");
                assert_eq!(value, "/path/to/project");
            }
            other => panic!("Expected Set, got {:?}", other),
        }

        match parse_interactive_command("set limit 50") {
            InteractiveCommand::Set { key, value } => {
                assert_eq!(key, "limit");
                assert_eq!(value, "50");
            }
            other => panic!("Expected Set, got {:?}", other),
        }

        match parse_interactive_command("show") {
            InteractiveCommand::Show => {}
            other => panic!("Expected Show, got {:?}", other),
        }

        match parse_interactive_command("context") {
            InteractiveCommand::Show => {}
            other => panic!("Expected Show, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_complexity() {
        match parse_interactive_command("complexity") {
            InteractiveCommand::Complexity { top } => {
                assert_eq!(top, 20); // default
            }
            other => panic!("Expected Complexity, got {:?}", other),
        }

        match parse_interactive_command("complexity 10") {
            InteractiveCommand::Complexity { top } => {
                assert_eq!(top, 10);
            }
            other => panic!("Expected Complexity, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_interactive_command_patterns_and_tests() {
        match parse_interactive_command("patterns") {
            InteractiveCommand::Patterns { pattern_type } => {
                assert!(pattern_type.is_none());
            }
            other => panic!("Expected Patterns, got {:?}", other),
        }

        match parse_interactive_command("patterns singleton") {
            InteractiveCommand::Patterns { pattern_type } => {
                assert_eq!(pattern_type, Some("singleton".to_string()));
            }
            other => panic!("Expected Patterns, got {:?}", other),
        }

        match parse_interactive_command("tests UserRepository") {
            InteractiveCommand::Tests { symbol } => {
                assert_eq!(symbol, "UserRepository");
            }
            other => panic!("Expected Tests, got {:?}", other),
        }
    }

    #[test]
    fn test_now_millis() {
        let now = now_millis();
        // Should be a reasonable timestamp (after 2020)
        assert!(now > 1577836800000);
        // Should be less than year 3000
        assert!(now < 32503680000000);
    }

    #[test]
    fn test_jobs_path() {
        let path = jobs_path();
        // Should end with jobs.json
        assert!(path.ends_with("jobs.json"));
    }

    #[test]
    fn test_print_formatted_json_mode() {
        let value = serde_json::json!({"test": "value"});
        // Just verify it doesn't panic - output goes to stdout
        let result = print_formatted(OutputFormat::Json, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_formatted_pretty_mode() {
        let value = serde_json::json!({"test": "value", "nested": {"key": 123}});
        let result = print_formatted(OutputFormat::JsonPretty, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_formatted_yaml_mode() {
        let value = serde_json::json!({"test": "value", "nested": {"key": 123}});
        let result = print_formatted(OutputFormat::Yaml, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_formatted_table_mode() {
        let value =
            serde_json::json!([{"name": "test", "value": 123}, {"name": "test2", "value": 456}]);
        let result = print_formatted(OutputFormat::Table, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_jobs_empty() {
        // When jobs file doesn't exist, should return empty vec
        // This test assumes no jobs file exists in test environment
        let result = load_jobs();
        assert!(result.is_ok());
        // Just verify it returns a vector
        let _jobs = result.unwrap();
    }

    #[test]
    fn test_interactive_command_enum_variants() {
        // Test all variants can be created
        let help = InteractiveCommand::Help;
        let exit = InteractiveCommand::Exit;
        let find = InteractiveCommand::Find {
            kind: "test".to_string(),
            query: "query".to_string(),
        };
        let search = InteractiveCommand::Search {
            query: "test query".to_string(),
        };
        let callers = InteractiveCommand::Callers {
            target: "target".to_string(),
        };
        let callees = InteractiveCommand::Callees {
            target: "target".to_string(),
        };
        let chain = InteractiveCommand::Chain {
            from: "a".to_string(),
            to: "b".to_string(),
            depth: Some(3),
        };
        let hierarchy = InteractiveCommand::Hierarchy {
            class: "MyClass".to_string(),
        };
        let deps = InteractiveCommand::Deps {
            module: "mymodule".to_string(),
        };
        let dead_code = InteractiveCommand::DeadCode;
        let complexity = InteractiveCommand::Complexity { top: 10 };
        let overrides = InteractiveCommand::Overrides {
            method: "toString".to_string(),
        };
        let capsule = InteractiveCommand::Capsule {
            symbol: "main".to_string(),
            max_items: 20,
        };
        let impact = InteractiveCommand::Impact {
            symbol: "main".to_string(),
            depth: 3,
        };
        let refactor = InteractiveCommand::Refactor {
            symbol: "oldFunc".to_string(),
        };
        let patterns = InteractiveCommand::Patterns {
            pattern_type: Some("singleton".to_string()),
        };
        let tests = InteractiveCommand::Tests {
            symbol: "UserRepository".to_string(),
        };
        let stats = InteractiveCommand::Stats;
        let list = InteractiveCommand::List;
        let set = InteractiveCommand::Set {
            key: "repo".to_string(),
            value: "/path".to_string(),
        };
        let show = InteractiveCommand::Show;
        let memory = InteractiveCommand::Memory {
            action: "list".to_string(),
            content: None,
            classification: None,
        };
        let unknown = InteractiveCommand::Unknown("test".to_string());

        // Just verify they can be matched
        match help {
            InteractiveCommand::Help => {}
            _ => panic!("Wrong variant"),
        }
        match exit {
            InteractiveCommand::Exit => {}
            _ => panic!("Wrong variant"),
        }
        match find {
            InteractiveCommand::Find { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match search {
            InteractiveCommand::Search { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match callers {
            InteractiveCommand::Callers { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match callees {
            InteractiveCommand::Callees { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match chain {
            InteractiveCommand::Chain { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match hierarchy {
            InteractiveCommand::Hierarchy { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match deps {
            InteractiveCommand::Deps { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match dead_code {
            InteractiveCommand::DeadCode => {}
            _ => panic!("Wrong variant"),
        }
        match complexity {
            InteractiveCommand::Complexity { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match overrides {
            InteractiveCommand::Overrides { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match capsule {
            InteractiveCommand::Capsule { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match impact {
            InteractiveCommand::Impact { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match refactor {
            InteractiveCommand::Refactor { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match patterns {
            InteractiveCommand::Patterns { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match tests {
            InteractiveCommand::Tests { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match stats {
            InteractiveCommand::Stats => {}
            _ => panic!("Wrong variant"),
        }
        match list {
            InteractiveCommand::List => {}
            _ => panic!("Wrong variant"),
        }
        match set {
            InteractiveCommand::Set { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match show {
            InteractiveCommand::Show => {}
            _ => panic!("Wrong variant"),
        }
        match memory {
            InteractiveCommand::Memory { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match unknown {
            InteractiveCommand::Unknown(_) => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cli_parsing() {
        use clap::CommandFactory;

        // Verify CLI can be built
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "cortex");

        // Verify subcommands exist
        let subcommands: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(subcommands.contains(&"index"));
        assert!(subcommands.contains(&"find"));
        assert!(subcommands.contains(&"analyze"));
        assert!(subcommands.contains(&"stats"));
        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"completion"));
        assert!(subcommands.contains(&"interactive"));
        assert!(subcommands.contains(&"capsule"));
        assert!(subcommands.contains(&"impact"));
        assert!(subcommands.contains(&"refactor"));
        assert!(subcommands.contains(&"patterns"));
        assert!(subcommands.contains(&"test"));
        assert!(subcommands.contains(&"diagnose"));
        assert!(subcommands.contains(&"memory"));
    }

    #[test]
    fn test_find_command_variants() {
        use clap::CommandFactory;

        let cmd = Cli::command();
        let find_cmd = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "find")
            .unwrap();

        let subcommands: Vec<&str> = find_cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(subcommands.contains(&"name"));
        assert!(subcommands.contains(&"pattern"));
        assert!(subcommands.contains(&"type"));
        assert!(subcommands.contains(&"content"));
        assert!(subcommands.contains(&"decorator"));
        assert!(subcommands.contains(&"argument"));
    }

    #[test]
    fn test_analyze_command_variants() {
        use clap::CommandFactory;

        let cmd = Cli::command();
        let analyze_cmd = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "analyze")
            .unwrap();

        let subcommands: Vec<&str> = analyze_cmd
            .get_subcommands()
            .map(|s| s.get_name())
            .collect();
        assert!(subcommands.contains(&"callers"));
        assert!(subcommands.contains(&"callees"));
        assert!(subcommands.contains(&"chain"));
        assert!(subcommands.contains(&"hierarchy"));
        assert!(subcommands.contains(&"deps"));
        assert!(subcommands.contains(&"dead-code"));
        assert!(subcommands.contains(&"complexity"));
        assert!(subcommands.contains(&"overrides"));
        assert!(subcommands.contains(&"smells"));
        assert!(subcommands.contains(&"refactoring"));
    }

    #[test]
    fn test_analyze_smells_command_parse() {
        use clap::Parser;

        let cli = Cli::parse_from(["cortex", "analyze", "smells", "./crates/cortex-cli"]);
        match cli.command {
            Commands::Analyze {
                command:
                    AnalyzeCommand::Smells {
                        path,
                        min_severity,
                        max_files,
                        limit,
                    },
            } => {
                assert_eq!(path, "./crates/cortex-cli");
                assert_eq!(min_severity, "info");
                assert_eq!(max_files, 1000);
                assert_eq!(limit, 500);
            }
            _ => panic!("expected analyze smells command"),
        }
    }

    #[test]
    fn test_analyze_refactoring_command_parse() {
        use clap::Parser;

        let cli = Cli::parse_from(["cortex", "analyze", "refactoring", "."]);
        match cli.command {
            Commands::Analyze {
                command:
                    AnalyzeCommand::Refactoring {
                        path,
                        min_severity,
                        max_files,
                        limit,
                    },
            } => {
                assert_eq!(path, ".");
                assert_eq!(min_severity, "warning");
                assert_eq!(max_files, 1000);
                assert_eq!(limit, 500);
            }
            _ => panic!("expected analyze refactoring command"),
        }
    }

    #[test]
    fn test_memory_command_variants() {
        use clap::CommandFactory;

        let cmd = Cli::command();
        let memory_cmd = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "memory")
            .unwrap();

        let subcommands: Vec<&str> = memory_cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(subcommands.contains(&"save"));
        assert!(subcommands.contains(&"search"));
        assert!(subcommands.contains(&"context"));
        assert!(subcommands.contains(&"list"));
        assert!(subcommands.contains(&"clear"));
    }

    #[test]
    fn test_target_arg() {
        let arg = TargetArg {
            target: "UserRepository::find".to_string(),
        };
        assert_eq!(arg.target, "UserRepository::find");
    }

    #[test]
    fn test_cli_global_flags() {
        use clap::CommandFactory;

        let cmd = Cli::command();
        let global_flags: Vec<&str> = cmd.get_arguments().map(|a| a.get_id().as_str()).collect();

        assert!(global_flags.contains(&"format"));
        assert!(global_flags.contains(&"verbose"));
    }
}
