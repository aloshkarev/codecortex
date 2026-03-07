//! PHP language parser using Tree-sitter queries

use super::common::{CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(class_declaration
  name: (name) @name) @class_entity

(interface_declaration
  name: (name) @name) @interface_entity

(trait_declaration
  name: (name) @name) @trait_entity

(function_definition
  name: (name) @name) @func_entity

(method_declaration
  name: (name) @name) @method_entity
"#;

const CALL_QUERY: &str = r#"
(function_call_expression
  function: (name) @call)

(member_call_expression
  name: (name) @call)
"#;

const IMPORT_QUERY: &str = r#"
(namespace_definition
  name: (namespace_name) @module)

(namespace_use_declaration) @module
"#;

const INHERIT_QUERY: &str = r#"
(class_declaration
  name: (name) @child
  extends: (class_extends
    name: (name) @parent))
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_php::LANGUAGE_PHP.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("php def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("php call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("php import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).ok();

    let def_sets = vec![
        DefCaptures {
            entity: def_q.capture_index_for_name("class_entity").unwrap_or(0),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Class,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("interface_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Interface,
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
                .capture_index_for_name("func_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
        DefCaptures {
            entity: def_q
                .capture_index_for_name("method_entity")
                .unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
    ];

    extract_all(
        source,
        path,
        Language::Php,
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
        inherit_q.as_ref(),
        inherit_q
            .as_ref()
            .map(|q| InheritCaptures {
                child: q.capture_index_for_name("child").unwrap_or(0),
                parent: q.capture_index_for_name("parent").unwrap_or(1),
                edge_kind: EdgeKind::Inherits,
            })
            .as_ref(),
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_php(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_parse_simple_class() {
        let source = r#"<?php
            class HelloWorld {
                public function main($args) {
                    echo "Hello, World!";
                }
            }
        ?>"#;
        let tree = parse_php(source);
        let path = Path::new("HelloWorld.php");
        let result = extract(source, path, &tree);

        assert!(!result.nodes.is_empty());
    }

    #[test]
    fn test_parse_interface() {
        let source = r#"<?php
            interface Runnable {
                public function run();
            }
        ?>"#;
        let tree = parse_php(source);
        let path = Path::new("Runnable.php");
        let result = extract(source, path, &tree);

        assert!(
            result
                .nodes
                .iter()
                .any(|n| n.name == "Runnable" && n.kind == EntityKind::Interface)
        );
    }
}
