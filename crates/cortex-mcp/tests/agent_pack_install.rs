//! Integration smoke: install agent pack from plugin/codecortex into a temp repo.

use cortex_mcp::{AgentPackInstallOptions, install_agent_pack};
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn install_from_plugin_codecortex_layout() {
    let pack = repo_root().join("plugin/codecortex");
    if !pack.join("skills").is_dir() {
        eprintln!("skip: plugin/codecortex not present");
        return;
    }

    let target = TempDir::new().expect("tempdir");
    let opts = AgentPackInstallOptions::for_repo(target.path(), &pack);
    let result = install_agent_pack(opts).expect("install");

    assert!(!result.installed.is_empty());
    assert!(target.path().join(".cursor/skills").exists());
    assert!(target.path().join(".cursor/mcp.json").exists());
    assert!(target.path().join("mcp.json").exists());
}
