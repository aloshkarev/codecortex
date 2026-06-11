//! End-to-end A2A MCP workflow: spawn → get_task → send_message → list_push_configs → cancel.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn cortex_bin() -> String {
    std::env::var("CORTEX_BIN").unwrap_or_else(|_| "cortex".to_string())
}

fn mcp_tool_call(name: &str, arguments: serde_json::Value, timeout: Duration) -> serde_json::Value {
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "mcp-a2a-workflow-test", "version": "1.0.0"}
        }
    });
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    });

    let mut proc = Command::new(cortex_bin())
        .args(["mcp", "start"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn cortex mcp");

    let mut stdin = proc.stdin.take().expect("stdin");
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .expect("write init");
    stdin.flush().expect("flush init");

    let stdout = proc.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    let deadline = Instant::now() + timeout;
    let mut saw_init = false;

    for line in reader.lines() {
        if Instant::now() > deadline {
            break;
        }
        let Ok(line) = line else { break };
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if msg.get("id") == Some(&serde_json::json!(1)) {
            saw_init = true;
            stdin
                .write_all(
                    format!(
                        "{}\n",
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/initialized",
                            "params": {}
                        })
                    )
                    .as_bytes(),
                )
                .expect("initialized");
            stdin
                .write_all(format!("{req}\n").as_bytes())
                .expect("write call");
            stdin.flush().expect("flush call");
        } else if saw_init && msg.get("id") == Some(&serde_json::json!(2)) {
            let _ = proc.kill();
            let _ = proc.wait();
            return msg;
        }
    }
    let _ = proc.kill();
    let _ = proc.wait();
    panic!("MCP tool call timed out: {name}");
}

fn tool_text_payload(msg: &serde_json::Value) -> Option<serde_json::Value> {
    let result = msg.get("result")?;
    if result.get("isError") == Some(&serde_json::json!(true)) {
        return None;
    }
    let content = result.get("content")?.as_array()?;
    let text = content.first()?.get("text")?.as_str()?;
    serde_json::from_str(text).ok()
}

#[tokio::test]
#[ignore = "requires CORTEX_TEST_GRAPH=1 and [a2a].enabled"]
async fn a2a_spawn_get_task_and_list_push_configs() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }

    let spawn = mcp_tool_call(
        "cortex_a2a_spawn_session",
        serde_json::json!({
            "task": "workflow integration test",
            "workflow": "consensus_review",
            "include_paths": ["crates/cortex-mcp"],
            "return_immediately": true
        }),
        Duration::from_secs(180),
    );
    let body = tool_text_payload(&spawn).expect("spawn payload");
    let task_id = body
        .get("task_id")
        .or_else(|| body.get("data").and_then(|d| d.get("task_id")))
        .and_then(|v| v.as_str())
        .expect("task_id from spawn");
    assert!(!task_id.is_empty());

    let get_task = mcp_tool_call(
        "cortex_a2a_get_task",
        serde_json::json!({"task_id": task_id}),
        Duration::from_secs(60),
    );
    assert!(
        tool_text_payload(&get_task).is_some(),
        "get_task should return JSON body"
    );

    let send = mcp_tool_call(
        "cortex_a2a_send_message",
        serde_json::json!({"task_id": task_id, "message": "workflow ping"}),
        Duration::from_secs(60),
    );
    assert!(
        tool_text_payload(&send).is_some(),
        "send_message should return JSON body"
    );

    let push = mcp_tool_call(
        "cortex_a2a_list_push_configs",
        serde_json::json!({"task_id": task_id}),
        Duration::from_secs(60),
    );
    assert!(
        tool_text_payload(&push).is_some(),
        "list_push_configs with task_id should return JSON body"
    );

    let push_all = mcp_tool_call(
        "cortex_a2a_list_push_configs",
        serde_json::json!({}),
        Duration::from_secs(60),
    );
    assert!(
        tool_text_payload(&push_all).is_some(),
        "list_push_configs without task_id should return JSON body"
    );

    let cancel = mcp_tool_call(
        "cortex_a2a_cancel_task",
        serde_json::json!({"task_id": task_id}),
        Duration::from_secs(60),
    );
    assert!(
        tool_text_payload(&cancel).is_some(),
        "cancel_task should return JSON body"
    );
}
