//! HTTP binding conformance helpers (spec errors and workflow registry).

use cortex_a2a::{A2aErrorBody, A2aErrorKind, A2aHub, SpawnSessionRequest};
use cortex_core::A2aConfig;

#[test]
fn spec_error_status_codes() {
    assert_eq!(
        A2aErrorBody::new(A2aErrorKind::TaskNotFoundError, "x").status_code(),
        404
    );
    assert_eq!(
        A2aErrorBody::new(A2aErrorKind::VersionNotSupportedError, "x").status_code(),
        400
    );
}

#[test]
fn disabled_workflow_rejected_at_spawn() {
    let mut config = A2aConfig {
        enabled: true,
        ..A2aConfig::default()
    };
    config.workflows.patch_plan.enabled = false;
    let hub = A2aHub::new(config);
    let err = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "plan",
            "patch_plan",
            vec![],
            1000,
        ))
        .unwrap_err();
    assert!(err.to_string().contains("disabled"));
}

#[test]
fn push_config_id_distinct_from_task() {
    let mut config = A2aConfig {
        enabled: true,
        ..A2aConfig::default()
    };
    config.push.enabled = true;
    let hub = A2aHub::new(config);
    let saved = hub
        .push()
        .create_config(cortex_a2a::TaskPushNotificationConfig {
            id: String::new(),
            task_id: "task-a".to_string(),
            url: "http://127.0.0.1/hook".to_string(),
            token: None,
        });
    assert!(!saved.id.is_empty());
    assert_eq!(saved.task_id, "task-a");
    assert_eq!(hub.push().list_for_task("task-a").len(), 1);
}
