//! Comprehensive setup wizard for CodeCortex
//!
//! This module provides an interactive setup wizard that guides users through:
//! - Dependency checking (Docker, Rust)
//! - Port conflict detection and resolution
//! - FalkorDB setup (Docker or existing URI)
//! - Vector store configuration
//! - LLM provider configuration
//! - Verification and testing

use anyhow::{Context, Result};
use cortex_core::CortexConfig;
use dialoguer::{Confirm, Input, Select};
use owo_colors::OwoColorize;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::Command;

/// Check if a port is available (not in use)
fn is_port_available(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    addr.to_socket_addrs()
        .map(|mut addrs| {
            addrs.all(|addr| {
                TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(1)).is_err()
            })
        })
        .unwrap_or(true)
}

/// Find an available port starting from the given port
fn find_available_port(host: &str, start_port: u16, max_attempts: u16) -> Option<u16> {
    for port in start_port..start_port + max_attempts {
        if is_port_available(host, port) {
            return Some(port);
        }
    }
    None
}

/// Check if a command exists
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if Docker is running
fn docker_is_running() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Start FalkorDB Docker container (Redis protocol on port 6379).
fn start_falkordb_docker(port: u16) -> Result<()> {
    println!("{}", "Starting FalkorDB container...".cyan());

    let exists = Command::new("docker")
        .args(["ps", "-a", "-q", "-f", "name=codecortex-falkordb"])
        .output()
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);

    if exists {
        println!("{}", "Found existing container, starting it...".yellow());
        Command::new("docker")
            .args(["start", "codecortex-falkordb"])
            .status()
            .context("Failed to start existing FalkorDB container")?;
    } else {
        Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                "codecortex-falkordb",
                "-p",
                &format!("{}:6379", port),
                "falkordb/falkordb:latest",
            ])
            .status()
            .context("Failed to start FalkorDB container")?;
    }

    println!("{} FalkorDB started on port {}", "✓".green(), port);
    Ok(())
}

fn ollama_socket_addrs(base_url: &str) -> Option<Vec<std::net::SocketAddr>> {
    let target = base_url
        .trim()
        .strip_prefix("http://")
        .or_else(|| base_url.trim().strip_prefix("https://"))
        .unwrap_or(base_url.trim())
        .trim_end_matches('/');

    target.to_socket_addrs().ok().map(|addrs| addrs.collect())
}

/// Check if Ollama is running by checking the configured base URL
fn ollama_is_running(base_url: &str) -> bool {
    ollama_socket_addrs(base_url).is_some_and(|addrs| {
        addrs.into_iter().any(|addr| {
            TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(2)).is_ok()
        })
    })
}

/// Pull Ollama embedding model
fn pull_ollama_model(model: &str) -> Result<()> {
    println!("{} Pulling Ollama model: {}", "→".cyan(), model);

    // Use ollama CLI to pull the model
    let status = Command::new("ollama")
        .args(["pull", model])
        .status()
        .context("Failed to run ollama pull")?;

    if status.success() {
        println!("{} Model {} pulled successfully", "✓".green(), model);
        Ok(())
    } else {
        anyhow::bail!("Failed to pull model with ollama CLI")
    }
}

/// Test FalkorDB TCP reachability from URI host:port.
fn test_falkordb_connection(uri: &str) -> Result<bool> {
    println!("{} Testing FalkorDB connection...", "→".cyan());

    let authority = uri
        .trim()
        .strip_prefix("falkor://")
        .or_else(|| uri.trim().strip_prefix("redis://"))
        .or_else(|| uri.trim().strip_prefix("rediss://"))
        .unwrap_or(uri.trim())
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("");

    let host_port = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:6379")
    };

    if let Ok(addrs) = host_port.to_socket_addrs() {
        for addr in addrs {
            if TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5)).is_ok() {
                println!("{} Port is reachable", "✓".green());
                return Ok(true);
            }
        }
    }

    println!("{} Could not connect to FalkorDB", "✗".red());
    Ok(false)
}

/// Format dependency status
fn format_status(installed: bool, running: bool, optional: bool) -> String {
    if running {
        "✓ installed and running".green().to_string()
    } else if installed {
        "⚠ installed but not running".yellow().to_string()
    } else if optional {
        "✗ not installed (optional)".dimmed().to_string()
    } else {
        "✗ not installed".red().to_string()
    }
}

/// Main setup wizard
pub fn run_setup_wizard(config: &mut CortexConfig) -> Result<()> {
    println!(
        "{}",
        "╔═══════════════════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║         CodeCortex Setup Wizard                               ║".cyan()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════════════════╝".cyan()
    );
    println!();

    // Step 1: Check dependencies
    println!("{}", "Step 1: Checking Dependencies".cyan().bold());
    println!();

    let has_docker = command_exists("docker");
    let docker_running = has_docker && docker_is_running();
    let has_ollama = command_exists("ollama");
    let ollama_running = has_ollama && ollama_is_running(&config.llm.ollama_base_url);

    println!(
        "  Docker: {}",
        format_status(has_docker, docker_running, false)
    );
    println!(
        "  Ollama: {}",
        format_status(has_ollama, ollama_running, true)
    );

    println!();

    // Step 2: Configure graph backend
    println!("{}", "Step 2: Configure Graph Database".cyan().bold());
    println!();

    let graph_options = if docker_running {
        vec![
            "FalkorDB via Docker (recommended)",
            "Connect to existing FalkorDB URI",
            "Skip (configure manually later)",
        ]
    } else {
        vec![
            "Connect to existing FalkorDB URI",
            "Skip (configure manually later)",
        ]
    };

    let graph_choice = Select::new()
        .with_prompt("How do you want to run the graph database?")
        .items(&graph_options)
        .default(0)
        .interact()?;

    let choice_label = graph_options[graph_choice];
    match choice_label {
        "FalkorDB via Docker (recommended)" => {
            let default_port = 6379u16;
            let port = if !is_port_available("127.0.0.1", default_port) {
                println!("  {} Port 6379 is in use", "⚠".yellow());
                find_available_port("127.0.0.1", 6380, 10).unwrap_or(default_port)
            } else {
                default_port
            };
            start_falkordb_docker(port)?;
            config.falkordb_uri = format!("falkor://127.0.0.1:{}", port);
            config.falkordb_password.clear();
            config.falkordb_graph = "codecortex".to_string();
        }
        "Connect to existing FalkorDB URI" => {
            let uri: String = Input::new()
                .with_prompt("FalkorDB URI (falkor://, redis://, or rediss://)")
                .default("falkor://127.0.0.1:6379".to_string())
                .interact_text()?;
            let password: String = Input::new()
                .with_prompt("Redis password (empty if none)")
                .allow_empty(true)
                .interact_text()?;
            let graph: String = Input::new()
                .with_prompt("Graph name")
                .default(config.falkordb_graph.clone())
                .interact_text()?;
            config.falkordb_uri = uri;
            config.falkordb_password = password;
            config.falkordb_graph = graph;
        }
        _ => {
            println!("  Skipping database configuration");
        }
    }

    if !config.falkordb_uri.is_empty() {
        let _ = test_falkordb_connection(&config.falkordb_uri);
    }

    println!();

    // Step 3: Configure Vector Store
    println!("{}", "Step 3: Configure Vector Store".cyan().bold());
    println!();

    let vector_options = vec![
        "LanceDB (embedded, recommended)",
        "JSON (simple, for development)",
        "Qdrant (cloud/self-hosted, for production)",
        "Disable vector search",
    ];

    let vector_choice = Select::new()
        .with_prompt("Select vector store")
        .items(&vector_options)
        .default(0)
        .interact()?;

    match vector_choice {
        0 => {
            config.vector.store_type = "lancedb".to_string();
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let default_path = PathBuf::from(&home).join(".cortex/vectors");

            let path: String = Input::new()
                .with_prompt("Vector store path")
                .default(default_path.to_string_lossy().to_string())
                .interact_text()?;

            config.vector.store_path = PathBuf::from(path);
            println!(
                "{} LanceDB configured at {}",
                "✓".green(),
                config.vector.store_path.display()
            );
        }
        1 => {
            config.vector.store_type = "json".to_string();
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            let default_path = PathBuf::from(&home).join(".cortex/vectors");

            let path: String = Input::new()
                .with_prompt("Vector store path")
                .default(default_path.to_string_lossy().to_string())
                .interact_text()?;

            config.vector.store_path = PathBuf::from(path);
            println!(
                "{} Vector store configured at {}",
                "✓".green(),
                config.vector.store_path.display()
            );
        }
        2 => {
            config.vector.store_type = "qdrant".to_string();
            let uri: String = Input::new()
                .with_prompt("Qdrant URI")
                .default(config.vector.qdrant_uri.clone())
                .interact_text()?;

            let api_key: String = Input::new()
                .with_prompt("Qdrant API Key (leave empty if not required)")
                .allow_empty(true)
                .interact_text()?;

            config.vector.qdrant_uri = uri;
            config.vector.qdrant_api_key = if api_key.is_empty() {
                None
            } else {
                Some(api_key)
            };
            println!("{} Qdrant configured", "✓".green());
        }
        _ => {
            config.vector.store_type = "none".to_string();
            println!("{} Vector search disabled", "⚠".yellow());
        }
    }

    println!();

    // Step 4: Configure LLM Provider
    println!(
        "{}",
        "Step 4: Configure LLM/Embedding Provider".cyan().bold()
    );
    println!();

    let llm_options = vec![
        "Ollama (local, free, recommended)",
        "OpenAI (cloud, requires API key)",
        "Disable LLM features",
    ];

    let llm_choice = Select::new()
        .with_prompt("Select embedding provider")
        .items(&llm_options)
        .default(0)
        .interact()?;

    match llm_choice {
        0 => {
            config.llm.provider = "ollama".to_string();

            let base_url: String = Input::new()
                .with_prompt("Ollama base URL")
                .default(config.llm.ollama_base_url.clone())
                .interact_text()?;

            config.llm.ollama_base_url = base_url.clone();

            // Check if Ollama is running
            if ollama_is_running(&base_url) {
                println!("{} Ollama is running", "✓".green());

                let model: String = Input::new()
                    .with_prompt("Embedding model")
                    .default(config.llm.ollama_embedding_model.clone())
                    .interact_text()?;

                config.llm.ollama_embedding_model = model.clone();

                // Offer to pull the model
                let pull_model = Confirm::new()
                    .with_prompt(&format!("Pull model '{}' if not present?", model))
                    .default(true)
                    .interact()?;

                if pull_model {
                    let _ = pull_ollama_model(&model);
                }
            } else {
                println!("{} Ollama is not running at {}", "⚠".yellow(), base_url);
                println!("  Start Ollama with: ollama serve");

                let continue_anyway = Confirm::new()
                    .with_prompt("Continue anyway?")
                    .default(true)
                    .interact()?;

                if !continue_anyway {
                    config.llm.provider = "none".to_string();
                }
            }
        }
        1 => {
            config.llm.provider = "openai".to_string();

            println!("  Get your API key from: https://platform.openai.com/api-keys");

            let api_key: String = Input::new().with_prompt("OpenAI API Key").interact_text()?;

            config.llm.openai_api_key = Some(api_key);

            let model: String = Input::new()
                .with_prompt("Embedding model")
                .default(config.llm.openai_embedding_model.clone())
                .interact_text()?;

            config.llm.openai_embedding_model = model;
            config.vector.embedding_dim = 1536; // OpenAI dimension

            println!("{} OpenAI configured", "✓".green());
        }
        _ => {
            config.llm.provider = "none".to_string();
            println!("{} LLM features disabled", "⚠".yellow());
        }
    }

    println!();

    // Step 5: Generate configuration files
    println!("{}", "Step 5: Generate Configuration Files".cyan().bold());
    println!();

    // Save main config
    config.save()?;
    println!(
        "{} Configuration saved to ~/.cortex/config.toml",
        "✓".green()
    );

    // Generate docker-compose.yml if using Docker
    if docker_running
        && Confirm::new()
            .with_prompt("Generate docker-compose.yml?")
            .default(true)
            .interact()?
    {
        let port: u16 = config
            .falkordb_uri
            .rsplit(':')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(6379);

        let compose = format!(
            r#"services:
  falkordb:
    image: falkordb/falkordb:latest
    container_name: codecortex-falkordb
    ports:
      - "{}:6379"

"#,
            port
        );

        std::fs::write("docker-compose.yml", compose)?;
        println!("{} docker-compose.yml created", "✓".green());
    }

    // Generate MCP config
    if Confirm::new()
        .with_prompt("Generate mcp.json for Cursor/VSCode?")
        .default(true)
        .interact()?
    {
        let command = std::env::current_exe()
            .ok()
            .and_then(|p| p.into_os_string().into_string().ok())
            .unwrap_or_else(|| "cortex".to_string());

        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());

        let mcp = serde_json::json!({
            "mcpServers": {
                "codecortex": {
                    "command": command,
                    "args": ["mcp", "start"],
                    "cwd": cwd
                }
            }
        });

        std::fs::write("mcp.json", serde_json::to_string_pretty(&mcp)?)?;
        println!("{} mcp.json created", "✓".green());
    }

    // Generate .env file
    if Confirm::new()
        .with_prompt("Generate .env file with configuration?")
        .default(true)
        .interact()?
    {
        let mut env_content = format!(
            r#"# CodeCortex Configuration
# Generated by setup wizard

# Graph Database (FalkorDB)
CORTEX_FALKORDB_URI={}
CORTEX_FALKORDB_PASSWORD={}

# Vector Store
"#,
            config.falkordb_uri, config.falkordb_password
        );

        if config.vector.store_type == "lancedb" || config.vector.store_type == "json" {
            env_content.push_str(&format!(
                "# Vector store path: {}\n",
                config.vector.store_path.display()
            ));
        } else if config.vector.store_type == "qdrant" {
            env_content.push_str(&format!("# Qdrant URI: {}\n", config.vector.qdrant_uri));
        }

        if config.llm.provider == "openai" {
            if let Some(ref key) = config.llm.openai_api_key {
                env_content.push_str(&format!("\n# OpenAI\nOPENAI_API_KEY={}\n", key));
            }
        }

        std::fs::write(".env.cortex", env_content)?;
        println!("{} .env.cortex created", "✓".green());
    }

    println!();

    // Summary
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".cyan()
    );
    println!("{}", "Setup Complete!".green().bold());
    println!(
        "{}",
        "═══════════════════════════════════════════════════════════════".cyan()
    );
    println!();
    println!("Configuration Summary:");
    println!("  Database:     {}", config.falkordb_uri);
    println!("  Vector Store: {}", config.vector.store_type);
    println!("  LLM Provider: {}", config.llm.provider);
    println!();
    println!("Next Steps:");
    println!("  1. Run 'cortex doctor' to verify your setup");
    println!("  2. Run 'cortex index /path/to/code' to index a repository");
    println!("  3. Run 'cortex mcp start' to start the MCP server");
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_socket_addrs_parses_http_url() {
        let addrs = ollama_socket_addrs("http://127.0.0.1:11434").unwrap();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().any(|addr| addr.port() == 11434));
    }

    #[test]
    fn ollama_socket_addrs_parses_bare_host_port() {
        let addrs = ollama_socket_addrs("localhost:11434").unwrap();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().any(|addr| addr.port() == 11434));
    }

    #[test]
    fn ollama_socket_addrs_rejects_invalid_url() {
        assert!(ollama_socket_addrs("http:///missing-host").is_none());
    }
}
