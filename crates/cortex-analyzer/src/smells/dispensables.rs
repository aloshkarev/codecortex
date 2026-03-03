//! Dispensable code smells detection
//!
//! A dispensable is something pointless and unneeded whose absence
//! would make the code cleaner, more efficient, and easier to understand.
//!
//! Includes:
//! - Comments
//! - Duplicate Code
//! - Data Class
//! - Dead Code
//! - Lazy Class
//! - Speculative Generality

use crate::{CodeSmell, Severity, SmellConfig, SmellType};
use std::collections::{HashMap, HashSet};

/// Detect excessive comments - code that needs comments to be understood
pub fn detect_comments(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let total_lines = lines.len();
    let mut comment_lines = 0;
    let mut code_lines = 0;
    let mut todo_count = 0;
    let mut fixme_count = 0;
    let mut commented_out_code_lines = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Count different comment types
        if is_comment_line(trimmed) {
            comment_lines += 1;

            // Check for TODO/FIXME accumulation
            if trimmed.contains("TODO") || trimmed.contains("todo!") {
                todo_count += 1;
            }
            if trimmed.contains("FIXME") {
                fixme_count += 1;
            }

            // Check if it looks like commented-out code
            if looks_like_code(trimmed) {
                commented_out_code_lines += 1;
                if commented_out_code_lines >= config.min_commented_code_lines {
                    smells.push(CodeSmell {
                        smell_type: SmellType::Comments,
                        severity: Severity::Warning,
                        file_path: file_path.to_string(),
                        line_number: (i + 1) as u32,
                        symbol_name: "commented_code".to_string(),
                        message: "Commented-out code detected - remove or restore it".to_string(),
                        metric_value: Some(commented_out_code_lines),
                        threshold: Some(config.min_commented_code_lines),
                        suggestion: Some("Remove commented-out code or restore it if needed. Version control remembers history.".to_string()),
                    });
                }
            }
        } else if !trimmed.is_empty() {
            code_lines += 1;
        }
    }

    // Calculate comment ratio
    if code_lines > 0 {
        let comment_ratio = comment_lines as f64 / code_lines as f64;

        if comment_ratio > config.max_comment_ratio {
            smells.push(CodeSmell {
                smell_type: SmellType::Comments,
                severity: if comment_ratio > 0.5 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                file_path: file_path.to_string(),
                line_number: 1,
                symbol_name: file_path.to_string(),
                message: format!(
                    "High comment-to-code ratio ({:.0}%) - code may need simplification",
                    comment_ratio * 100.0
                ),
                metric_value: Some((comment_ratio * 100.0) as usize),
                threshold: Some((config.max_comment_ratio * 100.0) as usize),
                suggestion: Some(
                    "Refactor code to be self-documenting. Use Extract Method with descriptive names."
                        .to_string(),
                ),
            });
        }
    }

    // Check for TODO/FIXME accumulation
    let total_todos = todo_count + fixme_count;
    if total_todos >= config.max_todos {
        smells.push(CodeSmell {
            smell_type: SmellType::Comments,
            severity: Severity::Info,
            file_path: file_path.to_string(),
            line_number: 1,
            symbol_name: "todos".to_string(),
            message: format!("{} TODO/FIXME comments accumulated", total_todos),
            metric_value: Some(total_todos),
            threshold: Some(config.max_todos),
            suggestion: Some(
                "Address accumulated TODO/FIXME comments or convert to tracked issues".to_string(),
            ),
        });
    }

    smells
}

/// Detect duplicate code - similar code in multiple places
pub fn detect_duplicate_code(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Build token sequences for each function/method
    let functions = extract_functions_with_tokens(&lines);

    // Compare functions for similarity
    let func_names: Vec<&String> = functions.keys().collect();
    for i in 0..func_names.len() {
        for j in (i + 1)..func_names.len() {
            let func1 = &functions[func_names[i]];
            let func2 = &functions[func_names[j]];

            // Calculate token similarity
            let similarity = calculate_token_similarity(&func1.tokens, &func2.tokens);

            if similarity >= config.min_duplicate_similarity {
                let common_tokens =
                    (similarity * func1.tokens.len().min(func2.tokens.len()) as f64) as usize;

                if common_tokens >= config.min_duplicate_tokens {
                    smells.push(CodeSmell {
                        smell_type: SmellType::DuplicateCode,
                        severity: if similarity > 0.9 {
                            Severity::Error
                        } else {
                            Severity::Warning
                        },
                        file_path: file_path.to_string(),
                        line_number: func1.line_number.min(func2.line_number),
                        symbol_name: format!("{} / {}", func_names[i], func_names[j]),
                        message: format!(
                            "Functions '{}' and '{}' have {:.0}% code similarity",
                            func_names[i],
                            func_names[j],
                            similarity * 100.0
                        ),
                        metric_value: Some((similarity * 100.0) as usize),
                        threshold: Some((config.min_duplicate_similarity * 100.0) as usize),
                        suggestion: Some(
                            "Extract common code into a shared function using Extract Method"
                                .to_string(),
                        ),
                    });
                }
            }
        }
    }

    // Also check for repeated code blocks (not just functions)
    let duplicate_blocks = find_duplicate_blocks(&lines, config.min_duplicate_lines);

    for block in duplicate_blocks {
        smells.push(CodeSmell {
            smell_type: SmellType::DuplicateCode,
            severity: Severity::Warning,
            file_path: file_path.to_string(),
            line_number: block.line_number,
            symbol_name: format!("block_{}", block.line_number),
            message: format!(
                "Code block appears {} times (lines {} each)",
                block.occurrences, block.length
            ),
            metric_value: Some(block.occurrences),
            threshold: Some(2),
            suggestion: Some(
                "Extract duplicate code block into a reusable function or component".to_string(),
            ),
        });
    }

    smells
}

/// Detect data classes - classes with only data, no behavior
pub fn detect_data_classes(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let classes = extract_class_info(&lines);

    for (class_name, class_info) in &classes {
        // A data class has mostly fields and only simple getters/setters
        let method_count = class_info.methods.len();
        let field_count = class_info.fields.len();

        if field_count == 0 {
            continue;
        }

        // Count non-trivial methods (not just getters/setters)
        let business_methods: Vec<_> = class_info
            .methods
            .iter()
            .filter(|m| !is_getter_or_setter(m))
            .collect();

        // If most methods are getters/setters, it's a data class
        if method_count > 0 && business_methods.len() <= config.max_data_class_methods {
            let getter_setter_ratio =
                (method_count - business_methods.len()) as f64 / method_count as f64;

            if getter_setter_ratio > 0.8 || (method_count <= 2 && field_count >= 3) {
                smells.push(CodeSmell {
                    smell_type: SmellType::DataClass,
                    severity: Severity::Info,
                    file_path: file_path.to_string(),
                    line_number: class_info.line_number,
                    symbol_name: class_name.clone(),
                    message: format!(
                        "Class '{}' appears to be a data class ({} fields, {} getters/setters, {} business methods)",
                        class_name, field_count, method_count - business_methods.len(), business_methods.len()
                    ),
                    metric_value: Some(business_methods.len()),
                    threshold: Some(config.max_data_class_methods),
                    suggestion: Some(
                        "Move related behavior into the class using Move Method, or accept it as a data transfer object"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect dead code - unreachable or unused code
pub fn detect_dead_code(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Extract all defined functions and their usages
    let defined_functions = extract_defined_functions(&lines);
    let called_functions = extract_all_function_calls(&lines);

    // Find unused functions
    for (func_name, line_number) in &defined_functions {
        if !called_functions.contains(func_name) && !is_entry_point(func_name) {
            smells.push(CodeSmell {
                smell_type: SmellType::DeadCode,
                severity: Severity::Warning,
                file_path: file_path.to_string(),
                line_number: *line_number,
                symbol_name: func_name.clone(),
                message: format!("Function '{}' is defined but never called", func_name),
                metric_value: Some(0),
                threshold: Some(1),
                suggestion: Some(
                    "Remove unused function or document why it's intentionally unused".to_string(),
                ),
            });
        }
    }

    // Check for unreachable code patterns
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Code after return/throw/break/continue
        if i > 0 {
            let prev_line = lines[i - 1].trim();
            if is_exit_statement(prev_line) && !is_comment_line(trimmed) && !trimmed.is_empty() {
                smells.push(CodeSmell {
                    smell_type: SmellType::DeadCode,
                    severity: Severity::Warning,
                    file_path: file_path.to_string(),
                    line_number: (i + 1) as u32,
                    symbol_name: "unreachable".to_string(),
                    message: "Potentially unreachable code after exit statement".to_string(),
                    metric_value: Some(1),
                    threshold: Some(0),
                    suggestion: Some("Remove unreachable code or fix the control flow".to_string()),
                });
            }
        }

        // Variables defined but never used
        if let Some(var_name) = extract_variable_definition(trimmed) {
            if !is_variable_used(&lines, &var_name, i + 1) {
                smells.push(CodeSmell {
                    smell_type: SmellType::DeadCode,
                    severity: Severity::Info,
                    file_path: file_path.to_string(),
                    line_number: (i + 1) as u32,
                    symbol_name: var_name.clone(),
                    message: format!("Variable '{}' is defined but never used", var_name),
                    metric_value: Some(0),
                    threshold: Some(1),
                    suggestion: Some("Remove unused variable or use it".to_string()),
                });
            }
        }
    }

    smells
}

/// Detect lazy classes - classes that don't do enough
pub fn detect_lazy_classes(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let classes = extract_class_info(&lines);

    for (class_name, class_info) in &classes {
        let method_count = class_info.methods.len();
        let field_count = class_info.fields.len();
        let line_count = class_info.end_line - class_info.line_number + 1;

        // A lazy class has very few methods and fields
        if method_count <= config.max_lazy_methods && field_count <= config.max_lazy_fields {
            // Skip if it's just a small data holder (might be intentional)
            if method_count >= 1 && line_count >= 5 {
                smells.push(CodeSmell {
                    smell_type: SmellType::LazyClass,
                    severity: Severity::Info,
                    file_path: file_path.to_string(),
                    line_number: class_info.line_number,
                    symbol_name: class_name.clone(),
                    message: format!(
                        "Class '{}' doesn't do enough ({} methods, {} fields, {} lines)",
                        class_name, method_count, field_count, line_count
                    ),
                    metric_value: Some(method_count),
                    threshold: Some(config.max_lazy_methods),
                    suggestion: Some(
                        "Consider collapsing this class into its caller using Inline Class"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect speculative generality - unused abstractions
pub fn detect_speculative_generality(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Find abstract classes/traits with only one implementation
    let abstract_types = find_abstract_types(&lines);

    for (abstract_name, info) in &abstract_types {
        if info.implementations == 1 {
            smells.push(CodeSmell {
                smell_type: SmellType::SpeculativeGenerality,
                severity: Severity::Info,
                file_path: file_path.to_string(),
                line_number: info.line_number,
                symbol_name: abstract_name.clone(),
                message: format!(
                    "Abstract type '{}' has only one implementation - YAGNI violation",
                    abstract_name
                ),
                metric_value: Some(info.implementations),
                threshold: Some(2),
                suggestion: Some(
                    "Remove abstraction until it's actually needed. Don't speculate on future needs."
                        .to_string(),
                ),
            });
        }
    }

    // Find unused generic parameters
    let generic_types = find_generic_types(&lines);

    for (type_name, info) in &generic_types {
        if info.unused_params > 0 {
            smells.push(CodeSmell {
                smell_type: SmellType::SpeculativeGenerality,
                severity: Severity::Info,
                file_path: file_path.to_string(),
                line_number: info.line_number,
                symbol_name: type_name.clone(),
                message: format!(
                    "Type '{}' has {} unused generic parameters",
                    type_name, info.unused_params
                ),
                metric_value: Some(info.unused_params),
                threshold: Some(1),
                suggestion: Some(
                    "Remove unused type parameters. Only add complexity when needed.".to_string(),
                ),
            });
        }
    }

    // Find methods with only one caller (over-abstraction)
    let defined_functions = extract_defined_functions(&lines);
    let call_counts = count_function_calls(&lines);

    for (func_name, line_number) in &defined_functions {
        let calls = call_counts.get(func_name).unwrap_or(&0);
        if *calls == 1 && !is_entry_point(func_name) && !is_overridden_method(&lines, func_name) {
            smells.push(CodeSmell {
                smell_type: SmellType::SpeculativeGenerality,
                severity: Severity::Info,
                file_path: file_path.to_string(),
                line_number: *line_number,
                symbol_name: func_name.clone(),
                message: format!("Function '{}' is only called once - may be over-abstraction", func_name),
                metric_value: Some(*calls),
                threshold: Some(2),
                suggestion: Some(
                    "Consider inlining this function using Inline Method unless it improves readability"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

// Data structures

struct FunctionInfo {
    line_number: u32,
    tokens: Vec<String>,
}

struct ClassInfo {
    line_number: u32,
    end_line: u32,
    methods: Vec<String>,
    fields: Vec<String>,
}

struct BlockInfo {
    line_number: u32,
    length: usize,
    occurrences: usize,
}

struct AbstractTypeInfo {
    line_number: u32,
    implementations: usize,
}

struct GenericTypeInfo {
    line_number: u32,
    unused_params: usize,
}

// Helper functions

fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("//")
        || trimmed.starts_with("#")
        || trimmed.starts_with("/*")
        || trimmed.starts_with("*")
        || trimmed.starts_with("<!--")
}

fn looks_like_code(trimmed: &str) -> bool {
    // Remove comment markers and check if it looks like code
    let code = trimmed
        .trim_start_matches("//")
        .trim_start_matches("#")
        .trim_start_matches("*")
        .trim();

    if code.is_empty() {
        return false;
    }

    // Code indicators
    code.contains("fn ")
        || code.contains("def ")
        || code.contains("function ")
        || code.contains("let ")
        || code.contains("const ")
        || code.contains("var ")
        || code.contains("if ")
        || code.contains("for ")
        || code.contains("while ")
        || code.contains("return ")
        || code.ends_with(';')
        || code.ends_with('{')
        || code.ends_with('}')
}

fn extract_functions_with_tokens(lines: &[&str]) -> HashMap<String, FunctionInfo> {
    let mut functions: HashMap<String, FunctionInfo> = HashMap::new();
    let mut current_func: Option<String> = None;
    let mut func_tokens: Vec<String> = Vec::new();
    let mut brace_count = 0;
    let mut func_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_function_definition(trimmed) {
            let func_name = extract_function_name(trimmed);
            current_func = Some(func_name.clone());
            func_tokens = Vec::new();
            brace_count = 0;
            func_start = i;

            functions.insert(
                func_name,
                FunctionInfo {
                    line_number: (i + 1) as u32,
                    tokens: Vec::new(),
                },
            );
        }

        if let Some(ref func_name) = current_func {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Tokenize the line
            let tokens = tokenize_line(trimmed);
            func_tokens.extend(tokens);

            if brace_count == 0 && i > func_start {
                if let Some(info) = functions.get_mut(func_name) {
                    info.tokens = func_tokens.clone();
                }
                current_func = None;
            }
        }
    }

    functions
}

fn tokenize_line(line: &str) -> Vec<String> {
    // Remove string literals and comments
    let cleaned = remove_strings_and_comments(line);

    // Split on non-alphanumeric characters
    cleaned
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && !is_keyword(s))
        .map(|s| s.to_lowercase())
        .collect()
}

fn remove_strings_and_comments(line: &str) -> String {
    let mut result = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut in_comment = false;
    let chars: Vec<char> = line.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        if in_comment {
            continue;
        }

        if in_string {
            if c == string_char && i > 0 && chars[i - 1] != '\\' {
                in_string = false;
            }
            continue;
        }

        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            continue;
        }

        if c == '/' && i + 1 < chars.len() {
            if chars[i + 1] == '/' {
                in_comment = true;
                continue;
            }
        }

        result.push(c);
    }

    result
}

fn calculate_token_similarity(tokens1: &[String], tokens2: &[String]) -> f64 {
    if tokens1.is_empty() || tokens2.is_empty() {
        return 0.0;
    }

    let set1: HashSet<_> = tokens1.iter().cloned().collect();
    let set2: HashSet<_> = tokens2.iter().cloned().collect();

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

fn find_duplicate_blocks(lines: &[&str], min_lines: usize) -> Vec<BlockInfo> {
    let mut blocks: Vec<BlockInfo> = Vec::new();
    let mut seen_blocks: HashMap<Vec<&str>, usize> = HashMap::new();

    // Use sliding window to find duplicate blocks
    for window_size in min_lines..=(lines.len() / 2).min(20) {
        for i in 0..=(lines.len() - window_size) {
            let block: Vec<&str> = lines[i..i + window_size]
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !is_comment_line(l))
                .collect();

            if block.len() >= min_lines {
                if let Some(count) = seen_blocks.get_mut(&block) {
                    *count += 1;
                } else {
                    seen_blocks.insert(block.clone(), 1);
                }
            }
        }
    }

    // Convert to BlockInfo
    for (block, count) in seen_blocks {
        if count >= 2 {
            blocks.push(BlockInfo {
                line_number: 1, // Would need to track actual position
                length: block.len(),
                occurrences: count,
            });
        }
    }

    blocks
}

fn extract_class_info(lines: &[&str]) -> HashMap<String, ClassInfo> {
    let mut classes: HashMap<String, ClassInfo> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut brace_count = 0;
    let mut class_start = 0u32;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_class_definition(trimmed) {
            let class_name = extract_class_name(trimmed);
            current_class = Some(class_name.clone());
            class_start = (i + 1) as u32;
            brace_count = 0;

            classes.insert(
                class_name,
                ClassInfo {
                    line_number: class_start,
                    end_line: class_start,
                    methods: Vec::new(),
                    fields: Vec::new(),
                },
            );
        }

        if let Some(ref class_name) = current_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            let info = classes.get_mut(class_name).unwrap();
            info.end_line = (i + 1) as u32;

            if is_method_definition(trimmed) {
                info.methods.push(extract_method_name(trimmed));
            }

            if is_field_definition(trimmed) {
                info.fields.push(extract_field_name(trimmed));
            }

            if brace_count == 0 {
                current_class = None;
            }
        }
    }

    classes
}

fn is_getter_or_setter(method: &str) -> bool {
    let lower = method.to_lowercase();
    lower.starts_with("get_")
        || lower.starts_with("set_")
        || lower.starts_with("is_")
        || lower.starts_with("has_")
        || lower.starts_with("get")
        || lower.starts_with("set")
        || lower.starts_with("is")
        || lower.starts_with("has")
}

fn extract_defined_functions(lines: &[&str]) -> HashMap<String, u32> {
    let mut functions: HashMap<String, u32> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        if is_function_definition(line.trim()) {
            let name = extract_function_name(line.trim());
            functions.insert(name, (i + 1) as u32);
        }
    }

    functions
}

fn extract_all_function_calls(lines: &[&str]) -> HashSet<String> {
    let mut calls = HashSet::new();

    for line in lines {
        let call_names = extract_function_calls(line.trim());
        calls.extend(call_names);
    }

    calls
}

fn extract_function_calls(line: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let cleaned = remove_strings_and_comments(line);
    let chars: Vec<char> = cleaned.chars().collect();
    let mut current_ident = String::new();

    for i in 0..chars.len() {
        let c = chars[i];

        if c.is_alphanumeric() || c == '_' {
            current_ident.push(c);
        } else if c == '(' && !current_ident.is_empty() {
            if !is_keyword(&current_ident) {
                calls.push(current_ident.clone());
            }
            current_ident.clear();
        } else {
            current_ident.clear();
        }
    }

    calls
}

fn is_entry_point(func_name: &str) -> bool {
    let entry_points = [
        "main",
        "init",
        "start",
        "run",
        "handle",
        "process",
        "serve",
        "test",
        "tests",
        "setup",
        "teardown",
        "before_each",
        "after_each",
        "before_all",
        "after_all",
    ];
    entry_points.contains(&func_name.to_lowercase().as_str())
}

fn is_exit_statement(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("return ")
        || trimmed == "return"
        || trimmed.starts_with("throw ")
        || trimmed.starts_with("raise ")
        || trimmed == "break"
        || trimmed == "continue"
}

fn extract_variable_definition(line: &str) -> Option<String> {
    let trimmed = line.trim();

    // Rust: let name = ...
    if trimmed.starts_with("let ") {
        let rest = &trimmed[4..];
        let name = rest.split('=').next()?.trim();
        // Get the variable name (may have type annotation)
        let name_part = name.split(':').next()?;
        return Some(name_part.trim().to_string());
    }

    // JavaScript: const/let/var name = ...
    if trimmed.starts_with("const ") || trimmed.starts_with("var ") {
        let rest = trimmed.split_whitespace().nth(1)?;
        return Some(rest.trim_end_matches(':').to_string());
    }

    None
}

fn is_variable_used(lines: &[&str], var_name: &str, def_line: usize) -> bool {
    for (i, line) in lines.iter().enumerate() {
        if i == def_line - 1 {
            continue; // Skip definition line
        }

        // Check for variable usage (not just definition)
        if line.contains(var_name) && !is_variable_definition(line, var_name) {
            return true;
        }
    }
    false
}

/// Check if line is a variable definition
fn is_variable_definition(line: &str, var_name: &str) -> bool {
    let trimmed = line.trim();
    // Rust: let name = ... or let mut name = ...
    // JavaScript: const name = ... or var name = ... or let name = ...
    // Python: name = ...
    if trimmed.starts_with("let ") || trimmed.starts_with("const ") || trimmed.starts_with("var ") {
        return trimmed
            .split_whitespace()
            .nth(1)
            .map_or(false, |n| n.trim_end_matches(':') == var_name);
    }
    if trimmed.starts_with("let mut ") {
        return trimmed
            .split_whitespace()
            .nth(2)
            .map_or(false, |n| n.trim_end_matches(':') == var_name);
    }
    false
}

fn find_abstract_types(lines: &[&str]) -> HashMap<String, AbstractTypeInfo> {
    let mut types: HashMap<String, AbstractTypeInfo> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Rust traits
        if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
            let name = extract_trait_name(trimmed);
            types.insert(
                name,
                AbstractTypeInfo {
                    line_number: (i + 1) as u32,
                    implementations: 0,
                },
            );
        }

        // Abstract classes in other languages
        if trimmed.contains("abstract class ") {
            let name = extract_class_name(trimmed);
            types.insert(
                name,
                AbstractTypeInfo {
                    line_number: (i + 1) as u32,
                    implementations: 0,
                },
            );
        }

        // Count implementations
        if trimmed.starts_with("impl ") {
            for (trait_name, info) in types.iter_mut() {
                if trimmed.contains(&format!(" for {}", trait_name))
                    || trimmed.contains(&format!("impl {} ", trait_name))
                {
                    info.implementations += 1;
                }
            }
        }

        // extends/implements
        if trimmed.contains("extends ") || trimmed.contains("implements ") {
            for (trait_name, info) in types.iter_mut() {
                if trimmed.contains(trait_name) {
                    info.implementations += 1;
                }
            }
        }
    }

    types
}

fn find_generic_types(lines: &[&str]) -> HashMap<String, GenericTypeInfo> {
    let mut types: HashMap<String, GenericTypeInfo> = HashMap::new();

    // This is simplified - would need proper parsing for full implementation
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Look for generic definitions
        if trimmed.contains('<') && trimmed.contains('>') {
            if is_class_definition(trimmed) || is_function_definition(trimmed) {
                let name = if is_class_definition(trimmed) {
                    extract_class_name(trimmed)
                } else {
                    extract_function_name(trimmed)
                };

                // Count type parameters
                let type_params = count_type_parameters(trimmed);

                if type_params > 0 {
                    types.insert(
                        name,
                        GenericTypeInfo {
                            line_number: (i + 1) as u32,
                            unused_params: 0, // Would need deeper analysis
                        },
                    );
                }
            }
        }
    }

    types
}

fn count_type_parameters(line: &str) -> usize {
    if let Some(start) = line.find('<') {
        if let Some(end) = line.rfind('>') {
            let params = &line[start + 1..end];
            return params.split(',').filter(|p| !p.trim().is_empty()).count();
        }
    }
    0
}

fn count_function_calls(lines: &[&str]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();

    for line in lines {
        let calls = extract_function_calls(line.trim());
        for call in calls {
            *counts.entry(call).or_default() += 1;
        }
    }

    counts
}

fn is_overridden_method(_lines: &[&str], _func_name: &str) -> bool {
    // Simplified - would need proper analysis
    false
}

fn is_class_definition(trimmed: &str) -> bool {
    trimmed.starts_with("class ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("pub class ")
        || trimmed.starts_with("pub struct ")
}

fn is_function_definition(trimmed: &str) -> bool {
    (trimmed.starts_with("fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private "))
        && trimmed.contains('(')
}

fn is_method_definition(trimmed: &str) -> bool {
    is_function_definition(trimmed)
}

fn is_field_definition(trimmed: &str) -> bool {
    trimmed.contains(':')
        && !trimmed.contains("::")
        && !trimmed.contains('(')
        && !trimmed.contains("fn ")
}

fn extract_class_name(line: &str) -> String {
    for prefix in [
        "impl ",
        "struct ",
        "class ",
        "pub struct ",
        "pub class ",
        "pub trait ",
        "trait ",
        "abstract class ",
    ] {
        if line.starts_with(prefix) {
            let rest = &line[prefix.len()..];
            return rest
                .split(|c: char| c.is_whitespace() || c == '{' || c == '<' || c == '(')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string();
        }
    }
    "unknown".to_string()
}

fn extract_trait_name(line: &str) -> String {
    extract_class_name(line)
}

fn extract_function_name(line: &str) -> String {
    let mut line = line.trim();

    for prefix in [
        "pub ",
        "pub(crate) ",
        "pub(super) ",
        "private ",
        "protected ",
        "async ",
        "static ",
    ] {
        if line.starts_with(prefix) {
            line = &line[prefix.len()..];
        }
    }

    for keyword in ["fn ", "def ", "function "] {
        if line.starts_with(keyword) {
            line = &line[keyword.len()..];
            break;
        }
    }

    if let Some(paren_pos) = line.find('(') {
        line[..paren_pos].trim().to_string()
    } else {
        line.split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string()
    }
}

fn extract_method_name(line: &str) -> String {
    extract_function_name(line)
}

fn extract_field_name(line: &str) -> String {
    if let Some(colon_pos) = line.find(':') {
        line[..colon_pos].trim().to_string()
    } else {
        line.split_whitespace()
            .last()
            .unwrap_or("unknown")
            .trim_end_matches(';')
            .to_string()
    }
}

fn is_keyword(s: &str) -> bool {
    let keywords = [
        "if",
        "else",
        "for",
        "while",
        "match",
        "switch",
        "fn",
        "def",
        "function",
        "return",
        "let",
        "const",
        "var",
        "struct",
        "class",
        "impl",
        "pub",
        "private",
        "public",
        "async",
        "await",
        "try",
        "catch",
        "throw",
        "new",
        "typeof",
        "instanceof",
        "use",
        "import",
    ];
    keywords.contains(&s.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_getter_or_setter() {
        assert!(is_getter_or_setter("getName"));
        assert!(is_getter_or_setter("setName"));
        assert!(is_getter_or_setter("isValid"));
        assert!(is_getter_or_setter("hasPermission"));
        assert!(!is_getter_or_setter("processData"));
    }

    #[test]
    fn test_is_entry_point() {
        assert!(is_entry_point("main"));
        assert!(is_entry_point("init"));
        assert!(is_entry_point("test"));
        assert!(!is_entry_point("helper"));
    }

    #[test]
    fn test_is_exit_statement() {
        assert!(is_exit_statement("return"));
        assert!(is_exit_statement("return value;"));
        assert!(is_exit_statement("throw new Error();"));
        assert!(is_exit_statement("break"));
        assert!(!is_exit_statement("let x = 1;"));
    }

    #[test]
    fn test_tokenize_line() {
        let tokens = tokenize_line("let x = calculate(y) + 1;");
        assert!(!tokens.contains(&"let".to_string()));
        assert!(tokens.contains(&"calculate".to_string()));
    }
}
