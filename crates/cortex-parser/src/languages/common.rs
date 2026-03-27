/// Shared extraction driver used by all language modules.
///
/// tree-sitter 0.25 `QueryMatches` does not implement `Iterator`, so we use
/// `while let Some(m) = matches.next()` throughout. Function node ranges
/// are stored as `tree_sitter::Range` (Copy) to avoid `Node<'_>` lifetime
/// issues across call-site boundaries.
use crate::parser_impl::ParseResult;
use cortex_core::{CodeEdge, CodeNode, EdgeKind, EntityKind, Language};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, StreamingIterator, Tree};

// ── id / text helpers ─────────────────────────────────────────────────────────

pub fn file_id(path: &Path) -> String {
    format!("file:{}", path.display())
}

pub fn entity_id(kind: &EntityKind, path: &Path, name: &str, line: u32) -> String {
    format!(
        "{}:{}:{}:{}",
        kind.cypher_label(),
        path.display(),
        name,
        line
    )
}

/// Internal: entity_id from precomputed path display string to avoid repeated allocations.
fn entity_id_display(kind: &EntityKind, path_display: &str, name: &str, line: u32) -> String {
    format!("{}:{}:{}:{}", kind.cypher_label(), path_display, name, line)
}

pub fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or_default()
}

// ── node / edge constructors ──────────────────────────────────────────────────

pub fn make_file_node(source: &str, path: &Path, lang: Language) -> CodeNode {
    let line_count = source.lines().count() as u32;
    CodeNode {
        id: file_id(path),
        kind: EntityKind::File,
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        path: Some(path.display().to_string()),
        line_number: Some(1),
        lang: Some(lang),
        source: None,
        docstring: None,
        properties: [("line_count".into(), line_count.to_string())]
            .into_iter()
            .collect(),
    }
}

/// Internal: use precomputed path_display for id and path field to avoid repeated allocations.
fn make_file_node_display(
    source: &str,
    path: &Path,
    path_display: &str,
    lang: Language,
) -> CodeNode {
    let line_count = source.lines().count() as u32;
    CodeNode {
        id: format!("file:{path_display}"),
        kind: EntityKind::File,
        name: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        path: Some(path_display.to_string()),
        line_number: Some(1),
        lang: Some(lang),
        source: None,
        docstring: None,
        properties: [("line_count".into(), line_count.to_string())]
            .into_iter()
            .collect(),
    }
}

fn truncate_source_snippet(source: &str, max_bytes: usize) -> String {
    if source.len() <= max_bytes {
        return source.to_string();
    }

    let cutoff = source
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|&index| index <= max_bytes)
        .last()
        .unwrap_or(0);
    source[..cutoff].to_string()
}

/// Inputs for [`make_entity_node_display`]; `path_display` is precomputed `path.display()` to avoid repeated allocations.
struct EntityNodeDisplay<'a> {
    kind: EntityKind,
    name: &'a str,
    path: &'a Path,
    path_display: &'a str,
    line: u32,
    lang: Language,
    source_snippet: Option<&'a str>,
    cyclomatic: Option<u32>,
}

/// Build entity node; `path_display` is precomputed path.display() to avoid repeated allocations in hot path.
fn make_entity_node_display(params: EntityNodeDisplay<'_>) -> CodeNode {
    let EntityNodeDisplay {
        kind,
        name,
        path,
        path_display,
        line,
        lang,
        source_snippet,
        cyclomatic,
    } = params;
    let mut props: HashMap<String, String> = HashMap::new();
    if let Some(cc) = cyclomatic {
        props.insert("cyclomatic_complexity".into(), cc.to_string());
    }
    let module_path = file_path_to_module_path(path);
    let qualified_name = if module_path.is_empty() {
        name.to_string()
    } else {
        format!("{module_path}::{name}")
    };
    props.insert("qualified_name".into(), qualified_name);
    CodeNode {
        id: entity_id_display(&kind, path_display, name, line),
        kind,
        name: name.to_string(),
        path: Some(path_display.to_string()),
        line_number: Some(line),
        lang: Some(lang),
        source: source_snippet.map(|s| truncate_source_snippet(s, 4096)),
        docstring: None,
        properties: props,
    }
}

fn file_path_to_module_path(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    for suffix in [".rs", ".py", ".ts", ".tsx", ".js", ".go", ".java"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
            break;
        }
    }
    s = s
        .replace("/mod", "")
        .replace("/__init__", "")
        .replace("/index", "");
    s.trim_matches('/').replace('/', "::")
}

fn contains_edge(from: &str, to: &str) -> CodeEdge {
    CodeEdge {
        from: from.to_string(),
        to: to.to_string(),
        kind: EdgeKind::Contains,
        properties: HashMap::new(),
    }
}

/// Strip language-specific import keywords and delimiters so that Java's
/// `import java.util.List;`, PHP's `use App\Models\User;`, etc. are
/// normalized to just the module path.
fn normalize_import_text(raw: &str) -> String {
    let s = raw
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('<')
        .trim_matches('>');
    // Strip leading keywords and trailing semicolons that leak through
    // when the tree-sitter query captures the whole declaration node.
    let s = s
        .trim_start_matches("import ")
        .trim_start_matches("package ")
        .trim_start_matches("use ")
        .trim_end_matches(';')
        .trim();
    // Java static imports: "static java.util.Collections.sort"
    let s = s.trim_start_matches("static ").trim();
    s.to_string()
}

fn imports_edge(from: &str, module: &str, line: u32) -> CodeEdge {
    CodeEdge {
        from: from.to_string(),
        to: format!("module:{}", module),
        kind: EdgeKind::Imports,
        properties: [
            ("module".into(), module.to_string()),
            ("line_number".into(), line.to_string()),
        ]
        .into_iter()
        .collect(),
    }
}

// ── capture-descriptor types ──────────────────────────────────────────────────

/// Maps capture indices produced by a *definition* query pattern.
pub struct DefCaptures {
    /// Capture index of the whole entity node (`@entity / @struct_entity / …`)
    pub entity: u32,
    /// Capture index of the name identifier (`@name`)
    pub name: u32,
    /// Entity kind this pattern represents
    pub kind: EntityKind,
}

pub struct CallCaptures {
    pub call: u32,
}

pub struct ImportCaptures {
    pub module: u32,
    /// Optional (capture_index, allowed_names) pair.
    /// When set, only matches whose captured text is in the allow-list are emitted.
    /// Used by Ruby to restrict imports to `require`/`require_relative`/`load`.
    pub method_filter: Option<(u32, &'static [&'static str])>,
}

pub struct InheritCaptures {
    pub child: u32,
    pub parent: u32,
    pub edge_kind: EdgeKind,
}

pub struct ParamCaptures {
    pub param: u32,
}

pub struct VariableCaptures {
    pub var: u32,
}

// ── unified extraction driver ─────────────────────────────────────────────────

/// Run definition, call, and import extraction in three passes and return
/// a fully-populated `ParseResult`.
///
/// Pass 1 — definitions: emits entity `CodeNode`s and `Contains` edges.
/// Pass 2 — calls: for each function range, restricts the call query to that
///           range with `QueryCursor::set_byte_range` (avoids `Node<'_>`
///           lifetime hazards).
/// Pass 3 — imports: emits `Imports` edges.
#[allow(clippy::too_many_arguments)]
pub fn extract_all(
    source: &str,
    path: &Path,
    lang: Language,
    tree: &Tree,
    def_query: &Query,
    def_capture_sets: &[DefCaptures],
    call_query: &Query,
    call_capture: &CallCaptures,
    import_query: &Query,
    import_capture: &ImportCaptures,
    inherit_query: Option<&Query>,
    inherit_capture: Option<&InheritCaptures>,
    param_query: Option<&Query>,
    param_capture: Option<&ParamCaptures>,
    variable_query: Option<&Query>,
    variable_capture: Option<&VariableCaptures>,
) -> ParseResult {
    let src = source.as_bytes();
    let root = tree.root_node();
    let path_display = path.display().to_string();
    let fid = format!("file:{path_display}");

    let mut nodes: Vec<CodeNode> = Vec::new();
    let mut edges: Vec<CodeEdge> = Vec::new();
    let mut seen_node_ids = HashSet::<String>::new();
    let mut seen_edge_ids = HashSet::<String>::new();
    let mut named_entities = HashMap::<String, String>::new();
    let mut imports = Vec::<String>::new();
    let mut calls = Vec::<String>::new();

    // ── pass 1: definitions ───────────────────────────────────────────────────
    // Collect (node_id, byte_range) for function nodes so we can scope call
    // queries in pass 2 without holding `Node<'_>` across the loop.
    let mut fn_ranges: Vec<(String, std::ops::Range<usize>)> = Vec::new();

    {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(def_query, root, src);
        while let Some(match_) = matches.next() {
            let mut caps = HashMap::with_capacity(match_.captures.len());
            for c in match_.captures.iter() {
                caps.insert(c.index, c.node);
            }

            for dc in def_capture_sets {
                let Some(entity_node) = caps.get(&dc.entity).copied() else {
                    continue;
                };
                let Some(name_node) = caps.get(&dc.name).copied() else {
                    continue;
                };

                let name = node_text(name_node, src).trim().to_string();
                if name.is_empty() {
                    continue;
                }
                let line = entity_node.start_position().row as u32 + 1;
                let snippet = node_text(entity_node, src);
                let cc = matches!(dc.kind, EntityKind::Function)
                    .then(|| cortex_core::compute_cyclomatic_complexity(snippet));

                let node = make_entity_node_display(EntityNodeDisplay {
                    kind: dc.kind.clone(),
                    name: &name,
                    path,
                    path_display: &path_display,
                    line,
                    lang,
                    source_snippet: Some(snippet),
                    cyclomatic: cc,
                });
                let nid = node.id.clone();
                if seen_edge_ids.insert(format!("{fid}|CONTAINS|{nid}")) {
                    edges.push(contains_edge(&fid, &nid));
                }
                if matches!(
                    dc.kind,
                    EntityKind::Class
                        | EntityKind::Struct
                        | EntityKind::Trait
                        | EntityKind::Interface
                        | EntityKind::Enum
                ) {
                    named_entities.insert(name.clone(), nid.clone());
                }

                if matches!(dc.kind, EntityKind::Function) {
                    fn_ranges.push((nid, entity_node.byte_range()));
                }
                if seen_node_ids.insert(node.id.clone()) {
                    nodes.push(node);
                }
                break; // one DefCaptures set matched — move to next query match
            }
        }
    }

    // Build a name → node_id map for all functions defined in this file so
    // same-file call targets can be resolved to real node IDs.
    let local_fn_ids: HashMap<String, String> = nodes
        .iter()
        .filter(|n| matches!(n.kind, EntityKind::Function))
        .map(|n| (n.name.clone(), n.id.clone()))
        .collect();

    // ── pass 2: calls scoped to each function's byte range ───────────────────
    for (fn_id, range) in &fn_ranges {
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(range.clone());
        let mut matches = cursor.matches(call_query, root, src);
        while let Some(m) = matches.next() {
            let mut caps = HashMap::with_capacity(m.captures.len());
            for c in m.captures.iter() {
                caps.insert(c.index, c.node);
            }
            let Some(call_node) = caps.get(&call_capture.call).copied() else {
                continue;
            };
            let callee = node_text(call_node, src).trim().to_string();
            if callee.is_empty() {
                continue;
            }
            calls.push(callee.clone());
            let line = call_node.start_position().row as u32 + 1;
            // Resolve same-file calls to the actual function node ID; cross-file
            // calls use the symbolic `call_target:<name>` placeholder (resolved
            // later by the indexer's post-processing pass or graph queries).
            let to_id = local_fn_ids
                .get(&callee)
                .cloned()
                .unwrap_or_else(|| format!("call_target:{callee}"));
            let mut props = HashMap::new();
            props.insert("callee_name".into(), callee.clone());
            props.insert("line_number".into(), line.to_string());
            let e = CodeEdge {
                from: fn_id.clone(),
                to: to_id,
                kind: EdgeKind::Calls,
                properties: props,
            };
            let edge_key = format!("{}|CALLS|{}|{}", e.from, e.to, line);
            if seen_edge_ids.insert(edge_key) {
                edges.push(e);
            }
        }
    }

    // ── pass 3: imports ───────────────────────────────────────────────────────
    {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(import_query, root, src);
        while let Some(m) = matches.next() {
            let mut caps = HashMap::with_capacity(m.captures.len());
            for c in m.captures.iter() {
                caps.insert(c.index, c.node);
            }
            let Some(mod_node) = caps.get(&import_capture.module).copied() else {
                continue;
            };

            // If a method filter is configured (e.g. Ruby), only accept
            // matches whose method capture text is in the allow-list.
            if let Some((method_idx, allowed)) = import_capture.method_filter {
                let ok = caps
                    .get(&method_idx)
                    .map(|n| {
                        let name = node_text(*n, src).trim();
                        allowed.contains(&name)
                    })
                    .unwrap_or(false);
                if !ok {
                    continue;
                }
            }

            let raw = node_text(mod_node, src).trim();
            let module = normalize_import_text(raw);
            if module.is_empty() {
                continue;
            }
            imports.push(module.clone());
            let line = mod_node.start_position().row as u32 + 1;
            let e = imports_edge(&fid, &module, line);
            let edge_key = format!("{}|IMPORTS|{}|{}", e.from, e.to, line);
            if seen_edge_ids.insert(edge_key) {
                edges.push(e);
            }
        }
    }

    // ── pass 4: inheritance / implementation relationships ───────────────────
    if let (Some(inherit_q), Some(inherit_cap)) = (inherit_query, inherit_capture) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(inherit_q, root, src);
        while let Some(m) = matches.next() {
            let mut caps = HashMap::with_capacity(m.captures.len());
            for c in m.captures.iter() {
                caps.insert(c.index, c.node);
            }
            let Some(child_node) = caps.get(&inherit_cap.child).copied() else {
                continue;
            };
            let Some(parent_node) = caps.get(&inherit_cap.parent).copied() else {
                continue;
            };
            let child_name = node_text(child_node, src).trim().to_string();
            let parent_name = node_text(parent_node, src).trim().to_string();
            if child_name.is_empty() || parent_name.is_empty() {
                continue;
            }
            let Some(from_id) = named_entities.get(&child_name).cloned() else {
                continue;
            };
            let to_id = named_entities
                .get(&parent_name)
                .cloned()
                .unwrap_or_else(|| format!("call_target:{parent_name}"));
            let edge_key = format!(
                "{}|{}|{}",
                from_id,
                inherit_cap.edge_kind.cypher_rel_type(),
                to_id
            );
            if !seen_edge_ids.insert(edge_key) {
                continue;
            }
            edges.push(CodeEdge {
                from: from_id,
                to: to_id,
                kind: inherit_cap.edge_kind.clone(),
                properties: HashMap::new(),
            });
        }
    }

    // ── pass 5: function parameters ───────────────────────────────────────────
    if let (Some(param_q), Some(param_cap)) = (param_query, param_capture) {
        for (fn_id, range) in &fn_ranges {
            let mut cursor = QueryCursor::new();
            cursor.set_byte_range(range.clone());
            let mut matches = cursor.matches(param_q, root, src);
            while let Some(m) = matches.next() {
                let mut caps = HashMap::with_capacity(m.captures.len());
                for c in m.captures.iter() {
                    caps.insert(c.index, c.node);
                }
                let Some(param_node) = caps.get(&param_cap.param).copied() else {
                    continue;
                };
                let param_name = node_text(param_node, src).trim().to_string();
                if param_name.is_empty() {
                    continue;
                }
                let line = param_node.start_position().row as u32 + 1;
                let param_id = format!("{fn_id}:param:{param_name}:{line}");
                if seen_node_ids.insert(param_id.clone()) {
                    nodes.push(CodeNode {
                        id: param_id.clone(),
                        kind: EntityKind::Parameter,
                        name: param_name.clone(),
                        path: Some(path_display.clone()),
                        line_number: Some(line),
                        lang: Some(lang),
                        source: None,
                        docstring: None,
                        properties: HashMap::new(),
                    });
                    if seen_edge_ids.insert(format!("{fid}|CONTAINS|{param_id}")) {
                        edges.push(contains_edge(&fid, &param_id));
                    }
                }
                let has_param_key = format!("{fn_id}|HAS_PARAMETER|{param_id}");
                if seen_edge_ids.insert(has_param_key) {
                    edges.push(CodeEdge {
                        from: fn_id.clone(),
                        to: param_id,
                        kind: EdgeKind::HasParameter,
                        properties: HashMap::new(),
                    });
                }
            }
        }
    }

    // ── pass 6: variable extraction ───────────────────────────────────────────
    if let (Some(var_q), Some(var_cap)) = (variable_query, variable_capture) {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(var_q, root, src);
        while let Some(m) = matches.next() {
            let mut caps = HashMap::with_capacity(m.captures.len());
            for c in m.captures.iter() {
                caps.insert(c.index, c.node);
            }
            let Some(var_node) = caps.get(&var_cap.var).copied() else {
                continue;
            };
            let var_name = node_text(var_node, src).trim().to_string();
            if var_name.is_empty() {
                continue;
            }
            let line = var_node.start_position().row as u32 + 1;
            let var_id = format!("var:{}:{}:{}", path_display, var_name, line);
            if seen_node_ids.insert(var_id.clone()) {
                nodes.push(CodeNode {
                    id: var_id.clone(),
                    kind: EntityKind::Variable,
                    name: var_name,
                    path: Some(path_display.clone()),
                    line_number: Some(line),
                    lang: Some(lang),
                    source: None,
                    docstring: None,
                    properties: HashMap::new(),
                });
                if seen_edge_ids.insert(format!("{fid}|CONTAINS|{var_id}")) {
                    edges.push(contains_edge(&fid, &var_id));
                }
            }
        }
    }

    // Prepend file node
    let mut all_nodes = vec![make_file_node_display(source, path, &path_display, lang)];
    all_nodes.extend(nodes);
    ParseResult {
        nodes: all_nodes,
        edges,
        imports,
        calls,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_import_text, truncate_source_snippet};

    #[test]
    fn truncate_source_snippet_preserves_utf8_boundaries() {
        let source = "€".repeat(2000);

        let truncated = truncate_source_snippet(&source, 4096);

        assert!(truncated.len() <= 4096);
        assert!(truncated.chars().all(|c| c == '€'));
        assert_eq!(truncated.len() % '€'.len_utf8(), 0);
    }

    #[test]
    fn normalize_import_strips_java_import() {
        assert_eq!(
            normalize_import_text("import java.util.List;"),
            "java.util.List"
        );
    }

    #[test]
    fn normalize_import_strips_java_package() {
        assert_eq!(normalize_import_text("package com.example;"), "com.example");
    }

    #[test]
    fn normalize_import_strips_java_static_import() {
        assert_eq!(
            normalize_import_text("import static java.util.Collections.sort;"),
            "java.util.Collections.sort"
        );
    }

    #[test]
    fn normalize_import_strips_php_use() {
        assert_eq!(
            normalize_import_text("use App\\Models\\User;"),
            "App\\Models\\User"
        );
    }

    #[test]
    fn normalize_import_strips_quotes() {
        assert_eq!(normalize_import_text("\"lodash\""), "lodash");
        assert_eq!(normalize_import_text("'express'"), "express");
    }

    #[test]
    fn normalize_import_noop_for_clean_module() {
        assert_eq!(normalize_import_text("os.path"), "os.path");
    }
}
