use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, VariableCaptures,
    extract_all,
};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

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

    extract_all(
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
    )
}
