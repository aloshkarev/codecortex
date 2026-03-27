/// Prefixes that add one to cyclomatic complexity when scanned byte-by-byte.
/// Order matters: longer tokens (e.g. `else if`) must precede shorter ones (`if `).
const CYCLOMATIC_TOKEN_PREFIXES: &[&str] = &[
    "else if", "if ", "for ", "while ", "match ", "case ", "&&", "||",
];

pub fn compute_cyclomatic_complexity(source: &str) -> u32 {
    let mut complexity = 1u32;
    // Iterate over valid char boundaries so multi-byte Unicode never causes a
    // panic when slicing `source[byte_pos..]`.
    for (byte_pos, _ch) in source.char_indices() {
        let rest = &source[byte_pos..];
        if CYCLOMATIC_TOKEN_PREFIXES
            .iter()
            .any(|prefix| rest.starts_with(prefix))
            || rest.starts_with('?')
        {
            complexity = complexity.saturating_add(1);
        }
    }
    complexity
}

/// Compute cognitive complexity (Nesting-aware complexity metric)
///
/// Cognitive complexity increments for:
/// - Control structures (if, for, while, switch, catch)
/// - Nesting levels (incremented for each nested level)
/// - Logical operators (&&, ||) but not at the top level
///
/// See: https://www.sonarsource.com/resources/cognitive-complexity/
#[allow(clippy::if_same_then_else)]
pub fn compute_cognitive_complexity(source: &str) -> u32 {
    let mut complexity = 0u32;
    let mut nesting_level = 0u32;
    let mut char_indices = source.char_indices().peekable();

    while let Some((byte_start, ch)) = char_indices.next() {
        let trimmed = source[byte_start..].trim_start();

        // Check for control structures that increase complexity
        if trimmed.starts_with("if ") || trimmed.starts_with("if(") {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if trimmed.starts_with("else if") {
            complexity += 1;
        } else if trimmed.starts_with("else") && !trimmed.starts_with("else if") {
            // else alone doesn't add complexity
        } else if trimmed.starts_with("for ")
            || trimmed.starts_with("for(")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("while(")
            || trimmed.starts_with("switch ")
            || trimmed.starts_with("switch(")
            || trimmed.starts_with("match ")
            || trimmed.starts_with("match(")
            || trimmed.starts_with("catch ")
            || trimmed.starts_with("catch(")
        {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if trimmed.starts_with("case ") {
            complexity += 1;
        } else if trimmed.starts_with("try ") || trimmed.starts_with("try{") {
            nesting_level += 1;
        }

        if ch == '}' && nesting_level > 0 {
            nesting_level = nesting_level.saturating_sub(1);
        }

        if let Some(&(_, next_ch)) = char_indices.peek()
            && ((ch == '&' && next_ch == '&') || (ch == '|' && next_ch == '|'))
        {
            complexity += 1;
            char_indices.next();
        }

        if ch == '?' {
            complexity += 1 + nesting_level;
        }
    }

    complexity
}

/// Complexity rating based on cognitive complexity value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityRating {
    /// Simple (0-5)
    Simple,
    /// Moderate (6-10)
    Moderate,
    /// Complex (11-20)
    Complex,
    /// VeryComplex (21+)
    VeryComplex,
}

impl ComplexityRating {
    /// Get rating from cognitive complexity value
    pub fn from_complexity(value: u32) -> Self {
        match value {
            0..=5 => Self::Simple,
            6..=10 => Self::Moderate,
            11..=20 => Self::Complex,
            _ => Self::VeryComplex,
        }
    }

    /// Get a human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Moderate => "moderate",
            Self::Complex => "complex",
            Self::VeryComplex => "very_complex",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complexity_simple_function() {
        let source = "fn main() { println!(\"hello\"); }";
        assert_eq!(compute_cyclomatic_complexity(source), 1);
    }

    #[test]
    fn complexity_empty_source() {
        assert_eq!(compute_cyclomatic_complexity(""), 1);
    }

    #[test]
    fn complexity_with_if() {
        let source = "fn test() { if x > 0 { y } }";
        assert_eq!(compute_cyclomatic_complexity(source), 2);
    }

    #[test]
    fn complexity_with_if_else_if() {
        let source = "fn test() { if x > 0 { a } else if x < 0 { b } }";
        // 1 base + 2 "if " (one standalone, one inside "else if") + 1 "else if" = 4
        assert_eq!(compute_cyclomatic_complexity(source), 4);
    }

    #[test]
    fn complexity_with_for_loop() {
        let source = "fn test() { for i in 0..10 { x } }";
        assert_eq!(compute_cyclomatic_complexity(source), 2);
    }

    #[test]
    fn complexity_with_while_loop() {
        let source = "fn test() { while x > 0 { x -= 1; } }";
        assert_eq!(compute_cyclomatic_complexity(source), 2);
    }

    #[test]
    fn complexity_with_match() {
        let source = "fn test() { match x { 1 => a, _ => b } }";
        assert_eq!(compute_cyclomatic_complexity(source), 2);
    }

    #[test]
    fn complexity_with_and_operator() {
        let source = "fn test() { if x > 0 && y > 0 { z } }";
        assert_eq!(compute_cyclomatic_complexity(source), 3);
    }

    #[test]
    fn complexity_with_or_operator() {
        let source = "fn test() { if x > 0 || y > 0 { z } }";
        assert_eq!(compute_cyclomatic_complexity(source), 3);
    }

    #[test]
    fn complexity_with_ternary() {
        let source = "let x = condition ? a : b;";
        assert_eq!(compute_cyclomatic_complexity(source), 2);
    }

    #[test]
    fn complexity_complex_function() {
        let source = r#"
            fn complex(x: i32, y: i32) -> i32 {
                if x > 0 {
                    for i in 0..x {
                        if i % 2 == 0 && y > 0 {
                            return i;
                        }
                    }
                } else if x < 0 {
                    while y > 0 {
                        y -= 1;
                    }
                }
                0
            }
        "#;
        // 1 base + 3 "if " (if x, if i, if x in else if) + 1 "else if" + 1 "for " + 1 "while " + 1 "&&" = 8
        assert_eq!(compute_cyclomatic_complexity(source), 8);
    }

    #[test]
    fn complexity_case_statement() {
        let source = "switch x { case 1: a; case 2: b; }";
        // 1 base + 2 "case " = 3
        assert_eq!(compute_cyclomatic_complexity(source), 3);
    }

    #[test]
    fn cognitive_complexity_simple() {
        let source = "fn main() { println!(\"hello\"); }";
        // No control structures = 0
        let complexity = compute_cognitive_complexity(source);
        assert_eq!(complexity, 0);
    }

    #[test]
    fn cognitive_complexity_with_if() {
        let source = "fn test() { if x > 0 { y } }";
        // Contains an if statement
        let complexity = compute_cognitive_complexity(source);
        assert!(
            complexity >= 1,
            "Expected complexity >= 1, got {}",
            complexity
        );
    }

    #[test]
    fn cognitive_complexity_nested_if() {
        let source = "fn test() { if x { if y { z } } }";
        // Nested ifs should have higher complexity than single if
        let nested_complexity = compute_cognitive_complexity(source);
        let single_complexity = compute_cognitive_complexity("fn test() { if x { y } }");
        assert!(
            nested_complexity > single_complexity,
            "Nested complexity ({}) should be > single complexity ({})",
            nested_complexity,
            single_complexity
        );
    }

    #[test]
    fn cognitive_complexity_deeply_nested() {
        let source = "fn test() { if a { if b { if c { d } } } }";
        // Deeply nested should have higher complexity than shallow
        let deep_complexity = compute_cognitive_complexity(source);
        let shallow_complexity = compute_cognitive_complexity("fn test() { if a { b } }");
        assert!(
            deep_complexity > shallow_complexity,
            "Deep complexity ({}) should be > shallow complexity ({})",
            deep_complexity,
            shallow_complexity
        );
    }

    #[test]
    fn complexity_rating_simple() {
        assert_eq!(
            ComplexityRating::from_complexity(3),
            ComplexityRating::Simple
        );
        assert_eq!(
            ComplexityRating::from_complexity(0),
            ComplexityRating::Simple
        );
    }

    #[test]
    fn complexity_rating_moderate() {
        assert_eq!(
            ComplexityRating::from_complexity(6),
            ComplexityRating::Moderate
        );
        assert_eq!(
            ComplexityRating::from_complexity(10),
            ComplexityRating::Moderate
        );
    }

    #[test]
    fn complexity_rating_complex() {
        assert_eq!(
            ComplexityRating::from_complexity(11),
            ComplexityRating::Complex
        );
        assert_eq!(
            ComplexityRating::from_complexity(20),
            ComplexityRating::Complex
        );
    }

    #[test]
    fn complexity_rating_very_complex() {
        assert_eq!(
            ComplexityRating::from_complexity(21),
            ComplexityRating::VeryComplex
        );
        assert_eq!(
            ComplexityRating::from_complexity(100),
            ComplexityRating::VeryComplex
        );
    }

    #[test]
    fn complexity_rating_as_str() {
        assert_eq!(ComplexityRating::Simple.as_str(), "simple");
        assert_eq!(ComplexityRating::Moderate.as_str(), "moderate");
        assert_eq!(ComplexityRating::Complex.as_str(), "complex");
        assert_eq!(ComplexityRating::VeryComplex.as_str(), "very_complex");
    }

    /// Regression: source containing multi-byte Unicode characters (em-dash, arrow, etc.)
    /// must not panic when computing cyclomatic complexity.
    #[test]
    fn complexity_unicode_in_source_does_not_panic() {
        let sources = [
            "fn f() { // — em-dash in comment\n if x { 1 } else { 2 } }",
            "fn g() { /* → arrow */ for i in 0..n { } }",
            "let s = \"≈ approximate\"; if ok { return s; }",
            "// ▲ ● ‑ … · – multi-byte chars everywhere\nwhile cond { }",
            "fn h() { match x { Some(v) => v, None => 0 } }",
        ];
        for src in &sources {
            // Must not panic; we don't assert a specific value.
            let _ = compute_cyclomatic_complexity(src);
            let _ = compute_cognitive_complexity(src);
        }
    }
}
