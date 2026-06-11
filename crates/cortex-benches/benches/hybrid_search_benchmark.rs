//! Hybrid search rerank path (mock vector store + embedder).

use async_trait::async_trait;
use cortex_vector::{
    Embedder, HybridSearch, MetadataValue, SearchResult, SearchType, VectorDocument, VectorError,
    VectorStore,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct BenchEmbedder;

#[async_trait]
impl Embedder for BenchEmbedder {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, cortex_vector::EmbeddingError> {
        Ok(vec![0.1; cortex_vector::EMBEDDING_DIMENSION])
    }

    async fn embed_batch(
        &self,
        texts: &[&str],
    ) -> Result<Vec<Vec<f32>>, cortex_vector::EmbeddingError> {
        Ok(texts
            .iter()
            .map(|_| vec![0.1; cortex_vector::EMBEDDING_DIMENSION])
            .collect())
    }

    fn provider(&self) -> cortex_vector::EmbeddingProvider {
        cortex_vector::EmbeddingProvider::Ollama
    }

    fn model(&self) -> &str {
        "bench-mock"
    }
}

#[derive(Default)]
struct BenchVectorStore {
    results: Arc<Mutex<Vec<SearchResult>>>,
}

#[async_trait]
impl VectorStore for BenchVectorStore {
    async fn upsert(
        &self,
        _id: &str,
        _embedding: Vec<f32>,
        _metadata: HashMap<String, MetadataValue>,
    ) -> Result<(), VectorError> {
        Ok(())
    }

    async fn upsert_batch(&self, _documents: Vec<VectorDocument>) -> Result<usize, VectorError> {
        Ok(0)
    }

    async fn search(&self, _query: Vec<f32>, k: usize) -> Result<Vec<SearchResult>, VectorError> {
        let all = self.results.lock().expect("lock");
        Ok(all.iter().take(k).cloned().collect())
    }

    async fn search_with_filter(
        &self,
        _query: Vec<f32>,
        k: usize,
        _filter: HashMap<String, MetadataValue>,
    ) -> Result<Vec<SearchResult>, VectorError> {
        self.search(_query, k).await
    }

    async fn get(&self, _id: &str) -> Result<Option<VectorDocument>, VectorError> {
        Ok(None)
    }

    async fn delete(&self, _id: &str) -> Result<bool, VectorError> {
        Ok(false)
    }

    async fn delete_by_filter(
        &self,
        _filter: HashMap<String, MetadataValue>,
    ) -> Result<usize, VectorError> {
        Ok(0)
    }

    async fn count(&self) -> Result<usize, VectorError> {
        Ok(self.results.lock().expect("lock").len())
    }

    async fn health_check(&self) -> Result<bool, VectorError> {
        Ok(true)
    }
}

fn fixture_results(n: usize) -> Vec<SearchResult> {
    (0..n)
        .map(|i| SearchResult {
            id: format!("sym:{i}"),
            score: 0.5 + (i as f32 * 0.001),
            content: Some(format!("fn example_{i}() {{ /* body */ }}")),
            metadata: HashMap::from([
                (
                    "path".to_string(),
                    MetadataValue::String(format!("src/module_{i}.rs")),
                ),
                (
                    "name".to_string(),
                    MetadataValue::String(format!("example_{i}")),
                ),
                (
                    "kind".to_string(),
                    MetadataValue::String("Function".to_string()),
                ),
            ]),
        })
        .collect()
}

fn bench_hybrid_search(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let store = Arc::new(BenchVectorStore {
        results: Arc::new(Mutex::new(fixture_results(200))),
    });
    let hybrid = HybridSearch::new(store, Arc::new(BenchEmbedder));

    c.bench_function("hybrid_search_rerank_200_candidates", |b| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let start = std::time::Instant::now();
                rt.block_on(async {
                    let out = hybrid
                        .search("refactor auth middleware", SearchType::Hybrid, 20)
                        .await
                        .expect("hybrid search");
                    assert!(!out.is_empty());
                });
                total += start.elapsed();
            }
            total
        });
    });
}

criterion_group!(benches, bench_hybrid_search);
criterion_main!(benches);
