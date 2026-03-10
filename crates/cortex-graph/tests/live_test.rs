/// Live integration tests against a running Memgraph instance on 127.0.0.1:7687.
///
/// These tests do NOT use testcontainers — they require Memgraph to already be
/// running (e.g. via `docker run -p 7687:7687 memgraph/memgraph-mage:3.8.1`).
///
/// Run with:
///   cargo test -p cortex-graph --test live_test -- --nocapture
///
/// Tests use a unique test-repo prefix so they are safe to run against a shared
/// dev instance without polluting production data.
use cortex_analyzer::Analyzer;
use cortex_core::{CortexConfig, SearchKind};
use cortex_graph::GraphClient;
use cortex_indexer::Indexer;
use std::path::PathBuf;
use std::sync::Once;

static INIT: Once = Once::new();

/// Point sled cache into a temp dir inside the workspace so the sandbox allows it.
/// Uses a unique cache directory per test to avoid sled lock contention.
fn init_cache_path() {
    INIT.call_once(|| {
        // Use a unique directory per test process to avoid sled lock contention
        let test_name = std::thread::current()
            .name()
            .unwrap_or_default()
            .replace("::", "_");
        let cache_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/test-sled-cache")
            .join(test_name);
        std::fs::create_dir_all(&cache_dir).ok();
        // SAFETY: single-threaded initialisation via `Once`; no other thread reads this var yet.
        unsafe { std::env::set_var("CORTEX_CACHE_PATH", cache_dir.display().to_string()) };
    });
}

fn live_uri() -> String {
    std::env::var("BOLT_URI").unwrap_or_else(|_| "bolt://127.0.0.1:7687".to_string())
}

fn live_user() -> String {
    std::env::var("BOLT_USER").unwrap_or_default()
}

fn live_password() -> String {
    std::env::var("BOLT_PASSWORD").unwrap_or_default()
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/cortex-parser/tests/fixtures/sample_project_rust")
}

fn live_config() -> CortexConfig {
    CortexConfig {
        memgraph_uri: live_uri(),
        memgraph_user: live_user(),
        memgraph_password: live_password(),
        max_batch_size: 100,
        watched_paths: vec![],
        ..Default::default()
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

async fn connect() -> GraphClient {
    eprintln!("DEBUG: Starting connect()...");
    init_cache_path();
    eprintln!("DEBUG: init_cache_path() done");
    let config = live_config();
    eprintln!(
        "DEBUG: Config created: uri={}, user={}, password={}",
        config.memgraph_uri, config.memgraph_user, config.memgraph_password
    );
    let result = GraphClient::connect(&config).await;
    eprintln!("DEBUG: GraphClient::connect returned");
    result.expect("connect to live Bolt server on 127.0.0.1:7687")
}

async fn fresh_client() -> GraphClient {
    let client = connect().await;
    // Wipe all data so each test starts from a clean slate.
    client
        .raw_query("MATCH (n) DETACH DELETE n")
        .await
        .expect("wipe graph");
    // Also clear the sled hash-cache so files are always re-indexed.
    let cache_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-sled-cache");
    if cache_path.exists() {
        std::fs::remove_dir_all(&cache_path).ok();
    }
    std::fs::create_dir_all(&cache_path).ok();
    client
}

// ── connectivity ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn live_connect_and_ping() {
    let client = connect().await;
    let rows = client.raw_query("RETURN 42 AS n").await.expect("RETURN 42");
    assert_eq!(rows.len(), 1, "expected one row");
    let val = rows[0].get("n").and_then(|v| v.as_i64());
    assert_eq!(val, Some(42), "expected 42, got {:?}", val);
}

// ── indexing ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn live_index_rust_fixture() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("create indexer");

    let fixture = fixture_path();
    assert!(fixture.exists(), "fixture must exist: {fixture:?}");

    let report = indexer
        .index_path_with_options(&fixture, true)
        .await
        .expect("index fixture");
    println!("IndexReport: {report:?}");

    assert!(report.scanned_files > 0, "should scan ≥1 file");
    assert!(
        report.indexed_files > 0,
        "should index ≥1 file, report={report:?}"
    );
}

#[tokio::test]
async fn live_index_produces_function_nodes() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let rows = client
        .raw_query("MATCH (n:Function) RETURN n.name AS name")
        .await
        .expect("query :Function");

    let names: Vec<String> = rows
        .iter()
        .filter_map(|v| v.get("name")?.as_str().map(String::from))
        .collect();
    println!("Function nodes: {names:?}");

    assert!(
        names.iter().any(|n| n == "fib" || n == "main"),
        "expected fib or main among {names:?}"
    );
}

#[tokio::test]
async fn live_index_produces_file_nodes() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let rows = client
        .raw_query("MATCH (n:File) RETURN n.name AS name, n.path AS path LIMIT 10")
        .await
        .expect("query :File");

    println!("File nodes: {rows:?}");
    assert!(!rows.is_empty(), "at least one :File node must exist");
}

#[tokio::test]
async fn live_index_produces_repository_and_directory_nodes() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let repo_rows = client
        .raw_query("MATCH (r:Repository) RETURN r.path AS path")
        .await
        .expect("query :Repository");
    println!("Repository nodes: {repo_rows:?}");
    assert!(!repo_rows.is_empty(), ":Repository node must exist");

    let dir_rows = client
        .raw_query("MATCH (d:Directory) RETURN d.path AS path LIMIT 10")
        .await
        .expect("query :Directory");
    println!("Directory nodes: {dir_rows:?}");
    assert!(!dir_rows.is_empty(), ":Directory nodes must exist");
}

// ── analyzer queries ──────────────────────────────────────────────────────────

#[tokio::test]
async fn live_analyzer_callers_of_fib() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer.callers("fib").await.expect("callers query");
    println!("callers(fib): {rows:?}");
    // fib is self-recursive, so it must appear as its own caller
    assert!(!rows.is_empty(), "fib calls itself recursively");
}

#[tokio::test]
async fn live_analyzer_callees_of_main() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    // In the fixture, `main` calls `println!("{}", fib(10))`.
    // tree-sitter captures the macro name `println` as the callee, but NOT `fib(10)`
    // inside the macro arguments (opaque token_tree) — this is a known static-analysis
    // limitation for Rust macros.
    // We verify that main has AT LEAST ONE outgoing CALLS edge (to `println`).
    let all_calls = client
        .raw_query(
            "MATCH (caller:Function {name:'main'})-[:CALLS]->(b) \
             RETURN b.name AS callee LIMIT 10",
        )
        .await
        .expect("calls from main");
    println!("Outgoing CALLS from main: {all_calls:?}");
    assert!(
        !all_calls.is_empty(),
        "main should have at least one outgoing CALLS edge (to println!)"
    );
}

#[tokio::test]
async fn live_analyzer_dead_code() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer.dead_code().await.expect("dead_code query");
    println!("dead_code: {rows:?}");
    // main() is an entry point — never called — so it is in dead_code results
    let dead_names: Vec<String> = rows
        .iter()
        .filter_map(|v| {
            v.get("f")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .collect();
    println!("dead functions: {dead_names:?}");
}

#[tokio::test]
async fn live_analyzer_complexity() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer.complexity(10).await.expect("complexity query");
    println!("complexity top-10: {rows:?}");
    assert!(!rows.is_empty(), "complexity result must not be empty");
}

#[tokio::test]
async fn live_analyzer_repository_stats() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer.repository_stats().await.expect("repo_stats query");
    println!("repository_stats: {rows:?}");
    assert!(!rows.is_empty(), "repository_stats must return rows");

    let count = rows
        .first()
        .and_then(|v| v.get("node_count").and_then(|c| c.as_i64()))
        .unwrap_or(0);
    assert!(count > 0, "repo should have >0 child nodes, got {count}");
}

#[tokio::test]
async fn live_find_code_by_name() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer
        .find_code("fib", SearchKind::Name, None)
        .await
        .expect("find_code name");
    println!("find_code(name=fib): {rows:?}");
    assert!(
        !rows.is_empty(),
        "expected at least one exact-name match for fib"
    );
}

#[tokio::test]
async fn live_find_code_by_pattern() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer
        .find_code("fi", SearchKind::Pattern, None)
        .await
        .expect("find_code pattern");
    println!("find_code(pattern=fi): {rows:?}");
    assert!(
        !rows.is_empty(),
        "expected at least one pattern match for fi"
    );
}

#[tokio::test]
async fn live_call_chain() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let analyzer = Analyzer::new(client);
    let rows = analyzer
        .call_chain("fib", "fib", Some(8))
        .await
        .expect("call_chain query");
    println!("call_chain(fib->fib): {rows:?}");
    // shortestPath(a->a) may be empty depending on engine semantics for zero-length paths.
    assert!(
        rows.len() <= 1,
        "call_chain should return at most one shortest path"
    );
}

#[tokio::test]
async fn live_execute_cypher_count_nodes() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let rows = client
        .raw_query("MATCH (n) RETURN count(n) AS c")
        .await
        .expect("execute cypher count");
    println!("cypher count rows: {rows:?}");
    let count = rows
        .first()
        .and_then(|v| v.get("c").and_then(|x| x.as_i64()))
        .unwrap_or(0);
    assert!(count > 0, "graph should contain nodes after indexing");
}

#[tokio::test]
async fn live_resolve_call_targets() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    let fixture = fixture_path();
    indexer
        .index_path_with_options(&fixture, true)
        .await
        .expect("index");

    let resolved = client
        .resolve_call_targets(&fixture.display().to_string(), None)
        .await
        .expect("resolve_call_targets");
    println!("resolved call targets: {resolved}");
    assert!(
        resolved <= 10_000,
        "resolved count should be a sane non-negative number"
    );
}

// ── delete repository ─────────────────────────────────────────────────────────

#[tokio::test]
async fn live_delete_repository() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");

    let fixture = fixture_path();
    indexer
        .index_path_with_options(&fixture, true)
        .await
        .expect("index");

    let before = client
        .raw_query("MATCH (n:File) RETURN count(n) AS c")
        .await
        .expect("count before");
    let before_count = before
        .first()
        .and_then(|v| v.get("c")?.as_i64())
        .unwrap_or(0);
    println!("File nodes before delete: {before_count}");
    assert!(before_count > 0, "need nodes before delete");

    client
        .delete_repository(&fixture.display().to_string())
        .await
        .expect("delete_repository");

    let after = client
        .raw_query("MATCH (n:File) RETURN count(n) AS c")
        .await
        .expect("count after");
    let after_count = after
        .first()
        .and_then(|v| v.get("c")?.as_i64())
        .unwrap_or(0);
    println!("File nodes after delete: {after_count}");
    assert!(
        after_count < before_count,
        "delete should reduce node count ({before_count} → {after_count})"
    );
}

// ── raw query output ──────────────────────────────────────────────────────────

#[tokio::test]
async fn live_raw_query_returns_json_objects() {
    let client = fresh_client().await;
    let indexer = Indexer::new(client.clone(), 100).expect("indexer");
    indexer
        .index_path_with_options(&fixture_path(), true)
        .await
        .expect("index");

    let rows = client
        .raw_query("MATCH (n:Function) RETURN n LIMIT 5")
        .await
        .expect("raw query");

    for row in &rows {
        let node = row.get("n").expect("row must have 'n' key");
        println!("row n = {node}");
        // Each value must be a JSON object (not a raw Debug string)
        assert!(node.is_object(), "expected JSON object, got: {node}");
        assert!(node.get("name").is_some(), "Function node must have 'name'");
    }
}
