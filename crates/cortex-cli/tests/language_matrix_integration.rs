mod integration {
    pub mod flow;
    pub mod harness;
    pub mod projects;
}

use integration::flow::run_full_functionality_suite;
use integration::harness::{IntegrationContext, command_available};
use integration::projects::{PROJECT_FIXTURES, RepoFixture};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn build_context() -> Option<IntegrationContext> {
    let bin = std::env::var_os("CARGO_BIN_EXE_cortex-cli").or_else(|| {
        Some(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/debug/cortex-cli")
                .into_os_string(),
        )
    })?;
    IntegrationContext::from_env(bin.into())
}

fn ensure_runtime_prerequisites() -> Option<()> {
    if !command_available("git") {
        return None;
    }
    Some(())
}

fn require_real_mode() -> bool {
    std::env::var("CORTEX_REAL_INTEGRATION").ok().as_deref() == Some("1")
}

fn real_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn run_language_test(slug: &str) {
    let _guard = real_test_guard();
    if ensure_runtime_prerequisites().is_none() || !require_real_mode() {
        return;
    }
    let Some(ctx) = build_context() else {
        return;
    };

    let fixture = fixture_by_slug(slug);
    run_full_functionality_suite(&ctx, fixture);
}

fn fixture_by_slug(slug: &str) -> RepoFixture {
    PROJECT_FIXTURES
        .iter()
        .copied()
        .find(|f| f.slug == slug)
        .unwrap_or_else(|| panic!("unknown fixture slug: {}", slug))
}

#[test]
fn fixtures_are_complete_and_pinned() {
    assert_eq!(PROJECT_FIXTURES.len(), 12);
    for fixture in &PROJECT_FIXTURES {
        assert!(
            fixture.commit_sha.len() >= 12,
            "fixture {} must include pinned commit SHA",
            fixture.slug
        );
        assert!(
            fixture.clone_url.starts_with("https://github.com/"),
            "fixture {} must use a GitHub clone URL",
            fixture.slug
        );
    }
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_rust() {
    run_language_test("serde-rs/serde");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_python() {
    run_language_test("pallets/click");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_go() {
    run_language_test("spf13/cobra");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_typescript() {
    run_language_test("axios/axios");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_javascript() {
    run_language_test("expressjs/express");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_c() {
    run_language_test("libcheck/check");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_cpp() {
    run_language_test("fmtlib/fmt");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_java() {
    run_language_test("google/gson");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_php() {
    run_language_test("Seldaek/monolog");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_ruby() {
    run_language_test("sinatra/sinatra");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_kotlin() {
    run_language_test("InsertKoinIO/koin");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_swift() {
    run_language_test("apple/swift-argument-parser");
}

#[test]
#[ignore = "requires CORTEX_INTEGRATION_ENABLE=1, CORTEX_REAL_INTEGRATION=1 and running Memgraph"]
fn integration_all_languages_ordered_one_by_one() {
    let _guard = real_test_guard();
    if ensure_runtime_prerequisites().is_none() || !require_real_mode() {
        return;
    }
    let Some(ctx) = build_context() else {
        return;
    };

    for fixture in PROJECT_FIXTURES {
        run_full_functionality_suite(&ctx, fixture);
    }
}
