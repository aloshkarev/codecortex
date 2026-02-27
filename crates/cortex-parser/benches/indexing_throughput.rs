use cortex_parser::ParserRegistry;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn bench_rust_parsing(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample_project_rust");
    let file = root.join("src/main.rs");
    let source = std::fs::read_to_string(&file).expect("fixture file");
    let parser = ParserRegistry::new()
        .parser_for_path(&file)
        .expect("rust parser");

    c.bench_function("parse_sample_project_rust_main", |b| {
        b.iter(|| parser.parse(&source, &file).expect("parse"))
    });
}

criterion_group!(benches, bench_rust_parsing);
criterion_main!(benches);
