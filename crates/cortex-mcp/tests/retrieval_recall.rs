//! Live retrieval recall eval via scripts/retrieval_eval.py (requires graph index).

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "requires live graph index and CORTEX_TEST_GRAPH=1"]
fn retrieval_recall_eval() {
    if std::env::var("CORTEX_TEST_GRAPH").ok().as_deref() != Some("1") {
        eprintln!("skip: CORTEX_TEST_GRAPH=1 required");
        return;
    }

    let repo = std::env::var("CORTEX_RETRIEVAL_REPO").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("codecortex repo root")
            .display()
            .to_string()
    });
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/retrieval_eval.py");
    let bin = std::env::var("CORTEX_BIN").unwrap_or_else(|_| "cortex".to_string());

    let status = Command::new("python3")
        .arg(&script)
        .arg("--repo")
        .arg(&repo)
        .arg("--token-efficiency")
        .env("CORTEX_TEST_GRAPH", "1")
        .env("CORTEX_BIN", &bin)
        .env("CORTEX_RETRIEVAL_REPO", &repo)
        .status()
        .expect("run retrieval_eval.py");
    assert!(
        status.success(),
        "retrieval eval failed (see target/retrieval-eval.json)"
    );
}
