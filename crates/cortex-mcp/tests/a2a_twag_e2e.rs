//! A2A real-repo E2E on a small TWAG diff scope (consensus_review + pr_review).
//!
//! Gated by `CORTEX_TEST_TWAG=1` and `CORTEX_TEST_GRAPH=1`.

use cortex_a2a::task_store::SledTaskStore;
use cortex_a2a::wire::TaskStateWire;
use cortex_a2a::{A2aHub, SpawnSessionRequest};
use cortex_core::{A2aConfig, A2aTaskStoreKind, CortexConfig, McpToolsConfig};
use cortex_graph::GraphClient;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

mod twag_common;

use twag_common::{rdiameter_repo, twag_repo};

fn twag_gate() -> bool {
    twag_common::skip_unless_twag_graph()
}

async fn build_twag_hub(task_store_path: PathBuf) -> A2aHub {
    let mut config = CortexConfig::default();
    config.a2a = A2aConfig {
        enabled: true,
        force_in_process: true,
        task_store: A2aTaskStoreKind::Sled,
        task_store_path,
        ..A2aConfig::default()
    };
    // Avoid running TWAG `./build.sh` during workflow E2E (minutes-long CMake).
    config.a2a.validate.command = vec!["/bin/true".to_string()];
    config.mcp.tools = McpToolsConfig {
        a2a_spawn_session: true,
        ..McpToolsConfig::default()
    };

    let blackboard = if config.a2a.blackboard.enabled {
        GraphClient::connect(&config).await.ok().map(|client| {
            Arc::new(cortex_graph::BlackboardWriter::new(
                client,
                config.a2a.blackboard_write_batch_size(64),
            ))
        })
    } else {
        None
    };

    A2aHub::with_options(
        config.a2a.clone(),
        Arc::new(cortex_mcp::a2a_services::McpA2aServices::new(config)),
        blackboard,
        Some(PathBuf::from(twag_repo())),
    )
}

async fn poll_until_terminal(hub: &A2aHub, task_id: &str) -> cortex_a2a::wire::TaskWire {
    for _ in 0..120 {
        if let Ok(task) = hub.get_task_wire(task_id) {
            if matches!(
                task.status.state,
                TaskStateWire::TaskStateCompleted | TaskStateWire::TaskStateFailed
            ) {
                return task;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    hub.get_task_wire(task_id).expect("task should exist")
}

fn workflow_artifact(task: &cortex_a2a::wire::TaskWire) -> Value {
    for artifact in &task.artifacts {
        if let Some(data) = artifact.parts.first().and_then(|p| p.data.as_ref()) {
            if data.get("workflow").and_then(Value::as_str).is_some() {
                return data.clone();
            }
            if data.get("status").and_then(Value::as_str) == Some("completed") {
                return data.clone();
            }
            if data.get("patch").is_some() || data.get("delta_context").is_some() {
                return data.clone();
            }
        }
    }
    task.artifacts
        .iter()
        .find_map(|a| a.parts.first().and_then(|p| p.data.clone()))
        .expect("workflow artifact")
}

#[tokio::test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, indexed TWAG"]
async fn twag_consensus_review_completes_with_sled_store() {
    if !twag_gate() {
        return;
    }

    let store_dir = TempDir::new().expect("tempdir");
    let store_path = store_dir.path().join("a2a-tasks.db");
    let hub = build_twag_hub(store_path.clone()).await;

    let mut req = SpawnSessionRequest::with_scope(
        "Review Orchestrator::snapshot call sites in orchestrator.cpp",
        "consensus_review",
        vec!["components/cp/src/orchestrator.cpp".to_string()],
        6000,
    );
    req.wait_for_completion = true;
    req.return_immediately = false;

    let resp = hub
        .spawn_session_async(req)
        .await
        .expect("spawn consensus_review");

    let task = poll_until_terminal(&hub, &resp.task_id).await;
    assert!(
        matches!(task.status.state, TaskStateWire::TaskStateCompleted),
        "expected completed, got {:?}",
        task.status.state
    );

    let artifact = workflow_artifact(&task);
    assert_eq!(
        artifact.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert!(
        artifact.get("patch").is_some(),
        "artifact should include patch"
    );

    drop(hub);
    let sled = SledTaskStore::open(&store_path).expect("reopen sled store");
    let task_uuid = uuid::Uuid::parse_str(&resp.task_id).expect("task uuid");
    assert!(
        sled.task_store().get(&task_uuid).is_some(),
        "sled task_store should persist task"
    );
}

#[tokio::test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, indexed TWAG"]
async fn twag_pr_review_completes_with_delta_artifact() {
    if !twag_gate() {
        return;
    }

    let store_dir = TempDir::new().expect("tempdir");
    let store_path = store_dir.path().join("a2a-tasks.db");
    let hub = build_twag_hub(store_path.clone()).await;

    let rd = rdiameter_repo();
    let twag = twag_repo();
    let rel = rd
        .strip_prefix(&twag)
        .expect("rdiameter under twag")
        .to_string();

    let mut req = SpawnSessionRequest::with_scope(
        "Review rdiameter-core relay changes",
        "pr_review",
        vec![rel],
        6000,
    );
    req.source_branch = Some("HEAD".to_string());
    req.target_branch = Some("main".to_string());
    req.wait_for_completion = true;
    req.return_immediately = false;

    let resp = hub.spawn_session_async(req).await.expect("spawn pr_review");
    let task = poll_until_terminal(&hub, &resp.task_id).await;
    assert!(
        matches!(task.status.state, TaskStateWire::TaskStateCompleted),
        "expected completed, got {:?}",
        task.status.state
    );

    let artifact = workflow_artifact(&task);
    assert!(
        artifact.get("workflow").and_then(Value::as_str) == Some("pr_review")
            || artifact.get("delta_context").is_some()
            || artifact.get("summary").and_then(Value::as_str).is_some(),
        "pr_review artifact should include review payload: {artifact}"
    );

    drop(hub);
    let sled = SledTaskStore::open(&store_path).expect("reopen sled store");
    let task_uuid = uuid::Uuid::parse_str(&resp.task_id).expect("task uuid");
    assert!(sled.task_store().get(&task_uuid).is_some());
}
