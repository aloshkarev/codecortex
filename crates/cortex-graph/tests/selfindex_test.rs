/// End-to-end self-indexing verification against a running Bolt server.
///
/// Requires a running Memgraph/Neo4j-compatible server.
/// Run with:
///   BOLT_URI=bolt://127.0.0.1:7687 BOLT_USER=memgraph BOLT_PASSWORD=memgraph \
///   cargo test -p cortex-graph --test selfindex_test -- --ignored --nocapture
use cortex_analyzer::Analyzer;
use cortex_core::{CortexConfig, SearchKind};
use cortex_graph::GraphClient;
use cortex_indexer::Indexer;
use std::path::PathBuf;

fn config_from_env() -> CortexConfig {
    CortexConfig {
        memgraph_uri: std::env::var("BOLT_URI")
            .unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string()),
        memgraph_user: std::env::var("BOLT_USER").unwrap_or_else(|_| "memgraph".to_string()),
        memgraph_password: std::env::var("BOLT_PASSWORD")
            .unwrap_or_else(|_| "memgraph".to_string()),
        max_batch_size: 500,
        watched_paths: vec![],
        ..Default::default()
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[tokio::test]
#[ignore = "requires running Memgraph and full build"]
async fn self_index_codecortex_repository() {
    // SAFETY: single test process configuration for cache location.
    unsafe {
        std::env::set_var(
            "CORTEX_CACHE_PATH",
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/selfindex-sled-cache")
                .display()
                .to_string(),
        );
    }
    let cfg = config_from_env();
    let client = GraphClient::connect(&cfg).await.expect("connect");
    let repo = repo_root();
    let repo_path = repo.display().to_string();

    let _ = client.delete_repository(&repo_path).await;

    let indexer = Indexer::new(client.clone(), cfg.max_batch_size).expect("indexer");
    let report = indexer
        .index_path_with_options(&repo, true)
        .await
        .expect("index repository");
    println!("self-index report: {report:?}");
    assert!(report.indexed_files > 20, "expected >20 indexed files");

    let count_rows = client
        .raw_query("MATCH (f:Function) RETURN count(f) AS c")
        .await
        .expect("count functions");
    let function_count = count_rows
        .first()
        .and_then(|v| v.get("c").and_then(|x| x.as_i64()))
        .unwrap_or(0);
    assert!(
        function_count > 50,
        "expected >50 functions, got {function_count}"
    );

    let analyzer = Analyzer::new(client.clone());
    let callers = analyzer
        .callers("index_path")
        .await
        .expect("callers(index_path)");
    assert!(
        !callers.is_empty(),
        "expected at least one caller for index_path after self-indexing"
    );

    let types = analyzer
        .find_code("GraphClient", SearchKind::Type, None)
        .await
        .expect("find GraphClient by type");
    assert!(!types.is_empty(), "expected GraphClient type in index");
}
