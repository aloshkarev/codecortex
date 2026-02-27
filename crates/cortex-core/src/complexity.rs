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
}
