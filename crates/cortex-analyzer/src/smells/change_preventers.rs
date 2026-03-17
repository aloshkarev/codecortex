//! Change Preventer code smells detection
//!
//! These smells make code hard to change and extend.
//!
//! Includes:
//! - Divergent Change
//! - Parallel Inheritance
//! - Shotgun Surgery

use super::language::{
    SourceLanguage, extract_function_name as shared_extract_function_name, is_function_signature,
    is_method_signature,
};
use crate::context::ProjectAnalysisContext;
use crate::{CodeSmell, Severity, SmellConfig, SmellType};
use std::collections::HashMap;

/// Detect divergent change - one class changed for many reasons
/// Note: This is also in oo_abusers.rs but duplicated here as it's
/// classified as a change preventer by refactoring.guru
pub fn detect_divergent_change(
    source: &str,
    file_path: &str,
    _config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let lang = SourceLanguage::from_file_path(file_path);

    let classes = extract_classes(&lines, lang);

    for (class_name, class_info) in &classes {
        if class_info.methods.len() < 5 {
            continue; // Too small to have divergent change
        }

        // Group methods by naming patterns (prefix/subject)
        let method_groups = group_methods_by_subject(&class_info.methods);

        // If there are multiple distinct groups with low overlap, it's divergent change
        if method_groups.len() >= 2 {
            let group_sizes: Vec<usize> = method_groups.values().map(|g| g.len()).collect();
            let total_methods: usize = group_sizes.iter().sum();
            let max_group = group_sizes.iter().max().unwrap_or(&0);

            // If the largest group is less than 50% of methods, we have divergence
            if (*max_group as f64) / (total_methods as f64) < 0.5 {
                smells.push(CodeSmell {
                    smell_type: SmellType::DivergentChange,
                    severity: Severity::Warning,
                    file_path: file_path.to_string(),
                    line_number: class_info.line_number,
                    symbol_name: class_name.clone(),
                    message: format!(
                        "Class '{}' has {} method groups suggesting multiple change reasons",
                        class_name, method_groups.len()
                    ),
                    metric_value: Some(method_groups.len()),
                    threshold: Some(2),
                    suggestion: Some(
                        "Split the class into separate classes, one for each responsibility using Extract Class"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect parallel inheritance - creating a subclass creates another
pub fn detect_parallel_inheritance(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let lang = SourceLanguage::from_file_path(file_path);

    // Find all inheritance hierarchies
    let hierarchies = extract_inheritance_hierarchies(&lines, lang);

    // Look for parallel naming patterns
    for (base1, children1) in &hierarchies {
        for (base2, children2) in &hierarchies {
            if base1 >= base2 {
                continue; // Avoid duplicate comparisons
            }

            // Check if children have parallel naming patterns
            let parallel_pairs = find_parallel_pairs(children1, children2);

            if parallel_pairs.len() >= config.min_parallel_pairs {
                smells.push(CodeSmell {
                    smell_type: SmellType::ParallelInheritance,
                    severity: if parallel_pairs.len() >= 3 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    file_path: file_path.to_string(),
                    line_number: 1, // General file-level smell
                    symbol_name: format!("{} / {}", base1, base2),
                    message: format!(
                        "Parallel inheritance hierarchies detected: '{}' and '{}' have {} parallel subclasses",
                        base1,
                        base2,
                        parallel_pairs.len()
                    ),
                    metric_value: Some(parallel_pairs.len()),
                    threshold: Some(config.min_parallel_pairs),
                    suggestion: Some(
                        "Apply Move Method and Move Field to make one hierarchy refer to instances of the other"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect shotgun surgery - one change requires many small changes
pub fn detect_shotgun_surgery(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let lang = SourceLanguage::from_file_path(file_path);

    // Find functions/methods and their dependencies
    let functions = extract_function_info(&lines, lang);

    // Track which functions are called from how many different files/contexts
    let mut call_counts: HashMap<String, usize> = HashMap::new();

    for (_func_name, func_info) in &functions {
        for called in &func_info.calls {
            *call_counts.entry(called.clone()).or_default() += 1;
        }
    }

    // Functions called from many places may indicate shotgun surgery
    for (called_func, callers) in &call_counts {
        if *callers >= config.min_shotgun_callers {
            // Find where this function is defined
            if let Some(func_info) = functions.get(called_func) {
                smells.push(CodeSmell {
                    smell_type: SmellType::ShotgunSurgery,
                    severity: if *callers >= config.min_shotgun_callers * 2 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    file_path: file_path.to_string(),
                    line_number: func_info.line_number,
                    symbol_name: called_func.clone(),
                    message: format!(
                        "Function '{}' is called from {} different places - changes may require shotgun surgery",
                        called_func, callers
                    ),
                    metric_value: Some(*callers),
                    threshold: Some(config.min_shotgun_callers),
                    suggestion: Some(
                        "Consider using Inline Method or Move Method to centralize the functionality"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect shotgun surgery using project-wide caller context.
pub fn detect_shotgun_surgery_with_context(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
    context: &ProjectAnalysisContext,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let lang = SourceLanguage::from_file_path(file_path);
    let functions = extract_function_info(&lines, lang);

    for (func_name, func_info) in &functions {
        let callers = context.symbols().caller_count(func_name);
        if callers >= config.min_shotgun_callers {
            smells.push(CodeSmell {
                smell_type: SmellType::ShotgunSurgery,
                severity: if callers >= config.min_shotgun_callers * 2 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                file_path: file_path.to_string(),
                line_number: func_info.line_number,
                symbol_name: func_name.clone(),
                message: format!(
                    "Function '{}' is called from {} different places across the project - changes may require shotgun surgery",
                    func_name, callers
                ),
                metric_value: Some(callers),
                threshold: Some(config.min_shotgun_callers),
                suggestion: Some(
                    "Consider using Inline Method or Move Method to centralize the functionality"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

// Data structures

struct ClassInfo {
    line_number: u32,
    methods: Vec<String>,
    fields: Vec<String>,
}

struct FunctionInfo {
    line_number: u32,
    calls: Vec<String>,
}

// Helper functions

fn extract_classes(lines: &[&str], lang: SourceLanguage) -> HashMap<String, ClassInfo> {
    let mut classes: HashMap<String, ClassInfo> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut brace_count = 0;
    let mut class_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_class_definition(trimmed, lang) {
            let class_name = extract_class_name(trimmed);
            current_class = Some(class_name.clone());
            class_start = i;
            brace_count = 0;

            classes.insert(
                class_name,
                ClassInfo {
                    line_number: (i + 1) as u32,
                    methods: Vec::new(),
                    fields: Vec::new(),
                },
            );
        }

        if let Some(ref class_name) = current_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            let info = classes.get_mut(class_name).unwrap();

            if is_method_definition(trimmed, lang) {
                info.methods.push(extract_method_name(trimmed));
            }

            if is_field_definition(trimmed) {
                info.fields.push(extract_field_name(trimmed));
            }

            if brace_count == 0 && i > class_start {
                current_class = None;
            }
        }
    }

    classes
}

fn extract_inheritance_hierarchies(
    lines: &[&str],
    lang: SourceLanguage,
) -> HashMap<String, Vec<String>> {
    let mut hierarchies: HashMap<String, Vec<String>> = HashMap::new();

    for line in lines {
        let trimmed = line.trim();

        // Look for inheritance patterns
        if let Some((child, parent)) = extract_inheritance_pair(trimmed, lang) {
            hierarchies.entry(parent).or_default().push(child);
        }
    }

    hierarchies
}

fn extract_function_info(lines: &[&str], lang: SourceLanguage) -> HashMap<String, FunctionInfo> {
    let mut functions: HashMap<String, FunctionInfo> = HashMap::new();
    let mut current_func: Option<String> = None;
    let mut func_start = 0;
    let mut brace_count = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_function_definition(trimmed, lang) {
            let func_name = extract_function_name(trimmed);
            current_func = Some(func_name.clone());
            func_start = i;
            brace_count = 0;

            functions.insert(
                func_name,
                FunctionInfo {
                    line_number: (i + 1) as u32,
                    calls: Vec::new(),
                },
            );
        }

        if let Some(ref func_name) = current_func {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Extract function calls
            let calls = extract_function_calls(trimmed);
            if let Some(info) = functions.get_mut(func_name) {
                info.calls.extend(calls);
            }

            if brace_count == 0 && i > func_start {
                current_func = None;
            }
        }
    }

    functions
}

fn group_methods_by_subject(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        // Extract subject from method name (e.g., "get" from "getUser", "save" from "saveUser")
        let subject = extract_method_subject(method);
        groups.entry(subject).or_default().push(method.clone());
    }

    // Filter out groups with only one method
    groups.retain(|_, v| v.len() > 1);

    groups
}

fn extract_method_subject(method: &str) -> String {
    // Common prefixes that indicate subject
    let prefixes = [
        "get", "set", "add", "remove", "delete", "update", "create", "find", "search", "load",
        "save", "validate", "process", "handle", "execute", "run", "init", "check", "is", "has",
        "can", "should", "will", "must", "on", "before", "after",
    ];

    let lower = method.to_lowercase();

    for prefix in prefixes {
        if lower.starts_with(prefix) {
            return prefix.to_string();
        }
    }

    // Try to extract first "word" from CamelCase
    let mut subject = String::new();
    for c in method.chars() {
        if c.is_uppercase() && !subject.is_empty() {
            break;
        }
        subject.push(c.to_lowercase().next().unwrap_or(c));
    }

    if subject.is_empty() {
        "other".to_string()
    } else {
        subject
    }
}

fn find_parallel_pairs(children1: &[String], children2: &[String]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for child1 in children1 {
        // Extract the "difference" part (e.g., "Button" from "HTMLButton")
        let suffix1 = extract_class_suffix(child1);

        for child2 in children2 {
            let suffix2 = extract_class_suffix(child2);

            // If suffixes match, we have a parallel pair
            if suffix1 == suffix2 && !suffix1.is_empty() {
                pairs.push((child1.clone(), child2.clone()));
            }
        }
    }

    pairs
}

fn extract_class_suffix(class_name: &str) -> String {
    // Try to find the distinguishing suffix
    // e.g., "HTMLButton" -> "Button", "WindowsButton" -> "Button"
    let common_suffixes = [
        "Button",
        "Window",
        "Dialog",
        "Factory",
        "Builder",
        "Handler",
        "Manager",
        "Service",
        "Repository",
        "Controller",
        "View",
        "Model",
        "Adapter",
        "Wrapper",
    ];

    for suffix in common_suffixes {
        if class_name.ends_with(suffix) {
            return suffix.to_string();
        }
    }

    // Fall back to extracting after last uppercase letter
    let mut last_upper = 0;
    for (i, c) in class_name.char_indices() {
        if c.is_uppercase() {
            last_upper = i;
        }
    }

    if last_upper > 0 {
        class_name[last_upper..].to_string()
    } else {
        class_name.to_string()
    }
}

fn is_class_definition(trimmed: &str, _lang: SourceLanguage) -> bool {
    trimmed.starts_with("class ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("pub class ")
        || trimmed.starts_with("pub struct ")
}

fn is_method_definition(trimmed: &str, lang: SourceLanguage) -> bool {
    is_method_signature(trimmed, lang)
}

fn is_field_definition(trimmed: &str) -> bool {
    trimmed.contains(':') && !trimmed.contains("::") && !trimmed.contains('(')
}

fn is_function_definition(trimmed: &str, lang: SourceLanguage) -> bool {
    is_function_signature(trimmed, lang)
}

fn extract_class_name(line: &str) -> String {
    let line = line.trim();

    for prefix in ["impl ", "struct ", "class ", "pub struct ", "pub class "] {
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

fn extract_method_name(line: &str) -> String {
    shared_extract_function_name(line)
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

fn extract_function_name(line: &str) -> String {
    shared_extract_function_name(line)
}

fn extract_function_calls(line: &str) -> Vec<String> {
    let mut calls = Vec::new();

    // Simple pattern: identifier followed by (
    let chars: Vec<char> = line.chars().collect();
    let mut current_ident = String::new();

    for i in 0..chars.len() {
        let c = chars[i];

        if c.is_alphanumeric() || c == '_' {
            current_ident.push(c);
        } else if c == '(' && !current_ident.is_empty() {
            // Check if this is likely a function call (not a keyword)
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

fn extract_inheritance_pair(line: &str, _lang: SourceLanguage) -> Option<(String, String)> {
    let trimmed = line.trim();

    // "class Child extends Parent"
    if trimmed.contains("extends ") {
        let parts: Vec<&str> = trimmed.split("extends").collect();
        if parts.len() == 2 {
            let child = parts[0]
                .strip_prefix("class ")
                .unwrap_or(parts[0])
                .trim()
                .to_string();
            let parent = parts[1]
                .trim()
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()?
                .to_string();
            if !child.is_empty() && !parent.is_empty() {
                return Some((child, parent));
            }
        }
    }

    // "class Child(Parent):" - Python
    if trimmed.starts_with("class ") && trimmed.contains('(') {
        let rest = &trimmed[6..];
        let paren_start = rest.find('(')?;
        let paren_end = rest.find(')')?;
        let child = rest[..paren_start].trim().to_string();
        let parent = rest[paren_start + 1..paren_end].trim().to_string();
        if !child.is_empty() && !parent.is_empty() {
            return Some((child, parent));
        }
    }

    None
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
    ];
    keywords.contains(&s.to_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectSymbolIndex;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_extract_method_subject() {
        assert_eq!(extract_method_subject("getUser"), "get");
        assert_eq!(extract_method_subject("saveUser"), "save");
        assert_eq!(extract_method_subject("isValid"), "is");
        assert_eq!(extract_method_subject("processData"), "process");
    }

    #[test]
    fn test_extract_class_suffix() {
        assert_eq!(extract_class_suffix("HTMLButton"), "Button");
        assert_eq!(extract_class_suffix("WindowsButton"), "Button");
        assert_eq!(extract_class_suffix("UserFactory"), "Factory");
    }

    #[test]
    fn test_extract_function_calls() {
        let line = "process(data); validate(input); save(result);";
        let calls = extract_function_calls(line);
        assert!(calls.contains(&"process".to_string()));
        assert!(calls.contains(&"validate".to_string()));
        assert!(calls.contains(&"save".to_string()));
    }

    #[test]
    fn test_shotgun_surgery_with_context() {
        let config = SmellConfig {
            min_shotgun_callers: 2,
            ..Default::default()
        };
        let source = r#"
fn target() {}
"#;

        let mut callers: HashMap<String, HashSet<String>> = HashMap::new();
        callers.insert(
            "target".to_string(),
            HashSet::from([String::from("a"), String::from("b"), String::from("c")]),
        );
        let context = crate::ProjectAnalysisContext::from_symbols_for_tests(ProjectSymbolIndex {
            callers,
            ..Default::default()
        });

        let smells = detect_shotgun_surgery_with_context(source, "src/lib.rs", &config, &context);
        assert!(smells.iter().any(|s| {
            s.smell_type == SmellType::ShotgunSurgery
                && s.symbol_name == "target"
                && s.metric_value == Some(3)
        }));
    }
}
