//! Compare token estimates: full file reads vs `get_api_contract` + `get_skeleton` on TWAG files.
//!
//! Writes `target/token-efficiency-twag.json`. Gated by `CORTEX_TEST_TWAG=1`.

use cortex_core::tokens::{count_tokens, estimate_baseline_from_sample};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_TWAG: &str = "/run/media/alex/artefacts/projects/work/twag";

#[derive(Debug, Serialize, Deserialize)]
struct FileSample {
    path: String,
    symbol: Option<String>,
    full_file_tokens: usize,
    bounded_tokens: usize,
    saved_tokens: usize,
    savings_pct: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenEfficiencyReport {
    repo_path: String,
    file_count: usize,
    full_file_tokens: usize,
    bounded_tokens: usize,
    saved_tokens: usize,
    savings_pct: f64,
    files: Vec<FileSample>,
}

fn twag_repo() -> PathBuf {
    PathBuf::from(std::env::var("CORTEX_TWAG_REPO").unwrap_or_else(|_| DEFAULT_TWAG.to_string()))
}

fn cortex_bin() -> String {
    std::env::var("CORTEX_BIN").unwrap_or_else(|_| "cortex".to_string())
}

fn default_samples(repo: &Path) -> Vec<(PathBuf, Option<String>)> {
    vec![
        (
            repo.join("components/cp/src/orchestrator.cpp"),
            Some("snapshot".to_string()),
        ),
        (
            repo.join("third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/relay.rs"),
            Some("should_relay".to_string()),
        ),
        (
            repo.join("third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/peer.rs"),
            Some("PeerState".to_string()),
        ),
        (
            repo.join("third_party/tngf_cp/rdiameter/crates/rdiameter-core/src/node.rs"),
            Some("Node".to_string()),
        ),
        (
            repo.join("components/cp/include/wmg/cp/orchestrator.hpp"),
            Some("Orchestrator".to_string()),
        ),
        (
            repo.join("components/platform/src/forwarding_ipc.cpp"),
            Some("ForwardingClient".to_string()),
        ),
        (
            repo.join("components/platform/src/forwarding_grpc_client.cpp"),
            Some("GrpcForwardingClient".to_string()),
        ),
        (
            repo.join("components/tngf_up/src/grpc_forwarding_service.cpp"),
            Some("GrpcForwardingService".to_string()),
        ),
        (
            repo.join("components/tngf_up/src/tunnel_registry.cpp"),
            Some("TunnelRegistry".to_string()),
        ),
        (
            repo.join("components/tngf_up/src/data_plane.cpp"),
            Some("DataPlane".to_string()),
        ),
        (
            repo.join("components/platform/src/cp_config.cpp"),
            Some("CpConfig".to_string()),
        ),
        (
            repo.join("components/twif/src/dhcp_server.cpp"),
            Some("DhcpServer".to_string()),
        ),
    ]
}

struct McpSession {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl McpSession {
    fn start() -> Self {
        let mut child = Command::new(cortex_bin())
            .args(["mcp", "start"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn cortex mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut session = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        session.initialize();
        session
    }

    fn initialize(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "token-efficiency-twag", "version": "1"}
            }
        }));
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            let msg = self.read_json();
            if msg.get("id") == Some(&json!(id)) {
                self.write(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                    "params": {}
                }));
                return;
            }
        }
        panic!("initialize timed out");
    }

    fn write(&mut self, value: Value) {
        let line = value.to_string();
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .expect("write");
        self.stdin.flush().expect("flush");
    }

    fn read_json(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read");
        serde_json::from_str(line.trim()).expect("json")
    }

    fn call(&mut self, tool: &str, args: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": tool, "arguments": args}
        }));
        let deadline = Instant::now() + Duration::from_secs(120);
        while Instant::now() < deadline {
            let msg = self.read_json();
            if msg.get("id") == Some(&json!(id)) {
                let text = msg["result"]["content"][0]["text"]
                    .as_str()
                    .expect("tool text");
                return serde_json::from_str(text).expect("tool json");
            }
        }
        panic!("tool timed out: {tool}");
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn bounded_payload(session: &mut McpSession, repo: &Path, file: &Path, symbol: Option<&str>) -> String {
    let repo_s = repo.display().to_string();
    let path_s = file.display().to_string();
    let skeleton = session.call(
        "get_skeleton",
        json!({"path": path_s, "repo_path": repo_s}),
    );
    let mut parts = vec![skeleton.to_string()];
    if let Some(sym) = symbol {
        let contract = session.call(
            "get_api_contract",
            json!({"symbol": sym, "repo_path": repo_s, "budget_tokens": 4000}),
        );
        parts.push(contract.to_string());
    }
    parts.join("\n")
}

fn main() {
    if std::env::var("CORTEX_TEST_TWAG").ok().as_deref() != Some("1") {
        eprintln!("Set CORTEX_TEST_TWAG=1 to run token-efficiency-twag benchmark");
        std::process::exit(2);
    }

    let repo = twag_repo();
    if !repo.exists() {
        eprintln!("TWAG repo missing: {}", repo.display());
        std::process::exit(2);
    }

    let mut session = McpSession::start();
    let _ = session.call("set_current_project", json!({"path": repo.display().to_string()}));

    let mut files = Vec::new();
    let mut total_full = 0usize;
    let mut total_bounded = 0usize;

    for (file, symbol) in default_samples(&repo) {
        if !file.exists() {
            eprintln!("skip missing file: {}", file.display());
            continue;
        }
        let raw = fs::read_to_string(&file).unwrap_or_default();
        let sample = raw.chars().take(8192).collect::<String>();
        let (full_tokens, _) = estimate_baseline_from_sample(raw.chars().count(), &sample);
        let bounded_text = bounded_payload(&mut session, &repo, &file, symbol.as_deref());
        let (bounded_tokens, _) = count_tokens(&bounded_text);
        let saved = full_tokens.saturating_sub(bounded_tokens);
        let pct = if full_tokens == 0 {
            0.0
        } else {
            (saved as f64 / full_tokens as f64) * 100.0
        };
        total_full += full_tokens;
        total_bounded += bounded_tokens;
        files.push(FileSample {
            path: file.strip_prefix(&repo)
                .unwrap_or(&file)
                .display()
                .to_string(),
            symbol,
            full_file_tokens: full_tokens,
            bounded_tokens,
            saved_tokens: saved,
            savings_pct: pct,
        });
    }

    let saved_total = total_full.saturating_sub(total_bounded);
    let report = TokenEfficiencyReport {
        repo_path: repo.display().to_string(),
        file_count: files.len(),
        full_file_tokens: total_full,
        bounded_tokens: total_bounded,
        saved_tokens: saved_total,
        savings_pct: if total_full == 0 {
            0.0
        } else {
            (saved_total as f64 / total_full as f64) * 100.0
        },
        files,
    };

    let out = PathBuf::from("target/token-efficiency-twag.json");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&out, serde_json::to_string_pretty(&report).expect("json")).expect("write report");
    println!("wrote {}", out.display());
    println!(
        "saved {} / {} tokens ({:.1}%) across {} files",
        report.saved_tokens, report.full_file_tokens, report.savings_pct, report.file_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_efficiency_report_shape() {
        let report = TokenEfficiencyReport {
            repo_path: "/tmp/twag".to_string(),
            file_count: 1,
            full_file_tokens: 1000,
            bounded_tokens: 200,
            saved_tokens: 800,
            savings_pct: 80.0,
            files: vec![FileSample {
                path: "src/a.rs".to_string(),
                symbol: Some("main".to_string()),
                full_file_tokens: 1000,
                bounded_tokens: 200,
                saved_tokens: 800,
                savings_pct: 80.0,
            }],
        };
        let json = serde_json::to_string(&report).expect("json");
        assert!(json.contains("saved_tokens"));
    }
}
