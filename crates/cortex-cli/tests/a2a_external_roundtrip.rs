//! External role dispatch_sync collects decoded replies from remote A2A tasks.

use cortex_a2a::manifest::RoleManifestRegistry;
use cortex_a2a::runtime::{RoleContext, RoleGateway, build_runners};
use cortex_a2a::services::{NullA2aServices, SharedA2aServices};
use cortex_a2a::{A2aBus, A2aEnvelope, A2aPayload, AgentRole};
use cortex_core::{A2aConfig, A2aRoleConfig, A2aRoleMode};
use httpmock::MockServer;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[test]
fn a2a_config_force_in_process_defaults_false() {
    assert!(!A2aConfig::default().force_in_process);
}

#[tokio::test]
async fn external_dispatch_sync_returns_decoded_replies() {
    let server = MockServer::start();
    let task_id = "550e8400-e29b-41d4-a716-446655440001";
    let context_id = "550e8400-e29b-41d4-a716-446655440000";

    let insight_data = json!({
        "codecortexRole": "patch_planner",
        "targetRole": "analyzer",
        "payload": {
            "type": "code_insight",
            "summary": "external patch proposal",
            "target_qualified_name": "src/transport.rs",
            "risk_level": "low",
            "suggested_action": "ordered_mutex"
        }
    });

    server.mock(|when, then| {
        when.method(httpmock::Method::POST)
            .path("/a2a/v1/message:send");
        then.status(200).json_body(json!({
            "task": {
                "id": task_id,
                "contextId": context_id,
                "status": { "state": "TASK_STATE_SUBMITTED" },
                "artifacts": [],
                "history": []
            }
        }));
    });

    server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path(format!("/a2a/v1/tasks/{task_id}"));
        then.status(200).json_body(json!({
            "id": task_id,
            "contextId": context_id,
            "status": { "state": "TASK_STATE_COMPLETED" },
            "artifacts": [{
                "artifactId": "artifact-1",
                "name": "codecortex.result",
                "parts": [{
                    "data": insight_data,
                    "mediaType": "application/vnd.codecortex.a2a+json"
                }]
            }],
            "history": []
        }));
    });

    let mut config = A2aConfig::default();
    config.roles.insert(
        "patch_planner".to_string(),
        A2aRoleConfig {
            mode: A2aRoleMode::External,
            agent_card_url: Some(format!(
                "{}/.well-known/agents/patch-planner.json",
                server.base_url()
            )),
            ..Default::default()
        },
    );

    let bus = A2aBus::new();
    let gateway = RoleGateway::new(config.clone(), bus, build_runners());

    let conversation_id = Uuid::parse_str(context_id).unwrap();
    let task_uuid = Uuid::parse_str(task_id).unwrap();

    let envelope = A2aEnvelope::new(
        conversation_id,
        AgentRole::Gateway,
        AgentRole::PatchPlanner,
        A2aPayload::TaskDelegation {
            task_description: "Fix deadlock".to_string(),
            context_capsule_uri: "codecortex://session/x/capsule".to_string(),
        },
    )
    .with_task_id(task_uuid);

    let ctx = RoleContext {
        config: config.clone(),
        services: Arc::new(NullA2aServices) as SharedA2aServices,
        manifests: Arc::new(RoleManifestRegistry::load(&config, None)),
        session_id: "session-1".to_string(),
        conversation_id,
        task_id: task_uuid,
        task: "external roundtrip".to_string(),
        budget_tokens: 6000,
        include_paths: vec!["src/transport.rs".to_string()],
        exclude_paths: Vec::new(),
        target_symbol: None,
        source_branch: None,
        target_branch: None,
        mode: None,
        repo_root: None,
        blackboard: None,
        force_in_process: false,
    };

    let replies = gateway.dispatch_sync(envelope, &ctx).await.unwrap();
    assert!(!replies.is_empty(), "expected external replies");
    assert!(
        matches!(replies[0].payload, A2aPayload::CodeInsight { .. }),
        "expected CodeInsight payload"
    );
    assert_eq!(replies[0].sender, AgentRole::PatchPlanner);
}
