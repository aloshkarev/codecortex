//! Graph Centrality Scoring for Code Nodes
//!
//! Implements centrality algorithms for ranking code nodes by importance
//! in the dependency/call graph. Used as a factor in context capsule scoring.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet, VecDeque};

/// Node identifier type
pub type NodeId = String;

/// Edge representation for the graph
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub weight: f64,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            weight: 1.0,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// In-memory graph for centrality computation
#[derive(Debug, Default)]
pub struct CentralityGraph {
    /// All nodes in the graph
    nodes: HashSet<NodeId>,
    /// Outgoing edges: node -> [(target, weight)]
    outgoing: HashMap<NodeId, Vec<(NodeId, f64)>>,
    /// Incoming edges: node -> [(source, weight)]
    incoming: HashMap<NodeId, Vec<(NodeId, f64)>>,
}

impl CentralityGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.nodes.insert(id);
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: Edge) {
        self.nodes.insert(edge.from.clone());
        self.nodes.insert(edge.to.clone());

        self.outgoing
            .entry(edge.from.clone())
            .or_default()
            .push((edge.to.clone(), edge.weight));

        self.incoming
            .entry(edge.to)
            .or_default()
            .push((edge.from, edge.weight));
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashSet<NodeId> {
        &self.nodes
    }

    /// Get outgoing edges for a node
    pub fn outgoing(&self, node: &str) -> Option<&[(NodeId, f64)]> {
        self.outgoing.get(node).map(|v| v.as_slice())
    }

    /// Get incoming edges for a node
    pub fn incoming(&self, node: &str) -> Option<&[(NodeId, f64)]> {
        self.incoming.get(node).map(|v| v.as_slice())
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.outgoing.values().map(|v| v.len()).sum()
    }
}

/// Centrality scorer using various algorithms
#[derive(Debug)]
pub struct CentralityScorer {
    graph: CentralityGraph,
    /// Cached centrality scores
    scores: HashMap<NodeId, f64>,
}

impl CentralityScorer {
    /// Create a new scorer with an empty graph
    pub fn new() -> Self {
        Self {
            graph: CentralityGraph::new(),
            scores: HashMap::new(),
        }
    }

    /// Create a scorer from an existing graph
    pub fn from_graph(graph: CentralityGraph) -> Self {
        Self {
            graph,
            scores: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, id: impl Into<String>) {
        self.graph.add_node(id);
        self.scores.clear();
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.graph.add_edge(Edge::new(from, to));
        self.scores.clear();
    }

    /// Get the underlying graph
    pub fn graph(&self) -> &CentralityGraph {
        &self.graph
    }

    /// Compute PageRank scores
    pub fn compute_pagerank(&mut self, damping: f64, iterations: usize) {
        let n = self.graph.node_count();
        if n == 0 {
            return;
        }

        let initial_score = 1.0 / n as f64;
        let mut current: HashMap<NodeId, f64> = self
            .graph
            .nodes()
            .iter()
            .map(|id| (id.clone(), initial_score))
            .collect();

        let mut next = HashMap::new();

        for _ in 0..iterations {
            // Reset next scores
            for id in self.graph.nodes() {
                next.insert(id.clone(), (1.0 - damping) / n as f64);
            }

            // Distribute PageRank
            for node in self.graph.nodes() {
                let outgoing = self.graph.outgoing(node);
                let out_degree = outgoing.map(|e| e.len()).unwrap_or(0);

                if out_degree > 0 {
                    let contribution =
                        current.get(node).copied().unwrap_or(0.0) * damping / out_degree as f64;

                    if let Some(edges) = outgoing {
                        for (target, _weight) in edges {
                            *next.get_mut(target).unwrap() += contribution;
                        }
                    }
                } else {
                    // Dangling node: distribute evenly
                    let contribution =
                        current.get(node).copied().unwrap_or(0.0) * damping / n as f64;
                    for id in self.graph.nodes() {
                        *next.get_mut(id).unwrap() += contribution;
                    }
                }
            }

            std::mem::swap(&mut current, &mut next);
        }

        self.scores = current;
    }

    /// Compute simplified PageRank (proxy) with default parameters
    pub fn compute(&mut self) {
        self.compute_pagerank(0.85, 20);
    }

    /// Get the centrality score for a node
    pub fn score(&self, node: &str) -> f64 {
        self.scores.get(node).copied().unwrap_or(0.0)
    }

    /// Get all scores
    pub fn all_scores(&self) -> &HashMap<NodeId, f64> {
        &self.scores
    }

    /// Get nodes sorted by centrality (descending)
    pub fn top_nodes(&self, limit: usize) -> Vec<(NodeId, f64)> {
        let mut sorted: Vec<_> = self.scores.iter().map(|(k, v)| (k.clone(), *v)).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(limit);
        sorted
    }
}

impl Default for CentralityScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Degree centrality (simple, fast approximation)
pub fn degree_centrality(graph: &CentralityGraph) -> HashMap<NodeId, f64> {
    let n = graph.node_count();
    if n == 0 {
        return HashMap::new();
    }

    let max_possible = (n - 1) as f64;

    graph
        .nodes()
        .iter()
        .map(|node| {
            let in_degree = graph.incoming(node).map(|e| e.len()).unwrap_or(0);
            let out_degree = graph.outgoing(node).map(|e| e.len()).unwrap_or(0);
            let total = (in_degree + out_degree) as f64;
            (node.clone(), total / max_possible)
        })
        .collect()
}

/// Betweenness centrality approximation (using sampling)
pub fn betweenness_centrality_approx(
    graph: &CentralityGraph,
    samples: usize,
) -> HashMap<NodeId, f64> {
    let mut betweenness: HashMap<NodeId, f64> =
        graph.nodes().iter().map(|id| (id.clone(), 0.0)).collect();

    let nodes: Vec<_> = graph.nodes().iter().cloned().collect();
    if nodes.len() < 2 {
        return betweenness;
    }

    let mut rng = SimpleRng::new(42);
    let n = nodes.len();

    for _ in 0..samples {
        let i = (rng.next() as usize) % n;
        let j = (rng.next() as usize) % n;

        if i == j {
            continue;
        }

        let source = &nodes[i];
        let target = &nodes[j];

        // BFS to find shortest paths
        if let Some(path) = bfs_shortest_path(graph, source, target) {
            for node in &path[1..path.len() - 1] {
                *betweenness.get_mut(node).unwrap() += 1.0;
            }
        }
    }

    // Normalize
    let scale = samples as f64;
    for value in betweenness.values_mut() {
        *value /= scale;
    }

    betweenness
}

/// BFS shortest path
fn bfs_shortest_path(graph: &CentralityGraph, source: &str, target: &str) -> Option<Vec<String>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent: HashMap<String, String> = HashMap::new();

    visited.insert(source.to_string());
    queue.push_back(source.to_string());

    while let Some(current) = queue.pop_front() {
        if current == target {
            // Reconstruct path
            let mut path = vec![target.to_string()];
            let mut node = target.to_string();
            while let Some(p) = parent.get(&node) {
                path.push(p.clone());
                node = p.clone();
            }
            path.reverse();
            return Some(path);
        }

        if let Some(edges) = graph.outgoing(&current) {
            for (next, _) in edges {
                if !visited.contains(next) {
                    visited.insert(next.clone());
                    parent.insert(next.clone(), current.clone());
                    queue.push_back(next.clone());
                }
            }
        }
    }

    None
}

/// Simple RNG for deterministic sampling
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

/// Combined centrality score for a node
#[derive(Debug, Clone, Default)]
pub struct CombinedCentrality {
    /// PageRank score
    pub pagerank: f64,
    /// Degree centrality
    pub degree: f64,
    /// Betweenness centrality
    pub betweenness: f64,
    /// Combined weighted score
    pub combined: f64,
}

impl CombinedCentrality {
    /// Compute combined score from individual scores
    pub fn compute(pagerank: f64, degree: f64, betweenness: f64) -> Self {
        // Weighted combination
        let combined = pagerank * 0.5 + degree * 0.3 + betweenness * 0.2;
        Self {
            pagerank,
            degree,
            betweenness,
            combined,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_basic_operations() {
        let mut graph = CentralityGraph::new();
        graph.add_node("a");
        graph.add_node("b");
        graph.add_edge(Edge::new("a", "b"));

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn graph_edge_access() {
        let mut graph = CentralityGraph::new();
        graph.add_edge(Edge::new("a", "b"));
        graph.add_edge(Edge::new("a", "c"));

        let outgoing = graph.outgoing("a").unwrap();
        assert_eq!(outgoing.len(), 2);

        let incoming = graph.incoming("b").unwrap();
        assert_eq!(incoming.len(), 1);
    }

    #[test]
    fn pagerank_simple_graph() {
        let mut scorer = CentralityScorer::new();

        // Create a simple graph: a -> b -> c
        scorer.add_edge("a", "b");
        scorer.add_edge("b", "c");

        scorer.compute();

        // All nodes should have positive scores
        assert!(scorer.score("a") > 0.0);
        assert!(scorer.score("b") > 0.0);
        assert!(scorer.score("c") > 0.0);
    }

    #[test]
    fn pagerank_hub_nodes() {
        let mut scorer = CentralityScorer::new();

        // Create a star graph: center <- a, center <- b, center <- c
        scorer.add_edge("a", "center");
        scorer.add_edge("b", "center");
        scorer.add_edge("c", "center");

        scorer.compute();

        // Center should have higher PageRank due to incoming links
        let center_score = scorer.score("center");
        let a_score = scorer.score("a");

        assert!(center_score > a_score);
    }

    #[test]
    fn top_nodes() {
        let mut scorer = CentralityScorer::new();

        scorer.add_edge("a", "hub");
        scorer.add_edge("b", "hub");
        scorer.add_edge("c", "hub");
        scorer.add_edge("hub", "d");

        scorer.compute();

        let top = scorer.top_nodes(2);
        assert_eq!(top.len(), 2);

        // Hub should be in top nodes
        let top_ids: Vec<_> = top.iter().map(|(id, _)| id.as_str()).collect();
        assert!(top_ids.contains(&"hub"));
    }

    #[test]
    fn test_degree_centrality() {
        let mut graph = CentralityGraph::new();
        // a -> b, a -> c, d -> b, e -> b
        graph.add_edge(Edge::new("a", "b"));
        graph.add_edge(Edge::new("a", "c"));
        graph.add_edge(Edge::new("d", "b"));
        graph.add_edge(Edge::new("e", "b"));

        let dc = super::degree_centrality(&graph);

        // Node 'a' has 2 outgoing + 0 incoming = 2 connections
        // Node 'b' has 0 outgoing + 3 incoming = 3 connections
        assert!(dc[&"b".to_string()] > dc[&"a".to_string()]);
    }

    #[test]
    fn bfs_shortest_path_finds_path() {
        let mut graph = CentralityGraph::new();
        graph.add_edge(Edge::new("a", "b"));
        graph.add_edge(Edge::new("b", "c"));
        graph.add_edge(Edge::new("c", "d"));

        let path = bfs_shortest_path(&graph, "a", "d").unwrap();
        assert_eq!(path, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn bfs_no_path_returns_none() {
        let mut graph = CentralityGraph::new();
        graph.add_edge(Edge::new("a", "b"));
        graph.add_edge(Edge::new("c", "d"));

        let path = bfs_shortest_path(&graph, "a", "d");
        assert!(path.is_none());
    }

    #[test]
    fn combined_centrality() {
        let combined = CombinedCentrality::compute(0.5, 0.4, 0.3);

        assert!((combined.combined - 0.43).abs() < 0.01); // 0.5*0.5 + 0.4*0.3 + 0.3*0.2
    }
}
