//! HTTP+JSON A2A v1.0 bindings on the network MCP server.

use crate::NetworkState;
use axum::extract::{Extension, Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use cortex_a2a::{
    A2aErrorBody, A2aErrorKind, AgentRole, ListTasksResponseWire, SendMessageRequestWire,
    SendMessageResponseWire, StreamResponseWire, TaskPushNotificationConfig, TaskStateWire,
    TaskWire, gateway_agent_card, role_agent_card,
};
use futures_util::stream::Stream;
use serde::Deserialize;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use uuid::Uuid;

pub fn a2a_routes() -> Router {
    Router::new()
        .route("/.well-known/agent-card.json", get(gateway_card))
        // Axum allows one param per segment; strip ".json" in the handler.
        .route("/.well-known/agents/{role_file}", get(role_card))
        .route("/a2a/v1/message:send", post(send_message))
        .route("/a2a/v1/message:stream", post(send_streaming_message))
        .route("/a2a/v1/tasks", get(list_tasks))
        .route("/a2a/v1/tasks/{id}", get(get_task))
        // Colon custom verbs (`{id}:subscribe`) are invalid in axum path segments; use slash form.
        .route("/a2a/v1/tasks/{id}/subscribe", get(subscribe_task))
        .route("/a2a/v1/tasks/{id}/cancel", post(cancel_task))
        .route(
            "/a2a/v1/tasks/{id}/pushNotificationConfigs",
            post(create_push_config).get(list_push_configs),
        )
        .route(
            "/a2a/v1/tasks/{id}/pushNotificationConfigs/{config_id}",
            get(get_push_config).delete(delete_push_config),
        )
}

#[derive(Debug, Deserialize)]
struct ListTasksQuery {
    context_id: Option<String>,
    #[serde(default)]
    page_size: Option<i32>,
    page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetTaskQuery {
    history_length: Option<i32>,
}

fn base_url(listen: &str) -> String {
    format!("http://{listen}")
}

fn check_a2a_enabled(state: &NetworkState) -> Result<(), Response> {
    if !state.config.a2a.enabled || !state.config.a2a.server.http_enabled {
        return Err(a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::UnsupportedOperationError,
            "A2A HTTP disabled",
        )));
    }
    Ok(())
}

fn check_a2a_version(state: &NetworkState, headers: &HeaderMap) -> Result<(), Response> {
    if let Some(expected) = state.a2a_version_header(headers) {
        if state.config.a2a.server.protocol_version != expected {
            return Err(a2a_error_response(A2aErrorBody::new(
                A2aErrorKind::VersionNotSupportedError,
                format!(
                    "expected A2A-Version {}",
                    state.config.a2a.server.protocol_version
                ),
            )));
        }
    }
    Ok(())
}

fn check_a2a_extensions(state: &NetworkState, headers: &HeaderMap) -> Result<(), Response> {
    let required = &state.config.a2a.server.extension_uri;
    if required.is_empty() {
        return Ok(());
    }
    if let Some(raw) = headers
        .get("a2a-extensions")
        .or_else(|| headers.get("A2A-Extensions"))
        .and_then(|v| v.to_str().ok())
    {
        if raw.split(',').any(|e| e.trim() == required) {
            return Ok(());
        }
    }
    Ok(())
}

fn ingress(state: &NetworkState, headers: &HeaderMap) -> Result<(), Response> {
    check_a2a_enabled(state)?;
    check_a2a_version(state, headers)?;
    check_a2a_extensions(state, headers)?;
    Ok(())
}

async fn gateway_card(Extension(state): Extension<NetworkState>) -> impl IntoResponse {
    if !state.config.a2a.enabled {
        return (StatusCode::NOT_FOUND, "A2A disabled").into_response();
    }
    let card = gateway_agent_card(
        &state.config.a2a,
        &base_url(&state.config.mcp.network.listen),
    );
    Json(card).into_response()
}

async fn role_card(
    Extension(state): Extension<NetworkState>,
    Path(role_file): Path<String>,
) -> impl IntoResponse {
    if !state.config.a2a.enabled {
        return (StatusCode::NOT_FOUND, "A2A disabled").into_response();
    }
    let role_name = role_file
        .strip_suffix(".json")
        .unwrap_or(role_file.as_str());
    let role = match role_name.parse::<AgentRole>() {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "unknown role").into_response(),
    };
    let card = role_agent_card(
        role,
        &state.config.a2a,
        &base_url(&state.config.mcp.network.listen),
        &state.a2a_hub.mcp_tools_for_role(role.as_str()),
    );
    Json(card).into_response()
}

async fn send_message(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Json(req): Json<SendMessageRequestWire>,
) -> Result<Json<SendMessageResponseWire>, Response> {
    ingress(&state, &headers)?;
    state.a2a_hub.send_message(req).map(Json).map_err(|e| {
        a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::InvalidAgentResponseError,
            e.to_string(),
        ))
    })
}

async fn send_streaming_message(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Json(req): Json<SendMessageRequestWire>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Response> {
    ingress(&state, &headers)?;
    let resp = state.a2a_hub.send_message(req).map_err(|e| {
        a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::InvalidAgentResponseError,
            e.to_string(),
        ))
    })?;
    let task = resp.task.ok_or_else(|| {
        a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::InvalidAgentResponseError,
            "missing task",
        ))
    })?;
    let task_id = Uuid::parse_str(&task.id)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())?;
    let mut rx = state.a2a_hub.subscribe_task(&task_id);
    let stream = async_stream::stream! {
        let initial = StreamResponseWire {
            task: Some(task),
            status_update: None,
            artifact_update: None,
        };
        if let Ok(data) = serde_json::to_string(&initial) {
            yield Ok(Event::default().data(data));
        }
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok(Event::default().data(data));
                    }
                    if event.task.as_ref().is_some_and(|t| terminal_state(&t.status.state)) {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

fn terminal_state(state: &TaskStateWire) -> bool {
    matches!(
        state,
        TaskStateWire::TaskStateCompleted
            | TaskStateWire::TaskStateFailed
            | TaskStateWire::TaskStateCanceled
    )
}

async fn list_tasks(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Query(q): Query<ListTasksQuery>,
) -> Result<Json<ListTasksResponseWire>, Response> {
    ingress(&state, &headers)?;
    let mut list = state
        .a2a_hub
        .list_tasks_wire(q.context_id.as_deref())
        .map_err(|e| {
            a2a_error_response(A2aErrorBody::new(
                A2aErrorKind::InvalidAgentResponseError,
                e.to_string(),
            ))
        })?;
    let page_size = q.page_size.unwrap_or(50).max(1) as usize;
    let offset = q
        .page_token
        .as_deref()
        .and_then(|t| t.parse::<usize>().ok())
        .unwrap_or(0);
    let total = list.tasks.len();
    list.tasks = list
        .tasks
        .into_iter()
        .skip(offset)
        .take(page_size)
        .collect();
    let next = if offset + page_size < total {
        Some((offset + page_size).to_string())
    } else {
        None
    };
    Ok(Json(ListTasksResponseWire {
        next_page_token: next,
        page_size: Some(page_size as i32),
        total_size: Some(total as i32),
        tasks: list.tasks,
    }))
}

async fn get_task(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<GetTaskQuery>,
) -> Result<Json<TaskWire>, Response> {
    ingress(&state, &headers)?;
    state
        .a2a_hub
        .get_task_wire_with_history(&id, q.history_length)
        .map(Json)
        .map_err(|_| {
            a2a_error_response(A2aErrorBody::new(
                A2aErrorKind::TaskNotFoundError,
                format!("task not found: {id}"),
            ))
        })
}

async fn cancel_task(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<TaskWire>, Response> {
    ingress(&state, &headers)?;
    state.a2a_hub.cancel_task(&id).map(Json).map_err(|_| {
        a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::TaskNotFoundError,
            format!("task not found: {id}"),
        ))
    })
}

async fn subscribe_task(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Response> {
    ingress(&state, &headers)?;
    let task_id = Uuid::parse_str(&id).map_err(|e| {
        a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::TaskNotFoundError,
            e.to_string(),
        ))
    })?;
    if let Ok(wire) = state.a2a_hub.get_task_wire(&id) {
        if terminal_state(&wire.status.state) {
            return Err(a2a_error_response(A2aErrorBody::new(
                A2aErrorKind::UnsupportedOperationError,
                "task already terminal",
            )));
        }
    }
    let mut rx = state.a2a_hub.subscribe_task(&task_id);
    let hub = state.a2a_hub.clone();
    let stream = async_stream::stream! {
        if let Ok(wire) = hub.get_task_wire(&id) {
            let event = StreamResponseWire {
                task: Some(wire.clone()),
                status_update: None,
                artifact_update: None,
            };
            if let Ok(data) = serde_json::to_string(&event) {
                yield Ok(Event::default().data(data));
            }
        }
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok(Event::default().data(data));
                    }
                    if event.task.as_ref().is_some_and(|t| terminal_state(&t.status.state)) {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

async fn create_push_config(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(mut cfg): Json<TaskPushNotificationConfig>,
) -> Result<Json<TaskPushNotificationConfig>, Response> {
    ingress(&state, &headers)?;
    if !state.config.a2a.push.enabled {
        return Err(a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::PushNotificationNotSupportedError,
            "push disabled",
        )));
    }
    cfg.task_id = id;
    let saved = state.a2a_hub.push().create_config(cfg);
    Ok(Json(saved))
}

async fn list_push_configs(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Response> {
    ingress(&state, &headers)?;
    if !state.config.a2a.push.enabled {
        return Err(a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::PushNotificationNotSupportedError,
            "push disabled",
        )));
    }
    Ok(Json(
        json!({ "configs": state.a2a_hub.push().list_for_task(&id) }),
    ))
}

async fn get_push_config(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path((task_id, config_id)): Path<(String, String)>,
) -> Result<Json<TaskPushNotificationConfig>, Response> {
    ingress(&state, &headers)?;
    let cfg = state
        .a2a_hub
        .push()
        .get_config(&config_id)
        .filter(|c| c.task_id == task_id)
        .ok_or_else(|| {
            a2a_error_response(A2aErrorBody::new(
                A2aErrorKind::TaskNotFoundError,
                "push config not found",
            ))
        })?;
    Ok(Json(cfg))
}

async fn delete_push_config(
    Extension(state): Extension<NetworkState>,
    headers: HeaderMap,
    Path((_task_id, config_id)): Path<(String, String)>,
) -> Result<StatusCode, Response> {
    ingress(&state, &headers)?;
    if state.a2a_hub.push().delete_config(&config_id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(a2a_error_response(A2aErrorBody::new(
            A2aErrorKind::TaskNotFoundError,
            "push config not found",
        )))
    }
}

fn a2a_error_response(err: A2aErrorBody) -> Response {
    let status =
        StatusCode::from_u16(err.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(err)).into_response()
}

impl NetworkState {
    fn a2a_version_header(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get("a2a-version")
            .or_else(|| headers.get("A2A-Version"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }
}

pub fn merge_a2a_router(app: Router, state: NetworkState) -> Router {
    if state.config.a2a.enabled && state.config.a2a.server.http_enabled {
        app.merge(a2a_routes())
    } else {
        app
    }
}
