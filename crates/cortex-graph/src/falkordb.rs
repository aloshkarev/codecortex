//! FalkorDB graph client (RESP `GRAPH.QUERY` via `redis` crate).

use crate::falkordb_params::prepare_cypher_query;
use crate::falkordb_profile;
use cortex_core::{CortexConfig, CortexError, Result};
use crate::falkordb_params::GraphParam;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

/// FalkorDB client using Redis protocol.
#[derive(Clone)]
pub struct FalkorDbClient {
    connection: Arc<Mutex<redis::aio::MultiplexedConnection>>,
    graph_name: String,
}

impl FalkorDbClient {
    #[instrument(skip(config))]
    pub async fn connect(config: &CortexConfig) -> Result<Self> {
        let uri = falkor_connection_uri(config);
        debug!(uri = %uri, graph = %config.falkordb_graph, "Connecting to FalkorDB");

        let client = redis::Client::open(uri.as_str())
            .map_err(|e| CortexError::Database(format!("Invalid FalkorDB Redis URL: {e}")))?;
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CortexError::Database(format!("Failed to connect to FalkorDB: {e}")))?;

        let falkor = Self {
            connection: Arc::new(Mutex::new(connection)),
            graph_name: config.falkordb_graph.clone(),
        };
        falkor.run("RETURN 1").await?;
        Ok(falkor)
    }

    pub async fn run(&self, cypher: &str) -> Result<()> {
        self.execute_params(cypher, HashMap::new()).await
    }

    pub async fn raw_query(&self, cypher: &str) -> Result<Vec<JsonValue>> {
        self.raw_query_with_params(cypher, None).await
    }

    pub async fn query_with_param(
        &self,
        cypher: &str,
        param_name: &str,
        param_value: &str,
    ) -> Result<Vec<JsonValue>> {
        let mut params = HashMap::new();
        params.insert(
            param_name.to_string(),
            GraphParam::String(param_value.to_string()),
        );
        self.raw_query_with_params(cypher, Some(params)).await
    }

    pub async fn query_with_params(
        &self,
        cypher: &str,
        params: Vec<(&str, String)>,
    ) -> Result<Vec<JsonValue>> {
        let params = params
            .into_iter()
            .map(|(name, value)| (name.to_string(), GraphParam::String(value)))
            .collect::<HashMap<_, _>>();
        self.raw_query_with_params(cypher, Some(params)).await
    }

    pub async fn execute_with_raw_params(
        &self,
        cypher: &str,
        params: HashMap<String, GraphParam>,
    ) -> Result<()> {
        self.execute_params(cypher, params).await
    }

    pub async fn raw_query_with_params(
        &self,
        cypher: &str,
        params: Option<HashMap<String, GraphParam>>,
    ) -> Result<Vec<JsonValue>> {
        let (cypher, string_params) = prepare_cypher_query(cypher, params.unwrap_or_default());
        let query = build_falkor_query(&cypher, &string_params);
        let value = self.graph_query(&query).await?;
        Ok(parse_graph_query_rows(value))
    }

    async fn execute_params(
        &self,
        cypher: &str,
        params: HashMap<String, GraphParam>,
    ) -> Result<()> {
        let (cypher, string_params) = prepare_cypher_query(cypher, params);
        let query = build_falkor_query(&cypher, &string_params);
        let _ = self.graph_query(&query).await?;
        Ok(())
    }

    async fn graph_query(&self, query: &str) -> Result<redis::Value> {
        let lock_t0 = Instant::now();
        let mut conn = self.connection.lock().await;
        let lock_wait = lock_t0.elapsed();
        let exec_t0 = Instant::now();
        let result = redis::cmd("GRAPH.QUERY")
            .arg(&self.graph_name)
            .arg(query)
            .query_async(&mut *conn)
            .await
            .map_err(|e| CortexError::Database(format!("FalkorDB GRAPH.QUERY failed: {e}")));
        let wall = exec_t0.elapsed();
        if falkordb_profile::falkordb_profile_enabled() {
            falkordb_profile::record_query(query.len(), lock_wait, wall);
            debug!(
                target: "cortex_graph::falkordb_profile",
                query_bytes = query.len(),
                lock_wait_ms = lock_wait.as_secs_f64() * 1000.0,
                wall_ms = wall.as_secs_f64() * 1000.0,
                "GRAPH.QUERY"
            );
        }
        result
    }
}

impl FalkorDbClient {
    /// Reset FalkorDB micro-profile counters (call at index start).
    pub fn reset_profile() {
        falkordb_profile::reset();
    }

    /// Snapshot FalkorDB micro-profile stats for this run.
    pub fn profile_snapshot(reset_after: bool) -> falkordb_profile::FalkorDbProfileSnapshot {
        falkordb_profile::snapshot(reset_after)
    }
}

fn build_falkor_query(cypher: &str, string_params: &HashMap<String, String>) -> String {
    if string_params.is_empty() {
        return cypher.to_string();
    }
    let prefix: Vec<String> = string_params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    format!("CYPHER {} {}", prefix.join(" "), cypher)
}

fn falkor_connection_uri(config: &CortexConfig) -> String {
    let base = normalize_falkor_uri(&config.falkordb_uri);
    if config.falkordb_password.is_empty() || base.contains('@') {
        return base;
    }
    let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
    let (scheme, rest) = base.split_at(scheme_end);
    format!(
        "{scheme}:{password}@{rest}",
        password = config.falkordb_password
    )
}

fn normalize_falkor_uri(uri: &str) -> String {
    let trimmed = uri.trim();
    if trimmed.starts_with("falkor://") {
        return trimmed.replacen("falkor://", "redis://", 1);
    }
    if trimmed.starts_with("falkors://") {
        return trimmed.replacen("falkors://", "rediss://", 1);
    }
    if trimmed.starts_with("redis://") || trimmed.starts_with("rediss://") {
        return trimmed.to_string();
    }
    if trimmed.starts_with("bolt://") {
        return trimmed.replacen("bolt://", "redis://", 1);
    }
    if trimmed.starts_with("memgraph://") {
        return trimmed.replacen("memgraph://", "redis://", 1);
    }
    format!("redis://{trimmed}")
}

/// Parse FalkorDB GRAPH.QUERY response into JSON rows (header-keyed when available).
fn parse_graph_query_rows(value: redis::Value) -> Vec<JsonValue> {
    let redis::Value::Array(parts) = value else {
        return Vec::new();
    };
    if parts.is_empty() {
        return Vec::new();
    }
    // Write-only: [stats]
    if parts.len() == 1 {
        return Vec::new();
    }
    // [header, stats] or [header, data, stats]
    let (header, data) = if parts.len() == 2 {
        (parse_header_columns(&parts[0]), None::<&redis::Value>)
    } else {
        (parse_header_columns(&parts[0]), Some(&parts[1]))
    };
    let Some(data) = data else {
        return Vec::new();
    };
    let redis::Value::Array(rows) = data else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let redis::Value::Array(cells) = row else {
                return None;
            };
            let mut obj = serde_json::Map::new();
            for (i, cell) in cells.iter().enumerate() {
                let key = header.get(i).cloned().unwrap_or_else(|| i.to_string());
                obj.insert(key, redis_value_to_json(cell));
            }
            Some(JsonValue::Object(obj))
        })
        .collect()
}

fn parse_header_columns(header: &redis::Value) -> Vec<String> {
    let redis::Value::Array(cols) = header else {
        return Vec::new();
    };
    cols.iter()
        .enumerate()
        .map(|(i, col)| match col {
            redis::Value::Array(pair) if !pair.is_empty() => redis_value_to_json(&pair[0])
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("col{i}")),
            other => redis_value_to_string(other).unwrap_or_else(|| format!("col{i}")),
        })
        .collect()
}

fn redis_value_to_string(value: &redis::Value) -> Option<String> {
    match value {
        redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone()).ok(),
        redis::Value::SimpleString(s) => Some(s.clone()),
        _ => redis::from_redis_value::<String>(value).ok(),
    }
}

fn redis_value_to_json(value: &redis::Value) -> JsonValue {
    match value {
        redis::Value::Nil => JsonValue::Null,
        redis::Value::Int(i) => JsonValue::Number((*i).into()),
        redis::Value::BulkString(bytes) => String::from_utf8(bytes.clone())
            .map(JsonValue::String)
            .unwrap_or(JsonValue::Null),
        redis::Value::SimpleString(s) => JsonValue::String(s.clone()),
        redis::Value::Okay => JsonValue::Bool(true),
        redis::Value::Double(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        redis::Value::Boolean(b) => JsonValue::Bool(*b),
        redis::Value::Array(items) => {
            JsonValue::Array(items.iter().map(redis_value_to_json).collect())
        }
        redis::Value::Map(map) => {
            let obj = map
                .iter()
                .map(|(k, v)| {
                    let key = redis_value_to_json(k)
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("{k:?}"));
                    (key, redis_value_to_json(v))
                })
                .collect();
            JsonValue::Object(obj)
        }
        redis::Value::VerbatimString { format: _, text } => JsonValue::String(text.clone()),
        redis::Value::BigNumber(n) => JsonValue::String(n.to_string()),
        redis::Value::Attribute { data, attributes } => {
            let mut obj = serde_json::Map::new();
            obj.insert("_data".to_string(), redis_value_to_json(data));
            for (k, v) in attributes {
                let key = redis_value_to_json(k)
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                obj.insert(key, redis_value_to_json(v));
            }
            JsonValue::Object(obj)
        }
        redis::Value::Set(items) => {
            JsonValue::Array(items.iter().map(redis_value_to_json).collect())
        }
        redis::Value::Push { kind: _, data } => {
            JsonValue::Array(data.iter().map(redis_value_to_json).collect())
        }
        redis::Value::ServerError(e) => JsonValue::String(format!("{e:?}")),
    }
}
