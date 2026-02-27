use crate::GraphClient;
use cortex_core::{CodeEdge, CodeNode, CortexError, EdgeKind, EntityKind, Language, Result};
use neo4rs::query;
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
        let mut node_result = client
            .inner()
            .execute(
                query(
                    "MATCH (r:Repository {path: $path})
                     OPTIONAL MATCH (r)-[:CONTAINS*0..]->(n)
                     RETURN DISTINCT n",
                )
                .param("path", repo_path.to_string()),
            )
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut nodes = Vec::<CodeNode>::new();
        while let Ok(Some(row)) = node_result.next().await {
            let Ok(node) = row.get::<neo4rs::Node>("n") else {
                continue;
            };
            nodes.push(decode_node(&node));
        }

        let mut edge_result = client
            .inner()
            .execute(
                query(
                    "MATCH (r:Repository {path: $path})
                     OPTIONAL MATCH (r)-[:CONTAINS*0..]->(a)-[e]->(b)
                     RETURN DISTINCT a.id AS from, b.id AS to, type(e) AS rel_type, e.properties AS properties",
                )
                .param("path", repo_path.to_string()),
            )
            .await
            .map_err(|e| CortexError::Database(e.to_string()))?;

        let mut edges = Vec::<CodeEdge>::new();
        while let Ok(Some(row)) = edge_result.next().await {
            let Ok(from) = row.get::<String>("from") else {
                continue;
            };
            let Ok(to) = row.get::<String>("to") else {
                continue;
            };
            let rel_type = row.get::<String>("rel_type").unwrap_or_default();
            let properties_raw = row.get::<String>("properties").unwrap_or_default();
            let properties = serde_json::from_str::<HashMap<String, String>>(&properties_raw)
                .unwrap_or_default();
            edges.push(CodeEdge {
                from,
                to,
                kind: decode_edge_kind(&rel_type),
                properties,
            });
        }

        Ok(GraphBundle { nodes, edges })
    }
}

fn decode_node(node: &neo4rs::Node) -> CodeNode {
    let kind_str = node
        .get::<String>("kind")
        .unwrap_or_else(|_| "File".to_string());
    let lang = node
        .get::<String>("lang")
        .ok()
        .and_then(|v| decode_lang(&v));
    let properties = node
        .get::<String>("properties")
        .ok()
        .and_then(|v| serde_json::from_str::<HashMap<String, String>>(&v).ok())
        .unwrap_or_default();
    CodeNode {
        id: node.get::<String>("id").unwrap_or_default(),
        kind: decode_entity_kind(&kind_str),
        name: node.get::<String>("name").unwrap_or_default(),
        path: node.get::<String>("path").ok(),
        line_number: node.get::<i64>("line_number").ok().map(|v| v as u32),
        lang,
        source: node.get::<String>("source").ok(),
        docstring: node.get::<String>("docstring").ok(),
        properties,
    }
}

fn decode_entity_kind(value: &str) -> EntityKind {
    match value {
        "Repository" => EntityKind::Repository,
        "Directory" => EntityKind::Directory,
        "File" => EntityKind::File,
        "Function" => EntityKind::Function,
        "Class" => EntityKind::Class,
        "Struct" => EntityKind::Struct,
        "Trait" => EntityKind::Trait,
        "Interface" => EntityKind::Interface,
        "Enum" => EntityKind::Enum,
        "Macro" => EntityKind::Macro,
        "Variable" => EntityKind::Variable,
        "Parameter" => EntityKind::Parameter,
        "Module" => EntityKind::Module,
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
        assert_eq!(decode_entity_kind("Class"), EntityKind::Class);
        assert_eq!(decode_entity_kind("Struct"), EntityKind::Struct);
        assert_eq!(decode_entity_kind("Trait"), EntityKind::Trait);
        assert_eq!(decode_entity_kind("Interface"), EntityKind::Interface);
        assert_eq!(decode_entity_kind("Enum"), EntityKind::Enum);
        assert_eq!(decode_entity_kind("Macro"), EntityKind::Macro);
        assert_eq!(decode_entity_kind("Variable"), EntityKind::Variable);
        assert_eq!(decode_entity_kind("Parameter"), EntityKind::Parameter);
        assert_eq!(decode_entity_kind("Module"), EntityKind::Module);
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
            nodes: vec![
                CodeNode {
                    id: "func:main".to_string(),
                    kind: EntityKind::Function,
                    name: "main".to_string(),
                    path: Some("src/main.rs".to_string()),
                    line_number: Some(1),
                    lang: Some(Language::Rust),
                    source: Some("fn main() {}".to_string()),
                    docstring: Some("Main entry point".to_string()),
                    properties: HashMap::new(),
                },
            ],
            edges: vec![
                CodeEdge {
                    from: "func:main".to_string(),
                    to: "func:helper".to_string(),
                    kind: EdgeKind::Calls,
                    properties: HashMap::new(),
                },
            ],
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
            nodes: vec![
                CodeNode {
                    id: "test".to_string(),
                    kind: EntityKind::Function,
                    name: "test".to_string(),
                    path: None,
                    line_number: None,
                    lang: None,
                    source: None,
                    docstring: None,
                    properties: HashMap::new(),
                },
            ],
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

        assert_eq!(parsed.nodes[0].properties.get("visibility").unwrap(), "public");
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
