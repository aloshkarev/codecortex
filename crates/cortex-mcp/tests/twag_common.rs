//! Shared helpers for TWAG corpus integration tests (MCP stdio session + golden oracles).

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const DEFAULT_TWAG_REPO: &str = "/run/media/alex/artefacts/projects/work/twag";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TwagGoldenCase {
    pub id: String,
    pub tool: String,
    #[serde(default)]
    pub setup: TwagGoldenSetup,
    pub args: Value,
    pub expect: TwagGoldenExpect,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TwagGoldenSetup {
    pub set_current_project: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TwagGoldenExpect {
    #[serde(default)]
    pub status_in: Vec<String>,
    #[serde(default)]
    pub min_results: usize,
    #[serde(default)]
    pub caller_names: Vec<String>,
    #[serde(default)]
    pub callee_path_suffix: Option<String>,
    #[serde(default)]
    pub definition: Option<TwagDefinitionExpect>,
    #[serde(default)]
    pub min_usages: usize,
    #[serde(default)]
    pub allow_empty: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TwagDefinitionExpect {
    pub file_path_suffix: String,
    pub line_number: u64,
    pub name: String,
}

pub fn twag_repo() -> String {
    std::env::var("CORTEX_TWAG_REPO").unwrap_or_else(|_| DEFAULT_TWAG_REPO.to_string())
}

pub fn rdiameter_repo() -> String {
    format!(
        "{}/third_party/tngf_cp/rdiameter/crates/rdiameter-core",
        twag_repo()
    )
}

pub fn cortex_bin() -> String {
    std::env::var("CORTEX_BIN").unwrap_or_else(|_| "cortex".to_string())
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/twag_goldens")
}

pub fn load_manifest() -> Value {
    let path = fixtures_dir().join("manifest.json");
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse manifest: {e}"))
}

pub fn load_case(id: &str) -> TwagGoldenCase {
    let path = fixtures_dir().join(format!("{id}.json"));
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

pub fn twag_gate_enabled() -> bool {
    std::env::var("CORTEX_TEST_TWAG").ok().as_deref() == Some("1")
        && std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() == Some("1")
}

pub fn skip_unless_twag_graph() -> bool {
    if !twag_gate_enabled() {
        eprintln!("skip: set CORTEX_TEST_TWAG=1 and CORTEX_TEST_GRAPH=1");
        return false;
    }
    let repo_path = twag_repo();
    let repo = Path::new(&repo_path);
    if !repo.exists() {
        eprintln!("skip: TWAG repo missing at {}", repo.display());
        return false;
    }
    true
}

pub struct McpSession {
    proc: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl McpSession {
    pub fn start() -> Self {
        let repo = twag_repo();
        let mut proc = Command::new(cortex_bin())
            .args(["mcp", "start"])
            .current_dir(&repo)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn cortex mcp in {repo}: {e}"));
        let stdin = proc.stdin.take().expect("stdin");
        let stdout = BufReader::new(proc.stdout.take().expect("stdout"));
        let mut session = Self {
            proc,
            stdin,
            stdout,
            next_id: 1,
        };
        session.initialize();
        session
    }

    fn initialize(&mut self) {
        let init_id = self.next_id;
        self.next_id += 1;
        self.write_json(json!({
            "jsonrpc": "2.0",
            "id": init_id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "twag-corpus-test", "version": "1.0.0"}
            }
        }));
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            let msg = self.read_line_json();
            if msg.get("id") == Some(&json!(init_id)) {
                self.write_json(json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/initialized",
                    "params": {}
                }));
                return;
            }
        }
        panic!("MCP initialize timed out");
    }

    fn write_json(&mut self, value: Value) {
        let line = value.to_string();
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .expect("write stdin");
        self.stdin.flush().expect("flush stdin");
    }

    fn read_line_json(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read stdout");
        serde_json::from_str(line.trim()).expect("parse stdout json")
    }

    pub fn resolve_buffered(&mut self, body: Value) -> Value {
        let data = match body.get("data") {
            Some(d) if d.get("buffered") == Some(&json!(true)) => d,
            _ => return body,
        };
        let response_id = data
            .get("response_id")
            .and_then(Value::as_str)
            .unwrap_or("resp_0");
        let stats = self.call_tool(
            "ctx_stats",
            json!({"response_id": response_id}),
            Duration::from_secs(30),
        );
        let byte_len = stats
            .get("data")
            .and_then(|d| d.get("entry"))
            .and_then(|e| e.get("byte_len"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                data.get("original_bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or(4096)
            }) as usize;
        let slice = self.call_tool(
            "ctx_slice",
            json!({"response_id": response_id, "from": 0, "to": byte_len}),
            Duration::from_secs(30),
        );
        let text = slice
            .get("data")
            .and_then(|d| d.get("slice"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("ctx_slice missing text: {slice}"));
        serde_json::from_str(text).unwrap_or_else(|e| panic!("parse buffered payload: {e}"))
    }

    pub fn call_tool(&mut self, name: &str, arguments: Value, timeout: Duration) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.write_json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments}
        }));
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let msg = self.read_line_json();
            if msg.get("id") == Some(&json!(id)) {
                return parse_tool_payload(&msg);
            }
        }
        panic!("MCP tool call timed out: {name}");
    }
}

impl Drop for McpSession {
    fn drop(&mut self) {
        let _ = self.proc.kill();
        let _ = self.proc.wait();
    }
}

pub fn parse_tool_payload(msg: &Value) -> Value {
    let result = msg.get("result").expect("result");
    if result.get("isError") == Some(&json!(true)) {
        panic!("tool error: {result}");
    }
    let text = result
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .expect("tool text payload");
    serde_json::from_str(text).expect("tool json payload")
}

pub fn resolve_setup_path(raw: &str) -> String {
    if raw.contains("${TWAG}") {
        return raw.replace("${TWAG}", &twag_repo());
    }
    if raw.contains("${RDIAMETER}") {
        return raw.replace("${RDIAMETER}", &rdiameter_repo());
    }
    raw.to_string()
}

pub fn run_golden_case(case: &TwagGoldenCase) {
    let mut session = McpSession::start();
    if let Some(project) = &case.setup.set_current_project {
        let path = resolve_setup_path(project);
        let body = session.call_tool(
            "set_current_project",
            json!({"path": path}),
            Duration::from_secs(60),
        );
        assert_eq!(
            body.get("status").and_then(Value::as_str),
            Some("ok"),
            "set_current_project failed for {}: {body}",
            case.id
        );
    }

    let body = session.call_tool(&case.tool, case.args.clone(), Duration::from_secs(180));
    let body = session.resolve_buffered(body);
    assert_golden(case, &body);
}

pub fn assert_golden(case: &TwagGoldenCase, body: &Value) {
    let status = body.get("status").and_then(Value::as_str).unwrap_or("");
    if !case.expect.status_in.is_empty() {
        assert!(
            case.expect.status_in.iter().any(|s| s == status),
            "{}: status {status} not in {:?}, body={body}",
            case.id,
            case.expect.status_in
        );
    }

    match case.tool.as_str() {
        "analyze_code_relationships" => assert_relationship_case(case, body),
        "go_to_definition" => assert_definition_case(case, body),
        "find_all_usages" => assert_usages_case(case, body),
        other => panic!("unsupported golden tool: {other}"),
    }
}

fn assert_relationship_case(case: &TwagGoldenCase, body: &Value) {
    let data = body.get("data").unwrap_or(body);
    if data.get("buffered") == Some(&json!(true)) {
        panic!(
            "{}: unexpected buffered response; narrow scope in golden args: {data}",
            case.id
        );
    }
    let rows = data
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let filtered = filter_relationship_rows(&rows, case.expect.callee_path_suffix.as_deref());
    assert!(
        filtered.len() >= case.expect.min_results,
        "{}: expected >= {} results, got {} (filtered from {}): {filtered:?}",
        case.id,
        case.expect.min_results,
        filtered.len(),
        rows.len()
    );
    if !case.expect.caller_names.is_empty() {
        let names = caller_names(&filtered);
        for expected in &case.expect.caller_names {
            assert!(
                names.contains(expected),
                "{}: missing caller {expected}; got {names:?}",
                case.id
            );
        }
    }
}

fn assert_definition_case(case: &TwagGoldenCase, body: &Value) {
    let expect = case
        .expect
        .definition
        .as_ref()
        .unwrap_or_else(|| panic!("{} missing definition expect", case.id));
    let defs = body
        .get("data")
        .and_then(|d| d.get("definitions"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!defs.is_empty(), "{}: no definitions: {body}", case.id);
    let hit = defs
        .iter()
        .find(|d| {
            d.get("file_path")
                .and_then(Value::as_str)
                .is_some_and(|p| p.ends_with(&expect.file_path_suffix))
        })
        .unwrap_or_else(|| panic!("{}: no definition with suffix {}", case.id, expect.file_path_suffix));
    assert_eq!(
        hit.get("name").and_then(Value::as_str),
        Some(expect.name.as_str()),
        "{}: definition name mismatch: {hit}",
        case.id
    );
    assert_eq!(
        hit.get("line_number").and_then(Value::as_u64),
        Some(expect.line_number),
        "{}: definition line mismatch: {hit}",
        case.id
    );
}

fn assert_usages_case(case: &TwagGoldenCase, body: &Value) {
    let usages = body
        .get("data")
        .and_then(|d| d.get("usages"))
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    if case.expect.allow_empty && usages == 0 {
        return;
    }
    assert!(
        usages >= case.expect.min_usages,
        "{}: expected >= {} usages, got {usages}: {body}",
        case.id,
        case.expect.min_usages
    );
}

pub fn filter_relationship_rows(rows: &[Value], callee_suffix: Option<&str>) -> Vec<Value> {
    rows.iter()
        .filter(|row| {
            let Some(suffix) = callee_suffix else {
                return true;
            };
            node_path(row.get("callee")).is_some_and(|p| p.ends_with(suffix))
                || row
                    .get("callee_name")
                    .and_then(Value::as_str)
                    .is_some_and(|_| {
                        node_path(row.get("caller"))
                            .is_some_and(|p| p.ends_with(suffix))
                    })
        })
        .cloned()
        .collect()
}

pub fn caller_names(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| node_name(row.get("caller")))
        .collect()
}

fn node_path(node: Option<&Value>) -> Option<String> {
    let node = node?;
    if let Some(path) = node.get("path").and_then(Value::as_str) {
        return Some(path.to_string());
    }
    if let Some(path) = node.get("file_path").and_then(Value::as_str) {
        return Some(path.to_string());
    }
    if let Some(path) = falkordb_node_field(node, "path") {
        return Some(path);
    }
    node.get("properties")
        .and_then(|props| props_array_get(props, "path"))
}

fn node_name(node: Option<&Value>) -> Option<String> {
    let node = node?;
    if let Some(name) = node.get("name").and_then(Value::as_str) {
        return Some(name.to_string());
    }
    if let Some(name) = falkordb_node_field(node, "name") {
        return Some(name);
    }
    node.get("properties")
        .and_then(|props| props_array_get(props, "name"))
}

fn falkordb_node_field(node: &Value, key: &str) -> Option<String> {
    let arr = node.as_array()?;
    for pair in arr {
        let pair = pair.as_array()?;
        if pair.first()?.as_str()? == "properties" {
            return props_array_get(pair.get(1)?, key);
        }
    }
    None
}

fn props_array_get(props: &Value, key: &str) -> Option<String> {
    let arr = props.as_array()?;
    for pair in arr {
        let pair = pair.as_array()?;
        if pair.first()?.as_str()? == key {
            return pair.get(1)?.as_str().map(str::to_string);
        }
    }
    None
}
