use crate::GraphClient;
use cortex_core::{CodeEdge, CodeNode, CortexError, EdgeKind, EntityKind, Language, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphBundle {
    pub nodes: Vec<CodeNode>,
    pub edges: Vec<CodeEdge>,
}

pub struct BundleStore;

impl BundleStore {
    pub fn export(path: &Path, bundle: &GraphBundle) -> Result<()> {
        let bytes = rmp_serde::to_vec(bundle)
            .map_err(|e| CortexError::Io(format!("encode bundle: {e}")))?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn import(path: &Path) -> Result<GraphBundle> {
        let bytes = std::fs::read(path)?;
        rmp_serde::from_slice::<GraphBundle>(&bytes)
            .map_err(|e| CortexError::Io(format!("decode bundle: {e}")))
    }

    pub async fn export_from_graph(client: &GraphClient, repo_path: &str) -> Result<GraphBundle> {
        // Use the unified query API
        let node_rows = client
            .query_with_param(
                "MATCH (r:Repository {path: $path})
                 OPTIONAL MATCH (r)-[:CONTAINS*0..]->(n)
                 RETURN DISTINCT n",
                "path",
                repo_path,
            )
            .await?;

        let mut nodes = Vec::<CodeNode>::new();
        for row in node_rows {
            let Some(node_obj) = row.get("n") else {
                continue;
            };
            nodes.push(decode_node_from_json(node_obj));
        }

        let edge_rows = client
            .query_with_param(
                "MATCH (r:Repository {path: $path})
                 OPTIONAL MATCH (r)-[:CONTAINS*0..]->(a)-[e]->(b)
                 RETURN DISTINCT a.id AS from, b.id AS to, type(e) AS rel_type, e.properties AS properties",
                "path",
                repo_path,
            )
            .await?;

        let mut edges = Vec::<CodeEdge>::new();
        for row in edge_rows {
            let from = row.get("from").and_then(|v| v.as_str()).map(String::from);
            let to = row.get("to").and_then(|v| v.as_str()).map(String::from);
            let rel_type = row
                .get("rel_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let properties_raw = row
                .get("properties")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let properties = serde_json::from_str::<HashMap<String, String>>(&properties_raw)
                .unwrap_or_default();

            if let (Some(from), Some(to)) = (from, to) {
                edges.push(CodeEdge {
                    from,
                    to,
                    kind: decode_edge_kind(&rel_type),
                    properties,
                });
            }
        }

        Ok(GraphBundle { nodes, edges })
    }
}

/// Decode a CodeNode from a JSON object (Memgraph or Neo4j format)
fn decode_node_from_json(node: &serde_json::Value) -> CodeNode {
    // Handle both Neo4j format (node with id, labels, properties)
    // and Memgraph format (direct properties)
    let (kind_str, name, id, path, line_number, lang, source, docstring, properties_raw) =
        if let Some(props) = node.get("properties") {
            // Neo4j-style: {labels: [...], properties: {kind, name, ...}}
            let kind = props.get("kind").and_then(|v| v.as_str()).unwrap_or("File");
            let name = props
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let id = props.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let path = props.get("path").and_then(|v| v.as_str());
            let line_number = props
                .get("line_number")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);
            let lang = props.get("lang").and_then(|v| v.as_str());
            let source = props.get("source").and_then(|v| v.as_str());
            let docstring = props.get("docstring").and_then(|v| v.as_str());
            let props_str = props.get("properties").and_then(|v| v.as_str());
            (
                kind,
                name,
                id,
                path,
                line_number,
                lang,
                source,
                docstring,
                props_str,
            )
        } else {
            // Direct format: {kind, name, id, path, ...}
            let kind = node.get("kind").and_then(|v| v.as_str()).unwrap_or("File");
            let name = node
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let id = node.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let path = node.get("path").and_then(|v| v.as_str());
            let line_number = node
                .get("line_number")
                .and_then(|v| v.as_i64())
                .map(|v| v as u32);
            let lang = node.get("lang").and_then(|v| v.as_str());
            let source = node.get("source").and_then(|v| v.as_str());
            let docstring = node.get("docstring").and_then(|v| v.as_str());
            let props_str = node.get("properties").and_then(|v| v.as_str());
            (
                kind,
                name,
                id,
                path,
                line_number,
                lang,
                source,
                docstring,
                props_str,
            )
        };

    let properties = properties_raw
        .and_then(|v| serde_json::from_str::<HashMap<String, String>>(v).ok())
        .unwrap_or_default();

    CodeNode {
        id: id.to_string(),
        kind: decode_entity_kind(kind_str),
        name: name.to_string(),
        path: path.map(String::from),
        line_number,
        lang: lang.and_then(decode_lang),
        source: source.map(String::from),
        docstring: docstring.map(String::from),
        properties,
    }
}

fn decode_entity_kind(value: &str) -> EntityKind {
    match value {
        "Repository" => EntityKind::Repository,
        "Directory" => EntityKind::Directory,
        "File" => EntityKind::File,
        "Function" => EntityKind::Function,
        "Method" => EntityKind::Method,
        "Class" => EntityKind::Class,
        "Struct" => EntityKind::Struct,
        "Trait" => EntityKind::Trait,
        "Interface" => EntityKind::Interface,
        "Enum" => EntityKind::Enum,
        "EnumVariant" => EntityKind::EnumVariant,
        "Macro" => EntityKind::Macro,
        "Variable" => EntityKind::Variable,
        "Constant" => EntityKind::Constant,
        "Parameter" => EntityKind::Parameter,
        "TypeAlias" => EntityKind::TypeAlias,
        "Module" => EntityKind::Module,
        "Property" => EntityKind::Property,
        "Field" => EntityKind::Field,
        _ => EntityKind::File,
    }
}

fn decode_edge_kind(value: &str) -> EdgeKind {
    match value {
        "CONTAINS" => EdgeKind::Contains,
        "CALLS" => EdgeKind::Calls,
        "IMPORTS" => EdgeKind::Imports,
        "INHERITS" => EdgeKind::Inherits,
        "IMPLEMENTS" => EdgeKind::Implements,
        "HAS_PARAMETER" => EdgeKind::HasParameter,
        "DEFINED_IN" => EdgeKind::DefinedIn,
        "REFERENCES" => EdgeKind::References,
        "USES" => EdgeKind::Uses,
        "THROWS" => EdgeKind::Throws,
        "RETURNS" => EdgeKind::Returns,
        "HAS_FIELD" => EdgeKind::HasField,
        "HAS_METHOD" => EdgeKind::HasMethod,
        "HAS_PROPERTY" => EdgeKind::HasProperty,
        "DOCUMENTS" => EdgeKind::Documents,
        "ANNOTATES" => EdgeKind::Annotates,
        _ => EdgeKind::Contains,
    }
}

fn decode_lang(value: &str) -> Option<Language> {
    match value {
        "rust" => Some(Language::Rust),
        "c" => Some(Language::C),
        "cpp" => Some(Language::Cpp),
        "python" => Some(Language::Python),
        "go" => Some(Language::Go),
        "typescript" => Some(Language::TypeScript),
        "javascript" => Some(Language::JavaScript),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_decode_entity_kind() {
        assert_eq!(decode_entity_kind("Repository"), EntityKind::Repository);
        assert_eq!(decode_entity_kind("Directory"), EntityKind::Directory);
        assert_eq!(decode_entity_kind("File"), EntityKind::File);
        assert_eq!(decode_entity_kind("Function"), EntityKind::Function);
        assert_eq!(decode_entity_kind("Method"), EntityKind::Method);
        assert_eq!(decode_entity_kind("Class"), EntityKind::Class);
        assert_eq!(decode_entity_kind("Struct"), EntityKind::Struct);
        assert_eq!(decode_entity_kind("Trait"), EntityKind::Trait);
        assert_eq!(decode_entity_kind("Interface"), EntityKind::Interface);
        assert_eq!(decode_entity_kind("Enum"), EntityKind::Enum);
        assert_eq!(decode_entity_kind("EnumVariant"), EntityKind::EnumVariant);
        assert_eq!(decode_entity_kind("Macro"), EntityKind::Macro);
        assert_eq!(decode_entity_kind("Variable"), EntityKind::Variable);
        assert_eq!(decode_entity_kind("Constant"), EntityKind::Constant);
        assert_eq!(decode_entity_kind("Parameter"), EntityKind::Parameter);
        assert_eq!(decode_entity_kind("TypeAlias"), EntityKind::TypeAlias);
        assert_eq!(decode_entity_kind("Module"), EntityKind::Module);
        assert_eq!(decode_entity_kind("Property"), EntityKind::Property);
        assert_eq!(decode_entity_kind("Field"), EntityKind::Field);
        // Unknown values default to File
        assert_eq!(decode_entity_kind("Unknown"), EntityKind::File);
        assert_eq!(decode_entity_kind(""), EntityKind::File);
    }

    #[test]
    fn test_decode_edge_kind() {
        assert_eq!(decode_edge_kind("CONTAINS"), EdgeKind::Contains);
        assert_eq!(decode_edge_kind("CALLS"), EdgeKind::Calls);
        assert_eq!(decode_edge_kind("IMPORTS"), EdgeKind::Imports);
        assert_eq!(decode_edge_kind("INHERITS"), EdgeKind::Inherits);
        assert_eq!(decode_edge_kind("IMPLEMENTS"), EdgeKind::Implements);
        assert_eq!(decode_edge_kind("HAS_PARAMETER"), EdgeKind::HasParameter);
        assert_eq!(decode_edge_kind("DEFINED_IN"), EdgeKind::DefinedIn);
        assert_eq!(decode_edge_kind("REFERENCES"), EdgeKind::References);
        assert_eq!(decode_edge_kind("USES"), EdgeKind::Uses);
        assert_eq!(decode_edge_kind("THROWS"), EdgeKind::Throws);
        assert_eq!(decode_edge_kind("RETURNS"), EdgeKind::Returns);
        assert_eq!(decode_edge_kind("HAS_FIELD"), EdgeKind::HasField);
        assert_eq!(decode_edge_kind("HAS_METHOD"), EdgeKind::HasMethod);
        assert_eq!(decode_edge_kind("HAS_PROPERTY"), EdgeKind::HasProperty);
        assert_eq!(decode_edge_kind("DOCUMENTS"), EdgeKind::Documents);
        assert_eq!(decode_edge_kind("ANNOTATES"), EdgeKind::Annotates);
        // Unknown values default to Contains
        assert_eq!(decode_edge_kind("Unknown"), EdgeKind::Contains);
        assert_eq!(decode_edge_kind(""), EdgeKind::Contains);
    }

    #[test]
    fn test_decode_lang() {
        assert_eq!(decode_lang("rust"), Some(Language::Rust));
        assert_eq!(decode_lang("c"), Some(Language::C));
        assert_eq!(decode_lang("cpp"), Some(Language::Cpp));
        assert_eq!(decode_lang("python"), Some(Language::Python));
        assert_eq!(decode_lang("go"), Some(Language::Go));
        assert_eq!(decode_lang("typescript"), Some(Language::TypeScript));
        assert_eq!(decode_lang("javascript"), Some(Language::JavaScript));
        // Unknown values return None
        assert_eq!(decode_lang("ruby"), None);
        assert_eq!(decode_lang(""), None);
    }

    #[test]
    fn test_graph_bundle_serialization() {
        let bundle = GraphBundle {
            nodes: vec![CodeNode {
                id: "func:main".to_string(),
                kind: EntityKind::Function,
                name: "main".to_string(),
                path: Some("src/main.rs".to_string()),
                line_number: Some(1),
                lang: Some(Language::Rust),
                source: Some("fn main() {}".to_string()),
                docstring: Some("Main entry point".to_string()),
                properties: HashMap::new(),
            }],
            edges: vec![CodeEdge {
                from: "func:main".to_string(),
                to: "func:helper".to_string(),
                kind: EdgeKind::Calls,
                properties: HashMap::new(),
            }],
        };

        let bytes = rmp_serde::to_vec(&bundle).unwrap();
        let parsed: GraphBundle = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].id, "func:main");
        assert_eq!(parsed.nodes[0].kind, EntityKind::Function);
        assert_eq!(parsed.edges.len(), 1);
        assert_eq!(parsed.edges[0].kind, EdgeKind::Calls);
    }

    #[test]
    fn test_bundle_store_export_import() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.ccx");

        let bundle = GraphBundle {
            nodes: vec![CodeNode {
                id: "test".to_string(),
                kind: EntityKind::Function,
                name: "test".to_string(),
                path: None,
                line_number: None,
                lang: None,
                source: None,
                docstring: None,
                properties: HashMap::new(),
            }],
            edges: vec![],
        };

        BundleStore::export(&path, &bundle).unwrap();
        assert!(path.exists());

        let loaded = BundleStore::import(&path).unwrap();
        assert_eq!(loaded.nodes.len(), 1);
        assert_eq!(loaded.nodes[0].id, "test");
    }

    #[test]
    fn test_graph_bundle_empty() {
        let bundle = GraphBundle {
            nodes: vec![],
            edges: vec![],
        };

        let bytes = rmp_serde::to_vec(&bundle).unwrap();
        let parsed: GraphBundle = rmp_serde::from_slice(&bytes).unwrap();
        assert!(parsed.nodes.is_empty());
        assert!(parsed.edges.is_empty());
    }

    #[test]
    fn test_graph_bundle_with_properties() {
        let mut props = HashMap::new();
        props.insert("visibility".to_string(), "public".to_string());
        props.insert("async".to_string(), "true".to_string());

        let bundle = GraphBundle {
            nodes: vec![CodeNode {
                id: "func:test".to_string(),
                kind: EntityKind::Function,
                name: "test".to_string(),
                path: None,
                line_number: None,
                lang: Some(Language::Python),
                source: None,
                docstring: None,
                properties: props,
            }],
            edges: vec![],
        };

        let bytes = rmp_serde::to_vec(&bundle).unwrap();
        let parsed: GraphBundle = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(
            parsed.nodes[0].properties.get("visibility").unwrap(),
            "public"
        );
        assert_eq!(parsed.nodes[0].lang, Some(Language::Python));
    }

    #[test]
    fn test_graph_bundle_large() {
        let nodes: Vec<CodeNode> = (0..100)
            .map(|i| CodeNode {
                id: format!("func:{}", i),
                kind: EntityKind::Function,
                name: format!("func_{}", i),
                path: Some(format!("src/file{}.rs", i / 10)),
                line_number: Some(i),
                lang: Some(Language::Rust),
                source: None,
                docstring: None,
                properties: HashMap::new(),
            })
            .collect();

        let edges: Vec<CodeEdge> = (0..50)
            .map(|i| CodeEdge {
                from: format!("func:{}", i),
                to: format!("func:{}", i + 1),
                kind: EdgeKind::Calls,
                properties: HashMap::new(),
            })
            .collect();

        let bundle = GraphBundle { nodes, edges };

        let bytes = rmp_serde::to_vec(&bundle).unwrap();
        let parsed: GraphBundle = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(parsed.nodes.len(), 100);
        assert_eq!(parsed.edges.len(), 50);
    }
}
