//! Live semantic MCP audit via scripts/mcp_semantic_audit.py (oracles.json ground truth).

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires live graph index and CORTEX_SEMANTIC_AUDIT=1"]
fn mcp_semantic_audit_profile_pr() {
    if std::env::var("CORTEX_SEMANTIC_AUDIT").ok().as_deref() != Some("1") {
        return;
    }
    let repo = std::env::var("CORTEX_SEMANTIC_REPO").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
            .display()
            .to_string()
    });
    let profile = std::env::var("CORTEX_SEMANTIC_PROFILE").unwrap_or_else(|_| "pr".to_string());
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/mcp_semantic_audit.py");
    let status = Command::new("python3")
        .arg(&script)
        .arg("--profile")
        .arg(&profile)
        .arg("--repo")
        .arg(&repo)
        .status()
        .expect("run mcp_semantic_audit.py");
    assert!(
        status.success(),
        "semantic audit failed (see target/mcp-semantic-ledger.json)"
    );
}
