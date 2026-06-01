//! TF-IDF Scoring for Code Search
//!
//! Implements Term Frequency-Inverse Document Frequency scoring
//! for ranking search results in context capsules and memory search.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

/// Tokenize text into terms
///
/// For all-ASCII input, lowercases per character (no full-string `to_lowercase`
/// allocation). Non-ASCII text uses `str::to_lowercase()` so Unicode special-case
/// rules (e.g. Greek final sigma) match `str` semantics.
pub fn tokenize(text: &str) -> Vec<String> {
    if text.is_ascii() {
        let mut out = Vec::new();
        let mut token = String::new();
        for c in text.chars() {
            if c.is_ascii_alphanumeric() || c == '_' {
                token.push(c.to_ascii_lowercase());
            } else if !token.is_empty() {
                if token.len() > 1 {
                    out.push(std::mem::take(&mut token));
                } else {
                    token.clear();
                }
            }
        }
        if token.len() > 1 {
            out.push(token);
        }
        out
    } else {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }
}

/// Calculate term frequency for a document
pub fn term_frequency(terms: &[String]) -> HashMap<String, f64> {
    let mut tf = HashMap::new();
    let total = terms.len() as f64;

    if total == 0.0 {
        return tf;
    }

    for term in terms {
        if let Some(count) = tf.get_mut(term) {
            *count += 1.0;
        } else {
            tf.insert(term.clone(), 1.0);
        }
    }

    // Normalize by document length
    for count in tf.values_mut() {
        *count /= total;
    }

    tf
}

/// Document representation for TF-IDF
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique document identifier
    pub id: String,
    /// Original text content
    pub text: String,
    /// Tokenized terms
    pub terms: Vec<String>,
    /// Precomputed term frequencies
    pub term_frequencies: HashMap<String, f64>,
}

impl Document {
    /// Create a new document from text
    pub fn new(id: impl Into<String>, text: &str) -> Self {
        let terms = tokenize(text);
        let term_frequencies = term_frequency(&terms);
        Self {
            id: id.into(),
            text: text.to_string(),
            terms,
            term_frequencies,
        }
    }
}

/// TF-IDF scorer for a collection of documents
#[derive(Debug, Clone)]
pub struct TfIdfScorer {
    /// Document frequency for each term
    document_frequencies: HashMap<String, usize>,
    /// Total number of documents
    total_documents: usize,
    /// Cached IDF values
    idf_cache: HashMap<String, f64>,
}

impl TfIdfScorer {
    /// Create a new empty scorer
    pub fn new() -> Self {
        Self {
            document_frequencies: HashMap::new(),
            total_documents: 0,
            idf_cache: HashMap::new(),
        }
    }

    /// Create a scorer from a collection of documents
    pub fn from_documents(documents: &[Document]) -> Self {
        let mut scorer = Self::new();
        for doc in documents {
            scorer.add_document(doc);
        }
        scorer.recompute_idf();
        scorer
    }

    /// Add a document to the corpus
    pub fn add_document(&mut self, doc: &Document) {
        self.total_documents += 1;

        // Track unique terms in this document
        let seen: HashSet<&String> = doc.terms.iter().collect();

        for term in seen {
            if let Some(count) = self.document_frequencies.get_mut(term.as_str()) {
                *count += 1;
            } else {
                self.document_frequencies.insert(term.clone(), 1);
            }
        }

        // Invalidate IDF cache
        self.idf_cache.clear();
    }

    /// Recompute IDF values after batch document addition
    pub fn recompute_idf(&mut self) {
        self.idf_cache.clear();
        for term in self.document_frequencies.keys() {
            self.idf_cache.insert(term.clone(), self.compute_idf(term));
        }
    }

    /// Compute IDF for a term
    fn compute_idf(&self, term: &str) -> f64 {
        let df = self.document_frequencies.get(term).copied().unwrap_or(0);
        if df == 0 || self.total_documents == 0 {
            return 0.0;
        }
        // Standard IDF formula with smoothing
        ((self.total_documents as f64 + 1.0) / (df as f64 + 1.0)).ln() + 1.0
    }

    /// Get IDF for a term (from cache or computed)
    pub fn idf(&self, term: &str) -> f64 {
        self.idf_cache
            .get(term)
            .copied()
            .unwrap_or_else(|| self.compute_idf(term))
    }

    /// Score a document against a query
    pub fn score(&self, query_terms: &[String], doc: &Document) -> f64 {
        let mut score = 0.0;

        for query_term in query_terms {
            let tf = doc.term_frequencies.get(query_term).copied().unwrap_or(0.0);
            let idf = self.idf(query_term);
            score += tf * idf;
        }

        // Normalize by document length to avoid bias toward long documents
        let doc_length = doc.terms.len() as f64;
        if doc_length > 0.0 {
            score /= doc_length.sqrt();
        }

        score
    }

    /// Score all documents and return sorted results
    pub fn score_all(&self, query: &str, documents: &[Document]) -> Vec<(String, f64)> {
        let query_terms = tokenize(query);

        let mut scores: Vec<(String, f64)> = documents
            .iter()
            .map(|doc| (doc.id.clone(), self.score(&query_terms, doc)))
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    /// Get the number of documents in the corpus
    pub fn document_count(&self) -> usize {
        self.total_documents
    }

    /// Get the vocabulary size
    pub fn vocabulary_size(&self) -> usize {
        self.document_frequencies.len()
    }
}

impl Default for TfIdfScorer {
    fn default() -> Self {
        Self::new()
    }
}

/// Simplified TF-IDF for on-demand scoring without a pre-built corpus
pub fn simple_tfidf_score(query: &str, document: &str) -> f64 {
    let query_terms = tokenize(query);
    let doc_terms = tokenize(document);

    if query_terms.is_empty() || doc_terms.is_empty() {
        return 0.0;
    }

    let doc_tf = term_frequency(&doc_terms);

    // Simple scoring: sum of term frequencies
    let mut score = 0.0;
    for query_term in &query_terms {
        if let Some(&tf) = doc_tf.get(query_term) {
            // Boost for exact match, with position-aware bonus
            score += tf * 2.0;
        }
    }

    // Normalize by document length
    score / (1.0 + doc_terms.len() as f64).ln_1p()
}

/// BM25 scoring variant for better ranking
#[derive(Debug, Clone)]
pub struct Bm25Scorer {
    /// Document frequency for each term
    document_frequencies: HashMap<String, usize>,
    /// Total number of documents
    total_documents: usize,
    /// Average document length
    avg_doc_length: f64,
    /// BM25 parameters
    k1: f64,
    b: f64,
}

impl Bm25Scorer {
    /// Create a new BM25 scorer
    pub fn new() -> Self {
        Self {
            document_frequencies: HashMap::new(),
            total_documents: 0,
            avg_doc_length: 0.0,
            k1: 1.5,
            b: 0.75,
        }
    }

    /// Set BM25 parameters
    pub fn with_params(mut self, k1: f64, b: f64) -> Self {
        self.k1 = k1;
        self.b = b;
        self
    }

    /// Add a document to the corpus
    pub fn add_document(&mut self, doc: &Document) {
        self.total_documents += 1;

        let doc_length = doc.terms.len() as f64;
        self.avg_doc_length = (self.avg_doc_length * (self.total_documents - 1) as f64
            + doc_length)
            / self.total_documents as f64;

        let seen: HashSet<&String> = doc.terms.iter().collect();
        for term in seen {
            if let Some(count) = self.document_frequencies.get_mut(term.as_str()) {
                *count += 1;
            } else {
                self.document_frequencies.insert(term.clone(), 1);
            }
        }
    }

    /// Compute IDF for a term (BM25 variant)
    fn idf(&self, term: &str) -> f64 {
        let df = self.document_frequencies.get(term).copied().unwrap_or(0) as f64;
        let n = self.total_documents as f64;
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    /// Score a document against a query
    pub fn score(&self, query_terms: &[String], doc: &Document) -> f64 {
        let doc_length = doc.terms.len() as f64;
        let mut score = 0.0;

        for query_term in query_terms {
            let tf = doc.term_frequencies.get(query_term).copied().unwrap_or(0.0);
            let idf = self.idf(query_term);

            // BM25 formula
            let numerator = tf * (self.k1 + 1.0);
            let denominator =
                tf + self.k1 * (1.0 - self.b + self.b * (doc_length / self.avg_doc_length));

            score += idf * (numerator / denominator);
        }

        score
    }
}

impl Default for Bm25Scorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_correctly() {
        let terms = tokenize("hello_world foo-bar baz");
        assert_eq!(terms, vec!["hello_world", "foo", "bar", "baz"]);
    }

    #[test]
    fn tokenize_lowercases() {
        let terms = tokenize("HELLO World");
        assert_eq!(terms, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_filters_short() {
        let terms = tokenize("a b cd efg");
        assert_eq!(terms, vec!["cd", "efg"]);
    }

    #[test]
    fn term_frequency_normalizes() {
        let terms = tokenize("foo foo bar");
        let tf = term_frequency(&terms);

        assert!((tf.get("foo").unwrap() - 0.6666666666666666).abs() < 0.01);
        assert!((tf.get("bar").unwrap() - 0.3333333333333333).abs() < 0.01);
    }

    #[test]
    fn document_creation() {
        let doc = Document::new("doc1", "hello world hello");

        assert_eq!(doc.id, "doc1");
        assert_eq!(doc.terms, vec!["hello", "world", "hello"]);
        assert!(doc.term_frequencies.contains_key("hello"));
    }

    #[test]
    fn tfidf_scorer_basic() {
        let docs = vec![
            Document::new("doc1", "the quick brown fox"),
            Document::new("doc2", "the lazy dog"),
            Document::new("doc3", "the quick dog"),
        ];

        let scorer = TfIdfScorer::from_documents(&docs);

        // "quick" appears in 2 of 3 docs, should have lower IDF than rare terms
        assert!(scorer.idf("quick") > 0.0);

        // Score for "quick" against doc1 (contains "quick")
        let query = tokenize("quick");
        let score1 = scorer.score(&query, &docs[0]);
        let score2 = scorer.score(&query, &docs[1]);

        assert!(score1 > score2);
    }

    #[test]
    fn tfidf_score_all() {
        let docs = vec![
            Document::new("doc1", "rust programming language"),
            Document::new("doc2", "python programming"),
            Document::new("doc3", "rust vs python"),
        ];

        let scorer = TfIdfScorer::from_documents(&docs);
        let results = scorer.score_all("rust programming", &docs);

        assert!(!results.is_empty());
        // doc1 and doc3 should score highest for "rust programming"
    }

    #[test]
    fn simple_tfidf_score_works() {
        let score1 = simple_tfidf_score("test", "this is a test document");
        let score2 = simple_tfidf_score("test", "no match here");

        assert!(score1 > score2);
        assert!(score2 == 0.0);
    }

    #[test]
    fn bm25_scorer() {
        let mut scorer = Bm25Scorer::new().with_params(1.5, 0.75);

        let docs = vec![
            Document::new("doc1", "the quick brown fox jumps"),
            Document::new("doc2", "the lazy dog sleeps"),
        ];

        for doc in &docs {
            scorer.add_document(doc);
        }

        let query = tokenize("quick fox");
        let score1 = scorer.score(&query, &docs[0]);
        let score2 = scorer.score(&query, &docs[1]);

        assert!(score1 > score2);
    }
}
