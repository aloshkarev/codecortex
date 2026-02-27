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
"#;

const CALL_QUERY: &str = r#"
(call_expression
  function: (identifier) @call)

(call_expression
  function: (field_expression
    field: (field_identifier) @call))

(macro_invocation
  macro: (identifier) @call)
"#;

const IMPORT_QUERY: &str = r#"
(use_declaration
  argument: (scoped_identifier
    name: _ @module))

(use_declaration
  argument: (identifier) @module)
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
