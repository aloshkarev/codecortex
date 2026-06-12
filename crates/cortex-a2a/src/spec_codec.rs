//! Conversions between internal [`wire`] types and normative [`proto`] types (JSON via pbjson).

use crate::proto::lf::a2a::v1::{
    ListTasksResponse, SendMessageConfiguration, SendMessageRequest, SendMessageResponse,
    StreamResponse, Task, TaskArtifactUpdateEvent, TaskState, TaskStatus, TaskStatusUpdateEvent,
    part, send_message_response, stream_response,
};
use crate::session::{A2aTaskRecord, TaskState as StoreState};
use crate::wire::{
    A2aMessage, A2aPart, ArtifactWire, SendMessageConfigurationWire, SendMessageRequestWire,
    SendMessageResponseWire, StreamResponseWire, TaskStateWire, TaskStatusWire, TaskWire,
};
use anyhow::{Result, anyhow};
use prost_types::Value as ProstValue;
use prost_types::value::Kind;
use serde_json::Value;

fn json_value_to_prost(v: Value) -> ProstValue {
    let kind = match v {
        Value::Null => Kind::NullValue(0),
        Value::Bool(b) => Kind::BoolValue(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Kind::NumberValue(i as f64)
            } else {
                Kind::NumberValue(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => Kind::StringValue(s),
        Value::Array(a) => Kind::ListValue(prost_types::ListValue {
            values: a.into_iter().map(json_value_to_prost).collect(),
        }),
        Value::Object(o) => {
            let mut fields = std::collections::BTreeMap::new();
            for (k, val) in o {
                fields.insert(k, json_value_to_prost(val));
            }
            Kind::StructValue(prost_types::Struct { fields })
        }
    };
    ProstValue { kind: Some(kind) }
}

pub fn task_state_wire_to_proto(state: TaskStateWire) -> i32 {
    let s = match state {
        TaskStateWire::TaskStateUnspecified => TaskState::Unspecified,
        TaskStateWire::TaskStateSubmitted => TaskState::Submitted,
        TaskStateWire::TaskStateWorking => TaskState::Working,
        TaskStateWire::TaskStateCompleted => TaskState::Completed,
        TaskStateWire::TaskStateFailed => TaskState::Failed,
        TaskStateWire::TaskStateCanceled => TaskState::Canceled,
        TaskStateWire::TaskStateRejected => TaskState::Rejected,
        TaskStateWire::TaskStateInputRequired => TaskState::InputRequired,
        TaskStateWire::TaskStateAuthRequired => TaskState::AuthRequired,
    };
    s as i32
}

pub fn task_state_proto_to_wire(state: i32) -> TaskStateWire {
    match TaskState::try_from(state) {
        Ok(TaskState::Submitted) => TaskStateWire::TaskStateSubmitted,
        Ok(TaskState::Working) => TaskStateWire::TaskStateWorking,
        Ok(TaskState::Completed) => TaskStateWire::TaskStateCompleted,
        Ok(TaskState::Failed) => TaskStateWire::TaskStateFailed,
        Ok(TaskState::Canceled) => TaskStateWire::TaskStateCanceled,
        Ok(TaskState::Rejected) => TaskStateWire::TaskStateRejected,
        Ok(TaskState::InputRequired) => TaskStateWire::TaskStateInputRequired,
        Ok(TaskState::AuthRequired) => TaskStateWire::TaskStateAuthRequired,
        _ => TaskStateWire::TaskStateUnspecified,
    }
}

pub fn store_state_to_proto(state: StoreState) -> i32 {
    task_state_wire_to_proto(state.into())
}

fn prost_value_to_json(v: &ProstValue) -> Value {
    match &v.kind {
        Some(Kind::NullValue(_)) => Value::Null,
        Some(Kind::BoolValue(b)) => Value::Bool(*b),
        Some(Kind::NumberValue(n)) => {
            serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number)
        }
        Some(Kind::StringValue(s)) => Value::String(s.clone()),
        Some(Kind::ListValue(list)) => {
            Value::Array(list.values.iter().map(prost_value_to_json).collect())
        }
        Some(Kind::StructValue(st)) => {
            let mut map = serde_json::Map::new();
            for (k, val) in &st.fields {
                map.insert(k.clone(), prost_value_to_json(val));
            }
            Value::Object(map)
        }
        None => Value::Null,
    }
}

fn optional_json_to_struct(v: &Option<Value>) -> Option<prost_types::Struct> {
    v.as_ref().and_then(|val| {
        if let Value::Object(o) = val {
            let mut fields = std::collections::BTreeMap::new();
            for (k, val) in o {
                fields.insert(k.clone(), json_value_to_prost(val.clone()));
            }
            Some(prost_types::Struct { fields })
        } else {
            None
        }
    })
}

fn struct_to_json(st: &prost_types::Struct) -> Value {
    let mut map = serde_json::Map::new();
    for (k, val) in &st.fields {
        map.insert(k.clone(), prost_value_to_json(val));
    }
    Value::Object(map)
}

fn part_proto_to_wire(p: &crate::proto::lf::a2a::v1::Part) -> A2aPart {
    let (text, data) = match &p.content {
        Some(part::Content::Text(t)) => (Some(t.clone()), None),
        Some(part::Content::Data(d)) => (None, Some(prost_value_to_json(d))),
        Some(part::Content::Raw(_)) | Some(part::Content::Url(_)) => (None, None),
        None => (None, None),
    };
    A2aPart {
        text,
        data,
        metadata: None,
        media_type: if p.media_type.is_empty() {
            None
        } else {
            Some(p.media_type.clone())
        },
    }
}

fn artifact_wire_to_proto(a: &ArtifactWire) -> crate::proto::lf::a2a::v1::Artifact {
    crate::proto::lf::a2a::v1::Artifact {
        artifact_id: a.artifact_id.clone(),
        name: a.name.clone().unwrap_or_default(),
        description: a.description.clone().unwrap_or_default(),
        parts: a.parts.iter().map(part_wire_to_proto).collect(),
        metadata: optional_json_to_struct(&a.metadata),
        extensions: a.extensions.clone(),
    }
}

fn artifact_proto_to_wire(a: &crate::proto::lf::a2a::v1::Artifact) -> ArtifactWire {
    ArtifactWire {
        artifact_id: a.artifact_id.clone(),
        name: if a.name.is_empty() {
            None
        } else {
            Some(a.name.clone())
        },
        description: if a.description.is_empty() {
            None
        } else {
            Some(a.description.clone())
        },
        parts: a.parts.iter().map(part_proto_to_wire).collect(),
        metadata: a.metadata.as_ref().map(struct_to_json),
        extensions: a.extensions.clone(),
    }
}

fn part_wire_to_proto(p: &A2aPart) -> crate::proto::lf::a2a::v1::Part {
    let content = if let Some(t) = &p.text {
        Some(part::Content::Text(t.clone()))
    } else if let Some(d) = &p.data {
        let pv = json_value_to_prost(d.clone());
        Some(part::Content::Data(pv))
    } else {
        None
    };
    crate::proto::lf::a2a::v1::Part {
        metadata: None,
        filename: String::new(),
        media_type: p.media_type.clone().unwrap_or_default(),
        content,
    }
}

pub fn message_wire_to_proto(m: &A2aMessage) -> crate::proto::lf::a2a::v1::Message {
    let role = if m.role == "user" {
        crate::proto::lf::a2a::v1::Role::User as i32
    } else {
        crate::proto::lf::a2a::v1::Role::Agent as i32
    };
    crate::proto::lf::a2a::v1::Message {
        message_id: m.message_id.clone(),
        context_id: m.context_id.clone().unwrap_or_default(),
        task_id: m.task_id.clone().unwrap_or_default(),
        role,
        parts: m.parts.iter().map(part_wire_to_proto).collect(),
        metadata: None,
        extensions: m.extensions.clone(),
        reference_task_ids: vec![],
    }
}

pub fn task_wire_to_proto(wire: &TaskWire) -> Task {
    Task {
        id: wire.id.clone(),
        context_id: wire.context_id.clone().unwrap_or_default(),
        status: Some(TaskStatus {
            state: task_state_wire_to_proto(wire.status.state.clone()),
            message: wire.status.message.as_ref().map(message_wire_to_proto),
            timestamp: None,
        }),
        artifacts: wire.artifacts.iter().map(artifact_wire_to_proto).collect(),
        history: wire.history.iter().map(message_wire_to_proto).collect(),
        metadata: optional_json_to_struct(&wire.metadata),
    }
}

pub fn task_record_to_proto(record: &A2aTaskRecord) -> Task {
    task_wire_to_proto(&record.to_wire())
}

pub fn task_proto_to_wire(task: &Task) -> TaskWire {
    let status = task.status.as_ref();
    TaskWire {
        id: task.id.clone(),
        context_id: if task.context_id.is_empty() {
            None
        } else {
            Some(task.context_id.clone())
        },
        status: TaskStatusWire {
            state: status
                .map(|s| task_state_proto_to_wire(s.state))
                .unwrap_or(TaskStateWire::TaskStateUnspecified),
            message: None,
        },
        artifacts: task.artifacts.iter().map(artifact_proto_to_wire).collect(),
        history: vec![],
        metadata: task.metadata.as_ref().map(struct_to_json),
    }
}

/// Spec §5.5 camelCase JSON from wire types (canonical HTTP/MCP path).
pub fn task_wire_to_spec_json(wire: &TaskWire) -> Result<Value> {
    task_wire_to_spec_json_with_options(wire, true)
}

pub fn task_wire_to_spec_json_with_options(
    wire: &TaskWire,
    include_artifacts: bool,
) -> Result<Value> {
    let mut value = serde_json::to_value(wire).map_err(|e| anyhow!("task json: {e}"))?;
    if !include_artifacts {
        if let Value::Object(map) = &mut value {
            map.remove("artifacts");
        }
    }
    Ok(value)
}

pub fn task_to_spec_json(task: &Task) -> Result<Value> {
    task_wire_to_spec_json(&task_proto_to_wire(task))
}

pub fn send_request_wire_to_proto(req: &SendMessageRequestWire) -> SendMessageRequest {
    let cfg = req.configuration.as_ref();
    SendMessageRequest {
        tenant: String::new(),
        message: Some(message_wire_to_proto(&req.message)),
        configuration: cfg.map(|c| SendMessageConfiguration {
            accepted_output_modes: vec![],
            task_push_notification_config: None,
            history_length: c.history_length,
            return_immediately: c.return_immediately,
        }),
        metadata: None,
    }
}

pub fn send_response_wire_to_proto(resp: &SendMessageResponseWire) -> SendMessageResponse {
    let payload = if let Some(t) = &resp.task {
        Some(send_message_response::Payload::Task(task_wire_to_proto(t)))
    } else if let Some(m) = &resp.message {
        Some(send_message_response::Payload::Message(
            message_wire_to_proto(m),
        ))
    } else {
        None
    };
    SendMessageResponse { payload }
}

pub fn stream_wire_to_proto(event: &StreamResponseWire) -> StreamResponse {
    if let Some(update) = &event.artifact_update {
        return StreamResponse {
            payload: Some(stream_response::Payload::ArtifactUpdate(
                TaskArtifactUpdateEvent {
                    task_id: update.task_id.clone(),
                    context_id: update.context_id.clone(),
                    artifact: Some(artifact_wire_to_proto(&update.artifact)),
                    append: update.append,
                    last_chunk: update.last_chunk,
                    metadata: None,
                },
            )),
        };
    }
    if let Some(update) = &event.status_update {
        return StreamResponse {
            payload: Some(stream_response::Payload::StatusUpdate(
                TaskStatusUpdateEvent {
                    task_id: update.task_id.clone(),
                    context_id: update.context_id.clone(),
                    status: Some(TaskStatus {
                        state: task_state_wire_to_proto(update.status.state.clone()),
                        message: update.status.message.as_ref().map(message_wire_to_proto),
                        timestamp: None,
                    }),
                    metadata: None,
                },
            )),
        };
    }
    StreamResponse {
        payload: event
            .task
            .as_ref()
            .map(|t| stream_response::Payload::Task(task_wire_to_proto(t))),
    }
}

pub fn list_tasks_response_spec(tasks: Vec<Task>, total: i32) -> ListTasksResponse {
    ListTasksResponse {
        tasks,
        next_page_token: String::new(),
        page_size: total,
        total_size: total,
    }
}

pub fn parse_send_request_json(value: Value) -> Result<SendMessageRequestWire> {
    serde_json::from_value(value).map_err(|e| anyhow!("SendMessageRequest: {e}"))
}

pub fn send_request_proto_to_wire(req: &SendMessageRequest) -> SendMessageRequestWire {
    let msg = req.message.as_ref();
    SendMessageRequestWire {
        message: msg
            .map(|m| {
                let text = m
                    .parts
                    .first()
                    .and_then(|p| match &p.content {
                        Some(part::Content::Text(t)) => Some(t.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                A2aMessage {
                    message_id: if m.message_id.is_empty() {
                        uuid::Uuid::new_v4().to_string()
                    } else {
                        m.message_id.clone()
                    },
                    context_id: if m.context_id.is_empty() {
                        None
                    } else {
                        Some(m.context_id.clone())
                    },
                    task_id: if m.task_id.is_empty() {
                        None
                    } else {
                        Some(m.task_id.clone())
                    },
                    role: if m.role == crate::proto::lf::a2a::v1::Role::User as i32 {
                        "user".to_string()
                    } else {
                        "agent".to_string()
                    },
                    parts: vec![A2aPart {
                        text: Some(text),
                        data: None,
                        metadata: None,
                        media_type: None,
                    }],
                    metadata: None,
                    extensions: m.extensions.clone(),
                }
            })
            .unwrap_or_else(|| A2aMessage {
                message_id: uuid::Uuid::new_v4().to_string(),
                context_id: None,
                task_id: None,
                role: "user".to_string(),
                parts: vec![A2aPart {
                    text: Some("A2A message".to_string()),
                    data: None,
                    metadata: None,
                    media_type: None,
                }],
                metadata: None,
                extensions: vec![],
            }),
        configuration: req
            .configuration
            .as_ref()
            .map(|c| SendMessageConfigurationWire {
                return_immediately: c.return_immediately,
                history_length: c.history_length,
            }),
    }
}
