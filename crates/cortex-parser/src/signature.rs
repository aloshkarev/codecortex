//! Enhanced Signature Extraction for AI Productivity
//!
//! Extracts rich type information from code for better AI understanding:
//! - Function signatures with parameter and return types
//! - Visibility modifiers
//! - Async/generics information
//! - Struct fields and enum variants

use cortex_core::{CodeNode, Language};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rich signature information for a code symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// The full signature text
    pub signature: String,
    /// Visibility level
    pub visibility: Visibility,
    /// Parameters (name, type)
    pub parameters: Vec<Parameter>,
    /// Return type (if any)
    pub return_type: Option<String>,
    /// Whether this is async
    pub is_async: bool,
    /// Generic type parameters
    pub generics: Vec<String>,
    /// Whether this is a method (has self parameter)
    pub is_method: bool,
    /// Self type (if method)
    pub self_type: Option<SelfType>,
    /// Additional modifiers
    pub modifiers: Vec<String>,
}

/// Parameter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
}

/// Visibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,
    Package,
    FilePrivate,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "pub"),
            Visibility::Protected => write!(f, "protected"),
            Visibility::Private => write!(f, "private"),
            Visibility::Internal => write!(f, "internal"),
            Visibility::Package => write!(f, "package"),
            Visibility::FilePrivate => write!(f, "file_private"),
        }
    }
}

/// Self parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfType {
    Value,  // self
    Ref,    // &self
    MutRef, // &mut self
    Owned,  // Box<self> etc
}

/// Signature extractor for different languages
pub struct SignatureExtractor;

impl SignatureExtractor {
    /// Extract signature from a code node
    pub fn extract(node: &CodeNode, source: &str) -> Option<Signature> {
        match node.lang.as_ref()? {
            Language::Rust => Self::extract_rust(node, source),
            Language::Python => Self::extract_python(node, source),
            Language::TypeScript | Language::JavaScript => Self::extract_typescript(node, source),
            Language::Go => Self::extract_go(node, source),
            Language::C | Language::Cpp => Self::extract_cpp(node, source),
            Language::Java => Self::extract_java(node, source),
            Language::Php => Self::extract_php(node, source),
            Language::Ruby => Self::extract_ruby(node, source),
        }
    }

    /// Extract Rust signature
    fn extract_rust(node: &CodeNode, source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");
        let line = source
            .lines()
            .nth(node.line_number?.saturating_sub(1) as usize)?;

        // Parse visibility
        let visibility = if line.contains("pub(crate)") || line.contains("pub(super)") {
            Visibility::Internal
        } else if line.contains("pub") {
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Check async
        let is_async = line.contains("async");

        // Extract generics
        let generics = Self::extract_generics_rust(full_source);

        // Extract parameters
        let (parameters, is_method, self_type) = Self::extract_params_rust(full_source);

        // Extract return type
        let return_type = Self::extract_return_type_rust(full_source);

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async,
            generics,
            is_method,
            self_type,
            modifiers: if is_async {
                vec!["async".to_string()]
            } else {
                vec![]
            },
        })
    }

    fn extract_generics_rust(source: &str) -> Vec<String> {
        let mut generics = Vec::new();
        if let Some(start) = source.find('<')
            && let Some(end) = source.find('>')
            && start < end
        {
            let generic_str = &source[start + 1..end];
            for part in generic_str.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() && !trimmed.contains(':') {
                    generics.push(trimmed.to_string());
                }
            }
        }
        generics
    }

    fn extract_params_rust(source: &str) -> (Vec<Parameter>, bool, Option<SelfType>) {
        let mut parameters = Vec::new();
        let mut is_method = false;
        let mut self_type = None;

        // Find parentheses
        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Check for self
                if trimmed == "self" {
                    is_method = true;
                    self_type = Some(SelfType::Value);
                    parameters.push(Parameter {
                        name: "self".to_string(),
                        param_type: Some("Self".to_string()),
                        is_optional: false,
                        default_value: None,
                    });
                    continue;
                }
                if trimmed == "&self" {
                    is_method = true;
                    self_type = Some(SelfType::Ref);
                    parameters.push(Parameter {
                        name: "self".to_string(),
                        param_type: Some("&Self".to_string()),
                        is_optional: false,
                        default_value: None,
                    });
                    continue;
                }
                if trimmed == "&mut self" {
                    is_method = true;
                    self_type = Some(SelfType::MutRef);
                    parameters.push(Parameter {
                        name: "self".to_string(),
                        param_type: Some("&mut Self".to_string()),
                        is_optional: false,
                        default_value: None,
                    });
                    continue;
                }

                // Parse name: type
                if let Some(colon_pos) = trimmed.find(':') {
                    let name = trimmed[..colon_pos].trim().to_string();
                    let param_type = Some(trimmed[colon_pos + 1..].trim().to_string());
                    parameters.push(Parameter {
                        name,
                        param_type,
                        is_optional: false,
                        default_value: None,
                    });
                } else {
                    // Just a name (pattern)
                    parameters.push(Parameter {
                        name: trimmed.to_string(),
                        param_type: None,
                        is_optional: false,
                        default_value: None,
                    });
                }
            }
        }

        (parameters, is_method, self_type)
    }

    fn extract_return_type_rust(source: &str) -> Option<String> {
        // Find -> after closing paren
        if let Some(paren_end) = source.find(')') {
            let after_paren = &source[paren_end + 1..];
            if let Some(arrow_pos) = after_paren.find("->") {
                let after_arrow = &after_paren[arrow_pos + 2..];
                // Find the end of the type (where block, where clause, or { starts)
                let end_pos = after_arrow
                    .find('{')
                    .or_else(|| after_arrow.find("where"))
                    .or(Some(after_arrow.len()));

                if let Some(end) = end_pos {
                    let return_type = after_arrow[..end].trim();
                    if !return_type.is_empty() {
                        return Some(return_type.to_string());
                    }
                }
            }
        }
        None
    }

    /// Extract Python signature
    fn extract_python(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        // Python uses def, visibility is typically by convention (_ prefix = private)
        let visibility = if node.name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let is_async = full_source.contains("async def");

        let parameters = Self::extract_params_python(full_source);

        let return_type = full_source.find("->").and_then(|arrow_pos| {
            let after_arrow = &full_source[arrow_pos + 2..];
            after_arrow
                .find(':')
                .map(|colon_pos| after_arrow[..colon_pos].trim().to_string())
        });

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async,
            generics: vec![],
            is_method: false,
            self_type: None,
            modifiers: if is_async {
                vec!["async".to_string()]
            } else {
                vec![]
            },
        })
    }

    fn extract_params_python(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() || trimmed == "self" || trimmed == "cls" {
                    continue;
                }

                // Check for type annotation: name: type
                // or default: name=value or name: type = value
                let (name, param_type, default_value) = if let Some(eq_pos) = trimmed.find('=') {
                    let before_eq = trimmed[..eq_pos].trim();
                    let default = trimmed[eq_pos + 1..].trim().to_string();

                    if let Some(colon_pos) = before_eq.find(':') {
                        (
                            before_eq[..colon_pos].trim().to_string(),
                            Some(before_eq[colon_pos + 1..].trim().to_string()),
                            Some(default),
                        )
                    } else {
                        (before_eq.to_string(), None, Some(default))
                    }
                } else if let Some(colon_pos) = trimmed.find(':') {
                    (
                        trimmed[..colon_pos].trim().to_string(),
                        Some(trimmed[colon_pos + 1..].trim().to_string()),
                        None,
                    )
                } else {
                    (trimmed.to_string(), None, None)
                };

                parameters.push(Parameter {
                    name,
                    param_type,
                    is_optional: default_value.is_some(),
                    default_value,
                });
            }
        }

        parameters
    }

    /// Extract TypeScript/JavaScript signature
    fn extract_typescript(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        let visibility = if full_source.contains("private") {
            Visibility::Private
        } else if full_source.contains("protected") {
            Visibility::Protected
        } else if full_source.contains("public") {
            Visibility::Public
        } else {
            Visibility::Package
        };

        let is_async = full_source.contains("async");

        let parameters = Self::extract_params_typescript(full_source);
        let is_method = full_source.contains("this")
            || parameters
                .first()
                .map(|p| p.name == "this")
                .unwrap_or(false);

        let return_type = full_source.rfind("): ").and_then(|colon_pos| {
            let after_paren = &full_source[colon_pos + 3..];
            after_paren
                .find('{')
                .map(|brace_pos| after_paren[..brace_pos].trim().to_string())
        });

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async,
            generics: vec![],
            is_method,
            self_type: None,
            modifiers: if is_async {
                vec!["async".to_string()]
            } else {
                vec![]
            },
        })
    }

    fn extract_params_typescript(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Check for optional (?), default (=), type (: type)
                let (name, param_type, default_value, is_optional) =
                    if let Some(eq_pos) = trimmed.find('=') {
                        let before_eq = trimmed[..eq_pos].trim();
                        let default = trimmed[eq_pos + 1..].trim().to_string();
                        let optional = before_eq.ends_with('?');
                        let name = if optional {
                            &before_eq[..before_eq.len() - 1]
                        } else {
                            before_eq
                        };

                        if let Some(colon_pos) = name.find(':') {
                            (
                                name[..colon_pos].trim().to_string(),
                                Some(name[colon_pos + 1..].trim().to_string()),
                                Some(default),
                                optional,
                            )
                        } else {
                            (name.to_string(), None, Some(default), optional)
                        }
                    } else if let Some(colon_pos) = trimmed.find(':') {
                        let optional = trimmed[..colon_pos].ends_with('?');
                        let name = trimmed[..colon_pos].trim();
                        let name = if optional {
                            &name[..name.len() - 1]
                        } else {
                            name
                        };
                        (
                            name.to_string(),
                            Some(trimmed[colon_pos + 1..].trim().to_string()),
                            None,
                            optional,
                        )
                    } else {
                        let optional = trimmed.ends_with('?');
                        let name = if optional {
                            &trimmed[..trimmed.len() - 1]
                        } else {
                            trimmed
                        };
                        (name.to_string(), None, None, optional)
                    };

                parameters.push(Parameter {
                    name,
                    param_type,
                    is_optional,
                    default_value,
                });
            }
        }

        parameters
    }

    /// Extract Go signature
    fn extract_go(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        // Go uses uppercase for public, lowercase for private
        let visibility = if node
            .name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let parameters = Self::extract_params_go(full_source, true);
        let return_type = Self::extract_return_type_go(full_source);

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async: false, // Go doesn't have async keyword
            generics: vec![],
            is_method: full_source.contains("func ("),
            self_type: None,
            modifiers: vec![],
        })
    }

    fn extract_params_go(source: &str, _is_input: bool) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        // Find func parameters
        if let Some(func_pos) = source.find("func ") {
            let after_func = &source[func_pos + 5..];

            // Skip receiver if present
            let params_start = if after_func.starts_with('(') {
                // Method with receiver, find the function params
                if let Some(close_paren) = after_func.find(')') {
                    if let Some(next_paren) = after_func[close_paren..].find('(') {
                        close_paren + next_paren + 1
                    } else {
                        return parameters;
                    }
                } else {
                    return parameters;
                }
            } else if let Some(open_paren) = after_func.find('(') {
                open_paren + 1
            } else {
                return parameters;
            };

            let remaining = &after_func[params_start..];
            if let Some(close_paren) = remaining.find(')') {
                let params_str = &remaining[..close_paren];
                for param in params_str.split(',') {
                    let trimmed = param.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Go params can be "name type" or just "type" or "name1, name2 type"
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[0].to_string();
                        let param_type = Some(parts[1..].join(" "));
                        parameters.push(Parameter {
                            name,
                            param_type,
                            is_optional: false,
                            default_value: None,
                        });
                    } else if parts.len() == 1 {
                        parameters.push(Parameter {
                            name: parts[0].to_string(),
                            param_type: None,
                            is_optional: false,
                            default_value: None,
                        });
                    }
                }
            }
        }

        parameters
    }

    fn extract_return_type_go(source: &str) -> Option<String> {
        // Find the return type after closing paren of params
        if let Some(close_paren) = source.rfind(')') {
            let after_paren = &source[close_paren + 1..];
            let trimmed = after_paren.trim();

            // Skip to the return type
            if trimmed.starts_with('{') {
                return None;
            }

            // Find end of return type (either { or end)
            let end_pos = trimmed.find('{').unwrap_or(trimmed.len());
            let return_type = trimmed[..end_pos].trim();
            if !return_type.is_empty() {
                return Some(return_type.to_string());
            }
        }
        None
    }

    /// Extract C/C++ signature
    fn extract_cpp(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        let visibility = if full_source.contains("public:") {
            Visibility::Public
        } else if full_source.contains("protected:") {
            Visibility::Protected
        } else if full_source.contains("private:") {
            Visibility::Private
        } else {
            Visibility::Package
        };

        let parameters = Self::extract_params_cpp(full_source);
        let return_type = Self::extract_return_type_cpp(full_source);

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async: false,
            generics: vec![],
            is_method: false,
            self_type: None,
            modifiers: vec![],
        })
    }

    fn extract_params_cpp(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // C++ params: "type name" or "type name = default"
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts.last().unwrap_or(&"").to_string();
                    let name = if name.contains('=') {
                        name.split('=').next().unwrap_or("").trim().to_string()
                    } else {
                        name
                    };
                    let param_type = Some(parts[..parts.len() - 1].join(" "));
                    let default_value = if trimmed.contains('=') {
                        Some(
                            trimmed
                                .split('=')
                                .next_back()
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                        )
                    } else {
                        None
                    };

                    parameters.push(Parameter {
                        name,
                        param_type,
                        is_optional: default_value.is_some(),
                        default_value,
                    });
                }
            }
        }

        parameters
    }

    fn extract_return_type_cpp(source: &str) -> Option<String> {
        if let Some(open_paren) = source.find('(') {
            let before_paren = &source[..open_paren];
            // The return type is typically the words before the function name
            let words: Vec<&str> = before_paren.split_whitespace().collect();
            if words.len() >= 2 {
                // Last word is usually the function name
                return Some(words[..words.len() - 1].join(" "));
            } else if words.len() == 1 {
                return Some(words[0].to_string());
            }
        }
        None
    }

    /// Extract Java signature
    fn extract_java(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        let visibility = if full_source.contains("public") {
            Visibility::Public
        } else if full_source.contains("protected") {
            Visibility::Protected
        } else if full_source.contains("private") {
            Visibility::Private
        } else {
            Visibility::Package
        };

        let is_async = full_source.contains("async") || full_source.contains("CompletableFuture");
        let is_static = full_source.contains("static");

        let parameters = Self::extract_params_java(full_source);
        let return_type = Self::extract_return_type_java(full_source);

        let mut modifiers = vec![];
        if is_static {
            modifiers.push("static".to_string());
        }
        if is_async {
            modifiers.push("async".to_string());
        }

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async,
            generics: Self::extract_generics_java(full_source),
            is_method: true, // All Java functions are methods in classes
            self_type: None,
            modifiers,
        })
    }

    fn extract_params_java(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Java params: "Type name" or "final Type name"
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts.last().unwrap_or(&"").to_string();
                    let param_type = if parts.len() > 2 {
                        Some(parts[..parts.len() - 1].join(" "))
                    } else {
                        Some(parts[0].to_string())
                    };
                    parameters.push(Parameter {
                        name,
                        param_type,
                        is_optional: false,
                        default_value: None,
                    });
                }
            }
        }

        parameters
    }

    fn extract_return_type_java(source: &str) -> Option<String> {
        // Find return type before the method name
        if let Some(open_paren) = source.find('(') {
            let before_paren = &source[..open_paren];
            let words: Vec<&str> = before_paren.split_whitespace().collect();
            if words.len() >= 2 {
                // Skip modifiers like public, static, etc.
                let type_start = words
                    .iter()
                    .position(|w| {
                        ![
                            "public",
                            "private",
                            "protected",
                            "static",
                            "final",
                            "abstract",
                            "synchronized",
                            "native",
                            "strictfp",
                        ]
                        .contains(w)
                    })
                    .unwrap_or(0);
                let name_idx = words.len() - 1; // Last word is method name
                if name_idx > type_start {
                    return Some(words[type_start..name_idx].join(" "));
                }
            }
        }
        None
    }

    fn extract_generics_java(source: &str) -> Vec<String> {
        let mut generics = Vec::new();
        // Java generics appear before the return type: <T, U>
        if let Some(start) = source.find('<')
            && let Some(end) = source.find('>')
            && start < end
            && (source[..start].contains("static")
                || source[..start].contains("public")
                || source[..start].contains("private"))
        {
            let generic_str = &source[start + 1..end];
            for part in generic_str.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    // Remove extends/super bounds
                    let generic_name = trimmed.split_whitespace().next().unwrap_or(trimmed);
                    generics.push(generic_name.to_string());
                }
            }
        }
        generics
    }

    /// Extract PHP signature
    fn extract_php(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        let visibility = if full_source.contains("public") {
            Visibility::Public
        } else if full_source.contains("protected") {
            Visibility::Protected
        } else if full_source.contains("private") {
            Visibility::Private
        } else {
            Visibility::Public
        };

        let is_static = full_source.contains("static");

        let parameters = Self::extract_params_php(full_source);
        let return_type = Self::extract_return_type_php(full_source);

        let modifiers = if is_static {
            vec!["static".to_string()]
        } else {
            vec![]
        };

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type,
            is_async: false,
            generics: vec![],
            is_method: true,
            self_type: None,
            modifiers,
        })
    }

    fn extract_params_php(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // PHP params: "$name", "Type $name", "$name = default"
                let has_default = trimmed.contains('=');
                let default_value = if has_default {
                    Some(
                        trimmed
                            .split('=')
                            .next_back()
                            .unwrap_or("")
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                };

                let without_default = if has_default {
                    trimmed.split('=').next().unwrap_or("").trim()
                } else {
                    trimmed
                };

                // Check for type hint
                let (name, param_type) = if without_default.starts_with('$') {
                    (without_default.to_string(), None)
                } else {
                    let parts: Vec<&str> = without_default.split_whitespace().collect();
                    if parts.len() >= 2 {
                        (parts[1].to_string(), Some(parts[0].to_string()))
                    } else {
                        (without_default.to_string(), None)
                    }
                };

                parameters.push(Parameter {
                    name,
                    param_type,
                    is_optional: has_default,
                    default_value,
                });
            }
        }

        parameters
    }

    fn extract_return_type_php(source: &str) -> Option<String> {
        // PHP return type: function name(): Type
        if let Some(colon_pos) = source.find(':') {
            let after_colon = &source[colon_pos + 1..];
            // Skip whitespace
            let trimmed = after_colon.trim_start();
            // Find end of return type (either { or end)
            let end_pos = trimmed
                .find('{')
                .unwrap_or_else(|| trimmed.find(';').unwrap_or(trimmed.len()));
            let return_type = trimmed[..end_pos].trim();
            if !return_type.is_empty() && !return_type.starts_with('$') {
                // Make sure it's not a ternary operator
                if !source[..colon_pos].contains('?') {
                    return Some(return_type.to_string());
                }
            }
        }
        None
    }

    /// Extract Ruby signature
    fn extract_ruby(node: &CodeNode, _source: &str) -> Option<Signature> {
        let full_source = node.source.as_deref().unwrap_or("");

        // Ruby uses conventions: public by default, private/protected keywords
        let visibility = if full_source.contains("private") {
            Visibility::Private
        } else if full_source.contains("protected") {
            Visibility::Protected
        } else {
            Visibility::Public
        };

        let is_self_method = full_source.contains("self.");

        let parameters = Self::extract_params_ruby(full_source);

        let modifiers = if is_self_method {
            vec!["self".to_string()]
        } else {
            vec![]
        };

        Some(Signature {
            signature: full_source.to_string(),
            visibility,
            parameters,
            return_type: None, // Ruby is dynamically typed
            is_async: false,
            generics: vec![],
            is_method: true,
            self_type: if is_self_method {
                Some(SelfType::Value)
            } else {
                None
            },
            modifiers,
        })
    }

    fn extract_params_ruby(source: &str) -> Vec<Parameter> {
        let mut parameters = Vec::new();

        // Ruby params can be: name, name:, *args, &block, name = default
        if let (Some(start), Some(end)) = (source.find('('), source.find(')'))
            && start < end
        {
            let params_str = &source[start + 1..end];
            for param in params_str.split(',') {
                let trimmed = param.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Keyword argument: name:
                if trimmed.ends_with(':') && !trimmed.starts_with('*') && !trimmed.starts_with('&')
                {
                    parameters.push(Parameter {
                        name: trimmed[..trimmed.len() - 1].to_string(),
                        param_type: None,
                        is_optional: true,
                        default_value: None,
                    });
                    continue;
                }

                // Splat: *args
                if trimmed.starts_with('*') {
                    parameters.push(Parameter {
                        name: trimmed.to_string(),
                        param_type: Some("splat".to_string()),
                        is_optional: true,
                        default_value: None,
                    });
                    continue;
                }

                // Block: &block
                if trimmed.starts_with('&') {
                    parameters.push(Parameter {
                        name: trimmed.to_string(),
                        param_type: Some("block".to_string()),
                        is_optional: true,
                        default_value: None,
                    });
                    continue;
                }

                // Default value: name = value
                if trimmed.contains('=') {
                    let parts: Vec<&str> = trimmed.split('=').collect();
                    let name = parts[0].trim().to_string();
                    let default = parts.get(1).map(|s| s.trim().to_string());
                    parameters.push(Parameter {
                        name,
                        param_type: None,
                        is_optional: true,
                        default_value: default,
                    });
                    continue;
                }

                // Regular param
                parameters.push(Parameter {
                    name: trimmed.to_string(),
                    param_type: None,
                    is_optional: false,
                    default_value: None,
                });
            }
        }

        parameters
    }
}

/// Convert signature to properties map for storage
impl From<&Signature> for HashMap<String, String> {
    fn from(sig: &Signature) -> Self {
        let mut props = HashMap::new();

        props.insert("visibility".to_string(), sig.visibility.to_string());
        props.insert("is_async".to_string(), sig.is_async.to_string());
        props.insert("is_method".to_string(), sig.is_method.to_string());

        if let Some(ref ret) = sig.return_type {
            props.insert("return_type".to_string(), ret.clone());
        }

        if !sig.generics.is_empty() {
            props.insert("generics".to_string(), sig.generics.join(", "));
        }

        if !sig.parameters.is_empty() {
            let params: Vec<String> = sig
                .parameters
                .iter()
                .map(|p| {
                    if let Some(ref t) = p.param_type {
                        format!("{}: {}", p.name, t)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            props.insert("parameters".to_string(), params.join(", "));
        }

        if let Some(ref self_type) = sig.self_type {
            props.insert(
                "self_type".to_string(),
                format!("{:?}", self_type).to_lowercase(),
            );
        }

        props
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::EntityKind;

    #[test]
    fn test_rust_function_signature() {
        let source =
            "pub async fn authenticate(user: &str, pass: &str) -> Result<Token, AuthError> {";
        let node = CodeNode {
            id: "test".to_string(),
            kind: EntityKind::Function,
            name: "authenticate".to_string(),
            path: Some("test.rs".to_string()),
            line_number: Some(1),
            lang: Some(Language::Rust),
            source: Some(source.to_string()),
            docstring: None,
            properties: HashMap::new(),
        };

        let sig = SignatureExtractor::extract(&node, source).unwrap();

        assert_eq!(sig.visibility, Visibility::Public);
        assert!(sig.is_async);
        assert_eq!(sig.parameters.len(), 2);
        assert_eq!(
            sig.return_type,
            Some("Result<Token, AuthError>".to_string())
        );
    }

    #[test]
    fn test_rust_method_signature() {
        let source = "pub fn build(&mut self, query: &str) -> ContextCapsuleResult {";
        let node = CodeNode {
            id: "test".to_string(),
            kind: EntityKind::Function,
            name: "build".to_string(),
            path: Some("test.rs".to_string()),
            line_number: Some(1),
            lang: Some(Language::Rust),
            source: Some(source.to_string()),
            docstring: None,
            properties: HashMap::new(),
        };

        let sig = SignatureExtractor::extract(&node, source).unwrap();

        assert!(sig.is_method);
        assert_eq!(sig.self_type, Some(SelfType::MutRef));
    }

    #[test]
    fn test_python_signature() {
        let source = "def authenticate(user: str, pass: str = '') -> Token:";
        let node = CodeNode {
            id: "test".to_string(),
            kind: EntityKind::Function,
            name: "authenticate".to_string(),
            path: Some("test.py".to_string()),
            line_number: Some(1),
            lang: Some(Language::Python),
            source: Some(source.to_string()),
            docstring: None,
            properties: HashMap::new(),
        };

        let sig = SignatureExtractor::extract(&node, source).unwrap();

        assert_eq!(sig.visibility, Visibility::Public);
        assert_eq!(sig.parameters.len(), 2);
        assert!(sig.parameters[1].is_optional);
        assert_eq!(sig.return_type, Some("Token".to_string()));
    }

    #[test]
    fn test_typescript_signature() {
        let source = "public async authenticate(user: string, pass?: string): Promise<Token> {";
        let node = CodeNode {
            id: "test".to_string(),
            kind: EntityKind::Function,
            name: "authenticate".to_string(),
            path: Some("test.ts".to_string()),
            line_number: Some(1),
            lang: Some(Language::TypeScript),
            source: Some(source.to_string()),
            docstring: None,
            properties: HashMap::new(),
        };

        let sig = SignatureExtractor::extract(&node, source).unwrap();

        assert_eq!(sig.visibility, Visibility::Public);
        assert!(sig.is_async);
        assert!(sig.parameters[1].is_optional);
    }
}
