//! Benchmarks for TF-IDF Scoring
//!
//! Measures performance of:
//! - Document tokenization
//! - TF-IDF score computation
//! - Corpus building
//! - Query scoring

#![allow(clippy::needless_borrows_for_generic_args)]

use cortex_mcp::{Document, TfIdfScorer};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

fn generate_documents(count: usize) -> Vec<Document> {
    (0..count)
        .map(|i| {
            Document::new(
                &format!("doc:{}", i),
                &format!(
                    "function {} implementation handles data processing and validation \
                     with support for async operations error handling and result caching \
                     module {} class service handler processor",
                    i,
                    i % 10
                ),
            )
        })
        .collect()
}

fn generate_code_documents(count: usize) -> Vec<Document> {
    let code_snippets = [
        "pub fn authenticate(user: &str, pass: &str) -> Result<Token, AuthError> { validate_credentials(user, pass) }",
        "impl UserService { pub fn create(&self, name: &str) -> User { User::new(name) } }",
        "async fn fetch_data(url: &str) -> Result<String, Error> { reqwest::get(url).await?.text().await }",
        "struct Cache { data: HashMap<String, String>, ttl: Duration }",
        "fn process_items<T: Clone>(items: &[T]) -> Vec<T> { items.to_vec() }",
    ];

    (0..count)
        .map(|i| {
            let snippet = code_snippets[i % code_snippets.len()];
            Document::new(&format!("func:{}", i), snippet)
        })
        .collect()
}

fn bench_tokenization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization");

    let test_cases = vec![
        ("short", "hello world"),
        (
            "medium",
            "function implementation handles data processing with async support",
        ),
        (
            "long",
            "pub fn authenticate_user_with_credentials(credentials: Credentials) -> Result<AuthToken, AuthenticationError> where Credentials: Validate { /* implementation */ }",
        ),
        (
            "code",
            "async fn fetch_data<T: Serialize>(url: &str, client: &Client) -> Result<T, Error>",
        ),
    ];

    for (name, text) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("tokenize", name),
            &(name, text),
            |b, &(_, t)| {
                b.iter(|| {
                    let tokens = cortex_mcp::tokenize(black_box(t));
                    black_box(tokens)
                });
            },
        );
    }

    group.finish();
}

fn bench_tfidf_corpus_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("tfidf_corpus");

    for size in [10, 50, 100, 500, 1000].iter() {
        let docs = generate_documents(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("build", size), size, |b, _| {
            b.iter(|| {
                let scorer = TfIdfScorer::from_documents(black_box(&docs));
                black_box(scorer)
            });
        });
    }

    group.finish();
}

fn bench_tfidf_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("tfidf_score");

    // Build scorer with 100 documents
    let docs = generate_documents(100);
    let scorer = TfIdfScorer::from_documents(&docs);

    let queries: [Vec<String>; 4] = [
        vec!["function".to_string()],
        vec!["data".to_string(), "processing".to_string()],
        vec![
            "async".to_string(),
            "error".to_string(),
            "handling".to_string(),
        ],
        vec![
            "implementation".to_string(),
            "validation".to_string(),
            "caching".to_string(),
        ],
    ];

    for (i, query) in queries.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("query", format!("{}_terms", query.len())),
            &i,
            |b, _| {
                let doc = &docs[0];
                b.iter(|| {
                    let score = scorer.score(black_box(query), black_box(doc));
                    black_box(score)
                });
            },
        );
    }

    group.finish();
}

fn bench_tfidf_score_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("tfidf_score_all");

    for size in [10, 50, 100, 500].iter() {
        let docs = generate_documents(*size);
        let scorer = TfIdfScorer::from_documents(&docs);
        let query = "function data";
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("score_all", size), size, |b, _| {
            b.iter(|| {
                let scores = scorer.score_all(black_box(query), black_box(&docs));
                black_box(scores)
            });
        });
    }

    group.finish();
}

fn bench_code_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("code_search");

    // Simulate a code search scenario
    let docs = generate_code_documents(500);
    let scorer = TfIdfScorer::from_documents(&docs);

    let code_queries = vec![
        ("single_term", "authenticate"),
        ("two_terms", "fetch data"),
        ("three_terms_unique", "cache data struct"),
        ("with_type", "result error"),
    ];

    for (name, query) in &code_queries {
        group.bench_with_input(BenchmarkId::new("query", name), name, |b, _| {
            b.iter(|| {
                let scores = scorer.score_all(black_box(query), black_box(&docs));
                // Find top results
                let mut scored: Vec<_> = scores.into_iter().enumerate().collect();
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                black_box(scored.into_iter().take(10).collect::<Vec<_>>())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_tokenization,
    bench_tfidf_corpus_build,
    bench_tfidf_scoring,
    bench_tfidf_score_all,
    bench_code_search,
);

criterion_main!(benches);
