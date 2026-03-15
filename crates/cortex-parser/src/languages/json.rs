use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, CodeNode, EdgeKind, EntityKind, Language};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Query, QueryCursor, StreamingIterator};

fn file_id(path: &Path) -> String {
    format!("file:{}", path.display())
}

fn key_node_id(path: &Path, key_path: &str, line: u32) -> String {
    format!("JsonKey:{}:{}:{}", path.display(), key_path, line)
}

pub fn extract(source: &str, path: &Path, tree: &tree_sitter::Tree) -> ParseResult {
    let lang: tree_sitter::Language = tree_sitter_json::LANGUAGE.into();
    let source_bytes = source.as_bytes();
    let file_node_id = file_id(path);

    let file_node = CodeNode {
        id: file_node_id.clone(),
        kind: EntityKind::File,
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        path: Some(path.display().to_string()),
        line_number: Some(1),
        lang: Some(Language::Json),
        source: None,
        docstring: None,
        properties: [("line_count".to_string(), source.lines().count().to_string())]
            .into_iter()
            .collect(),
    };

    let mut nodes = vec![file_node];
    let mut edges: Vec<CodeEdge> = Vec::new();
    let mut imports = Vec::new();
    let calls = Vec::new();

    let pair_query = Query::new(
        &lang,
        r#"
(pair
  key: (string (string_content) @key)
  value: (_) @value) @pair
"#,
    )
    .expect("json pair query");

    let key_cap = pair_query.capture_index_for_name("key").unwrap_or(0);
    let value_cap = pair_query.capture_index_for_name("value").unwrap_or(1);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&pair_query, tree.root_node(), source_bytes);

    let mut seen = HashMap::<String, String>::new();
    while let Some(m) = matches.next() {
        let mut key = None;
        let mut value_node = None;
        for c in m.captures {
            if c.index == key_cap {
                key = std::str::from_utf8(&source_bytes[c.node.byte_range()])
                    .ok()
                    .map(|s| s.to_string());
            } else if c.index == value_cap {
                value_node = Some(c.node);
            }
        }

        let Some(key_text) = key else { continue };
        let line = m
            .captures
            .first()
            .map(|c| c.node.start_position().row as u32 + 1)
            .unwrap_or(1);

        let path_suffix = if let Some(parent) = value_node {
            let kind = parent.kind();
            if kind == "object" {
                format!("{key_text}.object")
            } else if kind == "array" {
                format!("{key_text}.array")
            } else {
                key_text.clone()
            }
        } else {
            key_text.clone()
        };

        let id = key_node_id(path, path_suffix.as_str(), line);
        if seen.contains_key(&id) {
            continue;
        }
        seen.insert(id.clone(), key_text.clone());

        nodes.push(CodeNode {
            id: id.clone(),
            kind: EntityKind::Property,
            name: key_text.clone(),
            path: Some(path.display().to_string()),
            line_number: Some(line),
            lang: Some(Language::Json),
            source: None,
            docstring: None,
            properties: [("json_key".to_string(), key_text.clone())]
                .into_iter()
                .collect(),
        });

        edges.push(CodeEdge {
            from: file_node_id.clone(),
            to: id,
            kind: EdgeKind::Contains,
            properties: HashMap::new(),
        });

        // Heuristic import-like edge for common JSON dependency fields.
        if matches!(
            key_text.as_str(),
            "$ref" | "ref" | "extends" | "include" | "import" | "imports"
        ) {
            imports.push(key_text);
        }
    }

    ParseResult {
        nodes,
        edges,
        imports,
        calls,
    }
}

#[cfg(test)]
mod tests {
    use super::extract;
    use std::path::Path;
    use tree_sitter::Parser;

    #[test]
    fn extracts_json_keys_as_properties() {
        let source = r#"
            {
              "name": "codecortex",
              "scripts": { "build": "cargo build" },
              "$ref": "./schema.json"
            }
        "#;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_json::LANGUAGE.into();
        parser.set_language(&language).expect("set json grammar");
        let tree = parser.parse(source, None).expect("parse json");

        let result = extract(source, Path::new("package.json"), &tree);
        assert!(result.nodes.iter().any(|n| n.name == "name"));
        assert!(result.nodes.iter().any(|n| n.name == "scripts"));
        assert!(result.nodes.iter().any(|n| n.name == "$ref"));
        assert!(!result.edges.is_empty());
    }
}
