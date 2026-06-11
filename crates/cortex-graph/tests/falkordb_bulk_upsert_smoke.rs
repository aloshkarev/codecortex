//! FalkorDB bulk UNWIND smoke test (requires Docker).
//!
//! Run with:
//!   RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_upsert_smoke -- --ignored --nocapture

use cortex_core::{CodeNode, CortexConfig, EntityKind, Language};
use cortex_graph::GraphClient;
use cortex_graph::GraphParam;
use std::collections::HashMap;
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

fn falkordb_config(host: &str, port: u16) -> CortexConfig {
    CortexConfig {
        falkordb_uri: format!("falkor://{host}:{port}"),
        falkordb_graph: "codecortex_test".to_string(),
        max_batch_size: 100,
        ..Default::default()
    }
}

async fn start_falkordb_or_skip() -> Option<(testcontainers::ContainerAsync<GenericImage>, u16)> {
    if std::env::var("RUN_DOCKER_INTEGRATION").ok().as_deref() != Some("1") {
        return None;
    }
    if !docker_available() {
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
async fn falkordb_bulk_upsert_smoke() {
    let Some((_container, port)) = start_falkordb_or_skip().await else {
        eprintln!(
            "skipping FalkorDB integration (Docker unavailable or RUN_DOCKER_INTEGRATION unset)"
        );
        return;
    };

    let config = falkordb_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("connect");

    let node = CodeNode {
        id: "fn:test:smoke".to_string(),
        kind: EntityKind::Function,
        name: "smoke".to_string(),
        path: Some("smoke.rs".to_string()),
        line_number: Some(1),
        lang: Some(Language::Rust),
        source: None,
        docstring: None,
        properties: {
            let mut m = std::collections::HashMap::new();
            m.insert("repository_path".to_string(), "/tmp/smoke".to_string());
            m.insert("branch".to_string(), "".to_string());
            m
        },
    };
    client.bulk_upsert_nodes(&[node]).await.expect("bulk nodes");

    let mut item = HashMap::new();
    item.insert("id".to_string(), GraphParam::String("ct:smoke".to_string()));
    item.insert(
        "name".to_string(),
        GraphParam::String("smoke_fn".to_string()),
    );
    let mut params = HashMap::new();
    params.insert(
        "batch".to_string(),
        GraphParam::List(vec![GraphParam::Map(item)]),
    );
    client
        .execute_with_raw_param_map(
            "UNWIND $batch AS item MERGE (n:CodeNode:CallTarget {id: item.id}) SET n.name = item.name",
            params,
        )
        .await
        .expect("bulk call targets");

    let rows = client
        .raw_query("MATCH (n:CodeNode {id: 'fn:test:smoke'}) RETURN count(n) AS c")
        .await
        .expect("query");
    assert!(!rows.is_empty());
}
