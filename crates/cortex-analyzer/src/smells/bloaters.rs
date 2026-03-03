//! Bloater code smells detection
//!
//! Bloaters are code, methods, and classes that have increased to such
//! gigantic proportions that they're hard to work with.
//!
//! Includes:
//! - Long Function
//! - Large Class
//! - Primitive Obsession
//! - Long Parameter List
//! - Data Clumps
//! - Switch Statements

use crate::{CodeSmell, Severity, SmellConfig, SmellType};

/// Detect functions that are too long
pub fn detect_long_functions(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut in_function = false;
    let mut function_start = 0;
    let mut brace_count = 0;
    let mut function_name = String::new();
    let mut paren_depth = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track parenthesis depth for multi-line function signatures
        paren_depth += trimmed.matches('(').count() as i32;
        paren_depth -= trimmed.matches(')').count() as i32;

        // Detect function start (works for Rust, Python, JS, Java, C#, etc.)
        if !in_function
            && (trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("public ")
                || trimmed.starts_with("private ")
                || trimmed.starts_with("protected ")
                || trimmed.starts_with("internal ")
                || trimmed.starts_with("static ")
                || (trimmed.contains("fn ") && trimmed.contains('(')))
        {
            in_function = true;
            function_start = i;
            brace_count = 0;
            function_name = extract_function_name(trimmed);
        }

        if in_function {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Also track Python-style functions (using indentation)
            let is_python_style = !source.contains('{') && trimmed.starts_with("def ");

            if (brace_count == 0 && i > function_start && !is_python_style)
                || (is_python_style && is_python_function_end(&lines, i, function_start))
            {
                let function_lines = i - function_start + 1;

                if function_lines > config.max_function_lines {
                    let severity =
                        calculate_length_severity(function_lines, config.max_function_lines);

                    smells.push(CodeSmell {
                        smell_type: SmellType::LongFunction,
                        severity,
                        file_path: file_path.to_string(),
                        line_number: (function_start + 1) as u32,
                        symbol_name: function_name.clone(),
                        message: format!(
                            "Function '{}' has {} lines (max: {})",
                            function_name, function_lines, config.max_function_lines
                        ),
                        metric_value: Some(function_lines),
                        threshold: Some(config.max_function_lines),
                        suggestion: Some(
                            "Consider breaking this function into smaller, focused functions using Extract Method"
                                .to_string(),
                        ),
                    });
                }

                in_function = false;
            }
        }
    }

    smells
}

/// Detect classes/structs that are too large
pub fn detect_large_classes(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut in_class = false;
    let mut class_start = 0;
    let mut brace_count = 0;
    let mut class_name = String::new();
    let mut method_count = 0;
    let mut field_count = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect class/struct start
        if !in_class
            && (trimmed.starts_with("struct ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub class ")
                || trimmed.contains("impl "))
        {
            in_class = true;
            class_start = i;
            brace_count = 0;
            class_name = extract_class_name(trimmed);
            method_count = 0;
            field_count = 0;
        }

        if in_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Count methods
            if is_method_definition(trimmed) {
                method_count += 1;
            }

            // Count fields (simplified)
            if is_field_definition(trimmed) {
                field_count += 1;
            }

            if brace_count == 0 && i > class_start {
                // Check method count
                if method_count > config.max_methods_per_class {
                    smells.push(CodeSmell {
                        smell_type: SmellType::LargeClass,
                        severity: calculate_count_severity(method_count, config.max_methods_per_class),
                        file_path: file_path.to_string(),
                        line_number: (class_start + 1) as u32,
                        symbol_name: class_name.clone(),
                        message: format!(
                            "Class '{}' has {} methods (max: {})",
                            class_name, method_count, config.max_methods_per_class
                        ),
                        metric_value: Some(method_count),
                        threshold: Some(config.max_methods_per_class),
                        suggestion: Some(
                            "Consider extracting some methods into a separate class using Extract Class"
                                .to_string(),
                        ),
                    });
                }

                // Check field count
                if field_count > config.max_fields_per_class {
                    smells.push(CodeSmell {
                        smell_type: SmellType::LargeClass,
                        severity: calculate_count_severity(field_count, config.max_fields_per_class),
                        file_path: file_path.to_string(),
                        line_number: (class_start + 1) as u32,
                        symbol_name: class_name.clone(),
                        message: format!(
                            "Class '{}' has {} fields (max: {})",
                            class_name, field_count, config.max_fields_per_class
                        ),
                        metric_value: Some(field_count),
                        threshold: Some(config.max_fields_per_class),
                        suggestion: Some(
                            "Consider extracting some fields into a separate class using Extract Class"
                                .to_string(),
                        ),
                    });
                }

                in_class = false;
            }
        }
    }

    smells
}

/// Detect primitive obsession - overuse of primitive types
pub fn detect_primitive_obsession(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Track primitive parameter patterns
    let mut primitive_params: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
            continue;
        }

        // Look for function definitions with primitive parameters
        if is_function_definition(trimmed) {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.rfind(')') {
                    let params_str = &trimmed[paren_start + 1..paren_end];
                    let primitives = count_primitive_params(params_str);

                    if primitives > config.max_primitive_params {
                        let function_name = extract_function_name(trimmed);
                        smells.push(CodeSmell {
                            smell_type: SmellType::PrimitiveObsession,
                            severity: Severity::Warning,
                            file_path: file_path.to_string(),
                            line_number: (i + 1) as u32,
                            symbol_name: function_name.clone(),
                            message: format!(
                                "Function '{}' uses {} primitive parameters - consider using value objects",
                                function_name, primitives
                            ),
                            metric_value: Some(primitives),
                            threshold: Some(config.max_primitive_params),
                            suggestion: Some(
                                "Replace primitive parameters with small objects using Replace Type Code with Class"
                                    .to_string(),
                            ),
                        });
                    }

                    // Track for data clump detection
                    for primitive in extract_primitive_names(params_str) {
                        *primitive_params
                            .entry(primitive.to_lowercase())
                            .or_default() += 1;
                    }
                }
            }
        }
    }

    smells
}

/// Detect long parameter lists
pub fn detect_long_parameter_lists(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_function_definition(trimmed) {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = find_matching_paren(trimmed, paren_start) {
                    let params_str = &trimmed[paren_start + 1..paren_end];

                    if !params_str.trim().is_empty() {
                        let param_count = count_parameters(params_str);

                        if param_count > config.max_parameters {
                            let function_name = extract_function_name(trimmed);
                            let severity =
                                calculate_count_severity(param_count, config.max_parameters);

                            smells.push(CodeSmell {
                                smell_type: SmellType::LongParameterList,
                                severity,
                                file_path: file_path.to_string(),
                                line_number: (i + 1) as u32,
                                symbol_name: function_name.clone(),
                                message: format!(
                                    "Function '{}' has {} parameters (max: {})",
                                    function_name, param_count, config.max_parameters
                                ),
                                metric_value: Some(param_count),
                                threshold: Some(config.max_parameters),
                                suggestion: Some(
                                    "Consider using Introduce Parameter Object or Preserve Whole Object"
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    smells
}

/// Detect data clumps - same data appearing together in multiple places
pub fn detect_data_clumps(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Collect all parameter groups
    let mut param_groups: Vec<(String, Vec<String>, usize)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_function_definition(trimmed) {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = find_matching_paren(trimmed, paren_start) {
                    let params_str = &trimmed[paren_start + 1..paren_end];
                    let param_names: Vec<String> = extract_param_names(params_str);

                    if param_names.len() >= 3 {
                        let function_name = extract_function_name(trimmed);
                        param_groups.push((function_name, param_names, i));
                    }
                }
            }
        }
    }

    // Find overlapping parameter sets (data clumps)
    for i in 0..param_groups.len() {
        for j in (i + 1)..param_groups.len() {
            let (_, params1, line1) = &param_groups[i];
            let (func2, params2, line2) = &param_groups[j];

            // Calculate Jaccard similarity
            let intersection = params1.iter().filter(|p| params2.contains(p)).count();
            let union = params1.len() + params2.len() - intersection;
            let similarity = intersection as f64 / union as f64;

            if similarity >= config.min_data_clump_similarity && intersection >= 3 {
                smells.push(CodeSmell {
                    smell_type: SmellType::DataClumps,
                    severity: Severity::Warning,
                    file_path: file_path.to_string(),
                    line_number: (line1 + 1) as u32,
                    symbol_name: format!("{} / {}", param_groups[i].0, func2),
                    message: format!(
                        "Similar parameter groups found in '{}' and '{}' ({}% overlap)",
                        param_groups[i].0,
                        func2,
                        (similarity * 100.0) as u32
                    ),
                    metric_value: Some(intersection),
                    threshold: Some(3),
                    suggestion: Some(
                        "Extract common parameters into a class using Extract Class or Introduce Parameter Object"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect switch/match statements that are too complex
pub fn detect_switch_statements(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut in_switch = false;
    let mut switch_start = 0;
    let mut case_count = 0;
    let mut brace_count = 0;
    let mut switch_var = String::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect switch/match start
        if !in_switch {
            if trimmed.starts_with("switch ") || trimmed.starts_with("match ") {
                in_switch = true;
                switch_start = i;
                case_count = 0;
                brace_count = 0;
                switch_var = extract_switch_variable(trimmed);
            }
        }

        if in_switch {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Count cases
            if trimmed.starts_with("case ") || trimmed.starts_with("Case ") {
                case_count += 1;
            }
            // Rust match patterns
            if trimmed.contains(" => ") && !trimmed.starts_with("//") {
                case_count += 1;
            }

            // Check for end of switch
            if brace_count == 0 && i > switch_start && trimmed.contains('}') {
                if case_count > config.max_switch_cases {
                    smells.push(CodeSmell {
                        smell_type: SmellType::SwitchStatements,
                        severity: calculate_count_severity(case_count, config.max_switch_cases),
                        file_path: file_path.to_string(),
                        line_number: (switch_start + 1) as u32,
                        symbol_name: switch_var.clone(),
                        message: format!(
                            "Switch/match on '{}' has {} cases (max: {})",
                            switch_var, case_count, config.max_switch_cases
                        ),
                        metric_value: Some(case_count),
                        threshold: Some(config.max_switch_cases),
                        suggestion: Some(
                            "Consider using Replace Conditional with Polymorphism or Replace Type Code with Strategy"
                                .to_string(),
                        ),
                    });
                }

                in_switch = false;
            }
        }
    }

    smells
}

// Helper functions

fn extract_function_name(line: &str) -> String {
    let mut line = line.trim();

    // Strip visibility modifiers and other prefixes
    let prefixes = [
        "pub(crate) ",
        "pub(super) ",
        "pub ",
        "private ",
        "protected ",
        "internal ",
        "static ",
        "async ",
        "extern ",
        "unsafe ",
        "virtual ",
        "override ",
        "final ",
        "abstract ",
    ];

    loop {
        let mut stripped = false;
        for prefix in prefixes {
            if line.starts_with(prefix) {
                line = &line[prefix.len()..];
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    // Strip function keywords
    let keywords = ["fn ", "def ", "function ", "void ", "async fn "];
    for keyword in keywords {
        if line.starts_with(keyword) {
            line = &line[keyword.len()..];
            break;
        }
    }

    // Handle return types (e.g., "fn name() -> Type")
    if let Some(paren_pos) = line.find('(') {
        line = &line[..paren_pos];
    }

    // Get the first word that looks like a function name
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .find(|s| {
            !s.is_empty()
                && s.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
        })
        .unwrap_or("unknown")
        .to_string()
}

fn extract_class_name(line: &str) -> String {
    let line = line.trim();

    // Handle impl blocks
    if line.starts_with("impl ") {
        let rest = &line[5..];
        return rest
            .split(|c: char| c.is_whitespace() || c == '{' || c == '<')
            .find(|s| !s.is_empty())
            .unwrap_or("unknown")
            .to_string();
    }

    // Handle struct/class definitions
    let keywords = ["struct ", "class ", "pub struct ", "pub class "];
    for keyword in keywords {
        if line.starts_with(keyword) {
            let rest = &line[keyword.len()..];
            return rest
                .split(|c: char| c.is_whitespace() || c == '{' || c == '<' || c == '(')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string();
        }
    }

    "unknown".to_string()
}

fn is_function_definition(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private ")
        || trimmed.starts_with("protected ")
        || trimmed.starts_with("internal ")
        || trimmed.starts_with("static ")
        || (trimmed.contains("fn ") && trimmed.contains('('))
}

fn is_method_definition(trimmed: &str) -> bool {
    // Methods in classes
    (trimmed.starts_with("fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private "))
        && trimmed.contains('(')
}

fn is_field_definition(trimmed: &str) -> bool {
    // Rust struct fields
    if trimmed.contains(':') && !trimmed.contains("::") && !trimmed.contains('(') {
        let colon_pos = trimmed.find(':').unwrap();
        let before = &trimmed[..colon_pos];
        // Simple field: name or pub name
        if before.trim().split_whitespace().count() <= 2 {
            return true;
        }
    }
    false
}

fn count_parameters(params_str: &str) -> usize {
    let mut count = 0;
    let mut depth = 0;
    let mut in_param = false;

    for c in params_str.chars() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                if in_param {
                    count += 1;
                    in_param = false;
                }
            }
            c if !c.is_whitespace() && depth == 0 => {
                in_param = true;
            }
            _ => {}
        }
    }

    if in_param {
        count += 1;
    }

    count
}

fn find_matching_paren(s: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.chars().enumerate().skip(start) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn count_primitive_params(params_str: &str) -> usize {
    let primitives = [
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64", "bool", "char", "str", "String", "int", "long", "short", "byte", "float",
        "double", "boolean", "Integer", "Long", "Short", "Byte", "Float", "Double", "Boolean",
    ];

    params_str
        .split(',')
        .filter(|param| {
            let param = param.trim();
            primitives.iter().any(|&p| param.contains(p))
        })
        .count()
}

fn extract_primitive_names(params_str: &str) -> Vec<&str> {
    params_str
        .split(',')
        .filter_map(|param| {
            let param = param.trim();
            param.split(':').next().map(|s| s.trim())
        })
        .collect()
}

fn extract_param_names(params_str: &str) -> Vec<String> {
    params_str
        .split(',')
        .filter_map(|param| {
            let param = param.trim();
            // Handle "name: Type" or "Type name" formats
            if param.contains(':') {
                param.split(':').next().map(|s| s.trim().to_lowercase())
            } else {
                param.split_whitespace().last().map(|s| s.to_lowercase())
            }
        })
        .collect()
}

fn extract_switch_variable(line: &str) -> String {
    let line = line.trim();

    // Rust: match variable { or match variable.method() {
    if line.starts_with("match ") {
        let rest = &line[6..];
        return rest
            .split(|c: char| c == '{' || c == '?')
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string();
    }

    // Other languages: switch (variable) { or switch variable {
    if line.starts_with("switch ") {
        let rest = &line[7..];
        let rest = rest.trim_start_matches('(');
        return rest
            .split(|c: char| c == ')' || c == '{')
            .next()
            .unwrap_or("unknown")
            .trim()
            .to_string();
    }

    "unknown".to_string()
}

fn is_python_function_end(lines: &[&str], current_idx: usize, function_start: usize) -> bool {
    if current_idx <= function_start {
        return false;
    }

    let current_indent = get_indent_level(lines[current_idx]);
    let function_indent = get_indent_level(lines[function_start]);

    // Python function ends when we return to same or lesser indentation
    // and we're past the first line of the function
    current_indent <= function_indent && current_idx > function_start + 1
}

fn get_indent_level(line: &str) -> usize {
    line.len() - line.trim_start_matches(|c| c == ' ' || c == '\t').len()
}

fn calculate_length_severity(actual: usize, max: usize) -> Severity {
    if actual > max * 2 {
        Severity::Critical
    } else if actual > max * 3 / 2 {
        Severity::Error
    } else {
        Severity::Warning
    }
}

fn calculate_count_severity(actual: usize, max: usize) -> Severity {
    if actual > max * 2 {
        Severity::Critical
    } else if actual > max + 3 {
        Severity::Error
    } else {
        Severity::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_long_function() {
        let config = SmellConfig {
            max_function_lines: 5,
            ..Default::default()
        };

        let source =
            "fn long_function() {\n".to_string() + &"    println!(\"line\");\n".repeat(10) + "}\n";

        let smells = detect_long_functions(&source, "test.rs", &config);
        assert!(!smells.is_empty());
        assert_eq!(smells[0].smell_type, SmellType::LongFunction);
    }

    #[test]
    fn test_detect_large_class() {
        let config = SmellConfig {
            max_methods_per_class: 3,
            ..Default::default()
        };

        let source = r#"
struct LargeClass {
    fn method1() {}
    fn method2() {}
    fn method4() {}
    fn method5() {}
}
"#;

        let smells = detect_large_classes(source, "test.rs", &config);
        assert!(!smells.is_empty());
        assert_eq!(smells[0].smell_type, SmellType::LargeClass);
    }

    #[test]
    fn test_detect_long_parameter_list() {
        let config = SmellConfig {
            max_parameters: 3,
            ..Default::default()
        };

        let source = r#"
fn many_params(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
    a + b + c + d + e
}
"#;

        let smells = detect_long_parameter_lists(source, "test.rs", &config);
        assert!(!smells.is_empty());
        assert_eq!(smells[0].smell_type, SmellType::LongParameterList);
    }

    #[test]
    fn test_detect_switch_statements() {
        let config = SmellConfig {
            max_switch_cases: 3,
            ..Default::default()
        };

        let source = r#"
match value {
    1 => "one",
    2 => "two",
    3 => "three",
    4 => "four",
    5 => "five",
    _ => "other",
}
"#;

        let smells = detect_switch_statements(source, "test.rs", &config);
        assert!(!smells.is_empty());
        assert_eq!(smells[0].smell_type, SmellType::SwitchStatements);
    }

    #[test]
    fn test_extract_function_name() {
        assert_eq!(extract_function_name("fn hello_world() {"), "hello_world");
        assert_eq!(
            extract_function_name("pub fn my_function(x: i32) {"),
            "my_function"
        );
        assert_eq!(extract_function_name("async fn process() {"), "process");
    }

    #[test]
    fn test_count_parameters() {
        assert_eq!(count_parameters("a: i32, b: String"), 2);
        assert_eq!(count_parameters("a: i32"), 1);
        assert_eq!(count_parameters(""), 0);
        assert_eq!(count_parameters("a: Vec<HashMap<String, i32>>"), 1);
    }
}
