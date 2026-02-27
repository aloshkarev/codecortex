use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use cortex_analyzer::Analyzer;
use cortex_core::{CortexConfig, SearchKind};
use cortex_graph::{BundleStore, GraphClient};
use cortex_indexer::Indexer;
use cortex_mcp::tool_names;
use cortex_watcher::WatchSession;
use dialoguer::{Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "cortex", version, about = "CodeCortex CLI toolkit")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose)?;
    let mut config = CortexConfig::load()?;
    let json = cli.json;

    match cli.command {
        Commands::Setup => run_setup(&mut config)?,
        Commands::Doctor => run_doctor(&config).await?,
        Commands::Mcp { command } => run_mcp(&config, command).await?,
        Commands::Index { path, force } => run_index(&config, &path, force, json).await?,
        Commands::Watch { path } => run_watch(&config, &path).await?,
        Commands::Unwatch { path } => run_unwatch(&config, &path)?,
        Commands::Find { command } => run_find(&config, command, json).await?,
        Commands::Analyze { command } => run_analyze(&config, command, json).await?,
        Commands::Bundle { command } => run_bundle(&config, command, json).await?,
        Commands::Config { command } => run_config(&mut config, command, json)?,
        Commands::Clean => run_clean(&config, json).await?,
        Commands::List => run_list(&config, json).await?,
        Commands::Delete { path } => run_delete(&config, &path).await?,
        Commands::Stats => run_stats(&config, json).await?,
        Commands::Query { cypher } => run_query(&config, &cypher, json).await?,
        Commands::Jobs { command } => run_jobs(command, json)?,
        Commands::Debug { command } => run_debug(&config, command, json).await?,
        Commands::Completion { shell } => run_completion(shell),
        Commands::Interactive => run_interactive(&config).await?,
        Commands::Capsule { symbol, max_items } => run_capsule(&config, &symbol, max_items, json).await?,
        Commands::Impact { symbol, depth } => run_impact(&config, &symbol, depth, json).await?,
        Commands::Refactor { symbol } => run_refactor(&config, &symbol, json).await?,
        Commands::Patterns { pattern_type } => run_patterns(&config, pattern_type.as_deref(), json).await?,
        Commands::Test { symbol } => run_find_tests(&config, &symbol, json).await?,
        Commands::Diagnose { component } => run_diagnose(&config, component.as_deref(), json).await?,
        Commands::Memory { command } => run_memory(&config, command, json).await?,
    }
    Ok(())
}

fn run_setup(config: &mut CortexConfig) -> anyhow::Result<()> {
    let uri: String = Input::new()
        .with_prompt("Memgraph URI")
        .default(config.memgraph_uri.clone())
        .interact_text()?;
    let user: String = Input::new()
        .with_prompt("Memgraph user")
        .default(config.memgraph_user.clone())
        .interact_text()?;
    let password: String = Input::new()
        .with_prompt("Memgraph password")
        .default(config.memgraph_password.clone())
        .interact_text()?;
    config.memgraph_uri = uri;
    config.memgraph_user = user;
    config.memgraph_password = password;
    config.save()?;
    if Confirm::new()
        .with_prompt("Generate docker-compose.yml for local Memgraph?")
        .default(false)
        .interact()?
    {
        let compose = r#"services:
  memgraph:
    image: memgraph/memgraph:2.19.0
    ports:
      - "7687:7687"
      - "3000:3000"
    volumes:
      - memgraph_data:/var/lib/memgraph
    command:
      - "--also-log-to-stderr=true"
      - "--query-modules-directory=/usr/lib/memgraph/query_modules"
  memgraph_lab:
    image: memgraph/lab:2.11.1
    ports:
      - "3001:3000"
    depends_on:
      - memgraph
volumes:
  memgraph_data:
"#;
        let cortex_dir = CortexConfig::config_path()
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&cortex_dir)?;
        let compose_path = cortex_dir.join("docker-compose.yml");
        std::fs::write(&compose_path, compose)?;
        println!(
            "{}",
            format!("docker-compose.yml created at {}", compose_path.display()).green()
        );
    }
    if Confirm::new()
        .with_prompt("Generate local mcp.json for Cursor/VSCode?")
        .default(true)
        .interact()?
    {
        let command = std::env::current_exe()
            .ok()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_else(|| "cortex".to_string());
        let mcp = serde_json::json!({
            "mcpServers": {
                "codecortex": {
                    "command": command,
                    "args": ["mcp", "start"],
                    "cwd": std::env::current_dir()?.display().to_string()
                }
            }
        });
        std::fs::write("mcp.json", serde_json::to_string_pretty(&mcp)?)?;
        println!("{}", "mcp.json created".green());
    }
    println!("{}", "Configuration saved".green());
    Ok(())
}

async fn run_doctor(config: &CortexConfig) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let repos = client.list_repositories().await?;
    println!("{}", "Memgraph connection: OK".green());
    println!("Schema bootstrap: {}", "OK (applied on connect)".green());
    println!("Indexed repositories: {}", repos.len());
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
    json: bool,
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
    print_json_or_pretty(json, &serde_json::json!(report))?;
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

async fn run_find(config: &CortexConfig, command: FindCommand, json: bool) -> anyhow::Result<()> {
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
    print_json_or_pretty(json, &serde_json::to_value(out)?)?;
    Ok(())
}

async fn run_analyze(
    config: &CortexConfig,
    command: AnalyzeCommand,
    json: bool,
) -> anyhow::Result<()> {
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
    };
    print_json_or_pretty(json, &serde_json::to_value(out)?)?;
    Ok(())
}

async fn run_list(config: &CortexConfig, json: bool) -> anyhow::Result<()> {
    let repos = GraphClient::connect(config)
        .await?
        .list_repositories()
        .await?;
    print_json_or_pretty(json, &serde_json::to_value(repos)?)?;
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

async fn run_stats(config: &CortexConfig, json: bool) -> anyhow::Result<()> {
    let analyzer = Analyzer::new(GraphClient::connect(config).await?);
    let stats = analyzer.repository_stats().await?;
    print_json_or_pretty(json, &serde_json::to_value(stats)?)?;
    Ok(())
}

async fn run_query(config: &CortexConfig, cypher: &str, json: bool) -> anyhow::Result<()> {
    let rows = GraphClient::connect(config)
        .await?
        .raw_query(cypher)
        .await?;
    print_json_or_pretty(json, &serde_json::to_value(rows)?)?;
    Ok(())
}

async fn run_bundle(
    config: &CortexConfig,
    command: BundleCommand,
    json: bool,
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
            print_json_or_pretty(
                json,
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
            print_json_or_pretty(
                json,
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

fn run_config(config: &mut CortexConfig, command: ConfigCommand, json: bool) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Show => {
            print_json_or_pretty(json, &serde_json::to_value(config.clone())?)?;
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
            print_json_or_pretty(json, &serde_json::json!({"status":"ok"}))?;
        }
        ConfigCommand::Reset => {
            *config = CortexConfig::default();
            config.save()?;
            print_json_or_pretty(json, &serde_json::json!({"status":"ok"}))?;
        }
    }
    Ok(())
}

async fn run_clean(config: &CortexConfig, json: bool) -> anyhow::Result<()> {
    GraphClient::connect(config)
        .await?
        .run("MATCH (n) WHERE NOT (n)--() DELETE n")
        .await?;
    print_json_or_pretty(json, &serde_json::json!({"status":"ok"}))?;
    Ok(())
}

fn run_jobs(command: JobsCommand, json: bool) -> anyhow::Result<()> {
    let jobs = load_jobs()?;
    match command {
        JobsCommand::List => print_json_or_pretty(json, &serde_json::to_value(jobs)?)?,
        JobsCommand::Status { id } => {
            let job = jobs
                .into_iter()
                .find(|job| job.get("id").and_then(|v| v.as_str()) == Some(id.as_str()))
                .unwrap_or_else(|| serde_json::json!({"id": id, "state": "unknown"}));
            print_json_or_pretty(json, &job)?;
        }
    }
    Ok(())
}

async fn run_debug(config: &CortexConfig, command: DebugCommand, json: bool) -> anyhow::Result<()> {
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
                print_json_or_pretty(
                    json,
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

            print_json_or_pretty(json, &output)?;
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

                print_json_or_pretty(
                    json,
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

                print_json_or_pretty(json, &output)?;
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

            print_json_or_pretty(json, &output)?;
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

            print_json_or_pretty(json, &output)?;
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

    loop {
        let readline = rl.readline("cortex> ");
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
                        match analyzer.find_code(&query, search_kind, None).await {
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
                    InteractiveCommand::Stats => {
                        match analyzer.repository_stats().await {
                            Ok(stats) => println!("{}", serde_json::to_string_pretty(&stats)?),
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::List => {
                        match client.list_repositories().await {
                            Ok(repos) => {
                                for repo in repos {
                                    let status = if repo.watched { "watched" } else { "indexed" };
                                    println!("  {} ({})", repo.path.cyan(), status);
                                }
                            }
                            Err(e) => eprintln!("{} {}", "Error:".red(), e),
                        }
                    }
                    InteractiveCommand::Unknown(cmd) => {
                        eprintln!("{} Unknown command: '{}'. Type 'help' for available commands.", "Error:".red(), cmd);
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

#[derive(Debug)]
enum InteractiveCommand {
    Help,
    Exit,
    Find { kind: String, query: String },
    Callers { target: String },
    Callees { target: String },
    Stats,
    List,
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
        "find" | "f" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("find requires arguments".to_string());
            }
            let kind = parts.get(1).unwrap_or(&"name").to_string();
            let query = if parts.len() > 2 && matches!(kind.as_str(), "name" | "pattern" | "pat" | "type" | "t" | "content" | "c") {
                parts[2..].join(" ")
            } else {
                parts[1..].join(" ")
            };
            InteractiveCommand::Find { kind, query }
        }
        "callers" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("callers requires a target".to_string());
            }
            InteractiveCommand::Callers { target: parts[1..].join(" ") }
        }
        "callees" => {
            if parts.len() < 2 {
                return InteractiveCommand::Unknown("callees requires a target".to_string());
            }
            InteractiveCommand::Callees { target: parts[1..].join(" ") }
        }
        "stats" => InteractiveCommand::Stats,
        "list" | "ls" => InteractiveCommand::List,
        cmd => InteractiveCommand::Unknown(cmd.to_string()),
    }
}

fn print_interactive_help() {
    println!("{}", "Available Commands:".green().bold());
    println!();
    println!("  {} <query>        Find symbols by name", "find".cyan());
    println!("  {} pattern <pat>  Find symbols by regex pattern", "find".cyan());
    println!("  {} type <kind>    Find symbols by type (Function, Class, etc.)", "find".cyan());
    println!("  {} <symbol>       Find callers of a symbol", "callers".cyan());
    println!("  {} <symbol>       Find callees of a symbol", "callees".cyan());
    println!("  {}                   Show repository statistics", "stats".cyan());
    println!("  {}                   List indexed repositories", "list".cyan());
    println!("  {}                   Show this help message", "help".cyan());
    println!("  {}                   Exit interactive mode", "exit".cyan());
    println!();
    println!("{}", "Shortcuts:".yellow());
    println!("  f = find, ls = list, h = help, q = exit");
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
        println!("  {} {} more results...", "...".dimmed(), results.len() - 20);
    }
}

fn print_json_or_pretty(json: bool, value: &serde_json::Value) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
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
    json: bool,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find symbols matching the query
    let raw_results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if raw_results.is_empty() {
        print_json_or_pretty(
            json,
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
                source: r.get("source").and_then(|s| s.as_str()).map(|s| s.to_string()),
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

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_impact(
    config: &CortexConfig,
    symbol: &str,
    depth: usize,
    json: bool,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find the symbol first
    let results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if results.is_empty() {
        print_json_or_pretty(
            json,
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

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_refactor(
    config: &CortexConfig,
    symbol: &str,
    json: bool,
) -> anyhow::Result<()> {
    let client = GraphClient::connect(config).await?;
    let analyzer = Analyzer::new(client.clone());

    // Find the symbol
    let results = analyzer.find_code(symbol, SearchKind::Name, None).await?;

    if results.is_empty() {
        print_json_or_pretty(
            json,
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

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_patterns(
    config: &CortexConfig,
    pattern_type: Option<&str>,
    json: bool,
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

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_find_tests(
    config: &CortexConfig,
    symbol: &str,
    json: bool,
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

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_diagnose(
    config: &CortexConfig,
    component: Option<&str>,
    json: bool,
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

    let overall_status = if checks.iter().any(|c| c.get("status").and_then(|v| v.as_str()) == Some("error")) {
        "error"
    } else if checks.iter().any(|c| c.get("status").and_then(|v| v.as_str()) == Some("warning")) {
        "warning"
    } else {
        "ok"
    };

    let output = serde_json::json!({
        "overall_status": overall_status,
        "checks": checks,
    });

    print_json_or_pretty(json, &output)?;
    Ok(())
}

async fn run_memory(
    _config: &CortexConfig,
    command: MemoryCommand,
    json: bool,
) -> anyhow::Result<()> {
    let store = cortex_mcp::MemoryStore::open()?;

    match command {
        MemoryCommand::Save { content, classification, severity } => {
            let class: cortex_mcp::Classification = classification.parse().unwrap_or(cortex_mcp::Classification::Note);
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
                created_by: "cli".to_string(),
                text: content,
                symbol_refs: vec![],
                confidence: 1.0,
                stale: false,
                classification: class,
                severity: sev,
                tags: vec![],
                source_revision: String::new(),
            };

            store.save(&obs)?;

            print_json_or_pretty(
                json,
                &serde_json::json!({
                    "status": "saved",
                    "id": obs.observation_id,
                }),
            )?;
        }
        MemoryCommand::Search { query, limit } => {
            let results = store.search(&query, None, None, false, limit)?;

            print_json_or_pretty(
                json,
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

            print_json_or_pretty(
                json,
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

            print_json_or_pretty(
                json,
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
            print_json_or_pretty(
                json,
                &serde_json::json!({
                    "status": "unsupported",
                    "message": "Clear operation not supported. Delete the memory database file manually.",
                }),
            )?;
        }
    }

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
    fn test_print_json_or_pretty_json_mode() {
        let value = serde_json::json!({"test": "value"});
        // Just verify it doesn't panic - output goes to stdout
        let result = print_json_or_pretty(true, &value);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_json_or_pretty_pretty_mode() {
        let value = serde_json::json!({"test": "value", "nested": {"key": 123}});
        let result = print_json_or_pretty(false, &value);
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
        let callers = InteractiveCommand::Callers {
            target: "target".to_string(),
        };
        let callees = InteractiveCommand::Callees {
            target: "target".to_string(),
        };
        let stats = InteractiveCommand::Stats;
        let list = InteractiveCommand::List;
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
        match callers {
            InteractiveCommand::Callers { .. } => {}
            _ => panic!("Wrong variant"),
        }
        match callees {
            InteractiveCommand::Callees { .. } => {}
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

        let subcommands: Vec<&str> = analyze_cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(subcommands.contains(&"callers"));
        assert!(subcommands.contains(&"callees"));
        assert!(subcommands.contains(&"chain"));
        assert!(subcommands.contains(&"hierarchy"));
        assert!(subcommands.contains(&"deps"));
        assert!(subcommands.contains(&"dead-code"));
        assert!(subcommands.contains(&"complexity"));
        assert!(subcommands.contains(&"overrides"));
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
        let global_flags: Vec<&str> = cmd
            .get_arguments()
            .map(|a| a.get_id().as_str())
            .collect();

        assert!(global_flags.contains(&"json"));
        assert!(global_flags.contains(&"verbose"));
    }
}
