//! Context Capsule Builder for Hybrid Retrieval
//!
//! Implements a sophisticated retrieval system combining:
//! - Lexical search (CONTAINS queries)
//! - TF-IDF scoring
//! - Graph centrality
//! - Score fusion with intent-based weights
//! - Threshold relaxation
//! - Token budgeting
//! - Module-level relevance boosting

#![allow(dead_code)]

use crate::cache::CacheHierarchy;
use crate::centrality::CentralityScorer;
use crate::tfidf::{Document, TfIdfScorer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for the context capsule builder
#[derive(Debug, Clone)]
pub struct CapsuleConfig {
    /// Maximum number of items to return
    pub max_items: usize,
    /// Maximum token budget
    pub max_tokens: usize,
    /// Initial relevance threshold
    pub initial_threshold: f64,
    /// Minimum threshold after relaxation
    pub min_threshold: f64,
    /// Threshold relaxation step
    pub relaxation_step: f64,
    /// Whether to include test files
    pub include_tests: bool,
    /// Intent-based weight adjustments
    pub intent_weights: IntentWeights,
    /// Boost factor for items in the same module/path
    pub module_boost: f64,
    /// Minimum similarity for fuzzy matching
    pub fuzzy_threshold: f64,
    /// Field weights for different source fields
    pub field_weights: FieldWeights,
    /// Recency boost configuration
    pub recency_config: RecencyConfig,
    /// Test proximity boost configuration
    pub test_proximity_config: TestProximityConfig,
}

impl Default for CapsuleConfig {
    fn default() -> Self {
        Self {
            max_items: 40,
            max_tokens: 6000,
            initial_threshold: 0.2, // Lower initial threshold
            min_threshold: 0.05,    // Lower minimum threshold
            relaxation_step: 0.05,
            include_tests: false,
            intent_weights: IntentWeights::default(),
            module_boost: 0.35, // Higher module boost
            fuzzy_threshold: 0.5,
            field_weights: FieldWeights::default(),
            recency_config: RecencyConfig::default(),
            test_proximity_config: TestProximityConfig::default(),
        }
    }
}

/// Intent-based weight adjustments for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentWeights {
    /// Weight for lexical/FTS score
    pub fts_weight: f64,
    /// Weight for TF-IDF score
    pub tfidf_weight: f64,
    /// Weight for centrality score
    pub centrality_weight: f64,
    /// Weight for module/path proximity
    pub proximity_weight: f64,
}

impl Default for IntentWeights {
    fn default() -> Self {
        Self {
            fts_weight: 0.4,
            tfidf_weight: 0.3,
            centrality_weight: 0.1,
            proximity_weight: 0.2,
        }
    }
}

impl IntentWeights {
    /// Weights optimized for debugging tasks
    pub fn debug() -> Self {
        Self {
            fts_weight: 0.5,
            tfidf_weight: 0.25,
            centrality_weight: 0.1,
            proximity_weight: 0.15,
        }
    }

    /// Weights optimized for refactoring tasks
    pub fn refactor() -> Self {
        Self {
            fts_weight: 0.25,
            tfidf_weight: 0.35,
            centrality_weight: 0.25,
            proximity_weight: 0.15,
        }
    }

    /// Weights optimized for exploration tasks
    pub fn explore() -> Self {
        Self {
            fts_weight: 0.35,
            tfidf_weight: 0.35,
            centrality_weight: 0.15,
            proximity_weight: 0.15,
        }
    }

    /// Weights optimized for testing tasks
    pub fn test() -> Self {
        Self {
            fts_weight: 0.45,
            tfidf_weight: 0.25,
            centrality_weight: 0.15,
            proximity_weight: 0.15,
        }
    }

    /// Get weights by intent name
    pub fn for_intent(intent: &str) -> Self {
        match intent {
            "debug" => Self::debug(),
            "refactor" => Self::refactor(),
            "test" => Self::test(),
            "explore" => Self::explore(),
            _ => Self::explore(),
        }
    }
}

/// Field weights for boosting different source fields during retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldWeights {
    /// Weight for symbol name matches (highest priority)
    pub name_weight: f64,
    /// Weight for docstring/comment matches
    pub docstring_weight: f64,
    /// Weight for source code content matches
    pub source_weight: f64,
    /// Weight for path/module matches
    pub path_weight: f64,
    /// Weight for parameter name matches
    pub parameter_weight: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            name_weight: 1.0,
            docstring_weight: 0.8,
            source_weight: 0.6,
            path_weight: 0.7,
            parameter_weight: 0.5,
        }
    }
}

impl FieldWeights {
    /// Weights optimized for finding API signatures
    pub fn api_signature() -> Self {
        Self {
            name_weight: 1.0,
            docstring_weight: 0.9,
            source_weight: 0.4,
            path_weight: 0.6,
            parameter_weight: 0.8,
        }
    }

    /// Weights optimized for finding implementation details
    pub fn implementation() -> Self {
        Self {
            name_weight: 0.8,
            docstring_weight: 0.5,
            source_weight: 1.0,
            path_weight: 0.5,
            parameter_weight: 0.6,
        }
    }
}

/// Recency boost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecencyConfig {
    /// Whether to apply recency boost
    pub enabled: bool,
    /// Maximum boost for very recent files (1.0 = no change, >1.0 = boost)
    pub max_boost: f64,
    /// Half-life in days (files older than this get half the max boost)
    pub half_life_days: f64,
    /// Minimum boost (for very old files)
    pub min_boost: f64,
}

impl Default for RecencyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_boost: 1.2,
            half_life_days: 30.0,
            min_boost: 0.9,
        }
    }
}

impl RecencyConfig {
    /// Calculate recency boost based on file age in days
    pub fn compute_boost(&self, age_days: f64) -> f64 {
        if !self.enabled {
            return 1.0;
        }
        // Exponential decay: boost = min_boost + (max_boost - min_boost) * 0.5^(age/half_life)
        let decay = 0.5_f64.powf(age_days / self.half_life_days);
        self.min_boost + (self.max_boost - self.min_boost) * decay
    }
}

/// Test proximity boost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProximityConfig {
    /// Whether to apply test proximity boost
    pub enabled: bool,
    /// Boost for files containing tests
    pub test_file_boost: f64,
    /// Boost for symbols near test functions
    pub near_test_boost: f64,
    /// Maximum distance (in lines) to consider "near"
    pub near_test_max_lines: u32,
}

impl Default for TestProximityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            test_file_boost: 1.1,
            near_test_boost: 1.05,
            near_test_max_lines: 50,
        }
    }
}

/// A single item in the context capsule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleItem {
    /// Unique identifier
    pub id: String,
    /// Kind of code element (Function, Class, etc.)
    pub kind: String,
    /// File path
    pub path: String,
    /// Name of the symbol
    pub name: String,
    /// Code snippet (truncated)
    pub snippet: String,
    /// Combined relevance score
    pub score: f64,
    /// Score breakdown for explainability
    pub why: ScoreBreakdown,
    /// Line number in source file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u64>,
}

/// Breakdown of how the score was computed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Lexical/FTS score component
    pub fts: f64,
    /// TF-IDF score component
    pub tfidf: f64,
    /// Graph centrality component
    pub centrality: f64,
    /// Module/path proximity component
    pub proximity: f64,
}

/// Result of context capsule building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCapsuleResult {
    /// Detected intent from query
    pub intent_detected: String,
    /// Items in the capsule
    pub capsule_items: Vec<CapsuleItem>,
    /// Estimated token count
    pub token_estimate: usize,
    /// Threshold used for filtering
    pub threshold_used: f64,
    /// Whether fallback relaxation was applied
    pub fallback_relaxed: bool,
    /// Any warnings generated
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Raw search result from graph query
#[derive(Debug, Clone)]
pub struct GraphSearchResult {
    pub id: String,
    pub kind: String,
    pub path: String,
    pub name: String,
    pub source: Option<String>,
    pub line_number: Option<u64>,
}

/// Context capsule builder with hybrid retrieval
pub struct ContextCapsuleBuilder {
    config: CapsuleConfig,
    cache: Option<CacheHierarchy>,
    tfidf_scorer: TfIdfScorer,
    centrality_scorer: CentralityScorer,
    /// Query context for proximity scoring
    query_context: QueryContext,
}

/// Context extracted from the query for proximity scoring
#[derive(Debug, Clone, Default)]
struct QueryContext {
    /// Extracted terms from the query
    terms: Vec<String>,
    /// N-grams from the query
    ngrams: HashSet<String>,
    /// Potential module/package names
    module_hints: HashSet<String>,
}

impl QueryContext {
    fn from_query(query: &str) -> Self {
        let terms: Vec<String> = crate::tfidf::tokenize(query)
            .into_iter()
            .filter(|t| t.len() > 2)
            .collect();

        // Generate n-grams (2-3 chars)
        let mut ngrams = HashSet::new();
        for term in &terms {
            for window in 2..=3 {
                for i in 0..term.len().saturating_sub(window - 1) {
                    ngrams.insert(term[i..i + window].to_lowercase());
                }
            }
        }

        // Extract potential module hints from terms
        let module_hints: HashSet<String> = terms
            .iter()
            .filter(|t| t.len() >= 3)
            .map(|t| t.to_lowercase())
            .collect();

        Self {
            terms,
            ngrams,
            module_hints,
        }
    }
}

impl ContextCapsuleBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: CapsuleConfig::default(),
            cache: None,
            tfidf_scorer: TfIdfScorer::new(),
            centrality_scorer: CentralityScorer::new(),
            query_context: QueryContext::default(),
        }
    }

    /// Create a builder with custom configuration
    pub fn with_config(config: CapsuleConfig) -> Self {
        Self {
            config,
            cache: None,
            tfidf_scorer: TfIdfScorer::new(),
            centrality_scorer: CentralityScorer::new(),
            query_context: QueryContext::default(),
        }
    }

    /// Set the cache hierarchy
    pub fn with_cache(mut self, cache: CacheHierarchy) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set intent weights
    pub fn with_intent(mut self, intent: &str) -> Self {
        self.config.intent_weights = IntentWeights::for_intent(intent);
        self
    }

    /// Set maximum items
    pub fn with_max_items(mut self, max_items: usize) -> Self {
        self.config.max_items = max_items;
        self
    }

    /// Set maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.config.max_tokens = max_tokens;
        self
    }

    /// Set include tests flag
    pub fn with_include_tests(mut self, include_tests: bool) -> Self {
        self.config.include_tests = include_tests;
        self
    }

    /// Detect intent from a query string
    pub fn detect_intent(query: &str) -> &'static str {
        let q = query.to_lowercase();
        if q.contains("debug") || q.contains("error") || q.contains("fix") || q.contains("bug") {
            "debug"
        } else if q.contains("refactor") || q.contains("clean") || q.contains("improve") {
            "refactor"
        } else if q.contains("test") || q.contains("spec") || q.contains("verify") {
            "test"
        } else {
            "explore"
        }
    }

    /// Build a context capsule from search results
    pub fn build(
        &mut self,
        query: &str,
        results: Vec<GraphSearchResult>,
        intent: Option<&str>,
        path_filters: &[String],
    ) -> ContextCapsuleResult {
        let detected_intent = intent
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::detect_intent(query).to_string());

        self.config.intent_weights = IntentWeights::for_intent(&detected_intent);
        self.query_context = QueryContext::from_query(query);

        // Build TF-IDF corpus from results
        let documents: Vec<Document> = results
            .iter()
            .map(|r| {
                Document::new(
                    &r.id,
                    &format!(
                        "{} {} {}",
                        r.name,
                        r.path,
                        r.source.as_deref().unwrap_or("")
                    ),
                )
            })
            .collect();

        self.tfidf_scorer = TfIdfScorer::from_documents(&documents);

        // Build centrality graph (simplified - using co-occurrence)
        self.build_centrality_from_results(&results);

        // First pass: find directly matching items and extract their paths
        let best_paths = self.extract_relevant_paths(query, &results);

        // Score and filter results
        let mut warnings = Vec::new();
        let mut threshold = self.config.initial_threshold;
        let mut scored_items =
            self.score_results(query, &results, &best_paths, path_filters, threshold);

        // Try with initial threshold, relax if needed
        while scored_items.is_empty() && threshold > self.config.min_threshold {
            threshold -= self.config.relaxation_step;
            threshold = threshold.max(self.config.min_threshold);
            warnings.push(format!("threshold_relaxed_to_{}", threshold));
            scored_items =
                self.score_results(query, &results, &best_paths, path_filters, threshold);
        }

        if scored_items.is_empty() {
            warnings.push("no_results_found".to_string());
        }

        // Sort by score and apply token budget
        scored_items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply token budgeting
        let (budgeted_items, token_estimate, truncated) = self.apply_token_budget(scored_items);

        if truncated {
            warnings.push("token_budget_exceeded".to_string());
        }

        // Truncate to max items
        let final_items: Vec<CapsuleItem> = budgeted_items
            .into_iter()
            .take(self.config.max_items)
            .collect();

        ContextCapsuleResult {
            intent_detected: detected_intent,
            capsule_items: final_items,
            token_estimate,
            threshold_used: threshold,
            fallback_relaxed: threshold < self.config.initial_threshold,
            warnings,
        }
    }

    /// Extract paths that are most relevant to the query
    fn extract_relevant_paths(&self, query: &str, results: &[GraphSearchResult]) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut path_scores: HashMap<String, f64> = HashMap::new();

        for result in results {
            let path_lower = result.path.to_lowercase();
            let name_lower = result.name.to_lowercase();

            // Check if query matches name, source, or path
            let name_match = name_lower.contains(&query_lower);
            let path_match = path_lower.contains(&query_lower);
            let source_match = result
                .source
                .as_ref()
                .map(|s| s.to_lowercase().contains(&query_lower))
                .unwrap_or(false);

            // Also check if any query term (or substring) matches the path
            let term_match = self
                .query_context
                .module_hints
                .iter()
                .any(|h| path_lower.contains(h));

            let path_score = if name_match {
                1.0 // Direct name match is strongest signal
            } else if path_match {
                0.9 // Path match
            } else if term_match {
                0.7 // Query term in path
            } else if source_match {
                0.5 // Source match
            } else {
                0.0
            };

            if path_score > 0.0 {
                // Score the full path and the directory
                if let Some(score) = path_scores.get_mut(&result.path) {
                    *score += path_score;
                } else {
                    path_scores.insert(result.path.clone(), path_score);
                }

                if let Some(dir) = std::path::Path::new(&result.path).parent() {
                    let dir_str = dir.to_string_lossy().to_string();
                    if let Some(score) = path_scores.get_mut(&dir_str) {
                        *score += path_score * 0.8;
                    } else {
                        path_scores.insert(dir_str, path_score * 0.8);
                    }
                }
            }
        }

        // Sort by score and return top paths
        let mut scored: Vec<_> = path_scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(10).map(|(p, _)| p).collect()
    }

    /// Build centrality graph from results (simplified approach)
    fn build_centrality_from_results(&mut self, results: &[GraphSearchResult]) {
        self.centrality_scorer = CentralityScorer::new();

        // Group results by path to create file-based connections
        let mut by_path: HashMap<String, Vec<&GraphSearchResult>> = HashMap::new();
        for result in results {
            by_path.entry(result.path.clone()).or_default().push(result);
        }

        // Create edges between items in the same file
        for items in by_path.values() {
            for i in 0..items.len() {
                for j in (i + 1)..items.len() {
                    self.centrality_scorer.add_edge(&items[i].id, &items[j].id);
                    self.centrality_scorer.add_edge(&items[j].id, &items[i].id);
                }
            }
        }

        self.centrality_scorer.compute();
    }

    /// Score results with hybrid scoring
    fn score_results(
        &self,
        query: &str,
        results: &[GraphSearchResult],
        best_paths: &[String],
        path_filters: &[String],
        threshold: f64,
    ) -> Vec<CapsuleItem> {
        let query_lower = query.to_lowercase();

        results
            .iter()
            .filter(|r| {
                // Filter out tests if not included
                if !self.config.include_tests && r.path.contains("/test") {
                    return false;
                }

                // Apply path filters
                if !path_filters.is_empty() && !path_filters.iter().any(|f| r.path.contains(f)) {
                    return false;
                }

                true
            })
            .filter_map(|r| {
                let item = self.score_item(&query_lower, r, best_paths);
                if item.score >= threshold {
                    Some(item)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Score a single item
    fn score_item(
        &self,
        query: &str,
        result: &GraphSearchResult,
        best_paths: &[String],
    ) -> CapsuleItem {
        let name = &result.name;
        let source = result.source.as_deref().unwrap_or("");

        // Lexical score with fuzzy matching
        let fts = self.compute_lexical_score(query, name, source, &result.path);

        // TF-IDF score
        let doc = Document::new(&result.id, &format!("{} {} {}", name, result.path, source));
        let tfidf = self
            .tfidf_scorer
            .score(&crate::tfidf::tokenize(query), &doc);

        // Normalize TF-IDF to 0-1 range
        let tfidf_normalized = (tfidf / 2.5).min(1.0);

        // Centrality score
        let centrality = self.centrality_scorer.score(&result.id);

        // Proximity score (based on path relevance)
        let proximity = self.compute_proximity_score(&result.path, best_paths);

        // Combined score with intent weights
        let weights = &self.config.intent_weights;
        let combined = (fts * weights.fts_weight)
            + (tfidf_normalized * weights.tfidf_weight)
            + (centrality * weights.centrality_weight)
            + (proximity * weights.proximity_weight);

        // Create snippet
        let snippet: String = source.chars().take(320).collect();

        CapsuleItem {
            id: result.id.clone(),
            kind: result.kind.clone(),
            path: result.path.clone(),
            name: result.name.clone(),
            snippet,
            score: combined,
            why: ScoreBreakdown {
                fts,
                tfidf: tfidf_normalized,
                centrality,
                proximity,
            },
            line_number: result.line_number,
        }
    }

    /// Compute lexical/FTS score with fuzzy matching
    fn compute_lexical_score(&self, query: &str, name: &str, source: &str, path: &str) -> f64 {
        let name_lower = name.to_lowercase();
        let source_lower = source.to_lowercase();
        let path_lower = path.to_lowercase();

        // Exact match in name (highest score)
        let title_hit: f64 = if name_lower.contains(query) { 1.0 } else { 0.0 };

        // Exact match in source
        let body_hit: f64 = if source_lower.contains(query) {
            0.6
        } else {
            0.0
        };

        // Path match (module relevance)
        let path_hit: f64 = if path_lower.contains(query) { 0.7 } else { 0.0 };

        // Exact name match bonus
        let exact_bonus: f64 = if name_lower == query { 0.3 } else { 0.0 };

        // Fuzzy matching using n-grams
        let fuzzy_score = self.compute_fuzzy_score(query, &name_lower);

        // Prefix/suffix matching
        let prefix_score = if name_lower.starts_with(query) {
            0.4
        } else if query.starts_with(&name_lower) {
            0.3
        } else {
            0.0
        };

        // Combine scores
        let base_score = title_hit * 0.35 + body_hit * 0.2 + path_hit * 0.2 + exact_bonus;
        let fuzzy_boost = fuzzy_score * 0.15;
        let prefix_boost = prefix_score * 0.1;

        (base_score + fuzzy_boost + prefix_boost).min(1.0_f64)
    }

    /// Compute fuzzy match score using n-gram overlap
    fn compute_fuzzy_score(&self, query: &str, name: &str) -> f64 {
        if query == name {
            return 1.0;
        }

        // Generate n-grams for the name
        let name_chars: Vec<char> = name.chars().collect();
        let mut name_ngrams = HashSet::new();

        for window in 2..=3 {
            for w in name_chars.windows(window) {
                let ngram: String = w.iter().collect();
                name_ngrams.insert(ngram);
            }
        }

        // Count overlapping n-grams
        let overlap = self.query_context.ngrams.intersection(&name_ngrams).count();
        let total = self.query_context.ngrams.len().max(1);

        (overlap as f64 / total as f64).min(1.0)
    }

    /// Compute proximity score based on path relevance
    fn compute_proximity_score(&self, path: &str, best_paths: &[String]) -> f64 {
        if best_paths.is_empty() {
            return 0.0;
        }

        let path_lower = path.to_lowercase();

        // Check if this path is in or near a best path
        for best in best_paths {
            let best_lower = best.to_lowercase();

            // Exact path match
            if path_lower == best_lower {
                return self.config.module_boost;
            }

            // Same file
            if path_lower.starts_with(&best_lower) || best_lower.starts_with(&path_lower) {
                return self.config.module_boost;
            }

            // Same directory
            if let (Some(path_dir), Some(best_dir)) = (
                std::path::Path::new(path).parent(),
                std::path::Path::new(best).parent(),
            ) && path_dir == best_dir
            {
                return self.config.module_boost * 0.8;
            }

            // Path component overlap (e.g., both contain "auth")
            for component in std::path::Path::new(path).components() {
                if let Some(comp_str) = component.as_os_str().to_str() {
                    let comp_lower = comp_str.to_lowercase();
                    if best_lower.contains(&comp_lower as &str) && comp_lower.len() > 3 {
                        return self.config.module_boost * 0.6;
                    }
                }
            }
        }

        0.0
    }

    /// Apply token budget to items
    fn apply_token_budget(&self, items: Vec<CapsuleItem>) -> (Vec<CapsuleItem>, usize, bool) {
        let mut token_estimate = 0usize;
        let mut result = Vec::new();
        let mut truncated = false;

        for item in items {
            let item_tokens = item.snippet.len() / 4 + 32; // Rough estimate

            if token_estimate + item_tokens > self.config.max_tokens {
                truncated = true;
                break;
            }

            token_estimate += item_tokens;
            result.push(item);
        }

        (result, token_estimate, truncated)
    }
}

impl Default for ContextCapsuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, name: &str, path: &str, source: &str) -> GraphSearchResult {
        GraphSearchResult {
            id: id.to_string(),
            kind: "Function".to_string(),
            path: path.to_string(),
            name: name.to_string(),
            source: Some(source.to_string()),
            line_number: Some(1),
        }
    }

    #[test]
    fn detect_intent_debug() {
        assert_eq!(
            ContextCapsuleBuilder::detect_intent("debug the error"),
            "debug"
        );
        assert_eq!(
            ContextCapsuleBuilder::detect_intent("fix this bug"),
            "debug"
        );
    }

    #[test]
    fn detect_intent_refactor() {
        assert_eq!(
            ContextCapsuleBuilder::detect_intent("refactor this code"),
            "refactor"
        );
    }

    #[test]
    fn detect_intent_test() {
        assert_eq!(ContextCapsuleBuilder::detect_intent("write a test"), "test");
    }

    #[test]
    fn detect_intent_explore() {
        assert_eq!(
            ContextCapsuleBuilder::detect_intent("show me the code"),
            "explore"
        );
    }

    #[test]
    fn capsule_builder_basic() {
        let mut builder = ContextCapsuleBuilder::new();

        let results = vec![
            make_result(
                "func:auth",
                "authenticate",
                "/src/auth.rs",
                "pub fn authenticate(user: &str) -> bool { true }",
            ),
            make_result(
                "func:login",
                "login",
                "/src/login.rs",
                "pub fn login(user: &str) { }",
            ),
        ];

        let capsule = builder.build("authenticate", results, None, &[]);

        assert_eq!(capsule.intent_detected, "explore");
        assert!(!capsule.capsule_items.is_empty());
        assert!(capsule.token_estimate > 0);
    }

    #[test]
    fn capsule_builder_respects_max_items() {
        let config = CapsuleConfig {
            max_items: 1,
            ..Default::default()
        };

        let mut builder = ContextCapsuleBuilder::with_config(config);

        let results = vec![
            make_result("func:a", "func_a", "/src/a.rs", "fn a() {}"),
            make_result("func:b", "func_b", "/src/b.rs", "fn b() {}"),
        ];

        let capsule = builder.build("func", results, None, &[]);

        assert!(capsule.capsule_items.len() <= 1);
    }

    #[test]
    fn capsule_builder_filters_tests() {
        let config = CapsuleConfig {
            include_tests: false,
            ..Default::default()
        };

        let mut builder = ContextCapsuleBuilder::with_config(config);

        let results = vec![
            make_result("func:main", "main", "/src/main.rs", "fn main() {}"),
            make_result(
                "func:test_main",
                "test_main",
                "/src/test/main.rs",
                "fn test_main() {}",
            ),
        ];

        let capsule = builder.build("main", results, None, &[]);

        assert!(
            capsule
                .capsule_items
                .iter()
                .all(|i| !i.path.contains("/test"))
        );
    }

    #[test]
    fn capsule_builder_path_filters() {
        let mut builder = ContextCapsuleBuilder::new();

        let results = vec![
            make_result("func:a", "func", "/src/a.rs", "fn func() {}"),
            make_result("func:b", "func", "/lib/b.rs", "fn func() {}"),
        ];

        let capsule = builder.build("func", results, None, &["/src".to_string()]);

        assert!(
            capsule
                .capsule_items
                .iter()
                .all(|i| i.path.contains("/src"))
        );
    }

    #[test]
    fn capsule_builder_threshold_relaxation() {
        let config = CapsuleConfig {
            initial_threshold: 0.9,
            min_threshold: 0.1,
            relaxation_step: 0.2,
            ..Default::default()
        };

        let mut builder = ContextCapsuleBuilder::with_config(config);

        let results = vec![make_result(
            "func:a",
            "authenticate",
            "/src/a.rs",
            "fn authenticate() {}",
        )];

        let capsule = builder.build("authenticate", results, None, &[]);

        // Should have relaxed threshold to find results
        assert!(!capsule.capsule_items.is_empty());
        assert!(capsule.fallback_relaxed);
    }

    #[test]
    fn intent_weights() {
        let debug = IntentWeights::debug();
        assert!(debug.fts_weight > debug.centrality_weight);

        let refactor = IntentWeights::refactor();
        assert!(refactor.centrality_weight > debug.centrality_weight);
    }

    #[test]
    fn score_breakdown_included() {
        let mut builder = ContextCapsuleBuilder::new();

        let results = vec![make_result(
            "func:a",
            "test_func",
            "/src/a.rs",
            "fn test_func() {}",
        )];

        let capsule = builder.build("test_func", results, None, &[]);

        assert!(!capsule.capsule_items.is_empty());
        let item = &capsule.capsule_items[0];
        assert!(item.why.fts >= 0.0);
        assert!(item.why.tfidf >= 0.0);
        assert!(item.why.centrality >= 0.0);
        assert!(item.why.proximity >= 0.0);
    }
}
