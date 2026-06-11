//! Optional newline-delimited JSON audit log for MCP tool invocations (`CORTEX_MCP_AUDIT_LOG`).

use serde::Serialize;
use std::io::Write;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static AUDIT_PATH: Mutex<Option<String>> = Mutex::new(None);

fn audit_path_cached() -> Option<String> {
    let mut guard = AUDIT_PATH.lock().ok()?;
    if guard.is_none() {
        *guard = std::env::var("CORTEX_MCP_AUDIT_LOG")
            .ok()
            .filter(|s| !s.trim().is_empty());
    }
    guard.clone()
}

#[derive(Debug, Serialize)]
pub struct ToolAuditEvent {
    pub ts_ms: u64,
    pub tool: String,
    pub status: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
}

pub fn log_tool_audit(event: ToolAuditEvent) {
    tracing::info!(
        target: "cortex_mcp_audit",
        tool = %event.tool,
        status = %event.status,
        duration_ms = event.duration_ms,
        payload_chars = ?event.payload_chars,
        source_policy = ?event.source_policy,
        cost_class = ?event.cost_class,
        repo_path = ?event.repo_path,
        "mcp_tool_audit"
    );
    let Some(path) = audit_path_cached() else {
        return;
    };
    let line = match serde_json::to_string(&event) {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
