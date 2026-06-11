//! MinHash + LSH near-duplicate clone detection at index scale.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

const MINHASH_SLOTS: usize = 64;
const LSH_BANDS: usize = 16;
const LSH_ROWS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneCandidate {
    pub id_a: String,
    pub id_b: String,
    pub path_a: String,
    pub path_b: String,
    pub jaccard: f64,
}

#[derive(Debug, Clone)]
pub struct FunctionBody {
    pub id: String,
    pub path: String,
    pub tokens: Vec<String>,
}

/// Tokenize source body for MinHash (normalize identifiers and keywords).
pub fn tokenize_body(source: &str) -> Vec<String> {
    source
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() > 2)
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

fn hash_token(token: &str, seed: u64) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    token.hash(&mut hasher);
    hasher.finish()
}

/// 64-slot MinHash signature.
pub fn minhash_signature(tokens: &[String]) -> Vec<u64> {
    let mut sig = vec![u64::MAX; MINHASH_SLOTS];
    if tokens.is_empty() {
        return sig;
    }
    for (slot, entry) in sig.iter_mut().enumerate() {
        for token in tokens {
            let h = hash_token(token, slot as u64);
            if h < *entry {
                *entry = h;
            }
        }
    }
    sig
}

fn jaccard_from_minhash(a: &[u64], b: &[u64]) -> f64 {
    let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
    matches as f64 / MINHASH_SLOTS as f64
}

fn lsh_bands(sig: &[u64]) -> Vec<u64> {
    let mut bands = Vec::with_capacity(LSH_BANDS);
    for band in 0..LSH_BANDS {
        let start = band * LSH_ROWS;
        let end = start + LSH_ROWS;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for v in &sig[start..end] {
            v.hash(&mut hasher);
        }
        bands.push(hasher.finish());
    }
    bands
}

/// Find clone pairs above Jaccard threshold using MinHash + LSH.
pub fn find_clone_pairs(
    bodies: &[FunctionBody],
    min_tokens: usize,
    jaccard_threshold: f64,
) -> Vec<CloneCandidate> {
    let mut signatures: HashMap<String, (Vec<u64>, String)> = HashMap::new();
    let mut buckets: HashMap<u64, Vec<String>> = HashMap::new();

    for body in bodies {
        if body.tokens.len() < min_tokens {
            continue;
        }
        let sig = minhash_signature(&body.tokens);
        signatures.insert(body.id.clone(), (sig.clone(), body.path.clone()));
        for band_hash in lsh_bands(&sig) {
            buckets
                .entry(band_hash)
                .or_default()
                .push(body.id.clone());
        }
    }

    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
    let mut out = Vec::new();

    for ids in buckets.values() {
        if ids.len() < 2 {
            continue;
        }
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let (a, b) = if ids[i] < ids[j] {
                    (ids[i].clone(), ids[j].clone())
                } else {
                    (ids[j].clone(), ids[i].clone())
                };
                if !seen_pairs.insert((a.clone(), b.clone())) {
                    continue;
                }
                let (sig_a, path_a) = signatures.get(&a).unwrap();
                let (sig_b, path_b) = signatures.get(&b).unwrap();
                let jac = jaccard_from_minhash(sig_a, sig_b);
                if jac >= jaccard_threshold {
                    out.push(CloneCandidate {
                        id_a: a,
                        id_b: b,
                        path_a: path_a.clone(),
                        path_b: path_b.clone(),
                        jaccard: jac,
                    });
                }
            }
        }
    }

    out.sort_by(|a, b| {
        b.jaccard
            .partial_cmp(&a.jaccard)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_bodies_pair() {
        let tokens = tokenize_body("fn foo() { let x = 1; return x + 1; }");
        let bodies = vec![
            FunctionBody {
                id: "a".into(),
                path: "a.rs".into(),
                tokens: tokens.clone(),
            },
            FunctionBody {
                id: "b".into(),
                path: "b.rs".into(),
                tokens,
            },
        ];
        let pairs = find_clone_pairs(&bodies, 5, 0.85);
        assert!(!pairs.is_empty());
        assert!(pairs[0].jaccard >= 0.85);
    }
}
