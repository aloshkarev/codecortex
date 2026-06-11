use cortex_a2a::{A2aEnvelope, A2aPayload, AgentRole, envelope_to_message, message_to_envelope};
use uuid::Uuid;

#[test]
fn golden_envelope_roundtrip() {
    let env = A2aEnvelope::new(
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        AgentRole::Analyzer,
        AgentRole::PatchPlanner,
        A2aPayload::Reject {
            reason: "lock ordering".to_string(),
        },
    );
    let msg = envelope_to_message(&env);
    let back = message_to_envelope(&msg).expect("roundtrip");
    assert_eq!(back.sender, AgentRole::Analyzer);
    assert_eq!(back.receiver, AgentRole::PatchPlanner);
}

#[test]
fn strategy_proposal_envelope_roundtrip() {
    let env = A2aEnvelope::new(
        Uuid::new_v4(),
        AgentRole::PatchPlanner,
        AgentRole::Analyzer,
        A2aPayload::StrategyProposal {
            estimated_complexity: 3,
            required_sub_nodes: vec!["analyzer".to_string(), "validator".to_string()],
        },
    );
    let msg = envelope_to_message(&env);
    let back = message_to_envelope(&msg).expect("roundtrip");
    assert_eq!(back.sender, AgentRole::PatchPlanner);
    assert_eq!(back.receiver, AgentRole::Analyzer);
    match back.payload {
        A2aPayload::StrategyProposal {
            estimated_complexity,
            required_sub_nodes,
        } => {
            assert_eq!(estimated_complexity, 3);
            assert_eq!(required_sub_nodes, vec!["analyzer", "validator"]);
        }
        other => panic!("expected StrategyProposal, got {other:?}"),
    }
}
