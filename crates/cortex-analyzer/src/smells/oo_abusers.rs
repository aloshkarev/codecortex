//! Object-Oriented Abuser code smells detection
//!
//! These smells represent incomplete or incorrect application of
//! object-oriented programming principles.
//!
//! Includes:
//! - Alternative Classes with Different Interfaces
//! - Refused Bequest
//! - Temporary Field
//! - Divergent Change

use crate::{CodeSmell, Severity, SmellConfig, SmellType};
use std::collections::{HashMap, HashSet};

/// Detect alternative classes - classes that could be combined
/// but have different interfaces
pub fn detect_alternative_classes(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Extract all classes and their method signatures
    let classes = extract_class_signatures(&lines);

    // Compare each pair of classes for similarity
    let class_names: Vec<&String> = classes.keys().collect();
    for i in 0..class_names.len() {
        for j in (i + 1)..class_names.len() {
            let class1 = &classes[class_names[i]];
            let class2 = &classes[class_names[j]];

            // Calculate method signature similarity
            let similarity = calculate_signature_similarity(&class1.methods, &class2.methods);

            if similarity >= config.min_class_similarity && similarity < 1.0 {
                // High similarity but different classes = alternative classes
                let common_methods: Vec<_> = class1.methods.intersection(&class2.methods).collect();

                if common_methods.len() >= 3 {
                    smells.push(CodeSmell {
                        smell_type: SmellType::AlternativeClasses,
                        severity: Severity::Warning,
                        file_path: file_path.to_string(),
                        line_number: class1.line_number.min(class2.line_number),
                        symbol_name: format!("{} / {}", class_names[i], class_names[j]),
                        message: format!(
                            "Classes '{}' and '{}' have similar interfaces ({}% overlap) but different implementations",
                            class_names[i],
                            class_names[j],
                            (similarity * 100.0) as u32
                        ),
                        metric_value: Some((similarity * 100.0) as usize),
                        threshold: Some((config.min_class_similarity * 100.0) as usize),
                        suggestion: Some(
                            "Consider unifying the interfaces using Extract Interface or making them inherit from the same base class"
                                .to_string(),
                        ),
                    });
                }
            }
        }
    }

    smells
}

/// Detect refused bequest - subclass doesn't use inherited functionality
pub fn detect_refused_bequest(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Find inheritance relationships
    let inheritances = find_inheritance_relationships(&lines);

    for inheritance in inheritances {
        // Check if subclass overrides most parent methods
        let subclass_methods = extract_class_methods(&lines, &inheritance.subclass);
        let parent_methods = extract_class_methods(&lines, &inheritance.parent);

        if parent_methods.is_empty() {
            continue;
        }

        let overridden: HashSet<_> = subclass_methods
            .intersection(&parent_methods)
            .cloned()
            .collect();
        let override_ratio = overridden.len() as f64 / parent_methods.len() as f64;

        // If most methods are overridden or ignored, it's refused bequest
        if override_ratio < config.min_bequest_usage {
            let unused_count = parent_methods.len() - overridden.len();

            smells.push(CodeSmell {
                smell_type: SmellType::RefusedBequest,
                severity: if override_ratio < 0.2 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                file_path: file_path.to_string(),
                line_number: inheritance.line_number,
                symbol_name: inheritance.subclass.clone(),
                message: format!(
                    "Class '{}' inherits from '{}' but only uses {}% of inherited methods",
                    inheritance.subclass,
                    inheritance.parent,
                    (override_ratio * 100.0) as u32
                ),
                metric_value: Some(unused_count),
                threshold: Some((config.min_bequest_usage * 100.0) as usize),
                suggestion: Some(
                    "Replace inheritance with delegation using Replace Inheritance with Delegation, or simplify the hierarchy"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

/// Detect temporary fields - fields only used in certain contexts
pub fn detect_temporary_fields(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Find all classes and their fields
    let classes = extract_class_signatures(&lines);

    for (class_name, class_info) in &classes {
        // Find field usages across methods
        for field in &class_info.fields {
            let usage_count = count_field_usages(&lines, field, class_name);

            // If a field is rarely used, it's a temporary field
            let total_methods = class_info.methods.len().max(1);
            let usage_ratio = usage_count as f64 / total_methods as f64;

            if usage_ratio < config.min_field_usage_ratio && usage_count > 0 {
                smells.push(CodeSmell {
                    smell_type: SmellType::TemporaryField,
                    severity: if usage_ratio < 0.1 {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    file_path: file_path.to_string(),
                    line_number: class_info.line_number,
                    symbol_name: format!("{}.{}", class_name, field),
                    message: format!(
                        "Field '{}' in class '{}' is only used in {}% of methods",
                        field,
                        class_name,
                        (usage_ratio * 100.0) as u32
                    ),
                    metric_value: Some(usage_count),
                    threshold: Some((config.min_field_usage_ratio * 100.0) as usize),
                    suggestion: Some(
                        "Move this field to a separate object using Extract Class, or pass it as a parameter"
                            .to_string(),
                    ),
                });
            }
        }
    }

    smells
}

/// Detect divergent change - one class changed for many different reasons
#[allow(dead_code)]
pub fn detect_divergent_change(
    source: &str,
    file_path: &str,
    config: &SmellConfig,
) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let classes = extract_class_signatures(&lines);

    for (class_name, class_info) in &classes {
        // Analyze method cohesion
        let cohesion = calculate_method_cohesion(&class_info.method_groups);

        // Low cohesion = high divergent change potential
        if cohesion < config.min_method_cohesion && class_info.methods.len() > 5 {
            smells.push(CodeSmell {
                smell_type: SmellType::DivergentChange,
                severity: if cohesion < 0.3 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                file_path: file_path.to_string(),
                line_number: class_info.line_number,
                symbol_name: class_name.clone(),
                message: format!(
                    "Class '{}' has low method cohesion ({:.0}%), suggesting it changes for multiple reasons",
                    class_name,
                    cohesion * 100.0
                ),
                metric_value: Some((cohesion * 100.0) as usize),
                threshold: Some((config.min_method_cohesion * 100.0) as usize),
                suggestion: Some(
                    "Split the class into multiple classes by responsibility using Extract Class"
                        .to_string(),
                ),
            });
        }
    }

    smells
}

// Data structures and helper functions

struct ClassInfo {
    line_number: u32,
    methods: HashSet<String>,
    fields: HashSet<String>,
    method_groups: Vec<HashSet<String>>, // Groups of related methods
}

struct InheritanceRelation {
    subclass: String,
    parent: String,
    line_number: u32,
}

fn group_methods_by_prefix(methods: &HashSet<String>) -> Vec<HashSet<String>> {
    let prefixes = [
        "get", "set", "add", "remove", "delete", "update", "create", "find", "search", "load",
        "save", "validate", "process", "handle", "execute", "run", "init", "check", "is", "has",
        "can", "should", "on", "before", "after",
    ];
    let mut groups_map: HashMap<String, HashSet<String>> = HashMap::new();
    for method in methods {
        let lower = method.to_lowercase();
        let key = prefixes
            .iter()
            .find(|&&p| lower.starts_with(p))
            .map(|&p| p.to_string())
            .unwrap_or_else(|| "other".to_string());
        groups_map.entry(key).or_default().insert(method.clone());
    }
    groups_map.into_values().filter(|g| g.len() > 1).collect()
}

fn extract_class_signatures(lines: &[&str]) -> HashMap<String, ClassInfo> {
    let mut classes: HashMap<String, ClassInfo> = HashMap::new();
    let mut current_class: Option<String> = None;
    let mut brace_count = 0;
    let mut class_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Detect class/struct start
        if trimmed.starts_with("class ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("pub class ")
            || trimmed.starts_with("pub struct ")
        {
            let class_name = extract_class_name_from_line(trimmed);
            current_class = Some(class_name.clone());
            class_start = i;
            brace_count = 0;

            classes.insert(
                class_name,
                ClassInfo {
                    line_number: (i + 1) as u32,
                    methods: HashSet::new(),
                    fields: HashSet::new(),
                    method_groups: Vec::new(),
                },
            );
        }

        if let Some(ref class_name) = current_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            let info = classes.get_mut(class_name).unwrap();

            // Extract method names
            if is_method_line(trimmed) {
                let method_name = extract_method_name(trimmed);
                info.methods.insert(method_name);
            }

            // Extract field names
            if is_field_line(trimmed) {
                let field_name = extract_field_name(trimmed);
                info.fields.insert(field_name);
            }

            // Check for class end
            if brace_count == 0 && i > class_start {
                // Compute method groups from collected methods
                let groups = group_methods_by_prefix(&info.methods);
                info.method_groups = groups;
                current_class = None;
            }
        }
    }

    classes
}

fn extract_class_name_from_line(line: &str) -> String {
    let line = line.trim();

    // Handle impl blocks
    if line.starts_with("impl ") {
        let rest = &line[5..];
        return rest
            .split(|c: char| c.is_whitespace() || c == '{' || c == '<' || c == '>')
            .find(|s| !s.is_empty())
            .unwrap_or("unknown")
            .to_string();
    }

    // Handle struct/class definitions
    let keywords = ["struct ", "class "];
    for keyword in keywords {
        let full_keyword = if line.starts_with("pub ") {
            format!("pub {}", keyword)
        } else {
            keyword.to_string()
        };

        if line.starts_with(&full_keyword) {
            let rest = &line[full_keyword.len()..];
            return rest
                .split(|c: char| c.is_whitespace() || c == '{' || c == '<' || c == '(')
                .find(|s| !s.is_empty())
                .unwrap_or("unknown")
                .to_string();
        }
    }

    "unknown".to_string()
}

fn extract_class_methods(lines: &[&str], class_name: &str) -> HashSet<String> {
    let mut methods = HashSet::new();
    let mut in_class = false;
    let mut _target_class = String::new();
    let mut brace_count = 0;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.contains("class ") || trimmed.contains("struct ") || trimmed.contains("impl ") {
            if trimmed.contains(class_name) {
                in_class = true;
                _target_class = class_name.to_string();
                brace_count = 0;
            }
        }

        if in_class {
            brace_count += trimmed.matches('{').count() as i32;
            brace_count -= trimmed.matches('}').count() as i32;

            if is_method_line(trimmed) {
                methods.insert(extract_method_name(trimmed));
            }

            if brace_count == 0 && !trimmed.is_empty() {
                in_class = false;
            }
        }
    }

    methods
}

fn find_inheritance_relationships(lines: &[&str]) -> Vec<InheritanceRelation> {
    let mut relations = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Rust: struct Child(Parent); or struct Child { ... }
        // Java/C#: class Child extends Parent, class Child : Parent
        // Python: class Child(Parent):

        // Handle various inheritance patterns
        if trimmed.contains("extends ") {
            if let Some((child, parent)) = extract_extends_relation(trimmed) {
                relations.push(InheritanceRelation {
                    subclass: child,
                    parent,
                    line_number: (i + 1) as u32,
                });
            }
        }

        // Python style
        if trimmed.starts_with("class ") && trimmed.contains('(') && !trimmed.contains("():") {
            if let Some((child, parent)) = extract_python_inheritance(trimmed) {
                relations.push(InheritanceRelation {
                    subclass: child,
                    parent,
                    line_number: (i + 1) as u32,
                });
            }
        }

        // C# style with colon
        if trimmed.starts_with("class ") && trimmed.contains(':') && !trimmed.contains("::") {
            if let Some((child, parent)) = extract_csharp_inheritance(trimmed) {
                relations.push(InheritanceRelation {
                    subclass: child,
                    parent,
                    line_number: (i + 1) as u32,
                });
            }
        }
    }

    relations
}

fn extract_extends_relation(line: &str) -> Option<(String, String)> {
    // "class Child extends Parent"
    let parts: Vec<&str> = line.split("extends").collect();
    if parts.len() != 2 {
        return None;
    }

    let child = parts[0]
        .trim()
        .strip_prefix("class ")
        .unwrap_or("")
        .trim()
        .to_string();

    let parent = parts[1]
        .trim()
        .split(|c: char| c.is_whitespace() || c == '{')
        .next()
        .unwrap_or("")
        .to_string();

    if child.is_empty() || parent.is_empty() {
        return None;
    }

    Some((child, parent))
}

fn extract_python_inheritance(line: &str) -> Option<(String, String)> {
    // "class Child(Parent):"
    let line = line.trim().strip_prefix("class ")?;
    let paren_start = line.find('(')?;
    let paren_end = line.find(')')?;

    let child = line[..paren_start].trim().to_string();
    let parent = line[paren_start + 1..paren_end].trim().to_string();

    if child.is_empty() || parent.is_empty() {
        return None;
    }

    Some((child, parent))
}

fn extract_csharp_inheritance(line: &str) -> Option<(String, String)> {
    // "class Child : Parent"
    let line = line.trim().strip_prefix("class ")?;
    let colon_pos = line.find(':')?;

    let child = line[..colon_pos].trim().to_string();
    let parent = line[colon_pos + 1..]
        .trim()
        .split_whitespace()
        .next()?
        .to_string();

    if child.is_empty() || parent.is_empty() {
        return None;
    }

    Some((child, parent))
}

fn is_method_line(trimmed: &str) -> bool {
    (trimmed.starts_with("fn ")
        || trimmed.starts_with("def ")
        || trimmed.starts_with("function ")
        || trimmed.starts_with("public ")
        || trimmed.starts_with("private ")
        || trimmed.starts_with("protected "))
        && trimmed.contains('(')
}

fn is_field_line(trimmed: &str) -> bool {
    // Rust: field: Type,
    // Java: private Type field;
    if trimmed.contains(':') && !trimmed.contains("::") && !trimmed.contains('(') {
        return true;
    }
    if trimmed.starts_with("private ") || trimmed.starts_with("protected ") {
        return !trimmed.contains('(');
    }
    false
}

fn extract_method_name(line: &str) -> String {
    let mut line = line.trim();

    // Strip prefixes
    let prefixes = [
        "pub ",
        "pub(crate) ",
        "pub(super) ",
        "private ",
        "protected ",
        "async ",
        "static ",
    ];
    for prefix in prefixes {
        if line.starts_with(prefix) {
            line = &line[prefix.len()..];
        }
    }

    // Strip fn/def keywords
    for keyword in ["fn ", "def ", "function "] {
        if line.starts_with(keyword) {
            line = &line[keyword.len()..];
            break;
        }
    }

    // Get name before (
    if let Some(paren_pos) = line.find('(') {
        line[..paren_pos].trim().to_string()
    } else {
        line.split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string()
    }
}

fn extract_field_name(line: &str) -> String {
    let line = line.trim();

    // Rust: field: Type
    if let Some(colon_pos) = line.find(':') {
        return line[..colon_pos].trim().to_string();
    }

    // Java: private Type field;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        let last = parts.last().unwrap();
        return last.trim_end_matches(';').to_string();
    }

    "unknown".to_string()
}

fn calculate_signature_similarity(methods1: &HashSet<String>, methods2: &HashSet<String>) -> f64 {
    if methods1.is_empty() && methods2.is_empty() {
        return 1.0;
    }
    if methods1.is_empty() || methods2.is_empty() {
        return 0.0;
    }

    let intersection = methods1.intersection(methods2).count();
    let union = methods1.union(methods2).count();

    intersection as f64 / union as f64
}

fn count_field_usages(lines: &[&str], field: &str, _class_name: &str) -> usize {
    let mut count = 0;

    for line in lines {
        let trimmed = line.trim();

        // Skip field definition itself
        if is_field_line(trimmed) && trimmed.contains(field) {
            continue;
        }

        // Count usages of self.field or this.field or just field
        let patterns = [
            format!("self.{}", field),
            format!("this.{}", field),
            format!("&self.{}", field),
            format!("&mut self.{}", field),
        ];

        if patterns.iter().any(|p| line.contains(p)) {
            count += 1;
        } else if !trimmed.starts_with("//") && line.contains(&format!(" {}", field)) {
            // Simple field access without self/this
            count += 1;
        }
    }

    count
}

#[allow(dead_code)]
fn calculate_method_cohesion(method_groups: &[HashSet<String>]) -> f64 {
    if method_groups.is_empty() {
        return 1.0;
    }

    // Simplified LCOM calculation
    // In a cohesive class, all methods should share some commonality
    if method_groups.len() == 1 {
        return 1.0;
    }

    // Check overlap between method groups
    let mut total_overlap = 0;
    let mut comparisons = 0;

    for i in 0..method_groups.len() {
        for j in (i + 1)..method_groups.len() {
            let intersection = method_groups[i].intersection(&method_groups[j]).count();
            let union = method_groups[i].union(&method_groups[j]).count();
            if union > 0 {
                total_overlap += intersection;
                comparisons += 1;
            }
        }
    }

    if comparisons == 0 {
        return 0.0;
    }

    total_overlap as f64 / comparisons as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_alternative_classes() {
        let config = SmellConfig::default();
        let source = r#"
struct UserService {
    fn get_user() {}
    fn save_user() {}
    fn delete_user() {}
}

struct AdminService {
    fn get_user() {}
    fn save_user() {}
    fn delete_user() {}
}
"#;

        let smells = detect_alternative_classes(source, "test.rs", &config);
        // May or may not detect depending on similarity threshold
        assert!(smells.len() <= 2);
    }

    #[test]
    fn test_extract_class_name() {
        assert_eq!(
            extract_class_name_from_line("struct MyStruct {"),
            "MyStruct"
        );
        assert_eq!(
            extract_class_name_from_line("pub struct MyStruct {"),
            "MyStruct"
        );
        assert_eq!(extract_class_name_from_line("impl MyStruct {"), "MyStruct");
    }

    #[test]
    fn test_extract_method_name() {
        assert_eq!(extract_method_name("fn my_method() {"), "my_method");
        assert_eq!(
            extract_method_name("pub fn my_method(x: i32) {"),
            "my_method"
        );
        assert_eq!(extract_method_name("def my_method(self):"), "my_method");
    }

    #[test]
    fn test_calculate_signature_similarity() {
        let mut m1: HashSet<String> = HashSet::new();
        m1.insert("get".to_string());
        m1.insert("save".to_string());

        let mut m2: HashSet<String> = HashSet::new();
        m2.insert("get".to_string());
        m2.insert("save".to_string());
        m2.insert("delete".to_string());

        let similarity = calculate_signature_similarity(&m1, &m2);
        assert!(similarity > 0.5 && similarity < 1.0);
    }

    #[test]
    fn test_detect_divergent_change_with_mixed_responsibilities() {
        let config = SmellConfig::default();
        // Class with 6+ methods across different prefixes (get, set, save, load) - each prefix
        // has 2+ methods so they form groups; multiple groups = low cohesion
        let source = r#"
struct BadClass {
    fn get_user() {}
    fn get_config() {}
    fn set_user() {}
    fn set_config() {}
    fn save_config() {}
    fn load_config() {}
}
"#;

        let smells = detect_divergent_change(source, "test.rs", &config);
        assert!(
            !smells.is_empty(),
            "DivergentChange should be detected for class with mixed responsibilities"
        );
        assert!(
            smells
                .iter()
                .any(|s| matches!(s.smell_type, SmellType::DivergentChange))
        );
    }
}
