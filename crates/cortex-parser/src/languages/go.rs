use super::common::{CallCaptures, DefCaptures, ImportCaptures, ParamCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

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

    extract_all(
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
    )
}
