//! Live vector semantic MCP audit via scripts/mcp_semantic_audit.py (fixture + HashEmbedder).

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires live graph index, CORTEX_VECTOR_SEMANTIC=1, and CORTEX_TEST_EMBEDDER=1"]
fn mcp_vector_semantic_fixture() {
    if std::env::var("CORTEX_VECTOR_SEMANTIC").ok().as_deref() != Some("1") {
        return;
    }
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        eprintln!("skip: CORTEX_TEST_GRAPH=1 required");
        return;
    }

    let fixture = std::env::var("CORTEX_SEMANTIC_FIXTURE").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/vector_semantic")
            .canonicalize()
            .expect("vector_semantic fixture")
            .display()
            .to_string()
    });
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/mcp_semantic_audit.py");
    let bin = std::env::var("CORTEX_BIN").unwrap_or_else(|_| "cortex".to_string());

    let status = Command::new("python3")
        .arg(&script)
        .arg("--profile")
        .arg("vector_pr")
        .arg("--fixture")
        .arg(&fixture)
        .arg("--bootstrap-fixture")
        .env("CORTEX_TEST_EMBEDDER", "1")
        .env("CORTEX_TEST_GRAPH", "1")
        .env("CORTEX_BIN", &bin)
        .status()
        .expect("run mcp_semantic_audit.py vector_pr");
    assert!(
        status.success(),
        "vector semantic audit failed (see target/mcp-semantic-ledger.json)"
    );
}
