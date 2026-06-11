//! Multi-signal reranking for capsule and hybrid retrieval.

use crate::tfidf::tokenize;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

/// Tunable weights for rerank signals (Gortex-inspired defaults).
#[derive(Debug, Clone)]
pub struct RerankWeights {
    pub lexical: f64,
    pub vector: f64,
    pub centrality: f64,
    pub path_penalty: f64,
    pub definition_bias: f64,
    pub recency: f64,
    pub token_cost: f64,
}

impl Default for RerankWeights {
    fn default() -> Self {
        Self {
            lexical: 1.0,
            vector: 0.8,
            centrality: 0.6,
            path_penalty: 0.4,
            definition_bias: 0.6,
            recency: 0.3,
            token_cost: 0.25,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RerankCandidate {
    pub id: String,
    pub path: String,
    pub name: String,
    pub lexical_rank: usize,
    pub vector_rank: Option<usize>,
    pub lexical_score: f64,
    pub vector_score: f64,
    pub centrality: f64,
    pub token_estimate: usize,
    pub mtime_secs: Option<u64>,
}

/// Combined rerank score for a candidate.
pub fn rerank_score(query: &str, candidate: &RerankCandidate, weights: &RerankWeights) -> f64 {
    let lexical = rrf_kernel(candidate.lexical_rank, 60.0) * weights.lexical
        + candidate.lexical_score * 0.1;
    let vector = candidate
        .vector_rank
        .map(|r| rrf_kernel(r, 60.0) * weights.vector)
        .unwrap_or(candidate.vector_score * weights.vector);
    let centrality = candidate.centrality * weights.centrality;
    let path_penalty = path_penalty(&candidate.path) * weights.path_penalty;
    let definition = definition_bias(query, &candidate.name) * weights.definition_bias;
    let recency = recency_boost(candidate.mtime_secs) * weights.recency;
    let token_penalty =
        (candidate.token_estimate as f64 / 4000.0).min(1.0) * weights.token_cost;
    lexical + vector + centrality + definition + recency - path_penalty - token_penalty
}

pub fn rerank_candidates(
    query: &str,
    mut candidates: Vec<RerankCandidate>,
    weights: &RerankWeights,
) -> Vec<(String, f64)> {
    let mut scored: Vec<(String, f64)> = candidates
        .drain(..)
        .map(|c| {
            let score = rerank_score(query, &c, weights);
            (c.id, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn rrf_kernel(rank: usize, k: f64) -> f64 {
    1.0 / (k + rank as f64 + 1.0)
}

fn path_penalty(path: &str) -> f64 {
    let p = path.replace('\\', "/").to_lowercase();
    let mut penalty: f64 = 0.0;
    for marker in [
        "/test/",
        "/tests/",
        "_test.",
        "/fixtures/",
        "/generated/",
        "/vendor/",
        "/target/",
        "/node_modules/",
    ] {
        if p.contains(marker) {
            penalty += 0.35;
        }
    }
    penalty.min(1.0)
}

fn definition_bias(query: &str, symbol: &str) -> f64 {
    let q = query.trim();
    if q.is_empty() {
        return 0.0;
    }
    let qt = tokenize(q);
    if qt.len() == 1 && symbol.eq_ignore_ascii_case(&qt[0]) {
        1.0
    } else if symbol.to_lowercase().contains(&q.to_lowercase()) {
        0.5
    } else {
        0.0
    }
}

fn recency_boost(mtime_secs: Option<u64>) -> f64 {
    let Some(mtime) = mtime_secs else {
        return 0.0;
    };
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(mtime);
    let age_days = now.saturating_sub(mtime) as f64 / 86_400.0;
    (1.0 - (age_days / 365.0)).clamp(0.0, 1.0)
}

/// Stable etag from content bytes.
pub fn content_etag(content: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    format!("\"ccx-{:016x}\"", hasher.finish())
}

/// File mtime in seconds when path exists.
pub fn file_mtime_secs(path: &str) -> Option<u64> {
    let meta = std::fs::metadata(Path::new(path)).ok()?;
    meta.modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Lexical rank map from scored document ids.
pub fn rank_map_from_scores(scores: &[(String, f64)]) -> HashMap<String, usize> {
    scores
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_bias_prefers_exact_symbol() {
        let c = RerankCandidate {
            id: "a".into(),
            path: "src/lib.rs".into(),
            name: "authenticate".into(),
            lexical_rank: 0,
            vector_rank: None,
            lexical_score: 1.0,
            vector_score: 0.0,
            centrality: 0.1,
            token_estimate: 100,
            mtime_secs: None,
        };
        let w = RerankWeights::default();
        let s = rerank_score("authenticate", &c, &w);
        assert!(s > rerank_score("login flow", &c, &w));
    }

    #[test]
    fn path_penalty_demotes_tests() {
        assert!(path_penalty("src/auth/mod.rs") < path_penalty("src/auth/tests/mod.rs"));
    }
}
