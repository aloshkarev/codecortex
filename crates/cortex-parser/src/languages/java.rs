//! Java language parser using Tree-sitter queries

use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, extract_all,
};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(class_declaration
  name: (identifier) @name) @class_entity

(interface_declaration
  name: (identifier) @name) @interface_entity

(enum_declaration
  name: (identifier) @name) @enum_entity

(method_declaration
  name: (identifier) @name) @method_entity

(constructor_declaration
  name: (identifier) @name) @ctor_entity
"#;

const CALL_QUERY: &str = r#"
(method_invocation
  name: (identifier) @call)

(object_creation_expression
  type: (type_identifier) @call)
"#;

const IMPORT_QUERY: &str = r#"
(import_declaration) @module

(package_declaration) @module
"#;

const INHERIT_QUERY: &str = r#"
(class_declaration
  name: (identifier) @child
  superclass: (superclass
    name: (type_identifier) @parent))
"#;

const PARAM_QUERY: &str = r#"
(formal_parameter
  name: (identifier) @param)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_java::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("java def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("java call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("java import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).ok();
    let param_q = Query::new(&lang, PARAM_QUERY).expect("java param query");

    let def_sets = vec![
        DefCaptures {
            entity: def_q.capture_index_for_name("class_entity").unwrap_or(0),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Class,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("interface_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Interface,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("enum_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Enum,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("method_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("ctor_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
    ];

    extract_all(
        source,
        path,
        Language::Java,
        tree,
        &def_q,
        &def_sets,
        &call_q,
        &CallCaptures {
            call: call_q.capture_index_for_name("call").unwrap_or(0),
        },
        &import_q,
        &ImportCaptures {
            module: import_q.capture_index_for_name("module").unwrap_or(0),
        },
        inherit_q.as_ref(),
        inherit_q
            .as_ref()
            .map(|q| InheritCaptures {
                child: q.capture_index_for_name("child").unwrap_or(0),
                parent: q.capture_index_for_name("parent").unwrap_or(1),
                edge_kind: EdgeKind::Inherits,
            })
            .as_ref(),
        Some(&param_q),
        Some(&ParamCaptures {
            param: param_q.capture_index_for_name("param").unwrap_or(0),
        }),
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_java(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_parse_simple_class() {
        let source = r#"
            package com.example;

            public class HelloWorld {
                public static void main(String[] args) {
                    System.out.println("Hello, World!");
                }
            }
        "#;
        let tree = parse_java(source);
        let path = Path::new("HelloWorld.java");
        let result = extract(source, path, &tree);

        assert!(!result.nodes.is_empty());
        assert!(result.nodes.iter().any(|n| n.name == "HelloWorld"));
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
            public interface Runnable {
                void run();
            }
        "#;
        let tree = parse_java(source);
        let path = Path::new("Runnable.java");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Runnable" && n.kind == EntityKind::Interface)
        );
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            public enum Color {
                RED, GREEN, BLUE
            }
        "#;
        let tree = parse_java(source);
        let path = Path::new("Color.java");
        let result = extract(source, path, &tree);

        assert!(result.nodes.iter().any(|n| n.name == "Color"));
    }
}
