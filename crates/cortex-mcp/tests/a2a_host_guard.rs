//! Host guard: Cypher row truncation when `[a2a].enabled` (see `execute_cypher_query`).

use cortex_mcp::host_guard::truncate_cypher_rows;
use serde_json::json;

#[test]
fn truncate_cypher_rows_respects_host_guard_max() {
    let rows: Vec<_> = (0..120).map(|i| json!({"id": i})).collect();
    let max = 50;
    let (out, truncated) = truncate_cypher_rows(rows, max);
    assert!(truncated);
    assert_eq!(out.len(), max);
}

#[test]
fn host_guard_default_max_matches_a2a_config() {
    use cortex_core::A2aConfig;
    let max = A2aConfig::default().host_guard.max_cypher_rows;
    assert_eq!(max, 50);
    let rows: Vec<_> = (0..max + 1).map(|i| json!({"n": i})).collect();
    let (out, truncated) = truncate_cypher_rows(rows, max);
    assert!(truncated);
    assert_eq!(out.len(), max);
}
