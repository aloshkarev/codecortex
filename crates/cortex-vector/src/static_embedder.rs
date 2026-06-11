//! Zero-dependency static embedder fallback (hash-expanded bag-of-words vectors).

use crate::embedder::{Embedder, EmbeddingError, EmbeddingProvider};
use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const STATIC_DIM: usize = 128;

/// Deterministic static embedder — no network, no model download.
pub struct StaticEmbedder {
    dim: usize,
}

impl StaticEmbedder {
    pub fn new() -> Self {
        Self { dim: STATIC_DIM }
    }

    pub fn with_dim(dim: usize) -> Self {
        Self { dim: dim.max(16) }
    }

    fn tokenize(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch.to_ascii_lowercase());
            } else if !current.is_empty() {
                if current.len() > 1 {
                    out.push(current.clone());
                }
                current.clear();
            }
        }
        if current.len() > 1 {
            out.push(current);
        }
        out
    }

    fn embed_sync(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dim];
        for token in Self::tokenize(text) {
            let mut hasher = DefaultHasher::new();
            token.hash(&mut hasher);
            let h = hasher.finish();
            let idx = (h as usize) % self.dim;
            let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
            vec[idx] += sign;
        }
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vec {
                *v /= norm;
            }
        }
        vec
    }
}

impl Default for StaticEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Embedder for StaticEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Ok(self.embed_sync(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(texts.iter().map(|t| self.embed_sync(t)).collect())
    }

    fn provider(&self) -> EmbeddingProvider {
        EmbeddingProvider::Test
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model(&self) -> &str {
        "static-fallback"
    }
}

/// Wraps a primary embedder and falls back to static vectors on failure.
pub struct FallbackEmbedder {
    primary: std::sync::Arc<dyn Embedder>,
    fallback: StaticEmbedder,
    label: String,
}

impl FallbackEmbedder {
    pub fn new(primary: std::sync::Arc<dyn Embedder>, fallback: StaticEmbedder) -> Self {
        Self {
            primary,
            fallback,
            label: "static-fallback".to_string(),
        }
    }

    pub fn fallback_label(&self) -> &str {
        &self.label
    }
}

#[async_trait]
impl Embedder for FallbackEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        match self.primary.embed(text).await {
            Ok(v) => Ok(v),
            Err(_) => {
                self.fallback.embed(text).await
            }
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        match self.primary.embed_batch(texts).await {
            Ok(v) => Ok(v),
            Err(_) => self.fallback.embed_batch(texts).await,
        }
    }

    fn provider(&self) -> EmbeddingProvider {
        self.primary.provider()
    }

    fn dimension(&self) -> usize {
        self.primary.dimension()
    }

    fn model(&self) -> &str {
        self.fallback.model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn static_embedder_is_deterministic() {
        let e = StaticEmbedder::new();
        let a = e.embed("authenticate token").await.unwrap();
        let b = e.embed("authenticate token").await.unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), STATIC_DIM);
    }
}
