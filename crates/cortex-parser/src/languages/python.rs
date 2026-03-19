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
(function_definition
  name: (identifier) @name) @entity

(class_definition
  name: (identifier) @name) @class_entity
"#;

/// Reserved for future decorator-based analysis (e.g. @property, @staticmethod).
#[allow(dead_code)]
const DECORATOR_QUERY: &str = r#"
(decorator
  (identifier) @decorator)

(decorator
  (attribute
    attribute: (identifier) @decorator))

(decorated_definition
  (decorator
    (identifier) @decorator))
"#;

const CALL_QUERY: &str = r#"
(call
  function: (identifier) @call)

(call
  function: (attribute
    attribute: (identifier) @call))
"#;

const IMPORT_QUERY: &str = r#"
(import_statement
  name: (dotted_name) @module)

(import_from_statement
  module_name: (dotted_name) @module)
"#;

const INHERIT_QUERY: &str = r#"
(class_definition
  name: (identifier) @child
  superclasses: (argument_list
    (identifier) @parent))
"#;

const PARAM_QUERY: &str = r#"
(parameters (identifier) @param)
"#;

const VARIABLE_QUERY: &str = r#"
(assignment
  left: (identifier) @var)
"#;

const MEMBER_OF_QUERY: &str = r#"
(class_definition
  name: (identifier) @class
  body: (block
    (function_definition
      name: (identifier) @method) @method_entity))
"#;

const TYPE_REF_QUERY: &str = r#"
(type
  (identifier) @type_ref)
"#;

const FIELD_ACCESS_QUERY: &str = r#"
(attribute
  attribute: (identifier) @field)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("python def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("python call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("python import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).expect("python inherit query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("python param query");
    let var_q = Query::new(&lang, VARIABLE_QUERY).expect("python var query");

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
    ];

    let mut result = extract_all(
        source,
        path,
        Language::Python,
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
        Some(&inherit_q),
        Some(&InheritCaptures {
            child: inherit_q.capture_index_for_name("child").unwrap_or(0),
            parent: inherit_q.capture_index_for_name("parent").unwrap_or(1),
            edge_kind: EdgeKind::Inherits,
        }),
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
    let lang: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
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

    fn parse_python(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .expect("python language");
        parser.parse(source, None).expect("python parse")
    }

    #[test]
    fn test_parse_navigation_edges() {
        let source = r#"
class User:
    id: int

    def get_id(self) -> int:
        return self.id
"#;
        let tree = parse_python(source);
        let path = Path::new("user.py");
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
