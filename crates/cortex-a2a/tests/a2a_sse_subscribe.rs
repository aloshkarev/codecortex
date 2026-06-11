//! SSE task subscription receives terminal task wire from the hub.

use cortex_a2a::{A2aHub, SpawnSessionRequest, TaskStateWire};
use cortex_core::A2aConfig;
use uuid::Uuid;

#[tokio::test]
async fn subscribe_receives_completed_task() {
    let config = A2aConfig {
        enabled: true,
        force_in_process: true,
        consensus_max_rounds: 3,
        ..A2aConfig::default()
    };
    let hub = A2aHub::new(config);
    let resp = hub
        .spawn_session(SpawnSessionRequest::with_scope(
            "Fix deadlock in src/transport.rs",
            "consensus_review",
            vec!["src/transport.rs".to_string()],
            4000,
        ))
        .expect("spawn");

    let task_id = Uuid::parse_str(&resp.task_id).expect("uuid");
    let mut rx = hub.subscribe_task(&task_id);

    let mut saw_terminal = false;
    for _ in 0..80 {
        if let Ok(stream) = rx.try_recv() {
            if let Some(task) = stream.task {
                if matches!(task.status.state, TaskStateWire::TaskStateCompleted) {
                    saw_terminal = true;
                    break;
                }
            }
        }
        if let Ok(task) = hub.get_task_wire(&resp.task_id) {
            if matches!(task.status.state, TaskStateWire::TaskStateCompleted) {
                saw_terminal = true;
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        saw_terminal,
        "subscribe or poll should observe completed task"
    );
}
