//! Coupler code smells detection
//!
//! These smells represent excessive coupling between modules,
//! making code hard to maintain and modify.
//!
//! Includes:
//! - Feature Envy
//! - Inappropriate Intimacy
//! - Message Chains
//! - Middle Man

use crate::{CodeSmell, Severity, SmellConfig, SmellType};
use std::collections::{HashMap, HashSet};

/// Detect feature envy - method uses other class more than its own
pub fn detect_feature_envy(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let methods = extract_methods_with_access_patterns(&lines);

    for (method_name, method_info) in &methods {
        let own_accesses = method_info.own_class_accesses;
        let other_accesses = method_info.other_class_accesses;

        if other_accesses == 0 {
            continue;
        }

        // Calculate envy ratio
        let total = own_accesses + other_accesses;
        let envy_ratio = other_accesses as f64 / total as f64;

        if envy_ratio > config.max_envy_ratio && other_accesses >= config.min_other_accesses {
            smells.push(CodeSmell {
                smell_type: SmellType::FeatureEnvy,
                severity: if envy_ratio > 0.8 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                file_path: file_path.to_string(),
                line_number: method_info.line_number,
                symbol_name: method_name.clone(),
                message: format!(
                    "Method '{}' accesses {} foreign members vs {} own ({}% envy)",
                    method_name,
                    other_accesses,
                    own_accesses,
                    (envy_ratio * 100.0) as u32
                ),
                metric_value: Some((envy_ratio * 100.0) as usize),
                threshold: Some((config.max_envy_ratio * 100.0) as usize),
                suggestion: Some(
                    "Move this method to the class it's most interested in using Move Method"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

/// Detect inappropriate intimacy - classes access each other's internals
pub fn detect_inappropriate_intimacy(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let classes = extract_class_intimacy_data(&lines);

    // Compare pairs of classes for bidirectional access
    let class_names: Vec<&String> = classes.keys().collect();
    for i in 0..class_names.len() {
        for j in (i + 1)..class_names.len() {
            let class1 = &classes[class_names[i]];
            let class2 = &classes[class_names[j]];

            // Check for bidirectional access
            let class1_accesses_2 = class1.accesses.get(class_names[j]).unwrap_or(&0);
            let class2_accesses_1 = class2.accesses.get(class_names[i]).unwrap_or(&0);

            let total_cross_access = class1_accesses_2 + class2_accesses_1;

            if total_cross_access >= config.min_intimacy_accesses {
                let is_bidirectional = *class1_accesses_2 > 0 && *class2_accesses_1 > 0;

                smells.push(CodeSmell {
                    smell_type: SmellType::InappropriateIntimacy,
                    severity: if is_bidirectional {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    file_path: file_path.to_string(),
                    line_number: class1.line_number.min(class2.line_number),
                    symbol_name: format!("{} <-> {}", class_names[i], class_names[j]),
                    message: format!(
                        "Classes '{}' and '{}' are too intimate ({} cross-accesses{})",
                        class_names[i],
                        class_names[j],
                        total_cross_access,
                        if is_bidirectional {
                            ", bidirectional"
                        } else {
                            ""
                        }
                    ),
                    metric_value: Some(total_cross_access),
                    threshold: Some(config.min_intimacy_accesses),
                    suggestion: Some(
                        "Reduce coupling: use Move Method, Extract Class, or Hide Delegate"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect message chains - long call chains like a.b().c().d()
pub fn detect_message_chains(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments and strings
        if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
            continue;
        }

        // Find all message chains in the line
        let chains = extract_message_chains(trimmed);

        for chain in chains {
            if chain.length >= config.max_message_chain_length {
                smells.push(CodeSmell {
                    smell_type: SmellType::MessageChains,
                    severity: if chain.length >= config.max_message_chain_length * 2 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    file_path: file_path.to_string(),
                    line_number: (i + 1) as u32,
                    symbol_name: chain.start_variable.clone(),
                    message: format!(
                        "Message chain of length {} found: {}...",
                        chain.length,
                        chain.preview
                    ),
                    metric_value: Some(chain.length),
                    threshold: Some(config.max_message_chain_length),
                    suggestion: Some(
                        "Use Hide Delegate to encapsulate the chain, or Extract Method for intermediate results"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect middle man - class that mostly delegates to another
pub fn detect_middle_man(source: &str, file_path: &str, config: &SmellConfig) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let classes = extract_class_delegation_data(&lines);

    for (class_name, class_info) in &classes {
        if class_info.total_methods == 0 {
            continue;
        }

        let delegate_ratio = class_info.delegating_methods as f64 / class_info.total_methods as f64;

        // Use a minimum of 3 delegating methods as threshold
        if delegate_ratio >= config.max_delegate_ratio && class_info.delegating_methods >= 3 {
            smells.push(CodeSmell {
                smell_type: SmellType::MiddleMan,
                severity: if delegate_ratio > 0.9 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                file_path: file_path.to_string(),
                line_number: class_info.line_number,
                symbol_name: class_name.clone(),
                message: format!(
                    "Class '{}' is a middle man: {} of {} methods just delegate",
                    class_name, class_info.delegating_methods, class_info.total_methods
                ),
                metric_value: Some((delegate_ratio * 100.0) as usize),
                threshold: Some((config.max_delegate_ratio * 100.0) as usize),
                suggestion: Some(
                    "Remove the middle man using Remove Middle Man, or add meaningful behavior"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

// Data structures

struct MethodInfo {
    line_number: u32,
    own_class_accesses: usize,
    other_class_accesses: usize,
    class_name: String,
}

struct ClassIntimacyData {
    line_number: u32,
    accesses: HashMap<String, usize>,
}

struct ClassDelegationData {
    line_number: u32,
    total_methods: usize,
    delegating_methods: usize,
    delegate_target: Option<String>,
}

struct MessageChain {
    start_variable: String,
    length: usize,
    preview: String,
}

// Helper functions

fn extract_methods_with_access_patterns(lines: &[&str]) -> HashMap<String, MethodInfo> {
    let mut methods: HashMap<String, MethodInfo> = HashMap::new();
    let mut current_method: Option<String> = None;
    let mut current_class: Option<String> = None;
    let mut method_info: Option<MethodInfo> = None;
    let mut brace_count = 0;
    let mut method_start = 0u32;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Track class
        if is_class_definition(trimmed) {
            current_class = Some(extract_class_name(trimmed));
        }

        // Track method start
        if is_method_definition(trimmed) {
            let method_name = extract_method_name(trimmed);
            current_method = Some(method_name.clone());
            method_info = Some(MethodInfo {
                line_number: (i + 1) as u32,
                own_class_accesses: 0,
                other_class_accesses: 0,
                class_name: current_class.clone().unwrap_or_default(),
            });
            brace_count = 0;
            method_start = (i + 1) as u32;
        }

        if current_method.is_some() {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Count accesses
            if let Some(ref mut info) = method_info {
                let accesses = extract_member_accesses(trimmed);
                for (var, is_self) in accesses {
                    if is_self {
                        info.own_class_accesses += 1;
                    } else {
                        info.other_class_accesses += 1;
                    }
                }
            }

            // Method end
            if brace_count == 0 && i as u32 >= method_start {
                if let (Some(method_name), Some(info)) = (current_method.take(), method_info.take())
                {
                    methods.insert(method_name, info);
                }
            }
        }
    }

    methods
}

fn extract_class_intimacy_data(lines: &[&str]) -> HashMap<String, ClassIntimacyData> {
    let mut classes: HashMap<String, ClassIntimacyData> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut brace_count = 0;
    let mut class_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_class_definition(trimmed) {
            let class_name = extract_class_name(trimmed);
            current_class = Some(class_name.clone());
            class_start = i;
            brace_count = 0;

            classes.insert(
                class_name,
                ClassIntimacyData {
                    line_number: (i + 1) as u32,
                    accesses: HashMap::new(),
                },
            );
        }

        if let Some(ref class_name) = current_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            // Track what other classes this class accesses
            if let Some(info) = classes.get_mut(class_name) {
                let foreign_accesses = extract_foreign_class_accesses(trimmed);
                for foreign_class in foreign_accesses {
                    if foreign_class != *class_name {
                        *info.accesses.entry(foreign_class).or_default() += 1;
                    }
                }
            }

            if brace_count == 0 && i > class_start {
                current_class = None;
            }
        }
    }

    classes
}

fn extract_class_delegation_data(lines: &[&str]) -> HashMap<String, ClassDelegationData> {
    let mut classes: HashMap<String, ClassDelegationData> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut brace_count = 0;
    let mut class_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if is_class_definition(trimmed) {
            let class_name = extract_class_name(trimmed);
            current_class = Some(class_name.clone());
            class_start = i;
            brace_count = 0;

            classes.insert(
                class_name,
                ClassDelegationData {
                    line_number: (i + 1) as u32,
                    total_methods: 0,
                    delegating_methods: 0,
                    delegate_target: None,
                },
            );
        }

        if let Some(ref class_name) = current_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            if let Some(info) = classes.get_mut(class_name) {
                if is_method_definition(trimmed) {
                    info.total_methods += 1;

                    // Check if this method is a delegation
                    if is_delegating_method(trimmed, lines, i) {
                        info.delegating_methods += 1;
                    }
                }
            }

            if brace_count == 0 && i > class_start {
                current_class = None;
            }
        }
    }

    classes
}

fn extract_message_chains(line: &str) -> Vec<MessageChain> {
    let mut chains = Vec::new();
    let cleaned = remove_strings_and_comments(line);

    // Find patterns like var.method().method2().method3()
    let mut current_chain = String::new();
    let mut chain_length = 0;
    let mut start_var = String::new();
    let mut in_chain = false;
    let chars: Vec<char> = cleaned.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        // Start of identifier
        if c.is_alphanumeric() || c == '_' {
            let mut ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                ident.push(chars[i]);
                i += 1;
            }

            // Check if followed by method call
            if i < chars.len() && chars[i] == '.' {
                if !in_chain {
                    start_var = ident.clone();
                    in_chain = true;
                }
                current_chain = ident;
                chain_length = 0;
            } else if i < chars.len() && chars[i] == '(' {
                // Method call - continue chain
                if in_chain {
                    chain_length += 1;
                }
            }
        } else if c == '.' && in_chain {
            // Continue chain
        } else if c == ')' && in_chain {
            // End of method call - check if chain continues
            if i + 1 < chars.len() && chars[i + 1] == '.' {
                chain_length += 1;
                i += 1; // Skip the dot
            } else {
                // Chain ends
                if chain_length >= 2 {
                    let preview_len = (i + 1).min(50);
                    chains.push(MessageChain {
                        start_variable: start_var.clone(),
                        length: chain_length,
                        preview: cleaned[..preview_len].to_string() + "...",
                    });
                }
                in_chain = false;
                chain_length = 0;
            }
        } else if !c.is_whitespace() {
            in_chain = false;
            chain_length = 0;
        }

        i += 1;
    }

    // Check for chain at end of line
    if chain_length >= 2 {
        let preview_len = line.len().min(50);
        chains.push(MessageChain {
            start_variable: start_var,
            length: chain_length,
            preview: cleaned[..preview_len].to_string() + "...",
        });
    }

    chains
}

fn extract_member_accesses(line: &str) -> Vec<(String, bool)> {
    let mut accesses = Vec::new();
    let cleaned = remove_strings_and_comments(line);

    // Find patterns like self.field, this.field, other.field
    let patterns = ["self.", "this.", "&self.", "&mut self."];

    for pattern in patterns {
        let mut pos = 0;
        while let Some(start) = cleaned[pos..].find(pattern) {
            let rest = &cleaned[pos + start + pattern.len()..];
            if let Some(end) = rest.find(|c: char| !c.is_alphanumeric() && c != '_') {
                let field = &rest[..end];
                if !field.is_empty() {
                    accesses.push((format!("self.{}", field), true));
                }
            }
            pos += start + pattern.len();
        }
    }

    // Find other object accesses (simplified)
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Look for identifier.identifier patterns
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let mut first_ident = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                first_ident.push(chars[i]);
                i += 1;
            }

            if i < chars.len() && chars[i] == '.' && first_ident != "self" && first_ident != "this"
            {
                i += 1; // Skip dot
                if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                    let mut second_ident = String::new();
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        second_ident.push(chars[i]);
                        i += 1;
                    }
                    if !second_ident.is_empty() {
                        accesses.push((format!("{}.{}", first_ident, second_ident), false));
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    accesses
}

fn extract_foreign_class_accesses(line: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let cleaned = remove_strings_and_comments(line);

    // Find type names and foreign object instantiations
    let patterns = [
        ("new ", "("),
        ("::new(", ")"),
        (".getInstance()", ""),
        (".create(", ")"),
    ];

    for (start_pattern, end_pattern) in patterns {
        if let Some(start) = cleaned.find(start_pattern) {
            let before = &cleaned[..start];
            // Get the class name before the pattern
            if let Some(class_name) = before
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .last()
            {
                if !class_name.is_empty() && class_name != "self" && class_name != "this" {
                    classes.push(class_name.to_string());
                }
            }
        }
    }

    // Also check for type annotations
    for type_pattern in ["&mut ", "& ", ": "] {
        let mut pos = 0;
        while let Some(start) = cleaned[pos..].find(type_pattern) {
            let rest = &cleaned[pos + start + type_pattern.len()..];
            if let Some(type_name) = rest
                .split(|c: char| c.is_whitespace() || c == ',' || c == ')')
                .next()
            {
                if !type_name.is_empty() && type_name.chars().next().unwrap_or('_').is_uppercase() {
                    classes.push(type_name.to_string());
                }
            }
            pos += start + type_pattern.len();
        }
    }

    classes
}

fn is_delegating_method(method_line: &str, lines: &[&str], method_idx: usize) -> bool {
    if !is_method_definition(method_line.trim()) {
        return false;
    }

    // A delegating method should contain exactly one executable statement
    // that forwards to another object's method.
    let mut statements: Vec<String> = Vec::new();
    let mut brace_count = 0;
    let mut started = false;

    for (idx, line) in lines.iter().enumerate().skip(method_idx) {
        let trimmed = line.trim();
        brace_count += trimmed.matches('{').count() as i32;
        brace_count -= trimmed.matches('}').count() as i32;

        if idx == method_idx {
            started = trimmed.contains('{');
            continue;
        }

        if started {
            let statement = trimmed.trim_matches(|c| c == '{' || c == '}').trim();
            if !statement.is_empty() {
                statements.push(statement.to_string());
            }
        }

        if started && brace_count == 0 {
            break;
        }
    }

    if statements.len() != 1 {
        return false;
    }

    let statement = statements[0].trim_end_matches(';').trim();
    let forwarded = statement
        .strip_prefix("return ")
        .unwrap_or(statement)
        .trim();

    if forwarded.starts_with("if ")
        || forwarded.starts_with("for ")
        || forwarded.starts_with("while ")
        || forwarded.starts_with("match ")
    {
        return false;
    }

    forwarded.contains('.')
        && forwarded.contains('(')
        && forwarded.contains(')')
        && !forwarded.contains(" if ")
        && !forwarded.contains(" for ")
        && !forwarded.contains(" while ")
        && !forwarded.contains(" match ")
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

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            in_comment = true;
            continue;
        }

        result.push(c);
    }

    result
}

fn is_class_definition(trimmed: &str) -> bool {
    trimmed.starts_with("class ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("pub class ")
        || trimmed.starts_with("pub struct ")
}

fn is_method_definition(trimmed: &str) -> bool {
    (trimmed.starts_with("fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private "))
        && trimmed.contains('(')
}

fn extract_class_name(line: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_message_chains() {
        let line = "let result = obj.get_service().get_config().get_value().parse();";
        let chains = extract_message_chains(line);
        assert!(!chains.is_empty());
        assert!(chains[0].length >= 3);
    }

    #[test]
    fn test_extract_member_accesses() {
        let accesses = extract_member_accesses("let x = self.field + other.field;");
        assert!(
            accesses
                .iter()
                .any(|(name, is_self)| *is_self && name == "self.field")
        );
        assert!(
            accesses
                .iter()
                .any(|(name, is_self)| !*is_self && name == "other.field")
        );
    }

    #[test]
    fn test_remove_strings_and_comments() {
        let cleaned = remove_strings_and_comments("let s = \"hello.world\"; // comment");
        assert!(!cleaned.contains("hello.world"));
        assert!(!cleaned.contains("comment"));
    }

    #[test]
    fn test_is_delegating_method() {
        let lines = vec![
            "fn get_name(&self) -> String {",
            "    self.inner.get_name()",
            "}",
        ];
        assert!(is_delegating_method(lines[0], &lines, 0));
    }
}
