use crate::jobs::{JobInfo, JobRegistry, JobState};
use crate::tools::tool_names;
use cortex_analyzer::Analyzer;
use cortex_core::{CortexConfig, Result, SearchKind};
use cortex_graph::{BundleStore, GraphClient};
use cortex_indexer::Indexer;
use cortex_watcher::WatchSession;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use tokio::runtime::Runtime;

#[derive(Clone)]
#[deprecated(note = "Use rmcp-based handler::start_stdio instead")]
pub struct McpServer {
    pub config: CortexConfig,
    jobs: JobRegistry,
}

impl McpServer {
    pub fn new(config: CortexConfig) -> Self {
        Self {
            config,
            jobs: JobRegistry::default(),
        }
    }

    pub fn start_stdio(&self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let rt = Runtime::new().map_err(|e| cortex_core::CortexError::Runtime(
            format!("Failed to create tokio runtime: {}", e)
        ))?;

        for line in stdin.lock().lines().map_while(|r| r.ok()) {
            let req: Value = serde_json::from_str(&line).unwrap_or_else(|_| json!({}));
            let id = req.get("id").cloned().unwrap_or(json!(null));
            let method = req
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let result = rt.block_on(self.dispatch(&method, params));
            let payload = match result {
                Ok(data) => json!({ "jsonrpc": "2.0", "id": id, "result": data }),
                Err(err) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32000, "message": err.to_string() }
                }),
            };
            writeln!(stdout, "{}", payload).ok();
            stdout.flush().ok();
        }
        Ok(())
    }

    pub async fn dispatch(&self, method: &str, params: Value) -> Result<Value> {
        let client = GraphClient::connect(&self.config).await?;
        let analyzer = Analyzer::new(client.clone());
        let indexer = Indexer::new(client.clone(), self.config.max_batch_size)?;
        let watcher = WatchSession::new(&self.config);

        match method {
            "tools/list" => Ok(json!({ "tools": tool_names() })),
            "add_code_to_graph" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                let report = indexer.index_path(path).await?;
                Ok(json!(report))
            }
            "watch_directory" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                watcher.watch(PathBuf::from(path).as_path())?;
                let mut cfg = self.config.clone();
                watcher.persist_to_config(&mut cfg)?;
                Ok(json!({"status":"watching","path":path}))
            }
            "list_watched_paths" => Ok(json!({ "paths": watcher.list() })),
            "unwatch_directory" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                let removed = watcher.unwatch(PathBuf::from(path).as_path())?;
                Ok(json!({ "removed": removed }))
            }
            "find_code" => {
                let query = params.get("query").and_then(Value::as_str).unwrap_or_default();
                let kind = match params.get("kind").and_then(Value::as_str).unwrap_or("pattern") {
                    "name" => SearchKind::Name,
                    "type" => SearchKind::Type,
                    "content" => SearchKind::Content,
                    _ => SearchKind::Pattern,
                };
                let path_filter = params.get("path").and_then(Value::as_str);
                Ok(json!(analyzer.find_code(query, kind, path_filter).await?))
            }
            "analyze_code_relationships" => {
                let query_type = params
                    .get("query_type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let target = params
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let output = match query_type {
                    "find_callers" => analyzer.callers(target).await?,
                    "find_callees" => analyzer.callees(target).await?,
                    "find_all_callers" => analyzer.all_callers(target).await?,
                    "find_all_callees" => analyzer.all_callees(target).await?,
                    "class_hierarchy" => analyzer.class_hierarchy(target).await?,
                    "dead_code" => analyzer.dead_code().await?,
                    "overrides" => analyzer.overrides(target).await?,
                    "module_deps" => analyzer.module_dependencies(target).await?,
                    "variable_scope" => analyzer.variable_scope(target).await?,
                    _ => Vec::new(),
                };
                Ok(json!(output))
            }
            "execute_cypher_query" => {
                let cypher = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("RETURN 1 AS ok");
                Ok(json!(client.raw_query(cypher).await?))
            }
            "find_dead_code" => Ok(json!(analyzer.dead_code().await?)),
            "calculate_cyclomatic_complexity" | "find_most_complex_functions" => {
                let top_n = params.get("top_n").and_then(Value::as_u64).unwrap_or(20) as usize;
                Ok(json!(analyzer.complexity(top_n).await?))
            }
            "list_indexed_repositories" => Ok(json!(client.list_repositories().await?)),
            "delete_repository" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or_default();
                client.delete_repository(path).await?;
                Ok(json!({"deleted": path}))
            }
            "check_job_status" => {
                let id = params.get("id").and_then(Value::as_str).unwrap_or_default();
                Ok(json!(self.jobs.get(id)))
            }
            "list_jobs" => Ok(json!(self.jobs.list())),
            "load_bundle" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or_default();
                Ok(json!(BundleStore::import(PathBuf::from(path).as_path())?))
            }
            "search_registry_bundles" => Ok(json!({"bundles":[] })),
            "get_repository_stats" => Ok(json!(analyzer.repository_stats().await?)),
            "check_health" => Ok(json!({"status":"ok"})),
            "add_package_to_graph" | "visualize_graph_query" => Ok(json!({"status":"not_implemented"})),
            _ => Ok(json!({"error":"unknown_method"})),
        }
    }

    pub fn register_background_job(&self, id: &str, message: &str) {
        self.jobs.upsert(JobInfo {
            id: id.to_string(),
            state: JobState::Running,
            message: message.to_string(),
        });
    }
}
