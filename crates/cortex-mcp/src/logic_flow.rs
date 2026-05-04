//! Logic Flow Search for Multi-Path Ranking
//!
//! Implements logic flow path finding between two symbols:
//! - Finds shortest path + alternatives
//! - Supports partial results with blockers
//! - Ranked by relevance and confidence

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Maximum search depth
const DEFAULT_MAX_DEPTH: usize = 12;

/// Maximum number of paths to return
const DEFAULT_MAX_PATHS: usize = 5;

/// A node in a logic flow path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathNode {
    /// Unique identifier
    pub id: String,
    /// Symbol name
    pub name: String,
    /// File path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Node type (function, method, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Line number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u64>,
}

/// An edge in a logic flow path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Edge type (calls, returns, etc.)
    pub edge_type: String,
    /// Confidence score
    pub confidence: f64,
}

/// A scored path between two symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPath {
    /// Nodes in the path (in order)
    pub nodes: Vec<PathNode>,
    /// Edges in the path (in order)
    pub edges: Vec<PathEdge>,
    /// Path length (number of edges)
    pub length: usize,
    /// Overall path score
    pub score: f64,
    /// Score breakdown
    pub score_breakdown: PathScoreBreakdown,
}

/// Breakdown of path scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathScoreBreakdown {
    /// Score component for path length (shorter is better)
    pub length_score: f64,
    /// Score component for edge confidence
    pub confidence_score: f64,
    /// Score component for node relevance
    pub relevance_score: f64,
}

/// Blocker information when no path is found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Node that blocks the path
    pub node: PathNode,
    /// Reason why this node blocks
    pub reason: String,
    /// Suggestions for unblocking
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

/// Result of logic flow search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicFlowResult {
    /// Found paths (sorted by score)
    pub paths: Vec<ScoredPath>,
    /// Maximum depth that was searched
    pub searched_depth: usize,
    /// Whether this is a partial result
    pub partial: bool,
    /// Blockers if no path found and allow_partial=true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockers: Option<Vec<Blocker>>,
    /// Any warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Raw edge from graph query
#[derive(Debug, Clone)]
pub struct RawEdge {
    pub from_id: String,
    pub from_name: String,
    pub from_path: Option<String>,
    pub from_kind: Option<String>,
    pub from_line: Option<u64>,
    pub to_id: String,
    pub to_name: String,
    pub to_path: Option<String>,
    pub to_kind: Option<String>,
    pub to_line: Option<u64>,
    pub edge_type: String,
    pub confidence: f64,
}

/// Logic flow searcher
pub struct LogicFlowSearcher {
    max_depth: usize,
    max_paths: usize,
    allow_partial: bool,
}

impl LogicFlowSearcher {
    /// Create a new searcher with default settings
    pub fn new() -> Self {
        Self {
            max_depth: DEFAULT_MAX_DEPTH,
            max_paths: DEFAULT_MAX_PATHS,
            allow_partial: true,
        }
    }

    /// Set maximum search depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set maximum paths to return
    pub fn with_max_paths(mut self, max: usize) -> Self {
        self.max_paths = max;
        self
    }

    /// Set allow partial flag
    pub fn with_partial(mut self, allow: bool) -> Self {
        self.allow_partial = allow;
        self
    }

    /// Search for paths between two symbols
    pub fn search(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        edges: Vec<RawEdge>,
    ) -> LogicFlowResult {
        // Build adjacency list
        let mut graph: HashMap<String, Vec<&RawEdge>> = HashMap::new();
        for edge in &edges {
            graph.entry(edge.from_id.clone()).or_default().push(edge);
        }

        // Build reverse lookup for node info
        let mut node_info: HashMap<String, &RawEdge> = HashMap::new();
        for edge in &edges {
            if !node_info.contains_key(&edge.from_id) {
                node_info.insert(edge.from_id.clone(), edge);
            }
            if !node_info.contains_key(&edge.to_id) {
                node_info.insert(edge.to_id.clone(), edge);
            }
        }

        // Find candidate start and end nodes
        let start_candidates: Vec<String> = edges
            .iter()
            .filter(|e| e.from_name == from_symbol)
            .map(|e| e.from_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let end_candidates: Vec<String> = edges
            .iter()
            .filter(|e| e.to_name == to_symbol)
            .map(|e| e.to_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if start_candidates.is_empty() || end_candidates.is_empty() {
            return self.no_result(from_symbol, to_symbol, &edges);
        }

        // Find all paths using BFS
        let mut all_paths: Vec<Vec<String>> = Vec::new();
        let end_set: HashSet<_> = end_candidates.iter().cloned().collect();

        for start in &start_candidates {
            let paths = self.bfs_all_paths(start, &end_set, &graph);
            all_paths.extend(paths);
        }

        if all_paths.is_empty() {
            return self.handle_no_path(from_symbol, to_symbol, &edges);
        }

        // Convert paths to scored paths
        let mut scored_paths: Vec<ScoredPath> = all_paths
            .into_iter()
            .filter_map(|path| self.build_scored_path(&path, &edges, &node_info))
            .collect();

        // Sort by score (descending)
        scored_paths.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Truncate to max paths
        scored_paths.truncate(self.max_paths);

        LogicFlowResult {
            paths: scored_paths,
            searched_depth: self.max_depth,
            partial: false,
            blockers: None,
            warnings: Vec::new(),
        }
    }

    /// BFS to find all paths from start to any end node
    fn bfs_all_paths(
        &self,
        start: &str,
        end_set: &HashSet<String>,
        graph: &HashMap<String, Vec<&RawEdge>>,
    ) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut queue: VecDeque<(String, Vec<String>, HashSet<String>)> = VecDeque::new();

        let mut initial_visited = HashSet::new();
        initial_visited.insert(start.to_string());
        queue.push_back((start.to_string(), vec![start.to_string()], initial_visited));

        while let Some((current, path, visited)) = queue.pop_front() {
            if path.len() > self.max_depth + 1 {
                continue;
            }

            if end_set.contains(&current) && path.len() > 1 {
                paths.push(path);
                if paths.len() >= self.max_paths * 2 {
                    break; // Found enough paths
                }
                continue;
            }

            if let Some(neighbors) = graph.get(&current) {
                for edge in neighbors {
                    if !visited.contains(&edge.to_id) {
                        let mut new_visited = visited.clone();
                        new_visited.insert(edge.to_id.clone());

                        let mut new_path = path.clone();
                        new_path.push(edge.to_id.clone());

                        queue.push_back((edge.to_id.clone(), new_path, new_visited));
                    }
                }
            }
        }

        paths
    }

    /// Build a scored path from node IDs
    fn build_scored_path(
        &self,
        node_ids: &[String],
        edges: &[RawEdge],
        node_info: &HashMap<String, &RawEdge>,
    ) -> Option<ScoredPath> {
        if node_ids.len() < 2 {
            return None;
        }

        let mut path_nodes = Vec::new();
        let mut path_edges = Vec::new();
        let mut total_confidence = 0.0;

        for (i, node_id) in node_ids.iter().enumerate() {
            let info = node_info.get(node_id)?;

            let node = if i == 0 {
                PathNode {
                    id: node_id.clone(),
                    name: info.from_name.clone(),
                    path: info.from_path.clone(),
                    kind: info.from_kind.clone(),
                    line_number: info.from_line,
                }
            } else {
                PathNode {
                    id: node_id.clone(),
                    name: info.to_name.clone(),
                    path: info.to_path.clone(),
                    kind: info.to_kind.clone(),
                    line_number: info.to_line,
                }
            };

            path_nodes.push(node);

            if i < node_ids.len() - 1 {
                // Find the edge between this node and the next
                let edge = edges
                    .iter()
                    .find(|e| e.from_id == *node_id && e.to_id == node_ids[i + 1])?;

                path_edges.push(PathEdge {
                    from: edge.from_id.clone(),
                    to: edge.to_id.clone(),
                    edge_type: edge.edge_type.clone(),
                    confidence: edge.confidence,
                });

                total_confidence += edge.confidence;
            }
        }

        // Calculate scores
        let length = path_edges.len();
        let length_score = 1.0 / (1.0 + length as f64);
        let confidence_score = if path_edges.is_empty() {
            0.0
        } else {
            total_confidence / path_edges.len() as f64
        };
        let relevance_score = 0.5; // Placeholder for relevance scoring

        let score = length_score * 0.4 + confidence_score * 0.4 + relevance_score * 0.2;

        Some(ScoredPath {
            nodes: path_nodes,
            edges: path_edges,
            length,
            score,
            score_breakdown: PathScoreBreakdown {
                length_score,
                confidence_score,
                relevance_score,
            },
        })
    }

    /// Handle no path found case
    fn handle_no_path(
        &self,
        from_symbol: &str,
        to_symbol: &str,
        edges: &[RawEdge],
    ) -> LogicFlowResult {
        let mut warnings = vec!["no_path_found".to_string()];

        let blockers = if self.allow_partial {
            // Find potential blockers - nodes that are close to either end
            let from_nodes: Vec<_> = edges
                .iter()
                .filter(|e| e.from_name == from_symbol)
                .collect();

            let to_nodes: Vec<_> = edges.iter().filter(|e| e.to_name == to_symbol).collect();

            let mut blockers = Vec::new();

            for node in from_nodes.iter().take(3) {
                blockers.push(Blocker {
                    node: PathNode {
                        id: node.from_id.clone(),
                        name: node.from_name.clone(),
                        path: node.from_path.clone(),
                        kind: node.from_kind.clone(),
                        line_number: node.from_line,
                    },
                    reason: "no_outgoing_path_to_target".to_string(),
                    suggestions: vec![
                        "check_if_target_is_reachable".to_string(),
                        "verify_call_relationships".to_string(),
                    ],
                });
            }

            for node in to_nodes.iter().take(3) {
                blockers.push(Blocker {
                    node: PathNode {
                        id: node.to_id.clone(),
                        name: node.to_name.clone(),
                        path: node.to_path.clone(),
                        kind: node.to_kind.clone(),
                        line_number: node.to_line,
                    },
                    reason: "no_incoming_path_from_source".to_string(),
                    suggestions: vec![
                        "check_if_source_can_reach_this".to_string(),
                        "verify_import_relationships".to_string(),
                    ],
                });
            }

            if blockers.is_empty() {
                warnings.push("no_candidates_found".to_string());
            }

            Some(blockers)
        } else {
            None
        };

        LogicFlowResult {
            paths: Vec::new(),
            searched_depth: self.max_depth,
            partial: true,
            blockers,
            warnings,
        }
    }

    /// Create a no-result response
    fn no_result(&self, from_symbol: &str, to_symbol: &str, _edges: &[RawEdge]) -> LogicFlowResult {
        let mut warnings = vec!["no_candidates_found".to_string()];

        let blockers = if self.allow_partial {
            let mut blockers = Vec::new();

            // Add source blocker
            blockers.push(Blocker {
                node: PathNode {
                    id: format!("source:{}", from_symbol),
                    name: from_symbol.to_string(),
                    path: None,
                    kind: None,
                    line_number: None,
                },
                reason: "source_symbol_not_found".to_string(),
                suggestions: vec!["verify_symbol_exists".to_string()],
            });

            // Add target blocker
            blockers.push(Blocker {
                node: PathNode {
                    id: format!("target:{}", to_symbol),
                    name: to_symbol.to_string(),
                    path: None,
                    kind: None,
                    line_number: None,
                },
                reason: "target_symbol_not_found".to_string(),
                suggestions: vec!["verify_symbol_exists".to_string()],
            });

            Some(blockers)
        } else {
            None
        };

        warnings.push(format!("source_{}_not_found", from_symbol));
        warnings.push(format!("target_{}_not_found", to_symbol));

        LogicFlowResult {
            paths: Vec::new(),
            searched_depth: self.max_depth,
            partial: true,
            blockers,
            warnings,
        }
    }
}

impl Default for LogicFlowSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edge(from_id: &str, from_name: &str, to_id: &str, to_name: &str) -> RawEdge {
        RawEdge {
            from_id: from_id.to_string(),
            from_name: from_name.to_string(),
            from_path: Some("/src/a.rs".to_string()),
            from_kind: Some("Function".to_string()),
            from_line: Some(1),
            to_id: to_id.to_string(),
            to_name: to_name.to_string(),
            to_path: Some("/src/b.rs".to_string()),
            to_kind: Some("Function".to_string()),
            to_line: Some(2),
            edge_type: "calls".to_string(),
            confidence: 0.9,
        }
    }

    #[test]
    fn logic_flow_finds_direct_path() {
        let searcher = LogicFlowSearcher::new();

        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:b", "func_b", "func:c", "func_c"),
        ];

        let result = searcher.search("func_a", "func_b", edges);

        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].length, 1);
        assert!(!result.partial);
    }

    #[test]
    fn logic_flow_finds_transitive_path() {
        let searcher = LogicFlowSearcher::new();

        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:b", "func_b", "func:c", "func_c"),
        ];

        let result = searcher.search("func_a", "func_c", edges);

        assert!(!result.paths.is_empty());
        assert_eq!(result.paths[0].length, 2);
    }

    #[test]
    fn logic_flow_no_path_with_blockers() {
        let searcher = LogicFlowSearcher::new().with_partial(true);

        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:c", "func_c", "func:d", "func_d"), // Disconnected
        ];

        let result = searcher.search("func_a", "func_d", edges);

        assert!(result.paths.is_empty());
        assert!(result.partial);
        assert!(result.blockers.is_some());
    }

    #[test]
    fn logic_flow_no_path_without_partial() {
        let searcher = LogicFlowSearcher::new().with_partial(false);

        let edges = vec![make_edge("func:a", "func_a", "func:b", "func_b")];

        let result = searcher.search("func_a", "func_c", edges);

        assert!(result.paths.is_empty());
        assert!(result.partial);
        assert!(result.blockers.is_none());
    }

    #[test]
    fn logic_flow_respects_max_depth() {
        let searcher = LogicFlowSearcher::new().with_max_depth(2);

        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:b", "func_b", "func:c", "func_c"),
            make_edge("func:c", "func_c", "func:d", "func_d"),
        ];

        let result = searcher.search("func_a", "func_d", edges);

        // Path is longer than max_depth, should not be found
        assert!(result.paths.is_empty());
    }

    #[test]
    fn logic_flow_respects_max_paths() {
        let searcher = LogicFlowSearcher::new().with_max_paths(1);

        // Create two paths: a->b->d and a->c->d
        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:a", "func_a", "func:c", "func_c"),
            make_edge("func:b", "func_b", "func:d", "func_d"),
            make_edge("func:c", "func_c", "func:d", "func_d"),
        ];

        let result = searcher.search("func_a", "func_d", edges);

        assert!(result.paths.len() <= 1);
    }

    #[test]
    fn logic_flow_scores_paths() {
        let searcher = LogicFlowSearcher::new();

        let edges = vec![
            make_edge("func:a", "func_a", "func:b", "func_b"),
            make_edge("func:b", "func_b", "func:c", "func_c"),
        ];

        let result = searcher.search("func_a", "func_c", edges);

        assert!(!result.paths.is_empty());
        let path = &result.paths[0];

        assert!(path.score > 0.0);
        assert!(path.score_breakdown.length_score > 0.0);
        assert!(path.score_breakdown.confidence_score > 0.0);
    }

    #[test]
    fn scored_path_serialization() {
        let path = ScoredPath {
            nodes: vec![PathNode {
                id: "func:a".to_string(),
                name: "func_a".to_string(),
                path: Some("/src/a.rs".to_string()),
                kind: Some("Function".to_string()),
                line_number: Some(1),
            }],
            edges: vec![],
            length: 0,
            score: 0.5,
            score_breakdown: PathScoreBreakdown {
                length_score: 0.5,
                confidence_score: 0.5,
                relevance_score: 0.5,
            },
        };

        let json = serde_json::to_string(&path).unwrap();
        assert!(json.contains("func_a"));
        assert!(json.contains("score_breakdown"));
    }
}
