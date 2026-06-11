use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, ParamCaptures, entity_id, extract_all, file_id,
};
use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, EdgeKind, EntityKind, Language};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

const DEF_QUERY: &str = r#"
(function_declaration
  name: (identifier) @name) @entity

(method_declaration
  name: (field_identifier) @name) @entity

(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (struct_type))) @struct_entity

(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (interface_type))) @interface_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (selector_expression
    field: (field_identifier) @call))
"#;

const IMPORT_QUERY: &str = r#"
(import_spec
  path: (interpreted_string_literal) @module)
"#;

const PARAM_QUERY: &str = r#"
(parameter_declaration
  name: (identifier) @param)
"#;

const MEMBER_OF_QUERY: &str = r#"
(method_declaration
  receiver: (parameter_list
    (parameter_declaration
      type: (_) @receiver_type))
  name: (field_identifier) @method) @method_entity
"#;

const TYPE_REF_QUERY: &str = r#"
(type_identifier) @type_ref
"#;

const FIELD_ACCESS_QUERY: &str = r#"
(selector_expression
  field: (field_identifier) @field)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("go def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("go call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("go import query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("go param query");

    let def_sets = vec![
        DefCaptures {
            entity: def_q.capture_index_for_name("entity").unwrap_or(0),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("struct_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Struct,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("interface_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Interface,
        },
    ];

    let mut result = extract_all(
        source,
        path,
        Language::Go,
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
        None,
        None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_go(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
            .expect("go language");
        parser.parse(source, None).expect("go parse")
    }

    #[test]
    fn test_parse_navigation_edges() {
        let source = r#"
type User struct {
    ID int
}

func (u User) GetID() int {
    return u.ID
}
"#;
        let tree = parse_go(source);
        let path = Path::new("user.go");
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

fn augment_navigation_edges(
    source: &str,
    path: &Path,
    tree: &tree_sitter::Tree,
    result: &mut ParseResult,
) {
    let lang: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();
    let src = source.as_bytes();
    let root = tree.root_node();
    let fid = file_id(path);

    if let Ok(member_q) = Query::new(&lang, MEMBER_OF_QUERY) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&member_q, root, src);
        while let Some(m) = matches.next() {
            let mut receiver_name = None::<String>;
            let mut method_name = None::<String>;
            let mut method_line = None::<u32>;
            for cap in m.captures.iter() {
                let cap_name = &member_q.capture_names()[cap.index as usize];
                if *cap_name == "receiver_type" {
                    let txt = super::common::node_text(cap.node, src)
                        .trim()
                        .trim_start_matches('*');
                    receiver_name = Some(txt.to_string());
                } else if *cap_name == "method" {
                    method_name = Some(super::common::node_text(cap.node, src).trim().to_string());
                } else if *cap_name == "method_entity" {
                    method_line = Some(cap.node.start_position().row as u32 + 1);
                }
            }
            let (Some(receiver_name), Some(method_name), Some(method_line)) =
                (receiver_name, method_name, method_line)
            else {
                continue;
            };
            let Some(parent) = result
                .nodes
                .iter()
                .find(|n| {
                    matches!(n.kind, EntityKind::Struct | EntityKind::Interface)
                        && n.name == receiver_name
                })
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
