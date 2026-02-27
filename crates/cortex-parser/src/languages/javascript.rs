use super::common::{
    CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, ParamCaptures, VariableCaptures,
    extract_all,
};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(function_declaration
  name: (identifier) @name) @entity

(method_definition
  name: (property_identifier) @name) @entity

(class_declaration
  name: (identifier) @name) @class_entity

(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: (arrow_function))) @entity

(variable_declaration
  (variable_declarator
    name: (identifier) @name
    value: (function_expression))) @entity
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
  name: (identifier) @child
  (class_heritage
    (extends_clause
      (identifier) @parent))
)
"#;

const PARAM_QUERY: &str = r#"
(formal_parameters
  (identifier) @param)
"#;

const VARIABLE_QUERY: &str = r#"
(variable_declarator
  name: (identifier) @var)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("js def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("js call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("js import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).expect("js inherit query");
    let param_q = Query::new(&lang, PARAM_QUERY).expect("js param query");
    let var_q = Query::new(&lang, VARIABLE_QUERY).expect("js var query");

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
        Language::JavaScript,
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
