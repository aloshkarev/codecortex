//! Golden checks: hand-written wire types align with prost schema from `docs/a2a.proto`.

use chrono::Utc;
use cortex_a2a::envelope::A2aEnvelope;
use cortex_a2a::payload::A2aPayload;
use cortex_a2a::proto::lf::a2a::v1::{
    SendMessageConfiguration, SendMessageRequest, Task, TaskState, TaskStatus,
};
use cortex_a2a::roles::AgentRole;
use cortex_a2a::session::{A2aTaskRecord, TaskState as SessionTaskState};
use cortex_a2a::spec_codec::{task_record_to_proto, task_wire_to_proto, task_wire_to_spec_json};
use cortex_a2a::wire::{TaskStateWire, TaskStatusWire, TaskWire};
use cortex_a2a::{A2aHub, SpawnSessionRequest};
use cortex_core::A2aConfig;
use prost::Message;
use uuid::Uuid;

#[test]
fn task_wire_fields_match_proto() {
    let wire = TaskWire {
        id: "task-1".to_string(),
        context_id: Some("ctx-1".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateCompleted,
            message: None,
        },
        artifacts: vec![],
        history: vec![],
        metadata: None,
    };
    let json = serde_json::to_value(&wire).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some("task-1"));

    let proto = Task {
        id: wire.id.clone(),
        context_id: wire.context_id.clone().unwrap_or_default(),
        status: Some(TaskStatus {
            state: TaskState::Completed as i32,
            message: None,
            timestamp: None,
        }),
        artifacts: vec![],
        history: vec![],
        metadata: None,
    };
    let encoded = proto.encode_to_vec();
    let decoded = Task::decode(encoded.as_slice()).unwrap();
    assert_eq!(decoded.id, "task-1");
}

#[test]
fn send_message_configuration_proto_roundtrip() {
    let proto = SendMessageConfiguration {
        accepted_output_modes: vec![],
        task_push_notification_config: None,
        history_length: None,
        return_immediately: true,
    };
    let bytes = proto.encode_to_vec();
    let back = SendMessageConfiguration::decode(bytes.as_slice()).unwrap();
    assert!(back.return_immediately);

    let proto_req = SendMessageRequest {
        tenant: String::new(),
        message: None,
        configuration: Some(proto),
        metadata: None,
    };
    assert!(proto_req.configuration.is_some());
}

#[test]
fn task_spec_json_uses_camel_case() {
    let wire = TaskWire {
        id: "task-1".to_string(),
        context_id: Some("ctx-1".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateWorking,
            message: None,
        },
        artifacts: vec![],
        history: vec![],
        metadata: None,
    };
    let json = task_wire_to_spec_json(&wire).unwrap();
    assert!(json.get("contextId").is_some());
    assert_eq!(
        json.get("status")
            .and_then(|s| s.get("state"))
            .and_then(|v| v.as_str()),
        Some("TASK_STATE_WORKING")
    );
}

#[test]
fn wire_proto_roundtrip_preserves_task_id() {
    let wire = TaskWire {
        id: "t-round".to_string(),
        context_id: Some("c-round".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateSubmitted,
            message: None,
        },
        artifacts: vec![],
        history: vec![],
        metadata: None,
    };
    let proto = task_wire_to_proto(&wire);
    let encoded = proto.encode_to_vec();
    let decoded = Task::decode(encoded.as_slice()).unwrap();
    assert_eq!(decoded.id, "t-round");
    assert_eq!(decoded.context_id, "c-round");
}

#[tokio::test]
async fn spawn_registers_task_in_store() {
    let config = A2aConfig {
        enabled: true,
        force_in_process: true,
        consensus_max_rounds: 1,
        ..A2aConfig::default()
    };
    let hub = A2aHub::new(config);
    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "plan patch",
            "patch_plan",
            vec!["src".to_string()],
            2000,
        ))
        .expect("spawn");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let proto = task_record_to_proto(
        &hub.tasks
            .get(&uuid::Uuid::parse_str(&resp.task_id).unwrap())
            .expect("record"),
    );
    assert_eq!(proto.id, resp.task_id);
}

#[tokio::test]
async fn get_task_history_from_recorded_events_truncates() {
    let hub = A2aHub::new(A2aConfig {
        enabled: true,
        ..A2aConfig::default()
    });
    let task_id = Uuid::new_v4();
    let context_id = Uuid::new_v4();
    hub.tasks.insert(A2aTaskRecord {
        id: task_id,
        context_id,
        state: SessionTaskState::Working,
        workflow: "patch_plan".to_string(),
        goal: "test history".to_string(),
        artifacts: Vec::new(),
        metadata: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        result: None,
        error: None,
    });

    for i in 0..3 {
        let env = A2aEnvelope::new(
            context_id,
            AgentRole::Gateway,
            AgentRole::PatchPlanner,
            A2aPayload::TaskDelegation {
                task_description: format!("step {i}"),
                context_capsule_uri: "codecortex://test".to_string(),
            },
        )
        .with_task_id(task_id);
        hub.record_event(env).await;
    }

    let wire = hub
        .get_task_wire_with_history(&task_id.to_string(), Some(2))
        .expect("get task");
    assert_eq!(wire.history.len(), 2);
    assert_eq!(
        wire.history[0]
            .parts
            .first()
            .and_then(|p| p.data.as_ref())
            .and_then(|d| d.get("payload"))
            .and_then(|p| p.get("task_description"))
            .and_then(|v| v.as_str()),
        Some("step 1")
    );
    assert_eq!(
        wire.history[1]
            .parts
            .first()
            .and_then(|p| p.data.as_ref())
            .and_then(|d| d.get("payload"))
            .and_then(|p| p.get("task_description"))
            .and_then(|v| v.as_str()),
        Some("step 2")
    );

    let full = hub
        .get_task_wire_with_history(&task_id.to_string(), None)
        .expect("get task");
    assert_eq!(full.history.len(), 3);
}

#[tokio::test]
async fn spawn_workflow_populates_task_history() {
    let config = A2aConfig {
        enabled: true,
        force_in_process: true,
        consensus_max_rounds: 1,
        ..A2aConfig::default()
    };
    let hub = A2aHub::new(config);
    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "plan patch",
            "patch_plan",
            vec!["src".to_string()],
            2000,
        ))
        .expect("spawn");

    for _ in 0..50 {
        if let Ok(task) = hub.get_task_wire_with_history(&resp.task_id, None) {
            if matches!(task.status.state, TaskStateWire::TaskStateCompleted)
                && !task.history.is_empty()
            {
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let task = hub
        .get_task_wire_with_history(&resp.task_id, None)
        .expect("task");
    assert!(
        !task.history.is_empty(),
        "patch_plan workflow should record task-scoped envelopes"
    );
}

#[test]
fn task_metadata_roundtrips_proto() {
    use cortex_a2a::spec_codec::{task_proto_to_wire, task_wire_to_proto};
    use serde_json::json;

    let wire = TaskWire {
        id: "meta-task".to_string(),
        context_id: Some("ctx-meta".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateWorking,
            message: None,
        },
        artifacts: vec![],
        history: vec![],
        metadata: Some(json!({
            "workflow": "impact_review",
            "suggestedNextTools": ["get_impact_graph"],
            "freshness": "fresh",
        })),
    };
    let proto = task_wire_to_proto(&wire);
    let back = task_proto_to_wire(&proto);
    assert_eq!(
        back.metadata.as_ref().and_then(|m| m.get("workflow")),
        Some(&json!("impact_review"))
    );
}

#[test]
fn intelligence_data_part_roundtrips_proto() {
    use cortex_a2a::spec_codec::{task_proto_to_wire, task_wire_to_proto};
    use cortex_a2a::wire::{A2aPart, ArtifactWire};
    use serde_json::json;

    let wire = TaskWire {
        id: "art-task".to_string(),
        context_id: Some("ctx-art".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateCompleted,
            message: None,
        },
        artifacts: vec![ArtifactWire {
            artifact_id: "t1/intelligence_pack/get_impact_graph".to_string(),
            name: Some("codecortex.get_impact_graph".to_string()),
            description: None,
            parts: vec![A2aPart {
                text: None,
                data: Some(json!({"symbol": "foo", "nodes": []})),
                metadata: None,
                media_type: Some("application/json".to_string()),
            }],
            metadata: Some(json!({"mcpToolId": "get_impact_graph"})),
            extensions: vec![
                "https://codecortex.dev/extensions/intelligence-cooperation/v1".to_string(),
            ],
        }],
        history: vec![],
        metadata: None,
    };
    let proto = task_wire_to_proto(&wire);
    let back = task_proto_to_wire(&proto);
    let data = back.artifacts[0].parts[0].data.as_ref().expect("data part");
    assert_eq!(data.get("symbol"), Some(&json!("foo")));
}

#[test]
fn get_task_omits_artifacts_when_flag_false() {
    use cortex_a2a::spec_codec::task_wire_to_spec_json_with_options;
    use cortex_a2a::wire::{A2aPart, ArtifactWire};
    use serde_json::json;

    let wire = TaskWire {
        id: "omit-art".to_string(),
        context_id: Some("ctx".to_string()),
        status: TaskStatusWire {
            state: TaskStateWire::TaskStateCompleted,
            message: None,
        },
        artifacts: vec![ArtifactWire {
            artifact_id: "a1".to_string(),
            name: None,
            description: None,
            parts: vec![A2aPart {
                text: None,
                data: Some(json!({"x": 1})),
                metadata: None,
                media_type: Some("application/json".to_string()),
            }],
            metadata: None,
            extensions: vec![],
        }],
        history: vec![],
        metadata: None,
    };
    let json = task_wire_to_spec_json_with_options(&wire, false).unwrap();
    assert!(json.get("artifacts").is_none());
}
