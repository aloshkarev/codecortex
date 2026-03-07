use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, extract_all,
};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name)) @entity

(function_definition
  declarator: (function_declarator
    declarator: (qualified_identifier
      name: (identifier) @name))) @entity

(class_specifier
  name: (type_identifier) @name) @class_entity

(struct_specifier
  name: (type_identifier) @name) @struct_entity

(enum_specifier
  name: (type_identifier) @name) @enum_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (qualified_identifier
    name: (identifier) @call))

(call_expression
  function: (field_expression
    field: (field_identifier) @call))
"#;

const IMPORT_QUERY: &str = r#"
(preproc_include
  path: (system_lib_string) @module)

(preproc_include
  path: (string_literal) @module)
"#;

const INHERIT_QUERY: &str = r#"
(class_specifier
  name: (type_identifier) @child
  (base_class_clause
    (type_identifier) @parent))
"#;

const PARAM_QUERY: &str = r#"
(parameter_declaration
  declarator: (identifier) @param)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_cpp::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("cpp def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("cpp call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("cpp import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).expect("cpp inherit query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("cpp param query");

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
        Language::Cpp,
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
        None,
        None,
    )
}
