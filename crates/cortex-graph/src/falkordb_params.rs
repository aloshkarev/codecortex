//! FalkorDB Cypher parameter bridging.
//!
//! The official `falkordb` Rust client exposes `with_params` as `HashMap<String, String>`
//! (`CYPHER key=value` prefixes). Map-typed parameters are unreliable in the protocol, so
//! bulk `UNWIND $batch` writes inline the list-of-maps literal into the query text.

use std::collections::HashMap;

/// Typed Cypher query parameter for FalkorDB inline batch substitution.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphParam {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<GraphParam>),
    Map(HashMap<String, GraphParam>),
}

/// Escape a string for use inside single-quoted Cypher literals.
pub fn escape_cypher_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Render a [`GraphParam`] as a Cypher literal (for inline batch substitution).
pub fn query_param_to_cypher_literal(param: &GraphParam) -> String {
    match param {
        GraphParam::Null => "null".to_string(),
        GraphParam::Bool(b) => b.to_string(),
        GraphParam::Int(i) => i.to_string(),
        GraphParam::Float(f) => f.to_string(),
        GraphParam::String(s) => format!("'{}'", escape_cypher_string(s)),
        GraphParam::List(items) => {
            let inner: Vec<String> = items.iter().map(query_param_to_cypher_literal).collect();
            format!("[{}]", inner.join(", "))
        }
        GraphParam::Map(m) => {
            let pairs: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}: {}", k, query_param_to_cypher_literal(v)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
    }
}

/// Scalar params for `CYPHER key=value` prefixes (non-batch).
pub fn query_param_to_cypher_scalar(param: &GraphParam) -> String {
    match param {
        GraphParam::String(s) => format!("'{}'", escape_cypher_string(s)),
        GraphParam::Int(i) => i.to_string(),
        GraphParam::Float(f) => f.to_string(),
        GraphParam::Bool(b) => b.to_string(),
        GraphParam::Null => "null".to_string(),
        other => query_param_to_cypher_literal(other),
    }
}

/// Build FalkorDB `CYPHER` prefix params and optionally rewrite `UNWIND $batch` to an inline list.
pub fn prepare_cypher_query(
    cypher: &str,
    params: HashMap<String, GraphParam>,
) -> (String, HashMap<String, String>) {
    let mut cypher = cypher.to_string();
    let mut string_params = HashMap::new();
    let mut inlined_keys = Vec::new();

    for (key, value) in &params {
        if let GraphParam::List(_) = value {
            let placeholder = format!("${key}");
            if cypher.contains(&placeholder) {
                let literal = query_param_to_cypher_literal(value);
                cypher = cypher.replace(&placeholder, &literal);
                inlined_keys.push(key.clone());
            }
        }
    }

    for (key, value) in params {
        if inlined_keys.iter().any(|k| k == &key) {
            continue;
        }
        string_params.insert(key, query_param_to_cypher_scalar(&value));
    }

    (cypher, string_params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_string_quoted_for_cypher_prefix() {
        let mut params = HashMap::new();
        params.insert(
            "repo".to_string(),
            GraphParam::String("/tmp/my/repo".to_string()),
        );
        let (_, remaining) = prepare_cypher_query("RETURN 1", params);
        assert_eq!(
            remaining.get("repo").map(String::as_str),
            Some("'/tmp/my/repo'")
        );
    }

    #[test]
    fn inline_paths_replaces_dollar_paths() {
        let mut params = HashMap::new();
        params.insert(
            "paths".to_string(),
            GraphParam::List(vec![
                GraphParam::String("a.rs".to_string()),
                GraphParam::String("b.rs".to_string()),
            ]),
        );
        let (cypher, remaining) = prepare_cypher_query("UNWIND $paths AS path RETURN path", params);
        assert!(cypher.contains("'a.rs'"));
        assert!(!cypher.contains("$paths"));
        assert!(remaining.is_empty());
    }

    #[test]
    fn inline_batch_replaces_dollar_batch() {
        let mut params = HashMap::new();
        let mut item = HashMap::new();
        item.insert("id".to_string(), GraphParam::String("n1".to_string()));
        item.insert("name".to_string(), GraphParam::String("main".to_string()));
        params.insert(
            "batch".to_string(),
            GraphParam::List(vec![GraphParam::Map(item)]),
        );

        let (cypher, remaining) =
            prepare_cypher_query("UNWIND $batch AS item RETURN item.id", params);
        assert!(cypher.starts_with("UNWIND [{"));
        assert!(cypher.contains("id: 'n1'"));
        assert!(cypher.contains("name: 'main'"));
        assert!(!cypher.contains("$batch"));
        assert!(remaining.is_empty());
    }
}
