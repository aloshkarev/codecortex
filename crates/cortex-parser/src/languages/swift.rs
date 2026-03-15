use super::common::{CallCaptures, DefCaptures, ImportCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(function_declaration
  name: (_) @name) @entity

(class_declaration
  name: (_) @name) @class_entity
"#;

const CALL_QUERY: &str = r#"
(call_expression
  (simple_identifier) @call)

(call_expression
  (navigation_expression
    (simple_identifier) @call))
"#;

const IMPORT_QUERY: &str = r#"
(import_declaration
  (identifier) @module)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_swift::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("swift def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("swift call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("swift import query");

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
        Language::Swift,
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
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::extract;
    use std::path::Path;
    use tree_sitter::Parser;

    #[test]
    fn extracts_swift_symbols() {
        let source = r#"
            import Foundation
            class AuthService {}
            func authenticate(user: String) -> Bool { return !user.isEmpty }
        "#;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_swift::LANGUAGE.into();
        parser.set_language(&language).expect("set swift grammar");
        let tree = parser.parse(source, None).expect("parse swift");

        let result = extract(source, Path::new("Sources/App.swift"), &tree);
        assert!(result.nodes.iter().any(|n| n.name == "authenticate"));
        assert!(result.nodes.iter().any(|n| n.name == "AuthService"));
    }
}
