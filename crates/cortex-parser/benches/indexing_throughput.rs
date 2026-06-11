use cortex_parser::ParserRegistry;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn bench_rust_parsing(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_project_rust");
    let file = root.join("src/main.rs");
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let registry = ParserRegistry::new();
    let parser = registry.parser_for_path(&file).expect("rust parser");

    c.bench_function("parse_sample_project_rust_main", |b| {
        b.iter(|| parser.parse(&source, &file).expect("parse"))
    });
}

fn bench_typescript_parsing(c: &mut Criterion) {
    let file = PathBuf::from("sample.ts");
    let source = r#"
export function greet(name: string): string {
  if (name.length === 0) return "hello";
  return `hello, ${name}`;
}
"#;
    let registry = ParserRegistry::new();
    let parser = registry.parser_for_path(&file).expect("typescript parser");
    c.bench_function("parse_inline_typescript_greet", |b| {
        b.iter(|| parser.parse(source, &file).expect("parse"))
    });
}

fn bench_python_parsing(c: &mut Criterion) {
    let file = PathBuf::from("sample.py");
    let source = r#"
def greet(name: str) -> str:
    if not name:
        return "hello"
    return f"hello, {name}"
"#;
    let registry = ParserRegistry::new();
    let parser = registry.parser_for_path(&file).expect("python parser");
    c.bench_function("parse_inline_python_greet", |b| {
        b.iter(|| parser.parse(source, &file).expect("parse"))
    });
}

fn bench_rust_parsing_repeated_warm_pool(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_project_rust");
    let file = root.join("src/main.rs");
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let registry = ParserRegistry::new();
    let parser = registry.parser_for_path(&file).expect("rust parser");
    // Warm thread-local parser pool
    let _ = parser.parse(&source, &file);

    c.bench_function("parse_sample_project_rust_main_warm_tls", |b| {
        b.iter(|| parser.parse(&source, &file).expect("parse"))
    });
}

criterion_group!(
    benches,
    bench_rust_parsing,
    bench_rust_parsing_repeated_warm_pool,
    bench_typescript_parsing,
    bench_python_parsing
);
criterion_main!(benches);
