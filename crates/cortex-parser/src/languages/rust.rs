use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, VariableCaptures,
    extract_all,
};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

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

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("rust def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("rust call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("rust import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).expect("rust inherit query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("rust param query");
    let var_q = Query::new(&lang, VARIABLE_QUERY).expect("rust var query");

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
    ];

    extract_all(
        source,
        path,
        Language::Rust,
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
            edge_kind: EdgeKind::Implements,
        }),
        Some(&param_q),
        Some(&ParamCaptures {
            param: param_q.capture_index_for_name("param").unwrap_or(0),
        }),
        Some(&var_q),
        Some(&VariableCaptures {
            var: var_q.capture_index_for_name("var").unwrap_or(0),
        }),
    )
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
}
