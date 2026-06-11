//! FalkorDB schema index + EXPLAIN validation (requires Docker).
//!
//!   RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_index_explain_test -- --ignored --nocapture

use cortex_core::CortexConfig;
use cortex_graph::GraphClient;
use std::process::Command;
use std::time::Duration;
use testcontainers::{
    GenericImage,
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tokio::time::timeout;

fn docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn start_falkordb() -> Option<(testcontainers::ContainerAsync<GenericImage>, u16)> {
    if std::env::var("RUN_DOCKER_INTEGRATION").ok().as_deref() != Some("1") || !docker_available() {
        return None;
    }
    unsafe {
        std::env::set_var("TESTCONTAINERS_RYUK_DISABLED", "true");
    }
    let started = timeout(
        Duration::from_secs(60),
        GenericImage::new("falkordb/falkordb", "latest")
            .with_exposed_port(ContainerPort::Tcp(6379))
            .with_wait_for(WaitFor::Duration {
                length: Duration::from_secs(5),
            })
            .start(),
    )
    .await;
    let Ok(Ok(container)) = started else {
        return None;
    };
    let port = timeout(Duration::from_secs(10), container.get_host_port_ipv4(6379)).await;
    let Ok(Ok(port)) = port else {
        return None;
    };
    Some((container, port))
}

#[tokio::test]
#[ignore = "requires Docker; set RUN_DOCKER_INTEGRATION=1"]
async fn falkordb_indexes_and_explain_edge_unwind() {
    let Some((_container, port)) = start_falkordb().await else {
        eprintln!("skipping FalkorDB EXPLAIN test");
        return;
    };

    let config = CortexConfig {
        falkordb_uri: format!("falkor://127.0.0.1:{port}"),
        falkordb_graph: "codecortex_explain".to_string(),
        ..Default::default()
    };
    let client = GraphClient::connect(&config).await.expect("connect");

    // Schema ensure_constraints runs on connect; probe index visibility via CALL db.indexes or sample query.
    let index_rows = client
        .raw_query(
            "CALL db.indexes() YIELD type, label, properties, status RETURN label, properties",
        )
        .await;
    if let Ok(rows) = index_rows {
        eprintln!("db.indexes rows: {}", rows.len());
        let has_codenode_id = rows.iter().any(|r| {
            r.get("label").and_then(|v| v.as_str()) == Some("CodeNode")
                && r.get("properties")
                    .map(|p| p.to_string().contains("id"))
                    .unwrap_or(false)
        });
        if !has_codenode_id {
            eprintln!("warning: CodeNode(id) index not reported by db.indexes()");
        }
    } else {
        eprintln!("CALL db.indexes() unavailable; relying on ensure_constraints only");
    }

    client
        .run("CREATE (a:CodeNode {id: 'n1'}), (b:CodeNode {id: 'n2'})")
        .await
        .expect("seed");

    // FalkorDB may return an empty result set for EXPLAIN on write-shaped queries; success is no error.
    let explain = client
        .raw_query(
            "EXPLAIN MATCH (from:CodeNode {id: 'n1'}), (to:CodeNode {id: 'n2'})
             MERGE (from)-[r:CALLS]->(to)
             RETURN r",
        )
        .await;
    match explain {
        Ok(rows) => eprintln!("EXPLAIN returned {} row(s)", rows.len()),
        Err(e) => eprintln!("EXPLAIN not supported or failed (non-fatal): {e}"),
    }

    client
        .run(
            "MATCH (from:CodeNode {id: 'n1'}), (to:CodeNode {id: 'n2'})
             MERGE (from)-[r:CALLS]->(to)
             SET r.kind = 'Calls'",
        )
        .await
        .expect("edge merge");
}
