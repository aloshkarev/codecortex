use cortex_core::{EdgeKind, EntityKind};
use cortex_parser::ParserRegistry;
use std::path::PathBuf;

fn fixture_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_project_rust/src/main.rs")
}

#[test]
fn parses_rust_fixture_files() {
    let file = fixture_file();
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");
    let parsed = parser.parse(&source, &file).expect("parse success");
    assert!(!parsed.nodes.is_empty(), "should produce at least one node");
}

#[test]
fn extracts_function_nodes() {
    let file = fixture_file();
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");
    let parsed = parser.parse(&source, &file).expect("parse");

    let fns: Vec<_> = parsed
        .nodes
        .iter()
        .filter(|n| n.kind == EntityKind::Function)
        .collect();

    assert!(!fns.is_empty(), "should extract at least one Function node");

    let names: Vec<&str> = fns.iter().map(|n| n.name.as_str()).collect();
    assert!(
        names.contains(&"fib") || names.contains(&"main"),
        "expected fib or main in {:?}",
        names
    );
}

#[test]
fn extracts_contains_edges() {
    let file = fixture_file();
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");
    let parsed = parser.parse(&source, &file).expect("parse");

    let contains: Vec<_> = parsed
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Contains)
        .collect();
    assert!(
        !contains.is_empty(),
        "file should contain at least one entity via Contains edge"
    );
}

#[test]
fn extracts_call_edges() {
    let file = fixture_file();
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");
    let parsed = parser.parse(&source, &file).expect("parse");

    let calls: Vec<_> = parsed
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Calls)
        .collect();
    assert!(
        !calls.is_empty(),
        "main() calls fib() so at least one Calls edge expected"
    );
}

#[test]
fn function_node_has_line_number_and_cyclomatic_complexity() {
    let file = fixture_file();
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");
    let parsed = parser.parse(&source, &file).expect("parse");

    let fib = parsed
        .nodes
        .iter()
        .find(|n| n.name == "fib" && n.kind == EntityKind::Function);

    if let Some(node) = fib {
        assert!(node.line_number.is_some(), "fib should have a line number");
        let cc = node.properties.get("cyclomatic_complexity");
        assert!(cc.is_some(), "fib should have cyclomatic_complexity");
        let cc_val: u32 = cc.unwrap().parse().expect("integer");
        assert!(cc_val >= 1, "cc must be at least 1");
    }
}
