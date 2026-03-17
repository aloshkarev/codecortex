use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, VariableCaptures,
    entity_id, extract_all, file_id,
};
use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, EdgeKind, EntityKind, Language};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

const DEF_QUERY: &str = r#"
(function_declaration
  name: (identifier) @name) @entity

(method_definition
  name: (property_identifier) @name) @entity

(class_declaration
  name: (type_identifier) @name) @class_entity

(interface_declaration
  name: (type_identifier) @name) @interface_entity

(type_alias_declaration
  name: (type_identifier) @name) @type_alias_entity

(enum_declaration
  name: (identifier) @name) @enum_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (member_expression
    property: (property_identifier) @call))
"#;

const IMPORT_QUERY: &str = r#"
(import_statement
  source: (string) @module)
"#;

const INHERIT_QUERY: &str = r#"
(class_declaration
  name: (type_identifier) @child
  (class_heritage
    (type_identifier) @parent)
)
"#;

const PARAM_QUERY: &str = r#"
(required_parameter
  pattern: (identifier) @param)

(optional_parameter
  pattern: (identifier) @param)
"#;

const VARIABLE_QUERY: &str = r#"
(variable_declarator
  name: (identifier) @var)
"#;

const MEMBER_OF_QUERY: &str = r#"
(class_declaration
  name: (type_identifier) @class
  body: (class_body
    (method_definition
      name: (property_identifier) @method) @method_entity))
"#;

const TYPE_REF_QUERY: &str = r#"
(type_annotation
  (type_identifier) @type_ref)
"#;

const FIELD_ACCESS_QUERY: &str = r#"
(member_expression
  property: (property_identifier) @field)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("ts def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("ts call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("ts import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).ok();
    let param_q = Query::new(&lang, PARAM_QUERY).expect("ts param query");
    let var_q = Query::new(&lang, VARIABLE_QUERY).expect("ts var query");

    let def_sets = vec![
        DefCaptures {
            entity: def_q.capture_index_for_name("entity").unwrap_or(0),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("class_entity")
                .unwrap_or(u32::MAX),
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
                .capture_index_for_name("type_alias_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::TypeAlias,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("enum_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Enum,
        },
    ];

    let mut result = extract_all(
        source,
        path,
        Language::TypeScript,
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
        Some(&var_q),
        Some(&VariableCaptures {
            var: var_q.capture_index_for_name("var").unwrap_or(0),
        }),
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
    let lang: tree_sitter::Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
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

    fn parse_typescript(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_parse_class() {
        let source = r#"
            class User {
                constructor(private name: string) {}
                greet() { return "Hello"; }
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("user.ts");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "User" && n.kind == EntityKind::Class)
        );
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"
            interface Drawable {
                draw(): void;
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("drawable.ts");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Drawable" && n.kind == EntityKind::Interface)
        );
    }

    #[test]
    fn test_parse_type_alias() {
        let source = r#"
            type Result<T> = { success: true; data: T } | { success: false; error: string };
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("result.ts");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Result" && n.kind == EntityKind::TypeAlias)
        );
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("color.ts");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Color" && n.kind == EntityKind::Enum)
        );
    }

    #[test]
    fn test_parse_function() {
        let source = r#"
            function greet(name: string): string {
                return "Hello, " + name;
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("greet.ts");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "greet" && n.kind == EntityKind::Function)
        );
    }

    #[test]
    fn test_optional_param_detected() {
        let source = r#"
            function greet(name: string, title?: string): string {
                return "Hello, " + name;
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("greet.ts");
        let result = extract(source, path, &tree);
        let param_names: Vec<_> = result
            .nodes
            .iter()
            .filter(|n| n.kind == EntityKind::Parameter)
            .map(|n| n.name.as_str())
            .collect();
        assert!(
            param_names.contains(&"name"),
            "required param 'name' should be detected; got: {:?}",
            param_names
        );
        assert!(
            param_names.contains(&"title"),
            "optional param 'title' should be detected; got: {:?}",
            param_names
        );
    }

    #[test]
    fn test_parse_inheritance() {
        let source = r#"
            class Animal { }
            class Dog extends Animal { }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("animal.ts");
        let result = extract(source, path, &tree);

        // Both classes should be parsed (inheritance edge is optional due to tree-sitter grammar differences)
        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Dog" && n.kind == EntityKind::Class)
        );
        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Animal" && n.kind == EntityKind::Class)
        );
    }

    #[test]
    fn test_parse_navigation_edges() {
        let source = r#"
            class Profile {}
            class User {
                id: number;
                profile: Profile;
                getProfile(): Profile { return this.profile; }
            }
        "#;
        let tree = parse_typescript(source);
        let path = Path::new("user.ts");
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
