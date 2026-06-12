//! Precomputed incoming-caller reach index (depth 1–3) from parsed `CALLS` edges.

use cortex_core::{CodeEdge, EdgeKind, Result};
use cortex_graph::GraphParam;
use std::collections::{HashMap, HashSet, VecDeque};

pub const REACH_D1_COUNT: &str = "reach_d1_count";
pub const REACH_D3_IDS: &str = "reach_d3_ids";
pub const REACH_TRUNCATED: &str = "reach_truncated";

/// Per-symbol reach summary stored on graph nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReachEntry {
    pub reach_d1_count: usize,
    pub reach_d3_ids: Vec<String>,
    pub reach_truncated: bool,
}

/// Incoming-caller reach for all symbols seen in parsed call edges.
#[derive(Debug, Clone, Default)]
pub struct ReachIndex {
    pub entries: HashMap<String, ReachEntry>,
}

/// Collects `CALLS` edges during indexing without retaining the full edge list.
#[derive(Debug, Default)]
pub struct ReachAccumulator {
    calls: Vec<(String, String)>,
    symbol_ids: HashSet<String>,
}

impl ReachAccumulator {
    pub fn push_edge(&mut self, edge: &CodeEdge) {
        if edge.kind != EdgeKind::Calls {
            return;
        }
        self.symbol_ids.insert(edge.from.clone());
        self.symbol_ids.insert(edge.to.clone());
        self.calls.push((edge.from.clone(), edge.to.clone()));
    }

    pub fn calls(&self) -> &[(String, String)] {
        &self.calls
    }

    pub fn symbol_ids(&self) -> &HashSet<String> {
        &self.symbol_ids
    }
}

/// Build a reach index from parsed caller→callee pairs.
pub fn compute_reach_index(
    accumulator: &ReachAccumulator,
    depth: usize,
    max_ids: usize,
) -> ReachIndex {
    let depth = depth.clamp(1, 3);
    let max_ids = max_ids.max(1);

    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
    for (caller, callee) in &accumulator.calls {
        reverse
            .entry(callee.as_str())
            .or_default()
            .push(caller.as_str());
    }

    let mut entries = HashMap::new();
    for symbol_id in &accumulator.symbol_ids {
        let direct: Vec<&str> = reverse
            .get(symbol_id.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .to_vec();

        let reach_d1_count = direct.len();
        let mut reach_d3_ids = Vec::new();
        let mut reach_truncated = false;

        let mut seen: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(&str, usize)> = VecDeque::new();
        for caller in &direct {
            if seen.insert(*caller) {
                queue.push_back((*caller, 1));
            }
        }

        while let Some((id, d)) = queue.pop_front() {
            if reach_d3_ids.len() >= max_ids {
                reach_truncated = true;
                break;
            }
            reach_d3_ids.push(id.to_string());
            if d < depth {
                if let Some(parents) = reverse.get(id) {
                    for parent in parents {
                        if seen.insert(*parent) {
                            queue.push_back((*parent, d + 1));
                        }
                    }
                }
            }
        }

        entries.insert(
            symbol_id.clone(),
            ReachEntry {
                reach_d1_count,
                reach_d3_ids,
                reach_truncated,
            },
        );
    }

    ReachIndex { entries }
}

pub fn apply_reach_properties(props: &mut HashMap<String, String>, entry: &ReachEntry) {
    props.insert(REACH_D1_COUNT.to_string(), entry.reach_d1_count.to_string());
    props.insert(
        REACH_D3_IDS.to_string(),
        serde_json::to_string(&entry.reach_d3_ids).unwrap_or_else(|_| "[]".to_string()),
    );
    props.insert(
        REACH_TRUNCATED.to_string(),
        entry.reach_truncated.to_string(),
    );
}

/// Persist reach fields on existing graph nodes (top-level properties for slim bulk upserts).
pub async fn write_reach_to_graph(
    client: &cortex_graph::GraphClient,
    reach: &ReachIndex,
    chunk_size: usize,
) -> Result<()> {
    if reach.entries.is_empty() {
        return Ok(());
    }

    let chunk = chunk_size.max(1);
    let items: Vec<GraphParam> = reach
        .entries
        .iter()
        .map(|(id, entry)| {
            let mut item = HashMap::new();
            item.insert("id".to_string(), GraphParam::String(id.clone()));
            item.insert(
                "reach_d1_count".to_string(),
                GraphParam::Int(entry.reach_d1_count as i64),
            );
            item.insert(
                "reach_d3_ids".to_string(),
                GraphParam::String(
                    serde_json::to_string(&entry.reach_d3_ids).unwrap_or_else(|_| "[]".to_string()),
                ),
            );
            item.insert(
                "reach_truncated".to_string(),
                GraphParam::Bool(entry.reach_truncated),
            );
            GraphParam::Map(item)
        })
        .collect();

    for batch in items.chunks(chunk) {
        let mut params = HashMap::new();
        params.insert("batch".to_string(), GraphParam::List(batch.to_vec()));
        client
            .execute_with_raw_param_map(
                "UNWIND $batch AS item
                 MATCH (n:CodeNode {id: item.id})
                 SET n.reach_d1_count = item.reach_d1_count,
                     n.reach_d3_ids = item.reach_d3_ids,
                     n.reach_truncated = item.reach_truncated",
                params,
            )
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn calls_edge(from: &str, to: &str) -> CodeEdge {
        CodeEdge {
            from: from.to_string(),
            to: to.to_string(),
            kind: EdgeKind::Calls,
            properties: HashMap::new(),
        }
    }

    fn build_fixture_index() -> ReachIndex {
        // c -> b -> a, e -> b, d -> a
        let mut acc = ReachAccumulator::default();
        for edge in [
            calls_edge("func:c", "func:b"),
            calls_edge("func:b", "func:a"),
            calls_edge("func:e", "func:b"),
            calls_edge("func:d", "func:a"),
        ] {
            acc.push_edge(&edge);
        }
        compute_reach_index(&acc, 3, 64)
    }

    #[test]
    fn reach_d1_counts_direct_callers() {
        let index = build_fixture_index();
        let a = index.entries.get("func:a").expect("func:a");
        assert_eq!(a.reach_d1_count, 2);
        assert!(a.reach_d3_ids.contains(&"func:b".to_string()));
        assert!(a.reach_d3_ids.contains(&"func:d".to_string()));

        let b = index.entries.get("func:b").expect("func:b");
        assert_eq!(b.reach_d1_count, 2);
        assert!(b.reach_d3_ids.contains(&"func:c".to_string()));
        assert!(b.reach_d3_ids.contains(&"func:e".to_string()));
    }

    #[test]
    fn reach_includes_transitive_callers_within_depth() {
        let index = build_fixture_index();
        let a = index.entries.get("func:a").expect("func:a");
        assert!(a.reach_d3_ids.contains(&"func:c".to_string()));
        assert!(a.reach_d3_ids.contains(&"func:e".to_string()));
    }

    #[test]
    fn reach_truncates_id_list() {
        let mut acc = ReachAccumulator::default();
        for i in 0..10 {
            acc.push_edge(&calls_edge(&format!("func:caller_{i}"), "func:target"));
        }
        let index = compute_reach_index(&acc, 3, 4);
        let target = index.entries.get("func:target").expect("target");
        assert_eq!(target.reach_d3_ids.len(), 4);
        assert!(target.reach_truncated);
        assert_eq!(target.reach_d1_count, 10);
    }

    #[test]
    fn reach_zero_for_leaf_caller() {
        let index = build_fixture_index();
        let c = index.entries.get("func:c").expect("func:c");
        assert_eq!(c.reach_d1_count, 0);
        assert!(c.reach_d3_ids.is_empty());
        assert!(!c.reach_truncated);
    }

    #[test]
    fn apply_reach_properties_roundtrip() {
        let entry = ReachEntry {
            reach_d1_count: 2,
            reach_d3_ids: vec!["func:b".to_string()],
            reach_truncated: false,
        };
        let mut props = HashMap::new();
        apply_reach_properties(&mut props, &entry);
        assert_eq!(props.get(REACH_D1_COUNT).map(String::as_str), Some("2"));
        assert_eq!(
            props.get(REACH_D3_IDS).map(String::as_str),
            Some("[\"func:b\"]")
        );
        assert_eq!(
            props.get(REACH_TRUNCATED).map(String::as_str),
            Some("false")
        );
    }

    #[test]
    fn non_call_edges_ignored() {
        let mut acc = ReachAccumulator::default();
        acc.push_edge(&CodeEdge {
            from: "file:a".to_string(),
            to: "file:b".to_string(),
            kind: EdgeKind::Imports,
            properties: HashMap::new(),
        });
        assert!(acc.calls().is_empty());
        assert!(acc.symbol_ids().is_empty());
    }
}
