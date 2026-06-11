//! HTTP+JSON A2A ingress tests (version header + spec errors).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{Extension, Router};
use cortex_a2a::A2aHub;
use cortex_core::CortexConfig;
use cortex_mcp::a2a_http::a2a_routes;
use cortex_mcp::{FeatureFlags, NetworkState};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tower::ServiceExt;

fn test_state() -> NetworkState {
    let mut config = CortexConfig::default();
    config.a2a.enabled = true;
    config.a2a.server.http_enabled = true;
    config.a2a.server.protocol_version = "1.0".to_string();
    let hub = Arc::new(A2aHub::new(config.a2a.clone()));
    NetworkState {
        config: config.clone(),
        feature_flags: FeatureFlags::default(),
        token: None,
        max_clients: 8,
        idle_timeout_secs: 30,
        ws_clients: Arc::new(AtomicUsize::new(0)),
        a2a_hub: hub,
    }
}

#[tokio::test]
async fn version_mismatch_returns_bad_request() {
    let app = a2a_routes().layer(Extension(test_state()));
    let req = Request::builder()
        .method("GET")
        .uri("/a2a/v1/tasks")
        .header("A2A-Version", "0.1")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_missing_task_returns_not_found() {
    let app = a2a_routes().layer(Extension(test_state()));
    let req = Request::builder()
        .method("GET")
        .uri("/a2a/v1/tasks/00000000-0000-0000-0000-000000000099")
        .header("A2A-Version", "1.0")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
