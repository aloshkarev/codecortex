//! Ruby language parser using Tree-sitter queries

use super::common::{CallCaptures, DefCaptures, ImportCaptures, InheritCaptures, extract_all};
use crate::parser_impl::ParseResult;
use cortex_core::{EdgeKind, EntityKind, Language};
use std::path::Path;
use tree_sitter::Query;

const DEF_QUERY: &str = r#"
(class
  name: (constant) @name) @class_entity

(module
  name: (constant) @name) @module_entity

(method
  name: (identifier) @name) @method_entity

(singleton_method
  name: (identifier) @name) @singleton_method_entity
"#;

const CALL_QUERY: &str = r#"
(call
  method: (identifier) @call)
"#;

const IMPORT_QUERY: &str = r#"
(call
  method: (identifier) @method
  arguments: (argument_list
    (string) @module))
"#;

const INHERIT_QUERY: &str = r#"
(class
  name: (constant) @child
  superclass: (superclass
    (constant) @parent))
"#;

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_ruby::LANGUAGE.into();

    let def_q = Query::new(&lang, DEF_QUERY).expect("ruby def query");
    let call_q = Query::new(&lang, CALL_QUERY).expect("ruby call query");
    let import_q = Query::new(&lang, IMPORT_QUERY).expect("ruby import query");
    let inherit_q = Query::new(&lang, INHERIT_QUERY).ok();

    let def_sets = vec![
        DefCaptures {
            entity: def_q.capture_index_for_name("class_entity").unwrap_or(0),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Class,
        },
        DefCaptures {
            entity: def_q.capture_index_for_name("module_entity").unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Module,
        },
        DefCaptures {
            entity: def_q.capture_index_for_name("method_entity").unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
        DefCaptures {
            entity: def_q.capture_index_for_name("singleton_method_entity").unwrap_or(u32::MAX),
            name: def_q.capture_index_for_name("name").unwrap_or(1),
            kind: EntityKind::Function,
        },
    ];

    extract_all(
        source,
        path,
        Language::Ruby,
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
        inherit_q.as_ref(),
        inherit_q.as_ref().map(|q| InheritCaptures {
            child: q.capture_index_for_name("child").unwrap_or(0),
            parent: q.capture_index_for_name("parent").unwrap_or(1),
            edge_kind: EdgeKind::Inherits,
        }).as_ref(),
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

    fn parse_ruby(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_ruby::LANGUAGE.into()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_parse_simple_class() {
        let source = r#"
            class HelloWorld
              def main(args)
                puts "Hello, World!"
              end
            end
        "#;
        let tree = parse_ruby(source);
        let path = Path::new("hello_world.rb");
        let result = extract(source, path, &tree);

        assert!(!result.nodes.is_empty());
        assert!(result.nodes.iter().any(|n| n.name == "HelloWorld"));
    }

    #[test]
    fn test_parse_module() {
        let source = r#"
            module MyModule
              class MyClass
              end
            end
        "#;
        let tree = parse_ruby(source);
        let path = Path::new("my_module.rb");
        let result = extract(source, path, &tree);

        assert!(result.nodes.iter().any(|n| n.name == "MyModule" && n.kind == EntityKind::Module));
    }

    #[test]
    fn test_parse_standalone_method() {
        let source = r#"
            def greet(name)
              "Hello, #{name}"
            end
        "#;
        let tree = parse_ruby(source);
        let path = Path::new("greet.rb");
        let result = extract(source, path, &tree);

        assert!(result.nodes.iter().any(|n| n.name == "greet" && n.kind == EntityKind::Function));
    }

    #[test]
    fn test_parse_inheritance() {
        let source = r#"
            class Child < Parent
            end
        "#;
        let tree = parse_ruby(source);
        let path = Path::new("child.rb");
        let result = extract(source, path, &tree);

        assert!(result.nodes.iter().any(|n| n.name == "Child"));
        assert!(result.edges.iter().any(|e| matches!(e.kind, EdgeKind::Inherits)));
    }
}
