//! TWAG corpus accuracy tests: MCP navigation/relationship tools vs golden oracles.
//!
//! Gated by `CORTEX_TEST_TWAG=1` and `CORTEX_TEST_GRAPH=1` (requires indexed TWAG in FalkorDB).

mod twag_common;

use twag_common::{load_case, load_manifest, run_golden_case, skip_unless_twag_graph};

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_corpus_accuracy_manifest() {
    if !skip_unless_twag_graph() {
        return;
    }

    let manifest = load_manifest();
    let cases = manifest
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("manifest.cases array");

    for case_id in cases {
        let id = case_id.as_str().expect("case id string");
        run_golden_case(&load_case(id));
    }
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_find_callers_orchestrator_snapshot() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("find_callers_orchestrator_snapshot"));
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_find_callers_apply_config_snapshot() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("find_callers_apply_config_snapshot"));
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_go_to_definition_orchestrator_snapshot() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("go_to_definition_orchestrator_snapshot"));
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_go_to_definition_should_relay() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("go_to_definition_should_relay"));
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_find_all_usages_should_relay() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("find_all_usages_should_relay"));
}

#[test]
#[ignore = "requires CORTEX_TEST_TWAG=1, CORTEX_TEST_GRAPH=1, and indexed TWAG repo"]
fn twag_find_all_usages_peer_state() {
    if !skip_unless_twag_graph() {
        return;
    }
    run_golden_case(&load_case("find_all_usages_peer_state"));
}
