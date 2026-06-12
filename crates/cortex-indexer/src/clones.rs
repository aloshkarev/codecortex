//! Index-time MinHash+LSH clone detection and `SIMILAR_TO` edge materialization.

use cortex_analyzer::clones::{CloneCandidate, FunctionBody, find_clone_pairs, tokenize_body};
use cortex_core::{CodeEdge, EdgeKind, EntityKind, IndexedFile, Result};
use cortex_graph::GraphClient;
use std::collections::HashMap;

const MIN_TOKENS: usize = 50;
const JACCARD_THRESHOLD: f64 = 0.85;

/// Collects function bodies during indexing for clone detection.
#[derive(Debug, Default)]
pub struct CloneAccumulator {
    bodies: Vec<FunctionBody>,
}

impl CloneAccumulator {
    pub fn push_file(&mut self, file: &IndexedFile) {
        for node in &file.nodes {
            if !matches!(node.kind, EntityKind::Function | EntityKind::Method) {
                continue;
            }
            let Some(source) = node.source.as_deref() else {
                continue;
            };
            let tokens = tokenize_body(source);
            if tokens.len() < MIN_TOKENS {
                continue;
            }
            self.bodies.push(FunctionBody {
                id: node.id.clone(),
                path: file.path.clone(),
                tokens,
            });
        }
    }

    pub fn bodies(&self) -> &[FunctionBody] {
        &self.bodies
    }
}

/// Find clone pairs from accumulated bodies.
pub fn compute_clone_pairs(accumulator: &CloneAccumulator) -> Vec<CloneCandidate> {
    find_clone_pairs(accumulator.bodies(), MIN_TOKENS, JACCARD_THRESHOLD)
}

/// Remove existing `SIMILAR_TO` edges for a repository before rewriting clone index.
pub async fn clear_similar_to_edges(client: &GraphClient, repository_path: &str) -> Result<()> {
    let query = format!(
        "MATCH (a:CodeNode {{repository_path: '{}'}})-[r:SIMILAR_TO]->() DELETE r",
        repository_path.replace('\'', "\\'")
    );
    client.raw_query(&query).await?;
    Ok(())
}

/// Persist `SIMILAR_TO` edges for clone pairs (undirected: one edge per pair).
pub async fn write_clone_edges_to_graph(
    client: &GraphClient,
    repository_path: &str,
    pairs: &[CloneCandidate],
    chunk_size: usize,
    replace_existing: bool,
) -> Result<()> {
    if pairs.is_empty() {
        return Ok(());
    }

    if replace_existing {
        clear_similar_to_edges(client, repository_path).await?;
    }

    let mut edges = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let mut props = HashMap::new();
        props.insert("jaccard".to_string(), format!("{:.4}", pair.jaccard));
        edges.push(CodeEdge {
            from: pair.id_a.clone(),
            to: pair.id_b.clone(),
            kind: EdgeKind::SimilarTo,
            properties: props,
        });
    }

    let chunk = chunk_size.max(1);
    for batch in edges.chunks(chunk) {
        client.bulk_upsert_edges(batch).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::Language;

    #[test]
    fn accumulator_collects_function_bodies() {
        let mut acc = CloneAccumulator::default();
        let file = IndexedFile {
            path: "a.rs".into(),
            language: Language::Rust,
            content_hash: "h".into(),
            nodes: vec![cortex_core::CodeNode {
                id: "func:a".into(),
                kind: EntityKind::Function,
                name: "foo".into(),
                path: Some("a.rs".into()),
                line_number: Some(1),
                lang: Some(Language::Rust),
                source: Some("fn foo() { let x = 1; let y = 2; let z = x + y; return z; }".into()),
                docstring: None,
                properties: HashMap::new(),
            }],
            edges: vec![],
        };
        acc.push_file(&file);
        assert_eq!(acc.bodies().len(), 1);
    }
}
