//! Java language parser using Tree-sitter queries

use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, entity_id,
    extract_all, file_id,
};
use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, EdgeKind, EntityKind, Language};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

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

const MEMBER_OF_QUERY: &str = r#"
(class_declaration
  name: (identifier) @class
  body: (class_body
    (method_declaration
      name: (identifier) @method) @method_entity))
"#;

const TYPE_REF_QUERY: &str = r#"
(type_identifier) @type_ref
"#;

const FIELD_ACCESS_QUERY: &str = r#"
(field_access
  field: (identifier) @field)
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

    let mut result = extract_all(
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
            method_filter: None,
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
    );
    augment_navigation_edges(source, path, tree, &mut result);
    result
}

fn augment_navigation_edges(
    source: &str,
    path: &Path,
    tree: &tree_sitter::Tree,
    result: &mut ParseResult,
) {
    let lang: tree_sitter::Language = tree_sitter_java::LANGUAGE.into();
    let src = source.as_bytes();
    let root = tree.root_node();
    let fid = file_id(path);

    if let Ok(member_q) = Query::new(&lang, MEMBER_OF_QUERY) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&member_q, root, src);
        while let Some(m) = matches.next() {
            let mut class_name = None::<String>;
            let mut method_name = None::<String>;
            let mut method_line = None::<u32>;
            for cap in m.captures.iter() {
                let cap_name = &member_q.capture_names()[cap.index as usize];
                if *cap_name == "class" {
                    class_name = Some(super::common::node_text(cap.node, src).trim().to_string());
                } else if *cap_name == "method" {
                    method_name = Some(super::common::node_text(cap.node, src).trim().to_string());
                } else if *cap_name == "method_entity" {
                    method_line = Some(cap.node.start_position().row as u32 + 1);
                }
            }
            let (Some(class_name), Some(method_name), Some(method_line)) =
                (class_name, method_name, method_line)
            else {
                continue;
            };
            let Some(parent) = result
                .nodes
                .iter()
                .find(|n| n.kind == EntityKind::Class && n.name == class_name)
                .map(|n| n.id.clone())
            else {
                continue;
            };
            let from = entity_id(&EntityKind::Function, path, &method_name, method_line);
            push_edge(
                &mut result.edges,
                CodeEdge {
                    from,
                    to: parent,
                    kind: EdgeKind::MemberOf,
                    properties: HashMap::new(),
                },
            );
        }
    }

    if let Ok(type_q) = Query::new(&lang, TYPE_REF_QUERY) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&type_q, root, src);
        while let Some(m) = matches.next() {
            for cap in m.captures.iter() {
                let type_name = super::common::node_text(cap.node, src).trim().to_string();
                if type_name.is_empty() {
                    continue;
                }
                push_edge(
                    &mut result.edges,
                    CodeEdge {
                        from: fid.clone(),
                        to: format!("call_target:{type_name}"),
                        kind: EdgeKind::TypeReference,
                        properties: HashMap::new(),
                    },
                );
            }
        }
    }

    if let Ok(field_q) = Query::new(&lang, FIELD_ACCESS_QUERY) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&field_q, root, src);
        while let Some(m) = matches.next() {
            for cap in m.captures.iter() {
                let field_name = super::common::node_text(cap.node, src).trim().to_string();
                if field_name.is_empty() {
                    continue;
                }
                push_edge(
                    &mut result.edges,
                    CodeEdge {
                        from: fid.clone(),
                        to: format!("call_target:{field_name}"),
                        kind: EdgeKind::FieldAccess,
                        properties: HashMap::new(),
                    },
                );
            }
        }
    }
}

fn push_edge(edges: &mut Vec<CodeEdge>, edge: CodeEdge) {
    let exists = edges
        .iter()
        .any(|e| e.from == edge.from && e.to == edge.to && e.kind == edge.kind);
    if !exists {
        edges.push(edge);
    }
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
    fn test_import_normalized() {
        let source = r#"
            import java.util.List;
            import java.util.Map;
            package com.example;

            public class Foo {}
        "#;
        let tree = parse_java(source);
        let path = Path::new("Foo.java");
        let result = extract(source, path, &tree);
        assert!(
            result.imports.iter().any(|i| i == "java.util.List"),
            "imports should be normalized (no 'import' prefix or ';'); got: {:?}",
            result.imports
        );
        assert!(
            result.imports.iter().any(|i| i == "com.example"),
            "package should be normalized; got: {:?}",
            result.imports
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

    #[test]
    fn test_parse_navigation_edges() {
        let source = r#"
            class User {
                private Long id;
                Long getId() { return this.id; }
            }
        "#;
        let tree = parse_java(source);
        let path = Path::new("User.java");
        let result = extract(source, path, &tree);
        assert!(result.edges.iter().any(|e| e.kind == EdgeKind::MemberOf));
        assert!(
            result
                .edges
                .iter()
                .any(|e| e.kind == EdgeKind::TypeReference)
        );
        assert!(result.edges.iter().any(|e| e.kind == EdgeKind::FieldAccess));
    }
}
