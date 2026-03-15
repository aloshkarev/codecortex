use super::common::{CallCaptures, DefCaptures, ImportCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(function_definition
  name: (word) @name) @entity
"#;

const CALL_QUERY: &str = r#"
(command
  name: (command_name) @call)
"#;

const IMPORT_QUERY: &str = r#"
(command
  name: (command_name) @method
  argument: (word) @module)
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("shell def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("shell call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("shell import query");

    let def_sets = vec![DefCaptures {
        entity: def_q.capture_index_for_name("entity").unwrap_or(0),
        name: def_q.capture_index_for_name("name").unwrap_or(1),
        kind: EntityKind::Function,
    }];

    extract_all(
        source,
        path,
        Language::Shell,
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
            method_filter: Some((
                import_q.capture_index_for_name("method").unwrap_or(1),
                &["source", "."],
            )),
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
    fn extracts_shell_functions_and_commands() {
        let source = r#"
            source ./env.sh
            build() { echo "ok"; }
            build
        "#;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();
        parser.set_language(&language).expect("set bash grammar");
        let tree = parser.parse(source, None).expect("parse shell");

        let result = extract(source, Path::new("scripts/build.sh"), &tree);
        assert!(result.nodes.iter().any(|n| n.name == "build"));
        assert!(result.imports.iter().any(|m| m.ends_with("env.sh")));
    }
}
