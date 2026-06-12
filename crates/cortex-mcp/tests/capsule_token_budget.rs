//! Token-budget regression for ContextCapsuleBuilder (BM25 + rerank path).

use cortex_mcp::capsule::{CapsuleConfig, ContextCapsuleBuilder, GraphSearchResult};

fn sample_results(n: usize) -> Vec<GraphSearchResult> {
    (0..n)
        .map(|i| GraphSearchResult {
            id: format!("sym-{i}"),
            kind: "Function".to_string(),
            path: format!("components/cp/src/module_{i}.cpp"),
            name: format!("handler_{i}"),
            source: Some("void handler() {}".repeat(20)),
            line_number: Some(i as u64 + 1),
        })
        .collect()
}

#[test]
fn capsule_respects_token_budget_with_rerank() {
    let max_tokens = 800;
    let mut builder = ContextCapsuleBuilder::with_config(CapsuleConfig {
        max_items: 40,
        max_tokens,
        initial_threshold: 0.01,
        min_threshold: 0.001,
        rerank_enabled: true,
        use_bm25: true,
        ..Default::default()
    });
    let result = builder.build(
        "handler module orchestrator",
        sample_results(30),
        Some("debug"),
        &[],
    );
    assert!(
        result.token_estimate <= max_tokens,
        "token_estimate {} exceeds budget {}",
        result.token_estimate,
        max_tokens
    );
    assert!(!result.capsule_items.is_empty());
}

#[test]
fn capsule_keeps_high_signal_symbols() {
    let mut builder = ContextCapsuleBuilder::new();
    let mut results = sample_results(5);
    results.push(GraphSearchResult {
        id: "forwarding".into(),
        kind: "Class".into(),
        path: "components/platform/src/forwarding_ipc.cpp".into(),
        name: "ForwardingClient".into(),
        source: Some("class ForwardingClient { bool install_rule(); };".into()),
        line_number: Some(86),
    });
    let result = builder.build(
        "ForwardingClient install_rule IPC",
        results,
        Some("debug"),
        &["components/platform".to_string()],
    );
    assert!(
        result
            .capsule_items
            .iter()
            .any(|item| item.name.contains("ForwardingClient")),
        "expected ForwardingClient in capsule"
    );
}
