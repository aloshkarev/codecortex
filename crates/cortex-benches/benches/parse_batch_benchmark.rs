use cortex_parser::ParserRegistry;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn collect_rs_files(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
}

fn bench_fixture_tree(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../cortex-parser/tests/fixtures/sample_project_rust");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    let registry = ParserRegistry::new();

    c.bench_function("parse_batch_fixture_tree", |b| {
        b.iter(|| {
            for path in &files {
                let source = std::fs::read_to_string(path).expect("read fixture");
                let parser = registry.parser_for_path(path).expect("parser");
                let _ = parser.parse(&source, path).expect("parse");
            }
        })
    });
}

criterion_group!(benches, bench_fixture_tree);
criterion_main!(benches);
