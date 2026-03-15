use futures_util::{SinkExt, StreamExt};
use cortex_core::CortexConfig;
use cortex_mcp::{McpServeOptions, McpTransport, start_with_options};
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind free port");
    listener.local_addr().expect("local addr").port()
}

async fn connect_with_retry(url: &str) -> tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
> {
    let mut last_err = None;
    for _ in 0..120 {
        match connect_async(url).await {
            Ok((ws, _)) => return ws,
            Err(err) => {
                last_err = Some(err);
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
    }
    panic!("failed to connect websocket: {:?}", last_err);
}

#[tokio::test]
async fn mcp_websocket_accepts_multiple_clients_and_lists_tools() {
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");

    let options = McpServeOptions {
        transport: McpTransport::WebSocket,
        listen: listen.parse().expect("listen addr"),
        token: None,
        allow_remote: false,
        max_clients: 4,
        idle_timeout_secs: 60,
    };
    let server_task = tokio::spawn(async move {
        let _ = start_with_options(CortexConfig::default(), options).await;
    });

    let ws_url = format!("ws://{listen}/ws");
    let mut ws1 = connect_with_retry(ws_url.as_str()).await;
    let mut ws2 = connect_with_retry(ws_url.as_str()).await;

    let init = json!({
        "jsonrpc":"2.0",
        "id":1,
        "method":"initialize",
        "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"itest","version":"1.0.0"}}
    });
    ws1.send(Message::Text(init.to_string().into()))
        .await
        .expect("send init ws1");
    ws2.send(Message::Text(init.to_string().into()))
        .await
        .expect("send init ws2");

    let _ = ws1.next().await.expect("recv init ws1").expect("msg ws1");
    let _ = ws2.next().await.expect("recv init ws2").expect("msg ws2");

    let initialized = json!({
        "jsonrpc":"2.0",
        "method":"notifications/initialized",
        "params":{}
    });
    ws1.send(Message::Text(initialized.to_string().into()))
        .await
        .expect("send initialized ws1");
    ws2.send(Message::Text(initialized.to_string().into()))
        .await
        .expect("send initialized ws2");

    let tools_list = json!({
        "jsonrpc":"2.0",
        "id":2,
        "method":"tools/list",
        "params":{}
    });
    ws1.send(Message::Text(tools_list.to_string().into()))
        .await
        .expect("send tools/list");

    let response = ws1
        .next()
        .await
        .expect("recv tools/list")
        .expect("tools/list message");
    let text = match response {
        Message::Text(body) => body.to_string(),
        other => panic!("unexpected ws response: {:?}", other),
    };
    assert!(text.contains("\"tools\""));

    let _ = ws1.close(None).await;
    let _ = ws2.close(None).await;
    server_task.abort();
}
