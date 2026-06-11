//! Blackboard writes for all A2A insight/mutation payload types (requires live graph).

use cortex_a2a::{A2aEnvelope, A2aPayload, AgentRole, RiskLevel, blackboard_from_envelope};
use cortex_core::CortexConfig;
use cortex_graph::{BlackboardWriter, GraphClient};
use serde_json::json;
use uuid::Uuid;

async fn count_mutation_hints(client: &GraphClient, session_id: &str) -> usize {
    let rows = client
        .query_with_params(
            "MATCH (s:A2aSession {id: $sid})-[r:MUTATION_HINT]->() RETURN count(r) AS c",
            vec![("sid", session_id.to_string())],
        )
        .await
        .expect("mutation count query");
    rows.first()
        .and_then(|r| r.get("c"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize
}

async fn seed_code_node(client: &GraphClient, path: &str) {
    client
        .query_with_params(
            "MERGE (c:CodeNode {path: $path}) SET c.qualified_name = $path",
            vec![("path", path.to_string())],
        )
        .await
        .expect("seed CodeNode");
}

#[tokio::test]
#[ignore = "requires graph backend at CORTEX_TEST_GRAPH=1"]
async fn blackboard_all_payload_types() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }

    let config = CortexConfig::default();
    let client = GraphClient::connect(&config).await.expect("connect");
    let writer = BlackboardWriter::new(client.clone(), config.a2a.blackboard.write_batch_size);
    writer.ensure_schema().await.expect("schema");

    let session_id = format!("payload-test-{}", Uuid::new_v4());
    let conversation_id = Uuid::new_v4();
    writer
        .upsert_session(&session_id, &conversation_id.to_string(), "working")
        .await
        .expect("session");

    let test_path = "crates/cortex-a2a/src/lib.rs";
    seed_code_node(&client, test_path).await;

    let payloads: Vec<(A2aPayload, AgentRole)> = vec![
        (
            A2aPayload::TaskDelegation {
                task_description: "analyze transport deadlock".to_string(),
                context_capsule_uri: format!("codecortex://session/{session_id}/capsule"),
            },
            AgentRole::PatchPlanner,
        ),
        (
            A2aPayload::StrategyProposal {
                estimated_complexity: 3,
                required_sub_nodes: vec!["transport::ordered_mutex".to_string()],
            },
            AgentRole::PatchPlanner,
        ),
        (
            A2aPayload::CodeInsight {
                summary: "cycle detected in module graph".to_string(),
                target_qualified_name: "cortex_a2a::hub".to_string(),
                risk_level: RiskLevel::Medium,
                suggested_action: "review".to_string(),
            },
            AgentRole::Analyzer,
        ),
        (
            A2aPayload::Reject {
                reason: "freshness stale on affected paths".to_string(),
            },
            AgentRole::Validator,
        ),
        (A2aPayload::Accept, AgentRole::PrReviewer),
        (
            A2aPayload::GraphMutationSignal {
                event_type: "index_promoted".to_string(),
                affected_files: vec![test_path.to_string()],
            },
            AgentRole::Gateway,
        ),
        (
            A2aPayload::FinalResult {
                data: json!({
                    "status": "completed",
                    "patch": {"files": [{"path": "src/lib.rs", "content": "// huge patch body"}]},
                    "rounds": 2,
                    "task": "consensus review",
                }),
            },
            AgentRole::Gateway,
        ),
    ];

    for (payload, sender) in &payloads {
        let envelope = A2aEnvelope::new(
            conversation_id,
            *sender,
            AgentRole::Gateway,
            payload.clone(),
        );
        blackboard_from_envelope(Some(&writer), &session_id, &envelope)
            .await
            .expect("blackboard write");
    }

    let insight_count = writer.count_insights(&session_id).await.expect("count");
    assert_eq!(
        insight_count, 6,
        "TaskDelegation, StrategyProposal, CodeInsight, Reject, Accept, and FinalResult should each write an AgentInsight"
    );

    let insights = writer.list_insights(&session_id).await.expect("list");
    let final_insight = insights
        .iter()
        .find(|i| i.suggested_action == "final_result")
        .expect("FinalResult insight");
    assert_eq!(final_insight.summary, "completed");
    assert!(
        !final_insight.summary.contains("patch"),
        "summary must be status only, not full patch JSON"
    );

    let mutation_count = count_mutation_hints(&client, &session_id).await;
    assert_eq!(
        mutation_count, 1,
        "GraphMutationSignal should create one MUTATION_HINT edge per affected file"
    );
}
