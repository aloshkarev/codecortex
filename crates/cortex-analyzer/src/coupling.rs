//! Coupling and cohesion analysis for measuring code quality.
//!
//! This module provides:
//! - **Coupling Analysis**: Measure dependencies between modules
//! - **Cohesion Metrics**: Measure how well module elements belong together
//! - **Dependency Graph**: Build and analyze dependency relationships

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Coupling type between modules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouplingType {
    /// No direct coupling
    None,
    /// Data coupling - sharing data through parameters
    Data,
    /// Stamp coupling - sharing composite data structures
    Stamp,
    /// Control coupling - one module controls another's logic
    Control,
    /// External coupling - sharing external format/device
    External,
    /// Common coupling - sharing global data
    Common,
    /// Content coupling - one module accesses another's internals
    Content,
}

impl std::fmt::Display for CouplingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CouplingType::None => write!(f, "none"),
            CouplingType::Data => write!(f, "data"),
            CouplingType::Stamp => write!(f, "stamp"),
            CouplingType::Control => write!(f, "control"),
            CouplingType::External => write!(f, "external"),
            CouplingType::Common => write!(f, "common"),
            CouplingType::Content => write!(f, "content"),
        }
    }
}

/// Coupling relationship between two modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingRelation {
    /// Source module
    pub from: String,
    /// Target module
    pub to: String,
    /// Type of coupling
    pub coupling_type: CouplingType,
    /// Strength of coupling (0.0 - 1.0)
    pub strength: f64,
    /// Number of dependencies
    pub dependency_count: usize,
}

/// Coupling analysis result for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingMetrics {
    /// Module name
    pub module: String,
    /// Afferent coupling (incoming dependencies)
    pub ca: usize,
    /// Efferent coupling (outgoing dependencies)
    pub ce: usize,
    /// Instability index (ce / (ca + ce))
    pub instability: f64,
    /// Abstractness (abstract elements / total elements)
    pub abstractness: f64,
    /// Distance from main sequence (|abstractness + instability - 1|)
    pub distance: f64,
    /// Coupling relations
    pub relations: Vec<CouplingRelation>,
}

/// Cohesion type within a module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CohesionType {
    /// Coincidental - no meaningful relationship
    Coincidental,
    /// Logical - logically related but not functionally
    Logical,
    /// Temporal - executed together in time
    Temporal,
    /// Procedural - part of same algorithm
    Procedural,
    /// Communicational - operate on same data
    Communicational,
    /// Sequential - output of one is input of another
    Sequential,
    /// Functional - all elements contribute to single task
    Functional,
}

impl std::fmt::Display for CohesionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CohesionType::Coincidental => write!(f, "coincidental"),
            CohesionType::Logical => write!(f, "logical"),
            CohesionType::Temporal => write!(f, "temporal"),
            CohesionType::Procedural => write!(f, "procedural"),
            CohesionType::Communicational => write!(f, "communicational"),
            CohesionType::Sequential => write!(f, "sequential"),
            CohesionType::Functional => write!(f, "functional"),
        }
    }
}

/// Cohesion metrics for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CohesionMetrics {
    /// Module name
    pub module: String,
    /// Lack of Cohesion of Methods (LCOM) - lower is better
    pub lcom: f64,
    /// Cohesion type classification
    pub cohesion_type: CohesionType,
    /// Number of methods
    pub method_count: usize,
    /// Number of fields
    pub field_count: usize,
    /// Methods that share fields
    pub shared_field_methods: usize,
    /// Cohesion score (0.0 - 1.0, higher is better)
    pub cohesion_score: f64,
}

/// Coupling and cohesion analyzer
#[derive(Debug, Default)]
pub struct CouplingAnalyzer {
    /// Dependency graph: module -> set of dependencies
    dependencies: HashMap<String, HashSet<String>>,
    /// Reverse dependency graph: module -> set of dependents
    dependents: HashMap<String, HashSet<String>>,
    /// Method-to-field access map for cohesion analysis
    method_field_access: HashMap<String, HashSet<String>>,
}

impl CouplingAnalyzer {
    /// Create a new coupling analyzer
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.dependencies
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());

        self.dependents
            .entry(to.to_string())
            .or_default()
            .insert(from.to_string());
    }

    /// Add method-to-field access for cohesion analysis
    pub fn add_field_access(&mut self, method: &str, field: &str) {
        self.method_field_access
            .entry(method.to_string())
            .or_default()
            .insert(field.to_string());
    }

    /// Calculate coupling metrics for a module
    pub fn analyze_coupling(&self, module: &str) -> CouplingMetrics {
        let ca = self.dependents.get(module).map(|s| s.len()).unwrap_or(0);
        let ce = self.dependencies.get(module).map(|s| s.len()).unwrap_or(0);

        let total = ca + ce;
        let instability = if total > 0 {
            ce as f64 / total as f64
        } else {
            0.0
        };

        // Build relations
        let relations = self.build_coupling_relations(module, ce);

        CouplingMetrics {
            module: module.to_string(),
            ca,
            ce,
            instability,
            abstractness: 0.0, // Would need abstract class detection
            distance: (0.0 + instability - 1.0).abs(),
            relations,
        }
    }

    fn build_coupling_relations(&self, module: &str, _ce: usize) -> Vec<CouplingRelation> {
        let mut relations = Vec::new();

        if let Some(deps) = self.dependencies.get(module) {
            for dep in deps {
                relations.push(CouplingRelation {
                    from: module.to_string(),
                    to: dep.clone(),
                    coupling_type: CouplingType::Data, // Simplified
                    strength: 0.5,                     // Simplified
                    dependency_count: 1,
                });
            }
        }

        relations
    }

    /// Calculate cohesion metrics for a module
    pub fn analyze_cohesion(
        &self,
        module: &str,
        methods: &[String],
        fields: &[String],
    ) -> CohesionMetrics {
        let method_count = methods.len();
        let field_count = fields.len();

        // Calculate LCOM (Lack of Cohesion of Methods)
        // LCOM = P - Q where P = pairs of methods that don't share fields
        // and Q = pairs of methods that share fields
        let mut shared_pairs = 0usize;
        let mut non_shared_pairs = 0usize;

        let method_fields: Vec<HashSet<String>> = methods
            .iter()
            .map(|m| {
                self.method_field_access
                    .get(m)
                    .cloned()
                    .unwrap_or_default()
                    .intersection(&fields.iter().cloned().collect())
                    .cloned()
                    .collect()
            })
            .collect();

        for i in 0..methods.len() {
            for j in (i + 1)..methods.len() {
                let common: HashSet<_> = method_fields[i].intersection(&method_fields[j]).collect();

                if common.is_empty() {
                    non_shared_pairs += 1;
                } else {
                    shared_pairs += 1;
                }
            }
        }

        let total_pairs = shared_pairs + non_shared_pairs;
        let lcom = if total_pairs > 0 {
            non_shared_pairs as f64 / total_pairs as f64
        } else {
            0.0
        };

        // Determine cohesion type based on LCOM and patterns
        let cohesion_type = self.determine_cohesion_type(lcom, method_count, field_count);

        // Calculate cohesion score (inverse of LCOM)
        let cohesion_score = 1.0 - lcom;

        CohesionMetrics {
            module: module.to_string(),
            lcom,
            cohesion_type,
            method_count,
            field_count,
            shared_field_methods: shared_pairs,
            cohesion_score,
        }
    }

    fn determine_cohesion_type(
        &self,
        lcom: f64,
        method_count: usize,
        field_count: usize,
    ) -> CohesionType {
        if method_count == 0 || field_count == 0 {
            return CohesionType::Coincidental;
        }

        if lcom < 0.2 {
            CohesionType::Functional
        } else if lcom < 0.4 {
            CohesionType::Sequential
        } else if lcom < 0.6 {
            CohesionType::Communicational
        } else if lcom < 0.75 {
            CohesionType::Procedural
        } else if lcom < 0.9 {
            CohesionType::Logical
        } else {
            CohesionType::Coincidental
        }
    }

    /// Get all modules with high coupling (instability > 0.7 or < 0.3)
    pub fn find_unstable_modules(&self) -> Vec<(String, f64)> {
        let mut unstable = Vec::new();

        let all_modules: HashSet<String> = self
            .dependencies
            .keys()
            .chain(self.dependents.keys())
            .cloned()
            .collect();

        for module in all_modules {
            let metrics = self.analyze_coupling(&module);
            if metrics.instability > 0.7 || (metrics.instability < 0.3 && metrics.ca > 0) {
                unstable.push((module, metrics.instability));
            }
        }

        unstable.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        unstable
    }

    /// Find circular dependencies between modules
    pub fn find_circular_dependencies(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let all_modules: HashSet<String> = self.dependencies.keys().cloned().collect();

        for start in &all_modules {
            let mut visited = HashSet::new();
            let mut path = Vec::new();
            self.find_cycles_from(start, start, &mut visited, &mut path, &mut cycles);
        }

        // Remove duplicates
        let mut unique_cycles: Vec<Vec<String>> = Vec::new();
        for cycle in cycles {
            let mut sorted = cycle.clone();
            sorted.sort();
            if !unique_cycles.iter().any(|c| {
                let mut s = c.clone();
                s.sort();
                s == sorted
            }) {
                unique_cycles.push(cycle);
            }
        }

        unique_cycles
    }

    fn find_cycles_from(
        &self,
        current: &str,
        start: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        if visited.contains(current) {
            if current == start && path.len() > 1 {
                cycles.push(path.clone());
            }
            return;
        }

        visited.insert(current.to_string());
        path.push(current.to_string());

        if let Some(deps) = self.dependencies.get(current) {
            for dep in deps {
                self.find_cycles_from(dep, start, visited, path, cycles);
            }
        }

        path.pop();
        visited.remove(current);
    }

    /// Calculate overall system coupling score
    pub fn system_coupling_score(&self) -> f64 {
        let all_modules: HashSet<String> = self
            .dependencies
            .keys()
            .chain(self.dependents.keys())
            .cloned()
            .collect();

        if all_modules.is_empty() {
            return 0.0;
        }

        let total_instability: f64 = all_modules
            .iter()
            .map(|m| {
                let metrics = self.analyze_coupling(m);
                metrics.instability
            })
            .sum();

        total_instability / all_modules.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupling_analyzer_new() {
        let analyzer = CouplingAnalyzer::new();
        assert!(analyzer.dependencies.is_empty());
        assert!(analyzer.dependents.is_empty());
    }

    #[test]
    fn add_dependency() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_dependency("module_a", "module_b");

        assert!(analyzer.dependencies.contains_key("module_a"));
        assert!(analyzer.dependents.contains_key("module_b"));
    }

    #[test]
    fn analyze_coupling_empty() {
        let analyzer = CouplingAnalyzer::new();
        let metrics = analyzer.analyze_coupling("nonexistent");

        assert_eq!(metrics.ca, 0);
        assert_eq!(metrics.ce, 0);
        assert_eq!(metrics.instability, 0.0);
    }

    #[test]
    fn analyze_coupling_with_deps() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_dependency("a", "b");
        analyzer.add_dependency("a", "c");
        analyzer.add_dependency("d", "a");

        let metrics = analyzer.analyze_coupling("a");

        assert_eq!(metrics.ca, 1); // d depends on a
        assert_eq!(metrics.ce, 2); // a depends on b and c
        assert!(metrics.instability > 0.5);
    }

    #[test]
    fn cohesion_type_ordering() {
        assert!(matches!(CohesionType::Functional, CohesionType::Functional));
        assert_ne!(CohesionType::Functional, CohesionType::Coincidental);
    }

    #[test]
    fn analyze_cohesion_empty() {
        let analyzer = CouplingAnalyzer::new();
        let metrics = analyzer.analyze_cohesion("MyClass", &[], &[]);

        assert_eq!(metrics.method_count, 0);
        assert_eq!(metrics.field_count, 0);
        assert_eq!(metrics.cohesion_type, CohesionType::Coincidental);
    }

    #[test]
    fn analyze_cohesion_with_methods() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_field_access("get_name", "name");
        analyzer.add_field_access("set_name", "name");
        analyzer.add_field_access("get_age", "age");

        let methods = vec![
            "get_name".to_string(),
            "set_name".to_string(),
            "get_age".to_string(),
        ];
        let fields = vec!["name".to_string(), "age".to_string()];

        let metrics = analyzer.analyze_cohesion("Person", &methods, &fields);

        assert_eq!(metrics.method_count, 3);
        assert_eq!(metrics.field_count, 2);
    }

    #[test]
    fn find_unstable_modules() {
        let mut analyzer = CouplingAnalyzer::new();
        // Create a very unstable module (high ce, low ca)
        analyzer.add_dependency("unstable", "dep1");
        analyzer.add_dependency("unstable", "dep2");
        analyzer.add_dependency("unstable", "dep3");

        let unstable = analyzer.find_unstable_modules();
        assert!(!unstable.is_empty());
    }

    #[test]
    fn find_circular_dependencies_none() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_dependency("a", "b");
        analyzer.add_dependency("b", "c");

        let cycles = analyzer.find_circular_dependencies();
        assert!(cycles.is_empty());
    }

    #[test]
    fn find_circular_dependencies_with_cycle() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_dependency("a", "b");
        analyzer.add_dependency("b", "c");
        analyzer.add_dependency("c", "a");

        let cycles = analyzer.find_circular_dependencies();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn system_coupling_score() {
        let mut analyzer = CouplingAnalyzer::new();
        analyzer.add_dependency("a", "b");
        analyzer.add_dependency("c", "d");

        let score = analyzer.system_coupling_score();
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn coupling_type_display() {
        assert_eq!(CouplingType::None.to_string(), "none");
        assert_eq!(CouplingType::Data.to_string(), "data");
        assert_eq!(CouplingType::Content.to_string(), "content");
    }

    #[test]
    fn cohesion_type_display() {
        assert_eq!(CohesionType::Functional.to_string(), "functional");
        assert_eq!(CohesionType::Coincidental.to_string(), "coincidental");
    }

    #[test]
    fn coupling_metrics_serialization() {
        let metrics = CouplingMetrics {
            module: "test".to_string(),
            ca: 5,
            ce: 3,
            instability: 0.375,
            abstractness: 0.5,
            distance: 0.125,
            relations: vec![],
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("instability"));
    }

    #[test]
    fn cohesion_metrics_serialization() {
        let metrics = CohesionMetrics {
            module: "MyClass".to_string(),
            lcom: 0.25,
            cohesion_type: CohesionType::Functional,
            method_count: 5,
            field_count: 3,
            shared_field_methods: 8,
            cohesion_score: 0.75,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("MyClass"));
        assert!(json.contains("functional"));
    }
}
