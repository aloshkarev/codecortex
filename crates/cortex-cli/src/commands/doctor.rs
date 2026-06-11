//! `cortex doctor` — FalkorDB, vector tier, and embedder health checks.

use cortex_core::{
    CortexConfig, IndexingProfile, default_write_pool_size, indexing_settings, validate_falkordb_uri,
};
use cortex_graph::GraphClient;
use cortex_mcp::tool_names;
use cortex_vector::{JsonStore, LanceStore, VectorStore};
use owo_colors::OwoColorize;

pub async fn run_doctor(config: &CortexConfig) -> anyhow::Result<()> {
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

    fn parse_falkordb_uri(uri: &str, default_port: u16) -> Option<(&str, u16)> {
        let trimmed = uri.trim();
        let authority = trimmed
            .strip_prefix("falkor://")
            .or_else(|| trimmed.strip_prefix("falkors://"))
            .or_else(|| trimmed.strip_prefix("redis://"))
            .or_else(|| trimmed.strip_prefix("rediss://"))
            .unwrap_or(trimmed)
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
                .unwrap_or(default_port);
            return Some((host, port));
        }

        if let Some((host, port_str)) = authority.rsplit_once(':') {
            if !host.contains(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    return Some((host, port));
                }
            }
        }

        Some((authority, default_port))
    }

    println!("{}", "1. Configuration".cyan().bold());
    println!("   Config file: {}", CortexConfig::config_path().display());
    let profile = config.resolved_indexing_profile();
    let profile_label = match profile {
        IndexingProfile::Highspeed => "highspeed",
        IndexingProfile::Conservative => "conservative",
    };
    let effective = indexing_settings(profile);
    println!(
        "   Indexing profile: {} (CPUs: {}, write pool: {}, parse pipeline: {}, batch: {})",
        profile_label,
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        config.falkordb_write_pool_size,
        config.indexer_parse_pipeline_depth,
        config.indexer_parse_batch_size,
    );
    if profile == IndexingProfile::Conservative {
        println!(
            "   {} Default is highspeed; set indexing_profile = \"highspeed\" or omit for throughput tuning",
            "ℹ".blue()
        );
    }
    let _ = effective;
    let _ = default_write_pool_size();
    println!(
        "   Perf triage: cortex index-report analyze --file report.json (use CORTEX_INDEX_PROFILE=1 when indexing)"
    );

    match config.validate() {
        Ok(()) => {
            println!("   {}", "✓ Configuration valid".green());
        }
        Err(e) => {
            println!("   {} Configuration error: {}", "✗".red(), e);
            all_healthy = false;
        }
    }

    if config.falkordb_uri.is_empty() {
        println!("   {} FalkorDB URI not configured", "⚠".yellow());
        warnings.push("FalkorDB URI is empty".to_string());
    } else {
        println!("   FalkorDB URI: {}", config.falkordb_uri);
        println!("   FalkorDB graph: {}", config.falkordb_graph);
        if let Err(e) = validate_falkordb_uri(&config.falkordb_uri) {
            println!("   {} Invalid FalkorDB URI: {}", "✗".red(), e);
            all_healthy = false;
        }
    }

    if config.llm.provider == "openai" && config.llm.openai_api_key.is_none() {
        println!(
            "   {} OpenAI provider selected but no API key configured",
            "⚠".yellow()
        );
        warnings.push("OpenAI API key missing".to_string());
    }

    println!();

    println!("{}", "2. Graph Database (FalkorDB)".cyan().bold());
    println!("   Backend: FalkorDB");

    let mut can_connect = false;
    println!("   URI: {}", config.falkordb_uri);

    let default_port = cortex_graph::detect_backend_from_config(config).default_port();
    let (db_host, db_port) =
        parse_falkordb_uri(&config.falkordb_uri, default_port).unwrap_or(("127.0.0.1", default_port));

    if !check_port_reachable(db_host, db_port) {
        println!(
            "   {} Port {} is not reachable on {}",
            "✗".red(),
            db_port,
            db_host
        );
        println!("   {} Database is not running", "✗".red());
        println!("      Start with: docker start codecortex-falkordb");
        println!(
            "      Or run: docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest"
        );
        all_healthy = false;
    } else {
        println!("   {} Port {} is open on {}", "✓".green(), db_port, db_host);
        can_connect = true;
    }

    if can_connect {
        match GraphClient::connect(config).await {
            Ok(client) => {
                println!("   {} Connection established", "✓".green());

                match client.list_repositories().await {
                    Ok(repos) => {
                        println!("   {} Indexed repositories: {}", "✓".green(), repos.len());
                    }
                    Err(e) => {
                        println!("   {} Failed to list repositories: {}", "⚠".yellow(), e);
                        warnings.push("Could not list repositories".to_string());
                    }
                }

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
                println!("      Port is open but FalkorDB protocol handshake failed");
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

            if config.vector.store_path.exists() {
                println!("   {} Storage directory exists", "✓".green());

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

            if config.vector.store_path.exists() {
                println!("   {} Storage directory exists", "✓".green());

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
    println!(
        "   Hybrid fusion: {} | Rerank: {} | Embedder fallback: {}",
        config.vector.hybrid_fusion,
        config.vector.rerank_enabled,
        config.vector.embedding_fallback
    );
    if config.clone_detection_enabled {
        println!("   {} Clone detection: enabled (SIMILAR_TO edges at index time)", "✓".green());
    } else {
        println!(
            "   {} Clone detection: disabled (set clone_detection_enabled = true)",
            "○".yellow()
        );
    }

    println!();

    // 4. Check LLM/Embedding Provider
    println!("{}", "4. LLM/Embedding Provider".cyan().bold());
    println!("   Provider: {}", config.llm.provider);

    match config.llm.provider.as_str() {
        "ollama" => {
            println!("   Base URL: {}", config.llm.ollama_base_url);
            println!("   Model: {}", config.llm.ollama_embedding_model);

            let authority = config
                .llm
                .ollama_base_url
                .replace("http://", "")
                .replace("https://", "")
                .trim_end_matches('/')
                .to_string();
            let (host, port) = authority
                .rsplit_once(':')
                .and_then(|(h, p)| p.parse::<u16>().ok().map(|port| (h.to_string(), port)))
                .unwrap_or_else(|| (authority, 11434));
            let default_addr: std::net::SocketAddr = "127.0.0.1:11434"
                .parse()
                .expect("valid default Ollama address");
            let addr: std::net::SocketAddr =
                format!("{}:{}", host, port).parse().unwrap_or(default_addr);

            match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5)) {
                Ok(_) => {
                    println!("   {} Ollama server reachable", "✓".green());
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

    println!("{}", "A2A (agent-to-agent)".bold());
    if config.a2a.enabled {
        println!("   {} enabled", "✓".green());
        let registry = cortex_a2a::RoleManifestRegistry::load(&config.a2a, None);
        for w in registry.warnings() {
            warnings.push(format!("A2A manifest: {w}"));
        }
        if config.a2a.server.http_enabled {
            println!("   HTTP+JSON: {}", config.a2a.server.base_path);
        }
        if config.a2a.server.grpc_enabled {
            println!("   gRPC: {}", config.a2a.server.grpc_listen);
        }
        if config.a2a.push.enabled {
            println!("   Push webhooks: enabled");
        } else {
            println!("   Push webhooks: deferred (set [a2a.push].enabled = true)");
        }
        if config.a2a.blackboard.enabled {
            match GraphClient::connect(config).await {
                Ok(client) => {
                    let writer = cortex_graph::BlackboardWriter::new(
                        client,
                        config.a2a.blackboard_write_batch_size(64),
                    );
                    if let Err(e) = writer.ensure_schema().await {
                        warnings.push(format!("A2A blackboard schema: {e}"));
                    } else {
                        println!("   Blackboard schema: ok");
                    }
                }
                Err(e) => {
                    warnings.push(format!("A2A blackboard needs graph: {e}"));
                }
            }
        }
        for (name, role) in &config.a2a.roles {
            if role.mode == cortex_core::A2aRoleMode::External {
                if role.agent_card_url.is_none() {
                    warnings.push(format!(
                        "A2A role {name}: external mode without agent_card_url"
                    ));
                }
            }
        }
    } else {
        println!("   {} disabled ([a2a].enabled = false)", "○".yellow());
    }
    println!();

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
