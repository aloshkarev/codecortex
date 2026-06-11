//! After consensus proposes ordered_mutex, the deadlock transport fixture tests pass.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn deadlock_fixture_cargo_test_passes() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/a2a/transport_deadlock");
    let output = Command::new("cargo")
        .args(["test", "--quiet"])
        .current_dir(&fixture)
        .output()
        .expect("cargo test in fixture");
    assert!(
        output.status.success(),
        "fixture tests failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
