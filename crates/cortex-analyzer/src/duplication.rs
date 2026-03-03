//! Code duplication detection for identifying repeated code blocks.
//!
//! This module provides:
//! - **Duplicate Block Detection**: Find similar code blocks
//! - **Token-based Comparison**: Language-agnostic comparison
//! - **Similarity Scoring**: Measure code similarity

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A detected code duplicate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateBlock {
    /// First occurrence location
    pub location1: CodeLocation,
    /// Second occurrence location
    pub location2: CodeLocation,
    /// Similarity score (0.0 - 1.0)
    pub similarity: f64,
    /// Number of duplicated lines
    pub line_count: usize,
    /// The duplicated code snippet
    pub snippet: String,
}

/// Location of code in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    /// File path
    pub file_path: String,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
}

/// Configuration for duplication detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicationConfig {
    /// Minimum lines to consider as duplicate
    pub min_lines: usize,
    /// Minimum tokens to consider as duplicate
    pub min_tokens: usize,
    /// Similarity threshold (0.0 - 1.0)
    pub similarity_threshold: f64,
    /// Whether to ignore whitespace differences
    pub ignore_whitespace: bool,
    /// Whether to normalize identifiers
    pub normalize_identifiers: bool,
}

impl Default for DuplicationConfig {
    fn default() -> Self {
        Self {
            min_lines: 6,
            min_tokens: 50,
            similarity_threshold: 0.8,
            ignore_whitespace: true,
            normalize_identifiers: true,
        }
    }
}

/// Duplication detector
#[derive(Debug, Clone)]
pub struct DuplicationDetector {
    config: DuplicationConfig,
}

impl DuplicationDetector {
    /// Create a new detector with default configuration
    pub fn new() -> Self {
        Self {
            config: DuplicationConfig::default(),
        }
    }

    /// Create a detector with custom configuration
    pub fn with_config(config: DuplicationConfig) -> Self {
        Self { config }
    }

    /// Find duplicate code blocks in source files
    pub fn find_duplicates(&self, sources: &[(String, String)]) -> Vec<DuplicateBlock> {
        let mut duplicates = Vec::new();
        let mut all_blocks: Vec<(String, usize, usize, Vec<String>)> = Vec::new();

        // Extract code blocks from each source
        for (file_path, source) in sources {
            let blocks = self.extract_blocks(source);
            for (start, end, tokens) in blocks {
                all_blocks.push((file_path.clone(), start, end, tokens));
            }
        }

        // Compare blocks for similarity
        for i in 0..all_blocks.len() {
            for j in (i + 1)..all_blocks.len() {
                let (file1, start1, end1, tokens1) = &all_blocks[i];
                let (file2, start2, end2, tokens2) = &all_blocks[j];

                let similarity = self.calculate_similarity(tokens1, tokens2);

                if similarity >= self.config.similarity_threshold {
                    let line_count = end1 - start1 + 1;
                    let snippet = tokens1.join(" ");

                    duplicates.push(DuplicateBlock {
                        location1: CodeLocation {
                            file_path: file1.clone(),
                            start_line: *start1,
                            end_line: *end1,
                        },
                        location2: CodeLocation {
                            file_path: file2.clone(),
                            start_line: *start2,
                            end_line: *end2,
                        },
                        similarity,
                        line_count,
                        snippet: if snippet.len() > 200 {
                            format!("{}...", &snippet[..200])
                        } else {
                            snippet
                        },
                    });
                }
            }
        }

        // Sort by similarity descending
        duplicates.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        duplicates
    }

    /// Find duplicates within a single source file
    pub fn find_duplicates_in_file(&self, source: &str, file_path: &str) -> Vec<DuplicateBlock> {
        let blocks = self.extract_blocks(source);
        let mut duplicates = Vec::new();

        for i in 0..blocks.len() {
            for j in (i + 1)..blocks.len() {
                let (start1, end1, tokens1) = &blocks[i];
                let (start2, end2, tokens2) = &blocks[j];

                let similarity = self.calculate_similarity(tokens1, tokens2);

                if similarity >= self.config.similarity_threshold {
                    let line_count = end1 - start1 + 1;
                    let snippet = tokens1.join(" ");

                    duplicates.push(DuplicateBlock {
                        location1: CodeLocation {
                            file_path: file_path.to_string(),
                            start_line: *start1,
                            end_line: *end1,
                        },
                        location2: CodeLocation {
                            file_path: file_path.to_string(),
                            start_line: *start2,
                            end_line: *end2,
                        },
                        similarity,
                        line_count,
                        snippet: if snippet.len() > 200 {
                            format!("{}...", &snippet[..200])
                        } else {
                            snippet
                        },
                    });
                }
            }
        }

        duplicates
    }

    /// Extract code blocks from source code
    fn extract_blocks(&self, source: &str) -> Vec<(usize, usize, Vec<String>)> {
        let lines: Vec<&str> = source.lines().collect();
        let mut blocks = Vec::new();

        // Use sliding window to extract blocks
        for window_size in self.config.min_lines..=lines.len().min(100) {
            for start in 0..=(lines.len() - window_size) {
                let end = start + window_size;
                let block_lines = &lines[start..end];

                // Skip if block is mostly empty or comments
                if self.is_significant_block(block_lines) {
                    let tokens = self.tokenize_block(block_lines);
                    if tokens.len() >= self.config.min_tokens {
                        blocks.push((start + 1, end, tokens)); // 1-indexed
                    }
                }
            }
        }

        blocks
    }

    /// Check if a block contains significant code
    fn is_significant_block(&self, lines: &[&str]) -> bool {
        let mut code_line_count = 0;

        for line in lines {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Skip comment-only lines
            if trimmed.starts_with("//")
                || trimmed.starts_with("#")
                || trimmed.starts_with("/*")
                || trimmed.starts_with("*")
            {
                continue;
            }

            code_line_count += 1;
        }

        code_line_count >= self.config.min_lines / 2
    }

    /// Tokenize a code block for comparison
    fn tokenize_block(&self, lines: &[&str]) -> Vec<String> {
        let mut tokens = Vec::new();

        for line in lines {
            let trimmed = if self.config.ignore_whitespace {
                line.trim()
            } else {
                line
            };

            if trimmed.is_empty() {
                continue;
            }

            // Split into tokens
            for token in self.tokenize_line(trimmed) {
                let normalized = if self.config.normalize_identifiers {
                    self.normalize_token(&token)
                } else {
                    token
                };
                tokens.push(normalized);
            }
        }

        tokens
    }

    /// Tokenize a single line
    fn tokenize_line(&self, line: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for c in line.chars() {
            if c.is_whitespace() {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            } else if c.is_alphanumeric() || c == '_' {
                current_token.push(c);
            } else {
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                // Include punctuation as separate tokens
                tokens.push(c.to_string());
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }

    /// Normalize a token (replace identifiers with placeholders)
    fn normalize_token(&self, token: &str) -> String {
        // Check if it's an identifier (starts with letter or underscore)
        if token
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        {
            // Check if it's a keyword
            let keywords = [
                "fn", "let", "const", "if", "else", "for", "while", "loop", "match", "return",
                "struct", "enum", "impl", "trait", "pub", "mod", "use", "def", "class", "import",
                "from", "as", "try", "except", "with", "function", "var", "const", "let", "async",
                "await", "new", "this",
            ];

            if keywords.contains(&token) {
                token.to_string()
            } else {
                // Replace identifiers with a placeholder
                "_ID_".to_string()
            }
        } else if token.chars().all(|c| c.is_numeric() || c == '.') {
            // Replace numbers with placeholder
            "_NUM_".to_string()
        } else {
            token.to_string()
        }
    }

    /// Calculate similarity between two token sequences
    fn calculate_similarity(&self, tokens1: &[String], tokens2: &[String]) -> f64 {
        if tokens1.is_empty() || tokens2.is_empty() {
            return 0.0;
        }

        // Use Jaccard similarity
        let set1: HashSet<&str> = tokens1.iter().map(|s| s.as_str()).collect();
        let set2: HashSet<&str> = tokens2.iter().map(|s| s.as_str()).collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Calculate code duplication percentage
    pub fn duplication_percentage(&self, duplicates: &[DuplicateBlock], total_lines: usize) -> f64 {
        if total_lines == 0 {
            return 0.0;
        }

        let duplicated_lines: usize = duplicates.iter().map(|d| d.line_count).sum();
        (duplicated_lines as f64 / total_lines as f64) * 100.0
    }

    /// Find exact duplicate lines
    pub fn find_duplicate_lines(
        &self,
        sources: &[(String, String)],
    ) -> Vec<(String, Vec<CodeLocation>)> {
        let mut line_occurrences: HashMap<String, Vec<CodeLocation>> = HashMap::new();

        for (file_path, source) in sources {
            for (i, line) in source.lines().enumerate() {
                let normalized = if self.config.ignore_whitespace {
                    line.trim().to_string()
                } else {
                    line.to_string()
                };

                // Skip empty lines and comments
                if normalized.is_empty()
                    || normalized.starts_with("//")
                    || normalized.starts_with("#")
                {
                    continue;
                }

                line_occurrences
                    .entry(normalized)
                    .or_default()
                    .push(CodeLocation {
                        file_path: file_path.clone(),
                        start_line: i + 1,
                        end_line: i + 1,
                    });
            }
        }

        // Return only lines that appear multiple times
        line_occurrences
            .into_iter()
            .filter(|(_, locations)| locations.len() > 1)
            .collect()
    }
}

impl Default for DuplicationDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplication_detector_new() {
        let detector = DuplicationDetector::new();
        assert_eq!(detector.config.min_lines, 6);
        assert_eq!(detector.config.similarity_threshold, 0.8);
    }

    #[test]
    fn duplication_detector_with_config() {
        let config = DuplicationConfig {
            min_lines: 10,
            similarity_threshold: 0.9,
            ..Default::default()
        };
        let detector = DuplicationDetector::with_config(config);
        assert_eq!(detector.config.min_lines, 10);
    }

    #[test]
    fn find_duplicates_in_file_empty() {
        let detector = DuplicationDetector::new();
        let source = "fn main() {}";
        let duplicates = detector.find_duplicates_in_file(source, "test.rs");
        assert!(duplicates.is_empty());
    }

    #[test]
    fn find_duplicates_no_match() {
        let detector = DuplicationDetector::new();
        let source = r#"
fn function_one() {
    println!("One");
}

fn function_two() {
    println!("Two");
}
"#;
        let duplicates = detector.find_duplicates_in_file(source, "test.rs");
        // Should not find duplicates as functions are different
        assert!(duplicates.is_empty() || duplicates.iter().all(|d| d.similarity < 0.9));
    }

    #[test]
    fn tokenize_line() {
        let detector = DuplicationDetector::new();
        let tokens = detector.tokenize_line("fn main() {");
        assert!(tokens.contains(&"fn".to_string()));
        assert!(tokens.contains(&"main".to_string()));
    }

    #[test]
    fn normalize_token_identifier() {
        let detector = DuplicationDetector::new();
        assert_eq!(detector.normalize_token("my_variable"), "_ID_");
        assert_eq!(detector.normalize_token("fn"), "fn"); // keyword
    }

    #[test]
    fn normalize_token_number() {
        let detector = DuplicationDetector::new();
        assert_eq!(detector.normalize_token("42"), "_NUM_");
        assert_eq!(detector.normalize_token("3.14"), "_NUM_");
    }

    #[test]
    fn calculate_similarity_identical() {
        let detector = DuplicationDetector::new();
        let tokens1 = vec!["fn".to_string(), "main".to_string(), "(".to_string()];
        let tokens2 = vec!["fn".to_string(), "main".to_string(), "(".to_string()];
        let similarity = detector.calculate_similarity(&tokens1, &tokens2);
        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn calculate_similarity_different() {
        let detector = DuplicationDetector::new();
        let tokens1 = vec!["fn".to_string(), "main".to_string()];
        let tokens2 = vec!["struct".to_string(), "User".to_string()];
        let similarity = detector.calculate_similarity(&tokens1, &tokens2);
        assert_eq!(similarity, 0.0);
    }

    #[test]
    fn calculate_similarity_partial() {
        let detector = DuplicationDetector::new();
        let tokens1 = vec!["fn".to_string(), "main".to_string(), "(".to_string()];
        let tokens2 = vec!["fn".to_string(), "main".to_string()];
        let similarity = detector.calculate_similarity(&tokens1, &tokens2);
        assert!(similarity > 0.5 && similarity < 1.0);
    }

    #[test]
    fn duplication_percentage() {
        let detector = DuplicationDetector::new();
        let duplicates = vec![DuplicateBlock {
            location1: CodeLocation {
                file_path: "a.rs".to_string(),
                start_line: 1,
                end_line: 10,
            },
            location2: CodeLocation {
                file_path: "b.rs".to_string(),
                start_line: 1,
                end_line: 10,
            },
            similarity: 0.9,
            line_count: 10,
            snippet: "...".to_string(),
        }];
        let percentage = detector.duplication_percentage(&duplicates, 100);
        assert!((percentage - 10.0).abs() < 0.001);
    }

    #[test]
    fn find_duplicate_lines() {
        let detector = DuplicationDetector::new();
        let sources = vec![
            (
                "a.rs".to_string(),
                "fn main() {}\nfn helper() {}".to_string(),
            ),
            (
                "b.rs".to_string(),
                "fn main() {}\nfn other() {}".to_string(),
            ),
        ];
        let duplicates = detector.find_duplicate_lines(&sources);
        assert!(!duplicates.is_empty());
    }

    #[test]
    fn is_significant_block() {
        let detector = DuplicationDetector::new();
        let lines = vec!["fn main() {", "    println!(\"Hi\");", "}"];
        assert!(detector.is_significant_block(&lines));
    }

    #[test]
    fn is_significant_block_comments_only() {
        let detector = DuplicationDetector::new();
        let lines = vec!["// comment", "// another comment"];
        assert!(!detector.is_significant_block(&lines));
    }

    #[test]
    fn code_location_serialization() {
        let loc = CodeLocation {
            file_path: "test.rs".to_string(),
            start_line: 10,
            end_line: 20,
        };
        let json = serde_json::to_string(&loc).unwrap();
        assert!(json.contains("test.rs"));
        assert!(json.contains("10"));
    }

    #[test]
    fn duplicate_block_serialization() {
        let block = DuplicateBlock {
            location1: CodeLocation {
                file_path: "a.rs".to_string(),
                start_line: 1,
                end_line: 10,
            },
            location2: CodeLocation {
                file_path: "b.rs".to_string(),
                start_line: 5,
                end_line: 15,
            },
            similarity: 0.95,
            line_count: 10,
            snippet: "fn example()".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("0.95"));
        assert!(json.contains("example"));
    }

    #[test]
    fn duplication_config_default() {
        let config = DuplicationConfig::default();
        assert_eq!(config.min_lines, 6);
        assert_eq!(config.min_tokens, 50);
        assert!((config.similarity_threshold - 0.8).abs() < 0.001);
        assert!(config.ignore_whitespace);
        assert!(config.normalize_identifiers);
    }
}
