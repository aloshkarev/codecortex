use crate::jobs::{JobInfo, JobRegistry, JobState};
use crate::tool_names;
use cortex_analyzer::{AnalyzePathFilters, Analyzer};
use cortex_core::{CortexConfig, Result, SearchKind};
use cortex_graph::{BundleStore, GraphClient};
use cortex_indexer::Indexer;
use cortex_watcher::WatchSession;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use tokio::runtime::Runtime;

fn parse_string_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => vec![s.to_string()],
        _ => Vec::new(),
    }
}

fn parse_analyze_filters(params: &Value) -> Result<AnalyzePathFilters> {
    let filters = AnalyzePathFilters {
        include_paths: parse_string_list(params.get("include_paths")),
        include_files: parse_string_list(params.get("include_files")),
        include_globs: parse_string_list(params.get("include_globs")),
        exclude_paths: parse_string_list(params.get("exclude_paths")),
        exclude_files: parse_string_list(params.get("exclude_files")),
        exclude_globs: parse_string_list(params.get("exclude_globs")),
    };
    filters.validate()?;
    Ok(filters)
}

fn analyzer_capabilities_json() -> Value {
    json!({
        "path_filters": {
            "supported": true,
            "fields": [
                "include_paths",
                "include_files",
                "include_globs",
                "exclude_paths",
                "exclude_files",
                "exclude_globs"
            ]
        },
        "language_aware_smells": {
            "supported": true,
            "extensions": [
                "rs","py","rb","js","jsx","ts","tsx","go","java","c","cc","cpp","h","hpp",
                "cs","php","swift","kt","kts","m","mm","scala"
            ]
        }
    })
}

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
        let rt = Runtime::new().map_err(|e| {
            cortex_core::CortexError::Runtime(format!("Failed to create tokio runtime: {}", e))
        })?;

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

    fn current_watch_config(&self) -> CortexConfig {
        CortexConfig::load().unwrap_or_else(|_| self.config.clone())
    }

    pub async fn dispatch(&self, method: &str, params: Value) -> Result<Value> {
        match method {
            "tools/list" => Ok(json!({ "tools": tool_names() })),
            "add_code_to_graph" => {
                let client = GraphClient::connect(&self.config).await?;
                let indexer = Indexer::new(client, self.config.max_batch_size)?;
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                let report = indexer.index_path(path).await?;
                Ok(json!(report))
            }
            "watch_directory" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                let mut cfg = self.current_watch_config();
                let watcher = WatchSession::new(&cfg);
                watcher.watch(PathBuf::from(path).as_path())?;
                watcher.persist_to_config(&mut cfg)?;
                Ok(json!({"status":"watching","path":path}))
            }
            "list_watched_paths" => {
                let cfg = self.current_watch_config();
                let watcher = WatchSession::new(&cfg);
                Ok(json!({ "paths": watcher.list() }))
            }
            "unwatch_directory" => {
                let path = params.get("path").and_then(Value::as_str).unwrap_or(".");
                let mut cfg = self.current_watch_config();
                let watcher = WatchSession::new(&cfg);
                let removed = watcher.unwatch(PathBuf::from(path).as_path())?;
                watcher.persist_to_config(&mut cfg)?;
                Ok(json!({ "removed": removed }))
            }
            "find_code" => {
                let client = GraphClient::connect(&self.config).await?;
                let analyzer = Analyzer::new(client);
                let query = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let kind = match params
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("pattern")
                {
                    "name" => SearchKind::Name,
                    "type" => SearchKind::Type,
                    "content" => SearchKind::Content,
                    _ => SearchKind::Pattern,
                };
                let path_filter = params
                    .get("path_filter")
                    .and_then(Value::as_str)
                    .or_else(|| params.get("path").and_then(Value::as_str));
                Ok(json!(analyzer.find_code(query, kind, path_filter).await?))
            }
            "analyze_code_relationships" => {
                let client = GraphClient::connect(&self.config).await?;
                let analyzer = Analyzer::new(client);
                let filters = parse_analyze_filters(&params)?;
                let query_type = params
                    .get("query_type")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let target = params
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let target2 = params
                    .get("target2")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let depth = params
                    .get("depth")
                    .and_then(Value::as_u64)
                    .map(|d| d as usize);
                let output = match query_type {
                    "find_callers" => {
                        analyzer
                            .callers_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_callees" => {
                        analyzer
                            .callees_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_all_callers" => {
                        analyzer
                            .all_callers_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_all_callees" => {
                        analyzer
                            .all_callees_with_filters(target, Some(&filters))
                            .await?
                    }
                    "call_chain" => {
                        analyzer
                            .call_chain_with_filters(target, target2, depth, Some(&filters))
                            .await?
                    }
                    "class_hierarchy" => {
                        analyzer
                            .class_hierarchy_with_filters(target, Some(&filters))
                            .await?
                    }
                    "dead_code" => analyzer.dead_code_with_filters(Some(&filters)).await?,
                    "overrides" => {
                        analyzer
                            .overrides_with_filters(target, Some(&filters))
                            .await?
                    }
                    "module_deps" => {
                        analyzer
                            .module_dependencies_with_filters(target, Some(&filters))
                            .await?
                    }
                    "variable_scope" => {
                        analyzer
                            .variable_scope_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_importers" => {
                        analyzer
                            .find_importers_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_by_decorator" => {
                        analyzer
                            .find_by_decorator_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_by_argument" => {
                        analyzer
                            .find_by_argument_with_filters(target, Some(&filters))
                            .await?
                    }
                    "find_complexity" => {
                        analyzer
                            .find_complexity_with_filters(target, Some(&filters))
                            .await?
                    }
                    _ => Vec::new(),
                };
                Ok(json!(output))
            }
            "execute_cypher_query" => {
                let client = GraphClient::connect(&self.config).await?;
                let cypher = params
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("RETURN 1 AS ok");
                Ok(json!(client.raw_query(cypher).await?))
            }
            "find_dead_code" => {
                let client = GraphClient::connect(&self.config).await?;
                let analyzer = Analyzer::new(client);
                let filters = parse_analyze_filters(&params)?;
                Ok(json!(
                    analyzer.dead_code_with_filters(Some(&filters)).await?
                ))
            }
            "calculate_cyclomatic_complexity" | "find_most_complex_functions" => {
                let client = GraphClient::connect(&self.config).await?;
                let analyzer = Analyzer::new(client);
                let top_n = params.get("top_n").and_then(Value::as_u64).unwrap_or(20) as usize;
                let filters = parse_analyze_filters(&params)?;
                Ok(json!(
                    analyzer
                        .complexity_with_filters(top_n, Some(&filters))
                        .await?
                ))
            }
            "list_indexed_repositories" => {
                let client = GraphClient::connect(&self.config).await?;
                Ok(json!(client.list_repositories().await?))
            }
            "delete_repository" => {
                let client = GraphClient::connect(&self.config).await?;
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                client.delete_repository(path).await?;
                Ok(json!({"deleted": path}))
            }
            "check_job_status" => {
                let id = params.get("id").and_then(Value::as_str).unwrap_or_default();
                Ok(json!(self.jobs.get(id)))
            }
            "list_jobs" => Ok(json!(self.jobs.list())),
            "load_bundle" => {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(json!(BundleStore::import(PathBuf::from(path).as_path())?))
            }
            "search_registry_bundles" => Ok(json!({"bundles":[] })),
            "get_repository_stats" => {
                let client = GraphClient::connect(&self.config).await?;
                let analyzer = Analyzer::new(client);
                Ok(json!(analyzer.repository_stats().await?))
            }
            "check_health" => Ok(json!({
                "status":"ok",
                "analyzer": analyzer_capabilities_json()
            })),
            "add_package_to_graph" | "visualize_graph_query" => {
                Ok(json!({"status":"not_implemented"}))
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn tools_list_dispatch_does_not_require_graph_connection() {
        let server = McpServer::new(CortexConfig::default());
        let rt = Runtime::new().expect("runtime");
        let result = rt
            .block_on(server.dispatch("tools/list", json!({})))
            .expect("tools/list response");

        assert!(result.get("tools").is_some());
    }

    #[test]
    fn watch_dispatch_persists_and_lists_current_paths() {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let temp = tempdir().expect("tempdir");
        let watched = temp.path().join("repo");
        std::fs::create_dir_all(&watched).expect("watched dir");

        let old_home = std::env::var("HOME").ok();
        unsafe {
            std::env::set_var("HOME", temp.path());
        }

        let server = McpServer::new(CortexConfig::default());
        let rt = Runtime::new().expect("runtime");

        rt.block_on(server.dispatch(
            "watch_directory",
            json!({ "path": watched.display().to_string() }),
        ))
        .expect("watch response");

        let listed = rt
            .block_on(server.dispatch("list_watched_paths", json!({})))
            .expect("list response");
        let listed_text = listed["paths"].to_string();
        assert!(listed_text.contains(&watched.display().to_string()));

        let removed = rt
            .block_on(server.dispatch(
                "unwatch_directory",
                json!({ "path": watched.display().to_string() }),
            ))
            .expect("unwatch response");
        assert_eq!(removed["removed"], json!(true));

        match old_home {
            Some(home) => unsafe { std::env::set_var("HOME", home) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
}
