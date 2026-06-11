//! Optional live MCP audit gate (77 tools). Run with CORTEX_LIVE_MCP_AUDIT=1.

use std::process::Command;

fn run_python(script: &str, args: &[&str], env: &[(&str, &str)]) -> bool {
    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(script);
    let mut cmd = Command::new("python3");
    cmd.arg(script_path);
    for arg in args {
        cmd.arg(arg);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.status().expect("run audit script").success()
}

#[test]
#[ignore = "requires live graph, Ollama, and CORTEX_LIVE_MCP_AUDIT=1"]
fn live_mcp_tool_audit_zero_broken() {
    if std::env::var("CORTEX_LIVE_MCP_AUDIT").ok().as_deref() != Some("1") {
        return;
    }
    let repo = std::env::var("CORTEX_AUDIT_REPO")
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd").display().to_string());
    let base = [("CORTEX_AUDIT_REPO", repo.as_str())];
    assert!(
        run_python(
            "../../scripts/mcp_tool_audit.py",
            &[],
            &[
                ("CORTEX_AUDIT_REPO", &repo),
                ("CORTEX_AUDIT_SKIP_DESTRUCTIVE", "1"),
                ("CORTEX_AUDIT_SKIP_LONG", "1"),
            ],
        ),
        "mcp_tool_audit.py should exit 0 (0 BROKEN tools)"
    );
    assert!(
        run_python(
            "../../scripts/mcp_semantic_audit.py",
            &["--profile", "pr", "--repo", &repo],
            &base,
        ),
        "mcp_semantic_audit.py (pr) should exit 0"
    );
}

#[test]
#[ignore = "requires CORTEX_LIVE_MCP_AUDIT=1 and CORTEX_TEST_EMBEDDER=1"]
fn live_mcp_vector_semantic_pr() {
    if std::env::var("CORTEX_LIVE_MCP_AUDIT").ok().as_deref() != Some("1") {
        return;
    }
    let fixture = std::env::var("CORTEX_SEMANTIC_FIXTURE").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/vector_semantic")
            .display()
            .to_string()
    });
    assert!(
        run_python(
            "../../scripts/mcp_semantic_audit.py",
            &[
                "--profile",
                "vector_pr",
                "--fixture",
                &fixture,
                "--bootstrap-fixture",
            ],
            &[("CORTEX_TEST_EMBEDDER", "1"), ("CORTEX_TEST_GRAPH", "1"),],
        ),
        "vector_pr semantic audit should exit 0"
    );
}
