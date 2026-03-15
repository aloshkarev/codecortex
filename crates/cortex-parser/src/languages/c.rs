use super::common::{CallCaptures, DefCaptures, ImportCaptures, ParamCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name)) @entity

(struct_specifier
  name: (type_identifier) @name) @struct_entity

(enum_specifier
  name: (type_identifier) @name) @enum_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (field_expression
    field: (field_identifier) @call))

(call_expression
  function: (parenthesized_expression
    (pointer_expression
      argument: (identifier) @call)))
"#;

const IMPORT_QUERY: &str = r#"
(preproc_include
  path: (system_lib_string) @module)

(preproc_include
  path: (string_literal) @module)
"#;

const PARAM_QUERY: &str = r#"
(parameter_declaration
  declarator: (identifier) @param)

(parameter_declaration
  declarator: (pointer_declarator
    declarator: (identifier) @param))
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_c::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("c def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("c call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("c import query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("c param query");

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
    ];

    extract_all(
        source,
        path,
        Language::C,
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
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_c(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_c::LANGUAGE.into()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn extracts_c_field_calls() {
        let source = r#"
            struct Api { void (*ping)(int); };
            void run(struct Api api) { api.ping(1); }
        "#;
        let tree = parse_c(source);
        let result = extract(source, Path::new("api.c"), &tree);

        assert!(result.calls.iter().any(|call| call == "ping"));
    }

    #[test]
    fn extracts_c_dereferenced_function_pointer_calls() {
        let source = r#"
            void run(void (*cb)(int)) {
                (*cb)(1);
            }
        "#;
        let tree = parse_c(source);
        let result = extract(source, Path::new("callbacks.c"), &tree);

        assert!(result.calls.iter().any(|call| call == "cb"));
    }

    #[test]
    fn extracts_c_pointer_parameters() {
        let source = r#"
            void write_value(int *out_value) {
                *out_value = 42;
            }
        "#;
        let tree = parse_c(source);
        let result = extract(source, Path::new("params.c"), &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.kind == EntityKind::Parameter && n.name == "out_value")
        );
    }
}
