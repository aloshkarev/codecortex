//! Impact Graph Builder for Blast Radius Analysis
//!
//! Provides typed blast radius analysis for code changes including:
//! - Direct callers
//! - Transitive callers
//! - Importers
//! - Implementers/Overrides
//!
//! Supports truncation for large graphs and confidence scoring.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// Maximum number of nodes before truncation
const MAX_NODES: usize = 2000;

/// Maximum traversal depth
const MAX_DEPTH: usize = 8;

/// Type of impact relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpactNodeType {
    /// Direct caller of the target symbol
    DirectCaller,
    /// Transitive caller (calls something that calls the target)
    TransitiveCaller,
    /// File/module that imports the target
    Importer,
    /// Class/interface that implements the target
    Implementer,
    /// Method that overrides the target
    Overrider,
    /// File that contains the target
    Container,
}

/// Node in the impact graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactNode {
    /// Unique identifier
    pub id: String,
    /// Symbol name
    pub name: String,
    /// Type of impact relationship
    pub impact_type: ImpactNodeType,
    /// File path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Depth from root (0 for root)
    pub depth: usize,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Provenance of this edge
    pub provenance: Provenance,
}

/// Provenance tracking for edges
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// From static analysis
    Static,
    /// From LSP analysis
    Lsp,
    /// Inferred from naming patterns
    Inferred,
}

/// Edge in the impact graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Edge type (calls, imports, implements, overrides)
    pub edge_type: String,
    /// Confidence score
    pub confidence: f64,
    /// Provenance
    pub provenance: Provenance,
}

/// Summary statistics for the impact graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactSummary {
    /// Number of direct callers
    pub direct_callers: usize,
    /// Number of transitive callers
    pub transitive_callers: usize,
    /// Number of importers
    pub importers: usize,
    /// Number of implementers
    pub implementers: usize,
    /// Number of overriders
    pub overriders: usize,
    /// Total dependents
    pub total_dependents: usize,
    /// Blast radius classification
    pub blast_radius: BlastRadius,
    /// Depth used for analysis
    pub depth_used: usize,
}

impl ImpactSummary {
    fn new() -> Self {
        Self {
            direct_callers: 0,
            transitive_callers: 0,
            importers: 0,
            implementers: 0,
            overriders: 0,
            total_dependents: 0,
            blast_radius: BlastRadius::Low,
            depth_used: 0,
        }
    }
}

/// Blast radius classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlastRadius {
    /// Low impact (< 5 dependents)
    Low,
    /// Medium impact (5-20 dependents)
    Medium,
    /// High impact (> 20 dependents)
    High,
}

impl BlastRadius {
    /// Classify blast radius from total dependents
    pub fn from_count(count: usize) -> Self {
        if count > 20 {
            BlastRadius::High
        } else if count > 5 {
            BlastRadius::Medium
        } else {
            BlastRadius::Low
        }
    }
}

/// Warning when graph is truncated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationWarning {
    /// Original node count before truncation
    pub original_count: usize,
    /// Truncated count
    pub truncated_count: usize,
    /// Reason for truncation
    pub reason: String,
}

/// Complete impact graph result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactGraph {
    /// Root symbol information
    pub root: ImpactRoot,
    /// All impact nodes
    pub nodes: Vec<ImpactNode>,
    /// All impact edges
    pub edges: Vec<ImpactEdge>,
    /// Summary statistics
    pub summary: ImpactSummary,
    /// Truncation warning if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<TruncationWarning>,
}

/// Root symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactRoot {
    /// Symbol name
    pub name: String,
    /// Symbol type (function, class, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_type: Option<String>,
    /// File path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Raw relationship data from graph query
#[derive(Debug, Clone)]
pub struct RawRelation {
    pub from_id: String,
    pub from_name: String,
    pub from_path: Option<String>,
    pub to_id: String,
    pub relation_type: String,
    pub confidence: f64,
    pub provenance: Provenance,
}

/// Impact graph builder
pub struct ImpactGraphBuilder {
    max_nodes: usize,
    max_depth: usize,
    include_importers: bool,
    include_tests: bool,
}

impl ImpactGraphBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            max_nodes: MAX_NODES,
            max_depth: MAX_DEPTH,
            include_importers: true,
            include_tests: false,
        }
    }

    /// Set maximum nodes
    pub fn with_max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = max;
        self
    }

    /// Set maximum depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth.min(MAX_DEPTH);
        self
    }

    /// Set include importers flag
    pub fn with_importers(mut self, include: bool) -> Self {
        self.include_importers = include;
        self
    }

    /// Set include tests flag
    pub fn with_tests(mut self, include: bool) -> Self {
        self.include_tests = include;
        self
    }

    /// Build the impact graph from raw relationship data
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        &self,
        root_symbol: &str,
        root_type: Option<&str>,
        root_path: Option<&str>,
        direct_callers: Vec<RawRelation>,
        all_callers: Vec<RawRelation>,
        importers: Vec<RawRelation>,
        implementers: Vec<RawRelation>,
    ) -> ImpactGraph {
        let mut nodes: Vec<ImpactNode> = Vec::new();
        let mut edges: Vec<ImpactEdge> = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut summary = ImpactSummary::new();
        let mut truncation: Option<TruncationWarning> = None;

        // Process direct callers
        for rel in &direct_callers {
            if !self.include_tests && self.is_test_path(&rel.from_path) {
                continue;
            }

            if seen_ids.contains(&rel.from_id) {
                continue;
            }

            if nodes.len() >= self.max_nodes {
                truncation = Some(TruncationWarning {
                    original_count: direct_callers.len() + all_callers.len(),
                    truncated_count: nodes.len(),
                    reason: "max_nodes_exceeded".to_string(),
                });
                break;
            }

            seen_ids.insert(rel.from_id.clone());

            nodes.push(ImpactNode {
                id: rel.from_id.clone(),
                name: rel.from_name.clone(),
                impact_type: ImpactNodeType::DirectCaller,
                path: rel.from_path.clone(),
                depth: 1,
                confidence: rel.confidence,
                provenance: rel.provenance,
            });

            edges.push(ImpactEdge {
                from: rel.from_id.clone(),
                to: format!("symbol:{}", root_symbol),
                edge_type: rel.relation_type.clone(),
                confidence: rel.confidence,
                provenance: rel.provenance,
            });

            summary.direct_callers += 1;
        }

        // Process transitive callers
        let direct_ids: HashSet<_> = direct_callers.iter().map(|r| &r.from_id).collect();

        for rel in &all_callers {
            if direct_ids.contains(&rel.from_id) {
                continue; // Already counted as direct
            }

            if !self.include_tests && self.is_test_path(&rel.from_path) {
                continue;
            }

            if seen_ids.contains(&rel.from_id) {
                continue;
            }

            if nodes.len() >= self.max_nodes {
                if truncation.is_none() {
                    truncation = Some(TruncationWarning {
                        original_count: all_callers.len(),
                        truncated_count: nodes.len(),
                        reason: "max_nodes_exceeded".to_string(),
                    });
                }
                break;
            }

            seen_ids.insert(rel.from_id.clone());

            // Calculate depth (simplified - use max depth for transitive)
            let depth = self.calculate_depth(&rel.from_id, &edges).unwrap_or(2);

            nodes.push(ImpactNode {
                id: rel.from_id.clone(),
                name: rel.from_name.clone(),
                impact_type: ImpactNodeType::TransitiveCaller,
                path: rel.from_path.clone(),
                depth,
                confidence: rel.confidence * 0.9, // Slightly lower confidence for transitive
                provenance: rel.provenance,
            });

            summary.transitive_callers += 1;
        }

        // Process importers if requested
        if self.include_importers {
            for rel in &importers {
                if !self.include_tests && self.is_test_path(&rel.from_path) {
                    continue;
                }

                if seen_ids.contains(&rel.from_id) {
                    continue;
                }

                if nodes.len() >= self.max_nodes {
                    if truncation.is_none() {
                        truncation = Some(TruncationWarning {
                            original_count: importers.len(),
                            truncated_count: nodes.len(),
                            reason: "max_nodes_exceeded".to_string(),
                        });
                    }
                    break;
                }

                seen_ids.insert(rel.from_id.clone());

                nodes.push(ImpactNode {
                    id: rel.from_id.clone(),
                    name: rel.from_name.clone(),
                    impact_type: ImpactNodeType::Importer,
                    path: rel.from_path.clone(),
                    depth: 1,
                    confidence: rel.confidence,
                    provenance: rel.provenance,
                });

                edges.push(ImpactEdge {
                    from: rel.from_id.clone(),
                    to: format!("symbol:{}", root_symbol),
                    edge_type: "imports".to_string(),
                    confidence: rel.confidence,
                    provenance: rel.provenance,
                });

                summary.importers += 1;
            }
        }

        // Process implementers/overriders
        for rel in &implementers {
            if !self.include_tests && self.is_test_path(&rel.from_path) {
                continue;
            }

            if seen_ids.contains(&rel.from_id) {
                continue;
            }

            if nodes.len() >= self.max_nodes {
                break;
            }

            seen_ids.insert(rel.from_id.clone());

            let impact_type = if rel.relation_type == "implements" {
                ImpactNodeType::Implementer
            } else {
                ImpactNodeType::Overrider
            };

            nodes.push(ImpactNode {
                id: rel.from_id.clone(),
                name: rel.from_name.clone(),
                impact_type,
                path: rel.from_path.clone(),
                depth: 1,
                confidence: rel.confidence,
                provenance: rel.provenance,
            });

            edges.push(ImpactEdge {
                from: rel.from_id.clone(),
                to: format!("symbol:{}", root_symbol),
                edge_type: rel.relation_type.clone(),
                confidence: rel.confidence,
                provenance: rel.provenance,
            });

            if impact_type == ImpactNodeType::Implementer {
                summary.implementers += 1;
            } else {
                summary.overriders += 1;
            }
        }

        // Calculate totals
        summary.total_dependents = summary.direct_callers
            + summary.transitive_callers
            + summary.importers
            + summary.implementers
            + summary.overriders;

        summary.blast_radius = BlastRadius::from_count(summary.total_dependents);
        summary.depth_used = self.max_depth;

        ImpactGraph {
            root: ImpactRoot {
                name: root_symbol.to_string(),
                symbol_type: root_type.map(|s| s.to_string()),
                path: root_path.map(|s| s.to_string()),
            },
            nodes,
            edges,
            summary,
            truncation,
        }
    }

    /// Check if a path is a test path
    fn is_test_path(&self, path: &Option<String>) -> bool {
        match path {
            Some(p) => p.contains("/test") || p.contains("/tests") || p.contains("_test."),
            None => false,
        }
    }

    /// Calculate the depth of a node in the graph
    fn calculate_depth(&self, node_id: &str, edges: &[ImpactEdge]) -> Option<usize> {
        // Simplified BFS to find depth
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((node_id.to_string(), 0));
        visited.insert(node_id.to_string());

        while let Some((current, depth)) = queue.pop_front() {
            for edge in edges {
                if edge.from == current && !visited.contains(&edge.to) {
                    if edge.to.starts_with("symbol:") {
                        return Some(depth + 1);
                    }
                    visited.insert(edge.to.clone());
                    queue.push_back((edge.to.clone(), depth + 1));
                }
            }
        }

        None
    }
}

impl Default for ImpactGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relation(id: &str, name: &str, path: &str) -> RawRelation {
        RawRelation {
            from_id: id.to_string(),
            from_name: name.to_string(),
            from_path: Some(path.to_string()),
            to_id: "target".to_string(),
            relation_type: "calls".to_string(),
            confidence: 0.9,
            provenance: Provenance::Static,
        }
    }

    #[test]
    fn blast_radius_classification() {
        assert_eq!(BlastRadius::from_count(3), BlastRadius::Low);
        assert_eq!(BlastRadius::from_count(10), BlastRadius::Medium);
        assert_eq!(BlastRadius::from_count(25), BlastRadius::High);
    }

    #[test]
    fn impact_graph_builder_basic() {
        let builder = ImpactGraphBuilder::new();

        let direct_callers = vec![
            make_relation("func:a", "func_a", "/src/a.rs"),
            make_relation("func:b", "func_b", "/src/b.rs"),
        ];

        let graph = builder.build(
            "target_func",
            Some("function"),
            Some("/src/target.rs"),
            direct_callers,
            vec![],
            vec![],
            vec![],
        );

        assert_eq!(graph.root.name, "target_func");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.summary.direct_callers, 2);
        assert_eq!(graph.summary.total_dependents, 2);
        assert_eq!(graph.summary.blast_radius, BlastRadius::Low);
    }

    #[test]
    fn impact_graph_builder_with_transitive() {
        let builder = ImpactGraphBuilder::new();

        let direct_callers = vec![make_relation("func:a", "func_a", "/src/a.rs")];

        let all_callers = vec![
            make_relation("func:a", "func_a", "/src/a.rs"), // Duplicate of direct
            make_relation("func:b", "func_b", "/src/b.rs"), // Transitive
            make_relation("func:c", "func_c", "/src/c.rs"), // Transitive
        ];

        let graph = builder.build(
            "target_func",
            None,
            None,
            direct_callers,
            all_callers,
            vec![],
            vec![],
        );

        assert_eq!(graph.summary.direct_callers, 1);
        assert_eq!(graph.summary.transitive_callers, 2);
        assert_eq!(graph.summary.total_dependents, 3);
    }

    #[test]
    fn impact_graph_builder_excludes_tests() {
        let builder = ImpactGraphBuilder::new().with_tests(false);

        let direct_callers = vec![
            make_relation("func:a", "func_a", "/src/a.rs"),
            make_relation("test:b", "test_b", "/src/test/b.rs"),
        ];

        let graph = builder.build("target", None, None, direct_callers, vec![], vec![], vec![]);

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].id, "func:a");
    }

    #[test]
    fn impact_graph_builder_includes_tests() {
        let builder = ImpactGraphBuilder::new().with_tests(true);

        let direct_callers = vec![
            make_relation("func:a", "func_a", "/src/a.rs"),
            make_relation("test:b", "test_b", "/src/test/b.rs"),
        ];

        let graph = builder.build("target", None, None, direct_callers, vec![], vec![], vec![]);

        assert_eq!(graph.nodes.len(), 2);
    }

    #[test]
    fn impact_graph_builder_truncation() {
        let builder = ImpactGraphBuilder::new().with_max_nodes(2);

        let direct_callers = vec![
            make_relation("func:a", "func_a", "/src/a.rs"),
            make_relation("func:b", "func_b", "/src/b.rs"),
            make_relation("func:c", "func_c", "/src/c.rs"),
        ];

        let graph = builder.build("target", None, None, direct_callers, vec![], vec![], vec![]);

        assert_eq!(graph.nodes.len(), 2);
        assert!(graph.truncation.is_some());
        assert_eq!(graph.truncation.unwrap().reason, "max_nodes_exceeded");
    }

    #[test]
    fn impact_graph_builder_with_importers() {
        let builder = ImpactGraphBuilder::new().with_importers(true);

        let importers = vec![RawRelation {
            from_id: "file:main".to_string(),
            from_name: "main.rs".to_string(),
            from_path: Some("/src/main.rs".to_string()),
            to_id: "target".to_string(),
            relation_type: "imports".to_string(),
            confidence: 1.0,
            provenance: Provenance::Static,
        }];

        let graph = builder.build("target", None, None, vec![], vec![], importers, vec![]);

        assert_eq!(graph.summary.importers, 1);
        assert!(
            graph
                .nodes
                .iter()
                .any(|n| n.impact_type == ImpactNodeType::Importer)
        );
    }

    #[test]
    fn impact_node_serialization() {
        let node = ImpactNode {
            id: "func:a".to_string(),
            name: "func_a".to_string(),
            impact_type: ImpactNodeType::DirectCaller,
            path: Some("/src/a.rs".to_string()),
            depth: 1,
            confidence: 0.95,
            provenance: Provenance::Lsp,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("direct_caller"));
        assert!(json.contains("lsp"));
    }
}
