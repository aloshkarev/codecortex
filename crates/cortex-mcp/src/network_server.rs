use crate::handler::{CortexHandler, McpServeOptions, McpTransport};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Request;
use axum::extract::Extension;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{Next, from_fn};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, routing::any_service};
use cortex_core::CortexConfig;
use futures_util::{SinkExt, StreamExt, future};
use rmcp::ServiceExt;
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Clone)]
struct NetworkState {
    config: CortexConfig,
    token: Option<String>,
    max_clients: usize,
    idle_timeout_secs: u64,
    ws_clients: Arc<AtomicUsize>,
}

fn unauthorized_response() -> Response {
    (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
}

fn too_many_clients_response() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        "Too many clients connected",
    )
        .into_response()
}

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let (scheme, token) = auth.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") {
        Some(token.trim().to_string())
    } else {
        None
    }
}

fn is_authorized(headers: &HeaderMap, expected_token: &Option<String>) -> bool {
    match expected_token {
        None => true,
        Some(expected) => token_from_headers(headers)
            .map(|provided| provided == *expected)
            .unwrap_or(false),
    }
}

async fn auth_middleware(req: Request, next: Next) -> Response {
    let expected_token = req
        .extensions()
        .get::<NetworkState>()
        .and_then(|s| s.token.clone());
    if !is_authorized(req.headers(), &expected_token) {
        return unauthorized_response();
    }
    next.run(req).await
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    Extension(state): Extension<NetworkState>,
) -> Response {
    let active = state.ws_clients.load(Ordering::SeqCst);
    if active >= state.max_clients {
        return too_many_clients_response();
    }
    let shared = state.clone();
    ws.on_upgrade(move |socket| websocket_loop(socket, shared))
}

async fn websocket_loop(socket: WebSocket, state: NetworkState) {
    state.ws_clients.fetch_add(1, Ordering::SeqCst);

    let (sink, stream) = socket.split();
    let idle = Duration::from_secs(state.idle_timeout_secs);

    let incoming = stream.filter_map(move |msg| {
        let parsed = match msg {
            Ok(Message::Text(t)) => serde_json::from_str::<ClientJsonRpcMessage>(t.as_str()).ok(),
            Ok(Message::Binary(b)) => serde_json::from_slice::<ClientJsonRpcMessage>(&b).ok(),
            Ok(Message::Close(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => None,
            Err(_) => None,
        };
        future::ready(parsed)
    });

    let outgoing = sink
        .sink_map_err(|e| io::Error::other(format!("websocket sink error: {e}")))
        .with(|msg: ServerJsonRpcMessage| {
            let text = serde_json::to_string(&msg).map_err(|e| {
                io::Error::other(format!("serialize ws message: {e}"))
            });
            future::ready(text.map(|text| Message::Text(text.into())))
        });

    let handler = CortexHandler::new(state.config.clone());
    let service = match handler.serve::<_, io::Error, _>((outgoing, incoming)).await {
        Ok(svc) => svc,
        Err(err) => {
            tracing::warn!("failed to start websocket rmcp service: {err}");
            state.ws_clients.fetch_sub(1, Ordering::SeqCst);
            return;
        }
    };

    let _ = tokio::time::timeout(idle, service.waiting()).await;
    state.ws_clients.fetch_sub(1, Ordering::SeqCst);
}

pub async fn start_network(config: CortexConfig, options: McpServeOptions) -> anyhow::Result<()> {
    let state = NetworkState {
        config: config.clone(),
        token: options.token.clone(),
        max_clients: options.max_clients,
        idle_timeout_secs: options.idle_timeout_secs,
        ws_clients: Arc::new(AtomicUsize::new(0)),
    };

    let mut app = Router::new();

    if matches!(options.transport, McpTransport::HttpSse | McpTransport::Multi) {
        let cfg_for_factory = config.clone();
        let http_service: StreamableHttpService<CortexHandler, LocalSessionManager> =
            StreamableHttpService::new(
                move || Ok(CortexHandler::new(cfg_for_factory.clone())),
                Default::default(),
                StreamableHttpServerConfig::default(),
            );
        app = app.route_service("/mcp", any_service(http_service));
    }

    if matches!(options.transport, McpTransport::WebSocket | McpTransport::Multi) {
        app = app.route("/ws", get(ws_upgrade));
    }

    app = app
        .layer(Extension(state))
        .layer(from_fn(auth_middleware));

    let listener = tokio::net::TcpListener::bind(options.listen).await?;
    tracing::info!(
        "MCP network server listening on {} (transport: {:?})",
        options.listen,
        options.transport
    );
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message as WsMessage;

    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind free port");
        listener.local_addr().expect("local addr").port()
    }

    #[tokio::test]
    async fn websocket_supports_multiple_clients() {
        let port = free_port();
        let options = McpServeOptions {
            transport: McpTransport::WebSocket,
            listen: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            token: None,
            allow_remote: false,
            max_clients: 4,
            idle_timeout_secs: 60,
        };

        let handle = tokio::spawn(async move {
            let _ = start_network(CortexConfig::default(), options).await;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;

        let url = format!("ws://127.0.0.1:{port}/ws");
        let (mut ws1, _) = connect_async(url.as_str()).await.expect("ws1 connect");
        let (mut ws2, _) = connect_async(url.as_str()).await.expect("ws2 connect");

        let init = json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}
        });
        ws1.send(WsMessage::Text(init.to_string().into()))
            .await
            .expect("send init ws1");
        ws2.send(WsMessage::Text(init.to_string().into()))
            .await
            .expect("send init ws2");

        let r1 = ws1.next().await.expect("ws1 init recv").expect("ws1 msg");
        let r2 = ws2.next().await.expect("ws2 init recv").expect("ws2 msg");
        assert!(matches!(r1, WsMessage::Text(_)));
        assert!(matches!(r2, WsMessage::Text(_)));

        let initialized = json!({
            "jsonrpc":"2.0",
            "method":"notifications/initialized",
            "params":{}
        });
        ws1.send(WsMessage::Text(initialized.to_string().into()))
            .await
            .expect("send initialized ws1");
        ws2.send(WsMessage::Text(initialized.to_string().into()))
            .await
            .expect("send initialized ws2");

        let tools_call = json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"tools/list",
            "params":{}
        });
        ws1.send(WsMessage::Text(tools_call.to_string().into()))
            .await
            .expect("send tools/list");
        let response = ws1.next().await.expect("tools/list recv").expect("tools/list msg");
        let text = match response {
            WsMessage::Text(t) => t.to_string(),
            other => panic!("unexpected response: {:?}", other),
        };
        assert!(text.contains("tools"));

        let _ = ws1.close(None).await;
        let _ = ws2.close(None).await;
        handle.abort();
    }

    #[tokio::test]
    async fn http_sse_returns_event_stream() {
        let port = free_port();
        let options = McpServeOptions {
            transport: McpTransport::HttpSse,
            listen: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            token: None,
            allow_remote: false,
            max_clients: 8,
            idle_timeout_secs: 60,
        };

        let handle = tokio::spawn(async move {
            let _ = start_network(CortexConfig::default(), options).await;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;

        let client = reqwest::Client::new();
        let payload = json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}
        });
        let res = client
            .post(format!("http://127.0.0.1:{port}/mcp"))
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .expect("mcp post");

        assert_eq!(res.status(), StatusCode::OK);
        let ct = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert!(ct.contains("text/event-stream"));
        let body = res.text().await.expect("response body");
        assert!(body.contains("data:"));
        assert!(body.contains("\"result\""));

        handle.abort();
    }
}
