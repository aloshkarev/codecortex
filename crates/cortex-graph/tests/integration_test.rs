/// Integration tests against a real Memgraph container.
///
/// Run with:
///   cargo test -p cortex-graph --test integration_test -- --nocapture
///
/// Requires Docker. Tests are annotated `#[ignore]` by default so they don't
/// run in plain `cargo test`. Pass `-- --ignored` (or `--include-ignored`) to
/// execute them.
use cortex_core::CortexConfig;
use cortex_graph::GraphClient;
use cortex_indexer::Indexer;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use testcontainers::{
    GenericImage,
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
};
use tokio::time::timeout;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Fixture path: the small Rust project used for parser unit tests.
fn rust_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/cortex-parser/tests/fixtures/sample_project_rust")
}

/// Build a `CortexConfig` pointed at `host:port`.
fn memgraph_config(host: &str, port: u16) -> CortexConfig {
    CortexConfig {
        memgraph_uri: format!("bolt://{}:{}", host, port),
        memgraph_user: "".to_string(),
        memgraph_password: "".to_string(),
        max_batch_size: 100,
        watched_paths: vec![],
        ..Default::default()
    }
}

fn docker_available() -> bool {
    Command::new("docker")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Start a Memgraph container and return it + the bolt port.
/// Returns None when Docker is not available in this environment.
async fn start_memgraph_or_skip() -> Option<(testcontainers::ContainerAsync<GenericImage>, u16)> {
    if std::env::var("RUN_DOCKER_INTEGRATION").ok().as_deref() != Some("1") {
        return None;
    }
    if !docker_available() {
        return None;
    }
    // SAFETY: test-only process-level configuration for testcontainers behavior.
    unsafe {
        std::env::set_var("TESTCONTAINERS_RYUK_DISABLED", "true");
    }
    let started = timeout(
        Duration::from_secs(30),
        GenericImage::new("memgraph/memgraph", "3.8.1")
            .with_exposed_port(ContainerPort::Tcp(7687))
            .with_wait_for(WaitFor::Duration {
                length: Duration::from_secs(8),
            })
            .start(),
    )
    .await;
    let Ok(Ok(container)) = started else {
        return None;
    };

    let port = timeout(Duration::from_secs(10), container.get_host_port_ipv4(7687)).await;
    let Ok(Ok(port)) = port else {
        return None;
    };
    Some((container, port))
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_connect_to_memgraph() {
    let Some((_container, port)) = start_memgraph_or_skip().await else {
        eprintln!("Skipping test_connect_to_memgraph: Docker unavailable");
        return;
    };
    let config = memgraph_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("should connect");
    let rows = client
        .raw_query("RETURN 1 AS n")
        .await
        .expect("should execute");
    assert!(!rows.is_empty(), "RETURN 1 should return one row");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_index_rust_fixture_and_query_nodes() {
    let Some((_container, port)) = start_memgraph_or_skip().await else {
        eprintln!("Skipping test_index_rust_fixture_and_query_nodes: Docker unavailable");
        return;
    };
    let config = memgraph_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("connect");
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");

    let fixture = rust_fixture_path();
    assert!(fixture.exists(), "fixture path must exist: {:?}", fixture);

    let report = indexer.index_path(&fixture).await.expect("index");
    assert!(
        report.indexed_files > 0,
        "should have indexed at least one file"
    );
    assert!(report.scanned_files > 0);

    // Query that at least one CodeNode was written
    let rows = client
        .raw_query("MATCH (n:File) RETURN n.name AS name LIMIT 10")
        .await
        .expect("query");
    assert!(
        !rows.is_empty(),
        "at least one File node should exist after indexing"
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_index_produces_function_nodes() {
    let Some((_container, port)) = start_memgraph_or_skip().await else {
        eprintln!("Skipping test_index_produces_function_nodes: Docker unavailable");
        return;
    };
    let config = memgraph_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("connect");
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");

    indexer
        .index_path(&rust_fixture_path())
        .await
        .expect("index");

    // The fixture contains `fn fib(n: u32)` and `fn main()`.
    let rows = client
        .raw_query("MATCH (n:Function) RETURN n.name AS name")
        .await
        .expect("query functions");
    let names: Vec<String> = rows
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    assert!(
        names.iter().any(|n| n == "fib" || n == "main"),
        "expected fib or main in {:?}",
        names
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_dead_code_query_returns_results() {
    let Some((_container, port)) = start_memgraph_or_skip().await else {
        eprintln!("Skipping test_dead_code_query_returns_results: Docker unavailable");
        return;
    };
    let config = memgraph_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("connect");
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");

    indexer
        .index_path(&rust_fixture_path())
        .await
        .expect("index");

    let _analyzer = cortex_graph::QueryEngine::new(client.clone());
    // The dead-code query should run without errors (result may be empty or not).
    let _rows = client
        .raw_query(
            "MATCH (f:Function) WHERE NOT (()-[:CALLS]->(f)) \
             RETURN f.name AS name LIMIT 20",
        )
        .await
        .expect("dead code query should succeed");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_delete_repository() {
    let Some((_container, port)) = start_memgraph_or_skip().await else {
        eprintln!("Skipping test_delete_repository: Docker unavailable");
        return;
    };
    let config = memgraph_config("127.0.0.1", port);
    let client = GraphClient::connect(&config).await.expect("connect");
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");

    let fixture = rust_fixture_path();
    indexer.index_path(&fixture).await.expect("index");

    // Count nodes
    let before = client
        .raw_query("MATCH (n:File) RETURN count(n) AS c")
        .await
        .expect("count");
    let before_count: i64 = before
        .first()
        .and_then(|v| v.get("c").and_then(|c| c.as_i64()))
        .unwrap_or(0);
    assert!(before_count > 0, "nodes must exist before delete");

    // The fixture dir acts as the repo path key
    client
        .delete_repository(&fixture.display().to_string())
        .await
        .expect("delete");
}
