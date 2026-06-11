//! WebSocket A2A task event channel at `GET /a2a/v1/ws`.

use cortex_core::{A2aConfig, CortexConfig};
use cortex_mcp::FeatureFlags;
use cortex_mcp::handler::{McpServeOptions, McpTransport};
use cortex_mcp::start_network;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().expect("addr").port()
}

#[tokio::test]
async fn ws_a2a_subscribe_validates_payload() {
    let port = free_port();
    let options = McpServeOptions {
        transport: McpTransport::Multi,
        listen: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        token: None,
        allow_remote: false,
        max_clients: 8,
        idle_timeout_secs: 30,
        feature_flags: FeatureFlags::from_env(),
    };

    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        server: cortex_core::A2aServerConfig {
            http_enabled: true,
            ..Default::default()
        },
        ..A2aConfig::default()
    };

    let handle = tokio::spawn(async move {
        let _ = start_network(config, options).await;
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    let url = format!("ws://127.0.0.1:{port}/a2a/v1/ws");
    let (mut ws, _) = connect_async(url.as_str()).await.expect("ws connect");

    ws.send(WsMessage::Text(
        json!({ "type": "a2a_subscribe" }).to_string().into(),
    ))
    .await
    .expect("send");

    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("timeout")
        .expect("frame")
        .expect("text");
    let WsMessage::Text(text) = msg else {
        panic!("expected text frame");
    };
    assert!(
        text.contains("missing task_id") || text.contains("error"),
        "expected validation error, got: {text}"
    );

    handle.abort();
}

#[tokio::test]
async fn ws_a2a_subscribe_accepts_valid_task_id() {
    let port = free_port();
    let options = McpServeOptions {
        transport: McpTransport::Multi,
        listen: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        token: None,
        allow_remote: false,
        max_clients: 8,
        idle_timeout_secs: 30,
        feature_flags: FeatureFlags::from_env(),
    };

    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        consensus_max_rounds: 1,
        server: cortex_core::A2aServerConfig {
            http_enabled: true,
            ..Default::default()
        },
        ..A2aConfig::default()
    };

    let handle = tokio::spawn(async move {
        let _ = start_network(config, options).await;
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = reqwest::Client::new();
    let send_resp = client
        .post(format!("http://127.0.0.1:{port}/a2a/v1/message:send"))
        .json(&json!({
            "message": {
                "role": "user",
                "parts": [{ "text": "quick consensus" }],
                "messageId": "msg-ws-1"
            },
            "configuration": { "returnImmediately": true }
        }))
        .send()
        .await
        .expect("send");
    assert!(send_resp.status().is_success());
    let send_json: serde_json::Value = send_resp.json().await.expect("json");
    let task_id = send_json["task"]["id"].as_str().expect("task id");

    let url = format!("ws://127.0.0.1:{port}/a2a/v1/ws");
    let (mut ws, _) = connect_async(url.as_str()).await.expect("ws connect");
    ws.send(WsMessage::Text(
        json!({ "type": "a2a_subscribe", "task_id": task_id })
            .to_string()
            .into(),
    ))
    .await
    .expect("subscribe");

    let mut saw_payload = false;
    for _ in 0..40 {
        if let Ok(Some(Ok(WsMessage::Text(text)))) =
            tokio::time::timeout(Duration::from_millis(500), ws.next()).await
        {
            if text.contains("task") || text.contains("status") {
                saw_payload = true;
                break;
            }
        }
        if let Ok(resp) = client
            .get(format!("http://127.0.0.1:{port}/a2a/v1/tasks/{task_id}"))
            .send()
            .await
        {
            if resp.status().is_success() {
                saw_payload = true;
                break;
            }
        }
    }

    assert!(
        saw_payload,
        "WS subscribe should receive task payload for {task_id}"
    );
    handle.abort();
}
