use crate::Language;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityKind {
    Repository,
    Directory,
    File,
    Function,
    Method,
    Class,
    Struct,
    Trait,
    Interface,
    Enum,
    EnumVariant,
    Macro,
    Variable,
    Constant,
    Parameter,
    TypeAlias,
    Module,
    Property,
    Field,
}

impl EntityKind {
    /// Returns the Cypher node label used in Memgraph queries.
    pub fn cypher_label(&self) -> &'static str {
        match self {
            Self::Repository => "Repository",
            Self::Directory => "Directory",
            Self::File => "File",
            Self::Function => "Function",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Struct => "Struct",
            Self::Trait => "Trait",
            Self::Interface => "Interface",
            Self::Enum => "Enum",
            Self::EnumVariant => "EnumVariant",
            Self::Macro => "Macro",
            Self::Variable => "Variable",
            Self::Constant => "Constant",
            Self::Parameter => "Parameter",
            Self::TypeAlias => "TypeAlias",
            Self::Module => "Module",
            Self::Property => "Property",
            Self::Field => "Field",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeKind {
    Contains,
    Calls,
    Imports,
    Inherits,
    Implements,
    HasParameter,
    DefinedIn,
    References,
    Uses,
    Throws,
    Returns,
    HasField,
    HasMethod,
    HasProperty,
    Documents,
    Annotates,
}

impl EdgeKind {
    /// Returns the Cypher relationship type used in Memgraph queries.
    pub fn cypher_rel_type(&self) -> &'static str {
        match self {
            Self::Contains => "CONTAINS",
            Self::Calls => "CALLS",
            Self::Imports => "IMPORTS",
            Self::Inherits => "INHERITS",
            Self::Implements => "IMPLEMENTS",
            Self::HasParameter => "HAS_PARAMETER",
            Self::DefinedIn => "DEFINED_IN",
            Self::References => "REFERENCES",
            Self::Uses => "USES",
            Self::Throws => "THROWS",
            Self::Returns => "RETURNS",
            Self::HasField => "HAS_FIELD",
            Self::HasMethod => "HAS_METHOD",
            Self::HasProperty => "HAS_PROPERTY",
            Self::Documents => "DOCUMENTS",
            Self::Annotates => "ANNOTATES",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    Name,
    Pattern,
    Type,
    Content,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeNode {
    pub id: String,
    pub kind: EntityKind,
    pub name: String,
    pub path: Option<String>,
    pub line_number: Option<u32>,
    pub lang: Option<Language>,
    pub source: Option<String>,
    pub docstring: Option<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub language: Language,
    pub content_hash: String,
    pub nodes: Vec<CodeNode>,
    pub edges: Vec<CodeEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub path: String,
    pub name: String,
    pub watched: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_kind_cypher_labels() {
        assert_eq!(EntityKind::Repository.cypher_label(), "Repository");
        assert_eq!(EntityKind::Directory.cypher_label(), "Directory");
        assert_eq!(EntityKind::File.cypher_label(), "File");
        assert_eq!(EntityKind::Function.cypher_label(), "Function");
        assert_eq!(EntityKind::Method.cypher_label(), "Method");
        assert_eq!(EntityKind::Class.cypher_label(), "Class");
        assert_eq!(EntityKind::Struct.cypher_label(), "Struct");
        assert_eq!(EntityKind::Trait.cypher_label(), "Trait");
        assert_eq!(EntityKind::Interface.cypher_label(), "Interface");
        assert_eq!(EntityKind::Enum.cypher_label(), "Enum");
        assert_eq!(EntityKind::EnumVariant.cypher_label(), "EnumVariant");
        assert_eq!(EntityKind::Macro.cypher_label(), "Macro");
        assert_eq!(EntityKind::Variable.cypher_label(), "Variable");
        assert_eq!(EntityKind::Constant.cypher_label(), "Constant");
        assert_eq!(EntityKind::Parameter.cypher_label(), "Parameter");
        assert_eq!(EntityKind::TypeAlias.cypher_label(), "TypeAlias");
        assert_eq!(EntityKind::Module.cypher_label(), "Module");
        assert_eq!(EntityKind::Property.cypher_label(), "Property");
        assert_eq!(EntityKind::Field.cypher_label(), "Field");
    }

    #[test]
    fn edge_kind_cypher_types() {
        assert_eq!(EdgeKind::Contains.cypher_rel_type(), "CONTAINS");
        assert_eq!(EdgeKind::Calls.cypher_rel_type(), "CALLS");
        assert_eq!(EdgeKind::Imports.cypher_rel_type(), "IMPORTS");
        assert_eq!(EdgeKind::Inherits.cypher_rel_type(), "INHERITS");
        assert_eq!(EdgeKind::Implements.cypher_rel_type(), "IMPLEMENTS");
        assert_eq!(EdgeKind::HasParameter.cypher_rel_type(), "HAS_PARAMETER");
        assert_eq!(EdgeKind::DefinedIn.cypher_rel_type(), "DEFINED_IN");
        assert_eq!(EdgeKind::References.cypher_rel_type(), "REFERENCES");
        assert_eq!(EdgeKind::Uses.cypher_rel_type(), "USES");
        assert_eq!(EdgeKind::Throws.cypher_rel_type(), "THROWS");
        assert_eq!(EdgeKind::Returns.cypher_rel_type(), "RETURNS");
        assert_eq!(EdgeKind::HasField.cypher_rel_type(), "HAS_FIELD");
        assert_eq!(EdgeKind::HasMethod.cypher_rel_type(), "HAS_METHOD");
        assert_eq!(EdgeKind::HasProperty.cypher_rel_type(), "HAS_PROPERTY");
        assert_eq!(EdgeKind::Documents.cypher_rel_type(), "DOCUMENTS");
        assert_eq!(EdgeKind::Annotates.cypher_rel_type(), "ANNOTATES");
    }

    #[test]
    fn entity_kind_serialization() {
        let kind = EntityKind::Function;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"FUNCTION\"");

        let kind = EntityKind::Struct;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"STRUCT\"");
    }

    #[test]
    fn entity_kind_deserialization() {
        let kind: EntityKind = serde_json::from_str("\"CLASS\"").unwrap();
        assert_eq!(kind, EntityKind::Class);

        let kind: EntityKind = serde_json::from_str("\"INTERFACE\"").unwrap();
        assert_eq!(kind, EntityKind::Interface);
    }

    #[test]
    fn edge_kind_serialization() {
        let kind = EdgeKind::Calls;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"CALLS\"");

        let kind = EdgeKind::Imports;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"IMPORTS\"");
    }

    #[test]
    fn edge_kind_deserialization() {
        let kind: EdgeKind = serde_json::from_str("\"INHERITS\"").unwrap();
        assert_eq!(kind, EdgeKind::Inherits);
    }

    #[test]
    fn search_kind_serialization() {
        let kind = SearchKind::Name;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"name\"");

        let kind = SearchKind::Pattern;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"pattern\"");
    }

    #[test]
    fn search_kind_deserialization() {
        let kind: SearchKind = serde_json::from_str("\"type\"").unwrap();
        assert_eq!(kind, SearchKind::Type);

        let kind: SearchKind = serde_json::from_str("\"content\"").unwrap();
        assert_eq!(kind, SearchKind::Content);
    }

    #[test]
    fn code_node_serialization() {
        let node = CodeNode {
            id: "func:main".to_string(),
            kind: EntityKind::Function,
            name: "main".to_string(),
            path: Some("src/main.rs".to_string()),
            line_number: Some(1),
            lang: Some(Language::Rust),
            source: Some("fn main() {}".to_string()),
            docstring: Some("Main function".to_string()),
            properties: HashMap::new(),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"id\":\"func:main\""));
        assert!(json.contains("\"kind\":\"FUNCTION\""));
        assert!(json.contains("\"name\":\"main\""));

        let parsed: CodeNode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, node.id);
        assert_eq!(parsed.name, node.name);
    }

    #[test]
    fn code_edge_serialization() {
        let edge = CodeEdge {
            from: "func:a".to_string(),
            to: "func:b".to_string(),
            kind: EdgeKind::Calls,
            properties: HashMap::new(),
        };

        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains("\"from\":\"func:a\""));
        assert!(json.contains("\"to\":\"func:b\""));
        assert!(json.contains("\"kind\":\"CALLS\""));

        let parsed: CodeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.from, edge.from);
        assert_eq!(parsed.to, edge.to);
    }

    #[test]
    fn indexed_file_serialization() {
        let file = IndexedFile {
            path: "src/main.rs".to_string(),
            language: Language::Rust,
            content_hash: "abc123".to_string(),
            nodes: vec![],
            edges: vec![],
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("\"path\":\"src/main.rs\""));
        assert!(json.contains("\"language\":\"rust\""));

        let parsed: IndexedFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, file.path);
        assert_eq!(parsed.language, file.language);
    }

    #[test]
    fn repository_serialization() {
        let repo = Repository {
            path: "/home/user/project".to_string(),
            name: "project".to_string(),
            watched: true,
        };

        let json = serde_json::to_string(&repo).unwrap();
        assert!(json.contains("\"path\":\"/home/user/project\""));
        assert!(json.contains("\"watched\":true"));

        let parsed: Repository = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, repo.path);
        assert_eq!(parsed.watched, repo.watched);
    }

    #[test]
    fn code_node_with_properties() {
        let mut props = HashMap::new();
        props.insert("visibility".to_string(), "public".to_string());
        props.insert("async".to_string(), "true".to_string());

        let node = CodeNode {
            id: "func:test".to_string(),
            kind: EntityKind::Function,
            name: "test".to_string(),
            path: None,
            line_number: None,
            lang: None,
            source: None,
            docstring: None,
            properties: props,
        };

        let json = serde_json::to_string(&node).unwrap();
        let parsed: CodeNode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.properties.get("visibility").unwrap(), "public");
        assert_eq!(parsed.properties.get("async").unwrap(), "true");
    }

    #[test]
    fn entity_kind_equality() {
        assert_eq!(EntityKind::Function, EntityKind::Function);
        assert_ne!(EntityKind::Function, EntityKind::Class);
    }

    #[test]
    fn edge_kind_equality() {
        assert_eq!(EdgeKind::Calls, EdgeKind::Calls);
        assert_ne!(EdgeKind::Calls, EdgeKind::Imports);
    }
}
