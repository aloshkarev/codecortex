//! FalkorDB bulk upsert throughput benchmark (requires Docker).
//!
//!   RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_throughput_test -- --ignored --nocapture

use cortex_core::{CodeEdge, CodeNode, CortexConfig, EdgeKind, EntityKind, Language};
use cortex_graph::{FalkorDbClient, GraphClient};
use std::process::Command;
use std::time::{Duration, Instant};
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

fn synthetic_nodes(n: usize, with_source: bool) -> Vec<CodeNode> {
    (0..n)
        .map(|i| {
            let mut props = std::collections::HashMap::new();
            props.insert("repository_path".to_string(), "/bench".to_string());
            props.insert("branch".to_string(), "".to_string());
            CodeNode {
                id: format!("fn:bench:{i}"),
                kind: EntityKind::Function,
                name: format!("f{i}"),
                path: Some("bench.rs".to_string()),
                line_number: Some(i as u32),
                lang: Some(Language::Rust),
                source: if with_source {
                    Some("x".repeat(1024))
                } else {
                    None
                },
                docstring: None,
                properties: props,
            }
        })
        .collect()
}

fn synthetic_edges(n: usize) -> Vec<CodeEdge> {
    (0..n.saturating_sub(1))
        .map(|i| CodeEdge {
            from: format!("fn:bench:{i}"),
            to: format!("fn:bench:{}", i + 1),
            kind: EdgeKind::Calls,
            properties: std::collections::HashMap::new(),
        })
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker; set RUN_DOCKER_INTEGRATION=1"]
async fn falkordb_bulk_throughput_benchmark() {
    let Some((_container, port)) = start_falkordb().await else {
        eprintln!("skipping FalkorDB throughput benchmark");
        return;
    };

    let config = CortexConfig {
        falkordb_uri: format!("falkor://127.0.0.1:{port}"),
        falkordb_graph: "codecortex_bench".to_string(),
        max_batch_size: 512,
        ..Default::default()
    };

    unsafe {
        std::env::set_var("CORTEX_FALKORDB_PROFILE", "1");
    }
    FalkorDbClient::reset_profile();

    let client = GraphClient::connect(&config).await.expect("connect");
    let nodes = synthetic_nodes(2000, false);
    let edges = synthetic_edges(2000);

    let t0 = Instant::now();
    client.bulk_upsert_nodes(&nodes).await.expect("nodes");
    let node_secs = t0.elapsed().as_secs_f64();

    FalkorDbClient::reset_profile();
    let t1 = Instant::now();
    let bolts = client.bulk_upsert_edges(&edges).await.expect("edges");
    let edge_secs = t1.elapsed().as_secs_f64();
    let profile = FalkorDbClient::profile_snapshot(false);

    eprintln!("=== FalkorDB bulk throughput ===");
    eprintln!(
        "nodes: {} in {:.3}s ({:.0} nodes/s)",
        nodes.len(),
        node_secs,
        nodes.len() as f64 / node_secs
    );
    eprintln!(
        "edges: {} in {:.3}s ({:.0} edges/s), bolt_executions={}",
        edges.len(),
        edge_secs,
        edges.len() as f64 / edge_secs,
        bolts
    );
    eprintln!(
        "profile: queries={} avg_bytes={} max_bytes={} lock_wait_frac={:.2}",
        profile.query_count,
        profile.query_bytes_avg,
        profile.query_bytes_max,
        profile.lock_wait_fraction
    );

    assert!(node_secs < 120.0);
    assert!(edge_secs < 120.0);
    assert!(profile.query_count > 0);

    FalkorDbClient::reset_profile();
    let heavy = synthetic_nodes(500, true);
    let t2 = Instant::now();
    client.bulk_upsert_nodes(&heavy).await.expect("heavy nodes");
    let heavy_secs = t2.elapsed().as_secs_f64();
    let heavy_profile = FalkorDbClient::profile_snapshot(false);
    eprintln!(
        "heavy nodes (1KiB source): {} in {:.3}s, max_query_bytes={}",
        heavy.len(),
        heavy_secs,
        heavy_profile.query_bytes_max
    );
    assert!(heavy_profile.query_bytes_max > 100_000);
}
