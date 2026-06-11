//! Bus supervisor consumes role inboxes; index promotion dispatches to Analyzer.

use cortex_a2a::{A2aEnvelope, A2aHub, A2aPayload, AgentRole};
use cortex_core::A2aConfig;
use std::time::Duration;
use uuid::Uuid;

fn test_config() -> A2aConfig {
    A2aConfig {
        enabled: true,
        force_in_process: true,
        ..A2aConfig::default()
    }
}

#[tokio::test]
async fn notify_index_promotion_dispatches_to_analyzer() {
    let hub = A2aHub::new(test_config());
    hub.notify_index_promotion("crates/cortex-a2a/src/lib.rs")
        .await;

    let events = hub.events_snapshot().await;
    let has_mutation = events.iter().any(|e| {
        matches!(
            e.payload,
            A2aPayload::GraphMutationSignal {
                event_type: ref t,
                ..
            } if t == "index_promoted"
        )
    });
    assert!(has_mutation, "expected GraphMutationSignal in event log");

    let has_analyzer_insight = events.iter().any(|e| {
        e.sender == AgentRole::Analyzer && matches!(e.payload, A2aPayload::CodeInsight { .. })
    });
    assert!(
        has_analyzer_insight,
        "expected Analyzer CodeInsight from dispatch_sync"
    );
}

#[tokio::test]
async fn bus_supervisor_consumes_analyzer_inbox() {
    let hub = A2aHub::new(test_config());
    let conversation_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let signal = A2aEnvelope::new(
        conversation_id,
        AgentRole::Gateway,
        AgentRole::Analyzer,
        A2aPayload::GraphMutationSignal {
            event_type: "index_promoted".to_string(),
            affected_files: vec!["src/lib.rs".to_string()],
        },
    )
    .with_task_id(task_id);

    let mut sub = hub.bus.subscribe_all();
    hub.bus.publish(signal).await;

    let mut saw_insight = false;
    for _ in 0..40 {
        if let Ok(env) = sub.try_recv() {
            if env.sender == AgentRole::Analyzer
                && matches!(env.payload, A2aPayload::CodeInsight { .. })
            {
                saw_insight = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    assert!(
        saw_insight,
        "bus supervisor should run AnalyzerRunner on inbox publish"
    );
}
