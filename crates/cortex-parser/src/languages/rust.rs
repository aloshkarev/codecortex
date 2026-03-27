use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, VariableCaptures,
    entity_id, extract_all, file_id,
};
use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, EdgeKind, EntityKind, Language};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

const DEF_QUERY: &str = r#"
(function_item
  name: (identifier) @name) @entity

(struct_item
  name: (type_identifier) @name) @struct_entity

(enum_item
  name: (type_identifier) @name) @enum_entity

(trait_item
  name: (type_identifier) @name) @trait_entity

(impl_item
  type: (type_identifier) @name) @impl_entity

(type_item
  name: (type_identifier) @name) @type_alias_entity

(const_item
  name: (identifier) @name) @const_entity

(macro_definition
  name: (identifier) @name) @macro_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (scoped_identifier
    name: (identifier) @call))

(call_expression
  function: (generic_function
    function: (identifier) @call))

(call_expression
  function: (generic_function
    function: (scoped_identifier
      name: (identifier) @call)))

(call_expression
  function: (field_expression
    field: (field_identifier) @call))

(macro_invocation
  macro: (identifier) @call)
"#;

const IMPORT_QUERY: &str = r#"
(use_declaration) @module
"#;

const INHERIT_QUERY: &str = r#"
(impl_item
  trait: (type_identifier) @parent
  type: (type_identifier) @child)
"#;

const PARAM_QUERY: &str = r#"
(parameter
  pattern: (identifier) @param)

(self_parameter) @param
"#;

const VARIABLE_QUERY: &str = r#"
(let_declaration
  pattern: (identifier) @var)
"#;

const MEMBER_OF_QUERY: &str = r#"
(impl_item
  type: (type_identifier) @parent
  body: (declaration_list
    (function_item
      name: (identifier) @method) @method_entity))
"#;

const TYPE_REF_QUERY: &str = r#"
(type_identifier) @type_ref
"#;

const FIELD_ACCESS_QUERY: &str = r#"
(field_expression
  field: (field_identifier) @field)
"#;

fn rust_lang() -> &'static tree_sitter::Language {
    static L: OnceLock<tree_sitter::Language> = OnceLock::new();
    L.get_or_init(|| tree_sitter_rust::LANGUAGE.into())
}

fn def_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), DEF_QUERY).expect("rust def query"))
}

fn call_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), CALL_QUERY).expect("rust call query"))
}

fn import_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), IMPORT_QUERY).expect("rust import query"))
}

fn inherit_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), INHERIT_QUERY).expect("rust inherit query"))
}

fn param_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), PARAM_QUERY).expect("rust param query"))
}

fn variable_query() -> &'static Query {
    static Q: OnceLock<Query> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), VARIABLE_QUERY).expect("rust var query"))
}

fn member_of_query() -> Option<&'static Query> {
    static Q: OnceLock<Option<Query>> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), MEMBER_OF_QUERY).ok())
        .as_ref()
}

fn type_ref_query() -> Option<&'static Query> {
    static Q: OnceLock<Option<Query>> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), TYPE_REF_QUERY).ok())
        .as_ref()
}

fn field_access_query() -> Option<&'static Query> {
    static Q: OnceLock<Option<Query>> = OnceLock::new();
    Q.get_or_init(|| Query::new(rust_lang(), FIELD_ACCESS_QUERY).ok())
        .as_ref()
}

fn def_sets() -> &'static [DefCaptures] {
    static SETS: OnceLock<Vec<DefCaptures>> = OnceLock::new();
    SETS.get_or_init(|| {
        let def_q = def_query();
        vec![
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
                    .capture_index_for_name("enum_entity")
                    .unwrap_or(u32::MAX),
                name: def_q.capture_index_for_name("name").unwrap_or(1),
                kind: EntityKind::Enum,
            },
            DefCaptures {
                entity: def_q
                    .capture_index_for_name("trait_entity")
                    .unwrap_or(u32::MAX),
                name: def_q.capture_index_for_name("name").unwrap_or(1),
                kind: EntityKind::Trait,
            },
            DefCaptures {
                entity: def_q
                    .capture_index_for_name("impl_entity")
                    .unwrap_or(u32::MAX),
                name: def_q.capture_index_for_name("name").unwrap_or(1),
                kind: EntityKind::Module,
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
                    .capture_index_for_name("const_entity")
                    .unwrap_or(u32::MAX),
                name: def_q.capture_index_for_name("name").unwrap_or(1),
                kind: EntityKind::Constant,
            },
            DefCaptures {
                entity: def_q
                    .capture_index_for_name("macro_entity")
                    .unwrap_or(u32::MAX),
                name: def_q.capture_index_for_name("name").unwrap_or(1),
                kind: EntityKind::Macro,
            },
        ]
    })
}

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let def_q = def_query();
    let call_q = call_query();
    let import_q = import_query();
    let inherit_q = inherit_query();
    let param_q = param_query();
    let var_q = variable_query();
    let def_sets = def_sets();

    let mut result = extract_all(
        source,
        path,
        Language::Rust,
        tree,
        def_q,
        def_sets,
        call_q,
        &CallCaptures {
            call: call_q.capture_index_for_name("call").unwrap_or(0),
        },
        import_q,
        &ImportCaptures {
            module: import_q.capture_index_for_name("module").unwrap_or(0),
            method_filter: None,
        },
        Some(inherit_q),
        Some(&InheritCaptures {
            child: inherit_q.capture_index_for_name("child").unwrap_or(0),
            parent: inherit_q.capture_index_for_name("parent").unwrap_or(1),
            edge_kind: EdgeKind::Implements,
        }),
        Some(param_q),
        Some(&ParamCaptures {
            param: param_q.capture_index_for_name("param").unwrap_or(0),
        }),
        Some(var_q),
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
    let src = source.as_bytes();
    let root = tree.root_node();
    let fid = file_id(path);

    // MEMBER_OF edges from impl methods to parent type.
    if let Some(member_q) = member_of_query() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(member_q, root, src);
        while let Some(m) = matches.next() {
            let mut parent_name = None::<String>;
            let mut method_name = None::<String>;
            let mut method_line = None::<u32>;
            for cap in m.captures.iter() {
                let cap_name = &member_q.capture_names()[cap.index as usize];
                if *cap_name == "parent" {
                    parent_name = Some(super::common::node_text(cap.node, src).trim().to_string());
                } else if *cap_name == "method" {
                    method_name = Some(super::common::node_text(cap.node, src).trim().to_string());
                } else if *cap_name == "method_entity" {
                    method_line = Some(cap.node.start_position().row as u32 + 1);
                }
            }
            let (Some(parent_name), Some(method_name), Some(method_line)) =
                (parent_name, method_name, method_line)
            else {
                continue;
            };
            let Some(parent) = result
                .nodes
                .iter()
                .find(|n| {
                    matches!(
                        n.kind,
                        EntityKind::Struct
                            | EntityKind::Class
                            | EntityKind::Trait
                            | EntityKind::Enum
                            | EntityKind::TypeAlias
                    ) && n.name == parent_name
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

    // TYPE_REFERENCE edges from file node to placeholder targets.
    if let Some(type_q) = type_ref_query() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(type_q, root, src);
        while let Some(m) = matches.next() {
            for cap in m.captures.iter() {
                let name = super::common::node_text(cap.node, src).trim().to_string();
                if name.is_empty() {
                    continue;
                }
                push_edge(
                    &mut result.edges,
                    CodeEdge {
                        from: fid.clone(),
                        to: format!("call_target:{name}"),
                        kind: EdgeKind::TypeReference,
                        properties: HashMap::new(),
                    },
                );
            }
        }
    }

    // FIELD_ACCESS edges from file node to placeholder targets.
    if let Some(field_q) = field_access_query() {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(field_q, root, src);
        while let Some(m) = matches.next() {
            for cap in m.captures.iter() {
                let name = super::common::node_text(cap.node, src).trim().to_string();
                if name.is_empty() {
                    continue;
                }
                push_edge(
                    &mut result.edges,
                    CodeEdge {
                        from: fid.clone(),
                        to: format!("call_target:{name}"),
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

    fn parse_rust(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_parse_function() {
        let source = r#"
            fn main() {
                println!("hello");
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("main.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "main" && n.kind == EntityKind::Function)
        );
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
            struct Point {
                x: i32,
                y: i32,
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("point.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Point" && n.kind == EntityKind::Struct)
        );
    }

    #[test]
    fn test_parse_enum() {
        let source = r#"
            enum Color {
                Red,
                Green,
                Blue,
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("color.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Color" && n.kind == EntityKind::Enum)
        );
    }

    #[test]
    fn test_parse_trait() {
        let source = r#"
            trait Drawable {
                fn draw(&self);
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("drawable.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Drawable" && n.kind == EntityKind::Trait)
        );
    }

    #[test]
    fn test_parse_impl() {
        let source = r#"
            struct Point;
            impl Point {
                fn new() -> Self { Point }
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("point.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Point" && n.kind == EntityKind::Module)
        );
    }

    #[test]
    fn test_parse_type_alias() {
        let source = r#"
            type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
        "#;
        let tree = parse_rust(source);
        let path = Path::new("result.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Result" && n.kind == EntityKind::TypeAlias)
        );
    }

    #[test]
    fn test_parse_const() {
        let source = r#"
            const MAX_SIZE: usize = 1024;
        "#;
        let tree = parse_rust(source);
        let path = Path::new("config.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "MAX_SIZE" && n.kind == EntityKind::Constant)
        );
    }

    #[test]
    fn test_parse_macro() {
        let source = r#"
            macro_rules! vec_of_strings {
                ($($x:expr),*) => {
                    vec![$($x.to_string()),*]
                };
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("macros.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "vec_of_strings" && n.kind == EntityKind::Macro)
        );
    }

    #[test]
    fn test_parse_trait_impl() {
        let source = r#"
            struct Point;
            trait Drawable { fn draw(&self); }
            impl Drawable for Point {
                fn draw(&self) {}
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("point.rs");
        let result = extract(source, path, &tree);

        assert!(
            result
                .edges
                .iter()
                .any(|e| matches!(e.kind, EdgeKind::Implements))
        );
    }

    #[test]
    fn test_parse_result_populates_calls_and_imports() {
        let source = r#"
            use foo;

            fn helper() {}

            fn main() {
                helper();
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("main.rs");
        let result = extract(source, path, &tree);

        assert!(result.calls.iter().any(|call| call == "helper"));
        assert!(result.imports.iter().any(|module| module == "foo"));
    }

    #[test]
    fn test_parse_long_utf8_function_source_does_not_panic() {
        let body = "€".repeat(2000);
        let source = format!("fn unicode() {{ let text = \"{body}\"; }}");
        let tree = parse_rust(&source);
        let path = Path::new("unicode.rs");
        let result = extract(&source, path, &tree);

        let function = result
            .nodes
            .iter()
            .find(|node| node.name == "unicode" && node.kind == EntityKind::Function)
            .expect("unicode function should be extracted");

        let snippet = function
            .source
            .as_deref()
            .expect("function snippet should be present");
        assert!(snippet.len() <= 4096);
        assert!(snippet.contains("fn unicode()"));
    }

    #[test]
    fn test_parse_scoped_and_generic_calls() {
        let source = r#"
            fn run() {
                std::mem::drop::<i32>(1);
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("calls.rs");
        let result = extract(source, path, &tree);

        assert!(result.calls.iter().any(|call| call == "drop"));
    }

    #[test]
    fn test_parse_grouped_use_imports() {
        let source = r#"
            use std::{io, fs};
            use crate::module::{TypeA, TypeB};
        "#;
        let tree = parse_rust(source);
        let path = Path::new("imports.rs");
        let result = extract(source, path, &tree);

        assert!(result.imports.iter().any(|m| m == "std::{io, fs}"));
        assert!(
            result
                .imports
                .iter()
                .any(|m| m == "crate::module::{TypeA, TypeB}")
        );
    }

    #[test]
    fn test_parse_navigation_edges() {
        let source = r#"
            struct User { id: i32 }
            impl User {
                fn get_id(&self) -> i32 { self.id }
            }
        "#;
        let tree = parse_rust(source);
        let path = Path::new("user.rs");
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
