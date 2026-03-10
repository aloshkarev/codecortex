//! Memgraph Database Client
//!
//! This module provides a Memgraph-specific client using the official rsmgclient library.
//! Since rsmgclient's Connection is not Send, we use a channel-based approach where
//! a dedicated thread owns the connection and processes commands.

use cortex_core::{CortexError, Result};
use rsmgclient::{ConnectParams, Connection, QueryParam, Record, Value as MgValue};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use tracing::{debug, instrument, warn};

/// Command to send to the Memgraph worker thread
enum MemgraphCommand {
    /// Execute a query without results
    Execute {
        query: String,
        params: Option<HashMap<String, QueryParam>>,
        response: Sender<Result<()>>,
    },
    /// Execute a query with results
    Query {
        query: String,
        params: Option<HashMap<String, QueryParam>>,
        response: Sender<Result<(Vec<String>, Vec<Record>)>>,
    },
    /// Shutdown the worker
    Shutdown,
}

/// Connection status from worker thread
enum ConnectionStatus {
    Connected,
    Failed(String),
}

/// Memgraph client using a dedicated worker thread
#[derive(Clone)]
pub struct MemgraphClient {
    command_tx: Sender<MemgraphCommand>,
    #[allow(dead_code)]
    worker_handle: Arc<Option<JoinHandle<()>>>,
}

impl MemgraphClient {
    /// Connect to Memgraph database
    #[instrument(skip(config), fields(uri = %config.memgraph_uri))]
    pub async fn connect(config: &cortex_core::CortexConfig) -> Result<Self> {
        debug!(uri = %config.memgraph_uri, "Connecting to Memgraph");

        let (host, port) = Self::parse_uri(&config.memgraph_uri)?;

        let username = normalize_auth_field(config.memgraph_user.as_str());
        let password = normalize_auth_field(config.memgraph_password.as_str());

        let (command_tx, command_rx) = mpsc::channel();
        let (status_tx, status_rx) = mpsc::channel();

        // Spawn worker thread that owns the connection
        let worker = thread::spawn(move || {
            Self::worker_loop(host, port, username, password, command_rx, status_tx);
        });

        // Wait for connection status from worker thread
        let status = status_rx.recv().map_err(|_| {
            CortexError::Database("Worker thread crashed during connection".to_string())
        })?;

        match status {
            ConnectionStatus::Connected => {
                debug!("Worker thread connected successfully");
            }
            ConnectionStatus::Failed(err) => {
                // Wait for worker thread to finish
                let _ = worker.join();
                return Err(CortexError::Database(format!(
                    "Failed to connect to Memgraph: {}",
                    err
                )));
            }
        }

        let client = Self {
            command_tx,
            worker_handle: Arc::new(Some(worker)),
        };

        // Verify connection works
        client.run("RETURN 1").await?;

        debug!("Successfully connected to Memgraph");
        Ok(client)
    }

    /// Worker thread loop
    fn worker_loop(
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
        command_rx: Receiver<MemgraphCommand>,
        status_tx: Sender<ConnectionStatus>,
    ) {
        let connect_params = ConnectParams {
            host: Some(host.clone()),
            port,
            username: username.clone(),
            password,
            autocommit: true,
            ..Default::default()
        };

        debug!("Worker thread starting, connecting to {}:{}", host, port);

        // Initialize the mgclient library
        Connection::init();

        let conn_result = Connection::connect(&connect_params);
        let mut conn = match conn_result {
            Ok(c) => {
                debug!("Memgraph worker thread connected successfully");
                // Notify main thread of successful connection
                let _ = status_tx.send(ConnectionStatus::Connected);
                c
            }
            Err(e) => {
                warn!("Failed to connect to Memgraph in worker thread: {}", e);
                // Notify main thread of connection failure
                let _ = status_tx.send(ConnectionStatus::Failed(e.to_string()));
                return;
            }
        };

        debug!("Memgraph worker thread entering command loop");

        while let Ok(cmd) = command_rx.recv() {
            debug!("Worker received command");
            match cmd {
                MemgraphCommand::Shutdown => {
                    debug!("Memgraph worker shutting down");
                    break;
                }
                MemgraphCommand::Execute {
                    query,
                    params,
                    response,
                } => {
                    debug!("Executing query: {}", query);
                    let result = Self::execute_without_results(&mut conn, &query, params.as_ref());
                    let _ = response.send(result);
                    debug!("Execute command completed");
                }
                MemgraphCommand::Query {
                    query,
                    params,
                    response,
                } => {
                    debug!("Executing query with results: {}", query);
                    let result = Self::execute_query(&mut conn, &query, params.as_ref());
                    let _ = response.send(result);
                    debug!("Query command completed");
                }
            }
        }

        // Intentionally rely on Drop; explicit close may panic on "bad connection" in rsmgclient.
        debug!("Memgraph worker thread exiting");
    }

    /// Execute a query without returning results
    fn execute_without_results(
        conn: &mut Connection,
        query: &str,
        params: Option<&HashMap<String, QueryParam>>,
    ) -> Result<()> {
        if params.is_none() {
            return conn
                .execute_without_results(query)
                .map_err(|e| CortexError::Database(format!("Query failed: {}", e)));
        }

        conn.execute(query, params)
            .map_err(|e| CortexError::Database(format!("Query execution failed: {}", e)))?;

        conn.fetchall()
            .map(|_| ())
            .map_err(|e| CortexError::Database(format!("Failed to fetch results: {}", e)))
    }

    /// Execute a query and return column names and records
    fn execute_query(
        conn: &mut Connection,
        query: &str,
        params: Option<&HashMap<String, QueryParam>>,
    ) -> Result<(Vec<String>, Vec<Record>)> {
        let columns = conn
            .execute(query, params)
            .map_err(|e| CortexError::Database(format!("Query execution failed: {}", e)))?;

        let records = conn
            .fetchall()
            .map_err(|e| CortexError::Database(format!("Failed to fetch results: {}", e)))?;

        Ok((columns, records))
    }

    /// Parse URI to extract host and port
    fn parse_uri(uri: &str) -> Result<(String, u16)> {
        let uri = uri.trim();

        let uri = uri
            .strip_prefix("bolt://")
            .or_else(|| uri.strip_prefix("bolt+s://"))
            .or_else(|| uri.strip_prefix("bolt+ssc://"))
            .or_else(|| uri.strip_prefix("neo4j://"))
            .or_else(|| uri.strip_prefix("neo4j+s://"))
            .or_else(|| uri.strip_prefix("neo4j+ssc://"))
            .or_else(|| uri.strip_prefix("memgraph://"))
            .unwrap_or(uri);

        // Drop path/query/fragment and any userinfo
        let authority = uri
            .split(['/', '?', '#'])
            .next()
            .unwrap_or(uri)
            .rsplit('@')
            .next()
            .unwrap_or(uri);

        if authority.is_empty() {
            return Err(CortexError::Database(
                "Invalid Memgraph URI: missing host".to_string(),
            ));
        }

        let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
            // IPv6: [::1]:7687 or [::1]
            let end = rest.find(']').ok_or_else(|| {
                CortexError::Database("Invalid Memgraph URI: malformed IPv6 host".to_string())
            })?;
            let host = rest[..end].to_string();
            let tail = &rest[end + 1..];
            let port = if let Some(port_str) = tail.strip_prefix(':') {
                port_str
                    .parse::<u16>()
                    .map_err(|e| CortexError::Database(format!("Invalid port: {}", e)))?
            } else {
                7687
            };
            (host, port)
        } else if let Some((host, port_str)) = authority.rsplit_once(':') {
            // Hostname/IPv4 with explicit port
            if host.contains(':') {
                // Unbracketed IPv6 - treat entire authority as host and default port.
                (authority.to_string(), 7687)
            } else {
                let port = port_str
                    .parse::<u16>()
                    .map_err(|e| CortexError::Database(format!("Invalid port: {}", e)))?;
                (host.to_string(), port)
            }
        } else {
            (authority.to_string(), 7687)
        };

        Ok((host, port))
    }

    /// Execute a Cypher query without returning results
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn run(&self, cypher: &str) -> Result<()> {
        debug!(cypher = %cypher, "Executing Memgraph query");

        let (response_tx, response_rx) = mpsc::channel();

        self.command_tx
            .send(MemgraphCommand::Execute {
                query: cypher.to_string(),
                params: None,
                response: response_tx,
            })
            .map_err(|e| CortexError::Database(format!("Failed to send command: {}", e)))?;

        // Use tokio to wait without blocking the async runtime
        tokio::task::spawn_blocking(move || response_rx.recv())
            .await
            .map_err(|e| CortexError::Database(format!("Task failed: {}", e)))?
            .map_err(|e| CortexError::Database(format!("Failed to receive response: {}", e)))?
    }

    /// Execute a Cypher query and return results as JSON
    #[instrument(skip(self), fields(cypher_len = cypher.len()))]
    pub async fn raw_query(&self, cypher: &str) -> Result<Vec<JsonValue>> {
        debug!(cypher = %cypher, "Executing Memgraph raw query");

        let (response_tx, response_rx) = mpsc::channel();

        self.command_tx
            .send(MemgraphCommand::Query {
                query: cypher.to_string(),
                params: None,
                response: response_tx,
            })
            .map_err(|e| CortexError::Database(format!("Failed to send command: {}", e)))?;

        let (columns, records) = tokio::task::spawn_blocking(move || response_rx.recv())
            .await
            .map_err(|e| CortexError::Database(format!("Task failed: {}", e)))?
            .map_err(|e| CortexError::Database(format!("Failed to receive response: {}", e)))??;

        // Convert to JSON
        let rows: Vec<JsonValue> = records
            .iter()
            .map(|record| Self::record_to_json(&columns, record))
            .collect();

        debug!(row_count = rows.len(), "Memgraph query completed");
        Ok(rows)
    }

    /// Execute a parameterized query with a single string parameter
    pub async fn query_with_param(
        &self,
        cypher: &str,
        param_name: &str,
        param_value: &str,
    ) -> Result<Vec<JsonValue>> {
        let mut params = HashMap::new();
        params.insert(
            param_name.to_string(),
            QueryParam::String(param_value.to_string()),
        );
        self.raw_query_with_params(cypher, Some(params)).await
    }

    /// Execute a parameterized query with multiple parameters
    pub async fn query_with_params(
        &self,
        cypher: &str,
        params: Vec<(&str, String)>,
    ) -> Result<Vec<JsonValue>> {
        let params = params
            .into_iter()
            .map(|(name, value)| (name.to_string(), QueryParam::String(value)))
            .collect::<HashMap<_, _>>();
        self.raw_query_with_params(cypher, Some(params)).await
    }

    async fn raw_query_with_params(
        &self,
        cypher: &str,
        params: Option<HashMap<String, QueryParam>>,
    ) -> Result<Vec<JsonValue>> {
        let (response_tx, response_rx) = mpsc::channel();

        self.command_tx
            .send(MemgraphCommand::Query {
                query: cypher.to_string(),
                params,
                response: response_tx,
            })
            .map_err(|e| CortexError::Database(format!("Failed to send command: {}", e)))?;

        let (columns, records) = tokio::task::spawn_blocking(move || response_rx.recv())
            .await
            .map_err(|e| CortexError::Database(format!("Task failed: {}", e)))?
            .map_err(|e| CortexError::Database(format!("Failed to receive response: {}", e)))??;

        Ok(records
            .iter()
            .map(|record| Self::record_to_json(&columns, record))
            .collect())
    }

    /// Convert a Memgraph record to JSON
    fn record_to_json(columns: &[String], record: &Record) -> JsonValue {
        let mut obj = serde_json::Map::new();
        for (i, col) in columns.iter().enumerate() {
            let value = if i < record.values.len() {
                Self::mg_value_to_json(&record.values[i])
            } else {
                JsonValue::Null
            };
            obj.insert(col.clone(), value);
        }
        JsonValue::Object(obj)
    }

    /// Convert a Memgraph value to JSON
    fn mg_value_to_json(value: &MgValue) -> JsonValue {
        match value {
            MgValue::Null => JsonValue::Null,
            MgValue::Bool(b) => JsonValue::Bool(*b),
            MgValue::Int(i) => JsonValue::Number((*i).into()),
            MgValue::Float(f) => {
                if let Some(n) = serde_json::Number::from_f64(*f) {
                    JsonValue::Number(n)
                } else {
                    JsonValue::Null
                }
            }
            MgValue::String(s) => JsonValue::String(s.clone()),
            MgValue::List(items) => {
                JsonValue::Array(items.iter().map(Self::mg_value_to_json).collect())
            }
            MgValue::Map(items) => {
                let mut obj = serde_json::Map::new();
                for (k, v) in items {
                    obj.insert(k.clone(), Self::mg_value_to_json(v));
                }
                JsonValue::Object(obj)
            }
            MgValue::Node(node) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "labels".to_string(),
                    JsonValue::Array(
                        node.labels
                            .iter()
                            .map(|l| JsonValue::String(l.clone()))
                            .collect(),
                    ),
                );
                let mut props = serde_json::Map::new();
                for (k, v) in &node.properties {
                    props.insert(k.clone(), Self::mg_value_to_json(v));
                }
                obj.insert("properties".to_string(), JsonValue::Object(props));
                JsonValue::Object(obj)
            }
            MgValue::Relationship(rel) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), JsonValue::String(rel.type_.clone()));
                let mut props = serde_json::Map::new();
                for (k, v) in &rel.properties {
                    props.insert(k.clone(), Self::mg_value_to_json(v));
                }
                obj.insert("properties".to_string(), JsonValue::Object(props));
                JsonValue::Object(obj)
            }
            MgValue::UnboundRelationship(rel) => {
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), JsonValue::String(rel.type_.clone()));
                let mut props = serde_json::Map::new();
                for (k, v) in &rel.properties {
                    props.insert(k.clone(), Self::mg_value_to_json(v));
                }
                obj.insert("properties".to_string(), JsonValue::Object(props));
                JsonValue::Object(obj)
            }
            MgValue::Path(path) => {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "nodes".to_string(),
                    JsonValue::Array(
                        path.nodes
                            .iter()
                            .map(|n| Self::mg_value_to_json(&MgValue::Node(n.clone())))
                            .collect(),
                    ),
                );
                obj.insert(
                    "relationships".to_string(),
                    JsonValue::Array(
                        path.relationships
                            .iter()
                            .map(|r| {
                                Self::mg_value_to_json(&MgValue::UnboundRelationship(r.clone()))
                            })
                            .collect(),
                    ),
                );
                JsonValue::Object(obj)
            }
            // For date/time types, convert to string representation
            MgValue::Date(d) => JsonValue::String(format!("{}", d)),
            MgValue::LocalTime(t) => JsonValue::String(format!("{}", t)),
            MgValue::LocalDateTime(dt) => JsonValue::String(format!("{}", dt)),
            MgValue::DateTime(dt) => JsonValue::String(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:09}{}{:02}:{:02}",
                dt.year,
                dt.month,
                dt.day,
                dt.hour,
                dt.minute,
                dt.second,
                dt.nanosecond,
                if dt.time_zone_offset_seconds >= 0 {
                    "+"
                } else {
                    "-"
                },
                dt.time_zone_offset_seconds.abs() / 3600,
                (dt.time_zone_offset_seconds.abs() % 3600) / 60
            )),
            MgValue::Duration(d) => JsonValue::String(format!("{}", d)),
            MgValue::Point2D(p) => JsonValue::String(format!("{}", p)),
            MgValue::Point3D(p) => JsonValue::String(format!("{}", p)),
        }
    }
}

impl Drop for MemgraphClient {
    fn drop(&mut self) {
        let _ = self.command_tx.send(MemgraphCommand::Shutdown);
    }
}

fn normalize_auth_field(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri_with_protocol() {
        let (host, port) = MemgraphClient::parse_uri("bolt://localhost:7687").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 7687);
    }

    #[test]
    fn test_parse_uri_without_protocol() {
        let (host, port) = MemgraphClient::parse_uri("127.0.0.1:7687").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 7687);
    }

    #[test]
    fn test_parse_uri_default_port() {
        let (host, port) = MemgraphClient::parse_uri("localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 7687);
    }

    #[test]
    fn test_parse_uri_with_path_and_query() {
        let (host, port) =
            MemgraphClient::parse_uri("memgraph://localhost:17687/db?foo=bar").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 17687);
    }

    #[test]
    fn test_parse_uri_ipv6() {
        let (host, port) = MemgraphClient::parse_uri("bolt://[::1]:17687").unwrap();
        assert_eq!(host, "::1");
        assert_eq!(port, 17687);
    }

    #[test]
    fn test_normalize_auth_field() {
        assert_eq!(normalize_auth_field(""), None);
        assert_eq!(normalize_auth_field("   "), None);
        assert_eq!(normalize_auth_field("memgraph"), Some("memgraph".to_string()));
        assert_eq!(normalize_auth_field("  user  "), Some("user".to_string()));
    }
}
