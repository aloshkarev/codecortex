//! Concurrent AgentInsight writes (requires live FalkorDB/Memgraph).

use chrono::Utc;
use cortex_core::CortexConfig;
use cortex_graph::{AgentInsightRecord, BlackboardWriter, GraphClient, insight_id};
use std::sync::Arc;

#[tokio::test]
#[ignore = "requires graph backend at CORTEX_TEST_GRAPH=1"]
async fn concurrent_insight_writes() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        return;
    }
    let config = CortexConfig::default();
    let client = GraphClient::connect(&config).await.expect("connect");
    let writer = BlackboardWriter::new(client, config.a2a.blackboard.write_batch_size);
    writer.ensure_schema().await.expect("schema");

    let session_id = "load-test-session";
    writer
        .upsert_session(session_id, "conv-load", "working")
        .await
        .expect("session");

    let n_tasks = 10usize;
    let per_task = 100usize;
    let hub = Arc::new(writer);
    let mut handles = Vec::new();
    let start = std::time::Instant::now();

    for t in 0..n_tasks {
        let w = hub.clone();
        let sid = session_id.to_string();
        handles.push(tokio::spawn(async move {
            for i in 0..per_task {
                let summary = format!("insight-{t}-{i}");
                let id = insight_id(&sid, "analyzer", "sym", &summary);
                let rec = AgentInsightRecord {
                    id,
                    session_id: sid.clone(),
                    conversation_id: "conv-load".to_string(),
                    role: "analyzer".to_string(),
                    summary,
                    target_qualified_name: "crate::transport".to_string(),
                    risk_level: "low".to_string(),
                    suggested_action: "note".to_string(),
                    created_at: Utc::now(),
                };
                w.write_insight(&rec).await.expect("write");
            }
        }));
    }
    for h in handles {
        h.await.expect("join");
    }
    let elapsed = start.elapsed();
    let total = n_tasks * per_task;
    let per_insight_ms = elapsed.as_secs_f64() * 1000.0 / total as f64;
    eprintln!(
        "wrote {total} insights in {:?} ({per_insight_ms:.3} ms/insight)",
        elapsed
    );
    assert!(
        per_insight_ms < 2.0,
        "blackboard SLO violated: {per_insight_ms:.3} ms/insight (limit < 2.0 ms/insight); \
         wrote {total} insights in {elapsed:?}"
    );

    let count = hub.count_insights(session_id).await.expect("count");
    assert_eq!(count, total);
}
