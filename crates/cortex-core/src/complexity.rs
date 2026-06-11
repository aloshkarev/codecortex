pub fn compute_cyclomatic_complexity(source: &str) -> u32 {
    const TOKENS: &[&str] = &[
        "if ", "else if", "for ", "while ", "match ", "case ", "&&", "||", "?",
    ];

    let mut complexity = 1u32;
    for token in TOKENS {
        let count = source.matches(token).count() as u32;
        complexity = complexity.saturating_add(count);
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
pub fn compute_cognitive_complexity(source: &str) -> u32 {
    let bytes = source.as_bytes();
    let mut complexity = 0u32;
    let mut nesting_level = 0u32;
    let mut i = 0usize;

    while i < bytes.len() {
        if !source.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let tail = &source[i..];

        if matches_keyword(tail, "if ") || matches_keyword(tail, "if(") {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if matches_keyword(tail, "else if") {
            complexity += 1;
        } else if matches_keyword(tail, "else") && !matches_keyword(tail, "else if") {
            // else alone — no increment
        } else if matches_keyword(tail, "for ")
            || matches_keyword(tail, "for(")
            || matches_keyword(tail, "while ")
            || matches_keyword(tail, "while(")
        {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if matches_keyword(tail, "switch ")
            || matches_keyword(tail, "switch(")
            || matches_keyword(tail, "match ")
            || matches_keyword(tail, "match(")
        {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if matches_keyword(tail, "case ") {
            complexity += 1;
        } else if matches_keyword(tail, "catch ") || matches_keyword(tail, "catch(") {
            complexity += 1 + nesting_level;
            nesting_level += 1;
        } else if matches_keyword(tail, "try ") || matches_keyword(tail, "try{") {
            nesting_level += 1;
        }

        let b = bytes[i];
        if b == b'}' && nesting_level > 0 {
            nesting_level = nesting_level.saturating_sub(1);
        }
        if b == b'&' && i + 1 < bytes.len() && bytes[i + 1] == b'&' {
            complexity += 1;
            i += 1;
        } else if b == b'|' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            complexity += 1;
            i += 1;
        } else if b == b'?' {
            complexity += 1 + nesting_level;
        }

        i += 1;
    }

    complexity
}

fn matches_keyword(haystack: &str, keyword: &str) -> bool {
    haystack.starts_with(keyword)
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
            0..=5 => ComplexityRating::Simple,
            6..=10 => ComplexityRating::Moderate,
            11..=20 => ComplexityRating::Complex,
            _ => ComplexityRating::VeryComplex,
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ComplexityRating::Simple => "simple",
            ComplexityRating::Moderate => "moderate",
            ComplexityRating::Complex => "complex",
            ComplexityRating::VeryComplex => "very_complex",
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
        assert_eq!(compute_cyclomatic_complexity(source), 8);
    }

    #[test]
    fn complexity_case_statement() {
        let source = "switch x { case 1: a; case 2: b; }";
        assert_eq!(compute_cyclomatic_complexity(source), 3);
    }

    #[test]
    fn cognitive_complexity_simple() {
        let source = "fn main() { println!(\"hello\"); }";
        assert_eq!(compute_cognitive_complexity(source), 0);
    }

    #[test]
    fn cognitive_complexity_with_if() {
        let source = "fn test() { if x > 0 { y } }";
        assert!(compute_cognitive_complexity(source) >= 1);
    }

    #[test]
    fn cognitive_complexity_nested_if() {
        let source = "fn test() { if x { if y { z } } }";
        let nested = compute_cognitive_complexity(source);
        let single = compute_cognitive_complexity("fn test() { if x { y } }");
        assert!(nested > single);
    }

    #[test]
    fn cognitive_complexity_deeply_nested() {
        let source = "fn test() { if a { if b { if c { d } } } }";
        let deep = compute_cognitive_complexity(source);
        let shallow = compute_cognitive_complexity("fn test() { if a { b } }");
        assert!(deep > shallow);
    }

    #[test]
    fn complexity_rating_simple() {
        assert_eq!(
            ComplexityRating::from_complexity(3),
            ComplexityRating::Simple
        );
    }

    #[test]
    fn complexity_rating_moderate() {
        assert_eq!(
            ComplexityRating::from_complexity(6),
            ComplexityRating::Moderate
        );
    }

    #[test]
    fn complexity_rating_complex() {
        assert_eq!(
            ComplexityRating::from_complexity(11),
            ComplexityRating::Complex
        );
    }

    #[test]
    fn complexity_rating_very_complex() {
        assert_eq!(
            ComplexityRating::from_complexity(21),
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

    #[test]
    fn complexity_unicode_in_source_does_not_panic() {
        let sources = [
            "fn f() { // — em-dash in comment\n if x { 1 } else { 2 } }",
            "fn g() { /* → arrow */ for i in 0..n { } }",
            "let s = \"≈ approximate\"; if ok { return s; }",
            "// ▲ ● ‑ … · – multi-byte chars everywhere\nwhile cond { }",
            "fn h() { match x { Some(v) => v, None => 0 } }",
        ];

        for source in sources {
            let _ = compute_cyclomatic_complexity(source);
            let _ = compute_cognitive_complexity(source);
        }
    }
}
