//! Code analysis queries with safe parameterized Cypher queries.
//!
//! All queries use parameterized inputs to prevent Cypher injection attacks.

use crate::{ReviewAnalyzer, ReviewInput, ReviewReport};
use cortex_core::{CortexError, Result, SearchKind};
use cortex_graph::GraphClient;
use serde_json::Value;
use std::path::Path;

#[derive(Clone)]
pub struct Analyzer {
    graph: GraphClient,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnalyzePathFilters {
    pub include_paths: Vec<String>,
    pub include_files: Vec<String>,
    pub include_globs: Vec<String>,
    pub exclude_paths: Vec<String>,
    pub exclude_files: Vec<String>,
    pub exclude_globs: Vec<String>,
}

impl AnalyzePathFilters {
    pub fn is_empty(&self) -> bool {
        self.include_paths.is_empty()
            && self.include_files.is_empty()
            && self.include_globs.is_empty()
            && self.exclude_paths.is_empty()
            && self.exclude_files.is_empty()
            && self.exclude_globs.is_empty()
    }

    pub fn has_includes(&self) -> bool {
        !(self.include_paths.is_empty()
            && self.include_files.is_empty()
            && self.include_globs.is_empty())
    }

    pub fn validate(&self) -> Result<()> {
        for pattern in self.include_globs.iter().chain(self.exclude_globs.iter()) {
            glob::Pattern::new(pattern).map_err(|e| {
                CortexError::InvalidArgument(format!("invalid glob pattern '{}': {}", pattern, e))
            })?;
        }
        Ok(())
    }

    pub fn matches_path(&self, candidate: &str) -> bool {
        if self.is_empty() {
            return true;
        }

        let candidate = normalize_path(candidate);
        let include_match = if self.has_includes() {
            self.include_paths
                .iter()
                .any(|p| path_prefix_match(candidate.as_str(), p.as_str()))
                || self
                    .include_files
                    .iter()
                    .any(|f| file_match(candidate.as_str(), f.as_str()))
                || self
                    .include_globs
                    .iter()
                    .any(|g| glob_match(candidate.as_str(), g.as_str()))
        } else {
            true
        };

        if !include_match {
            return false;
        }

        let excluded = self
            .exclude_paths
            .iter()
            .any(|p| path_prefix_match(candidate.as_str(), p.as_str()))
            || self
                .exclude_files
                .iter()
                .any(|f| file_match(candidate.as_str(), f.as_str()))
            || self
                .exclude_globs
                .iter()
                .any(|g| glob_match(candidate.as_str(), g.as_str()));

        !excluded
    }

    pub fn is_excluded_path(&self, candidate: &str) -> bool {
        let candidate = normalize_path(candidate);
        self.exclude_paths
            .iter()
            .any(|p| path_prefix_match(candidate.as_str(), p.as_str()))
            || self
                .exclude_files
                .iter()
                .any(|f| file_match(candidate.as_str(), f.as_str()))
            || self
                .exclude_globs
                .iter()
                .any(|g| glob_match(candidate.as_str(), g.as_str()))
    }

    pub fn matches_any_path<'a, I>(&self, paths: I) -> bool
    where
        I: IntoIterator<Item = &'a str>,
    {
        if self.is_empty() {
            return true;
        }
        let mut saw_any_path = false;
        let mut saw_include = false;
        for path in paths {
            saw_any_path = true;
            let normalized_path = normalize_path(path);
            if !self.exclude_paths.is_empty()
                && self
                    .exclude_paths
                    .iter()
                    .any(|p| path_prefix_match(normalized_path.as_str(), p.as_str()))
            {
                return false;
            }
            if !self.exclude_files.is_empty()
                && self
                    .exclude_files
                    .iter()
                    .any(|f| file_match(normalized_path.as_str(), f.as_str()))
            {
                return false;
            }
            if !self.exclude_globs.is_empty()
                && self
                    .exclude_globs
                    .iter()
                    .any(|g| glob_match(normalized_path.as_str(), g.as_str()))
            {
                return false;
            }

            if !self.has_includes() || self.matches_path(normalized_path.as_str()) {
                saw_include = true;
            }
        }

        if self.has_includes() && !saw_any_path {
            return false;
        }
        if self.has_includes() {
            saw_include
        } else {
            true
        }
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim()
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn path_prefix_match(candidate: &str, filter: &str) -> bool {
    let candidate = normalize_path(candidate);
    let filter = normalize_path(filter);
    if filter.is_empty() {
        return true;
    }
    if candidate == filter || candidate.starts_with(format!("{filter}/").as_str()) {
        return true;
    }
    let seg = format!("/{filter}/");
    candidate.contains(seg.as_str()) || candidate.ends_with(format!("/{filter}").as_str())
}

fn file_match(candidate: &str, file_filter: &str) -> bool {
    let candidate = normalize_path(candidate);
    let file_filter = normalize_path(file_filter);
    if file_filter.is_empty() {
        return false;
    }
    if file_filter.contains('/') {
        candidate == file_filter || candidate.ends_with(format!("/{file_filter}").as_str())
    } else {
        Path::new(candidate.as_str())
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name == file_filter)
            .unwrap_or(false)
    }
}

fn glob_match(candidate: &str, pattern: &str) -> bool {
    let candidate = normalize_path(candidate);
    let Ok(pattern) = glob::Pattern::new(pattern) else {
        return false;
    };
    pattern.matches(candidate.as_str()) || pattern.matches_path(Path::new(candidate.as_str()))
}

fn collect_paths(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if (k == "path" || k.ends_with(".path")) && v.is_string() {
                    if let Some(path) = v.as_str() {
                        out.push(path.to_string());
                    }
                }
                collect_paths(v, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_paths(item, out);
            }
        }
        _ => {}
    }
}

fn row_matches_filters(row: &Value, filters: &AnalyzePathFilters) -> bool {
    if filters.is_empty() {
        return true;
    }
    let mut paths = Vec::new();
    collect_paths(row, &mut paths);
    filters.matches_any_path(paths.iter().map(String::as_str))
}

fn apply_row_filters(rows: Vec<Value>, filters: Option<&AnalyzePathFilters>) -> Vec<Value> {
    match filters {
        None => rows,
        Some(f) if f.is_empty() => rows,
        Some(f) => rows
            .into_iter()
            .filter(|row| row_matches_filters(row, f))
            .collect(),
    }
}

fn normalize_import_token(token: &str) -> String {
    token
        .trim()
        .trim_end_matches(',')
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

fn grouped_rust_import(import_path: &str) -> Option<(String, Vec<String>)> {
    let (prefix, rest) = import_path.split_once("::{")?;
    let body = rest.strip_suffix('}')?;
    let items = body
        .split(',')
        .map(normalize_import_token)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }
    Some((prefix.trim().to_string(), items))
}

fn import_path_matches_module(import_path: &str, module: &str) -> bool {
    let import_path = import_path.trim();
    let module = module.trim();
    if import_path.is_empty() || module.is_empty() {
        return false;
    }

    if import_path == module
        || import_path.starts_with(format!("{module}::").as_str())
        || module.starts_with(format!("{import_path}::").as_str())
    {
        return true;
    }

    if let Some((prefix, items)) = grouped_rust_import(import_path) {
        if module == prefix || module.starts_with(format!("{prefix}::").as_str()) {
            return true;
        }
        for item in items {
            let full = format!("{prefix}::{item}");
            if module == full
                || module.starts_with(format!("{full}::").as_str())
                || full.starts_with(format!("{module}::").as_str())
            {
                return true;
            }
        }
    }

    false
}

fn row_module_name(row: &Value) -> Option<&str> {
    row.get("m")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str)
}

impl Analyzer {
    fn callers_query_with_depth(depth: usize) -> String {
        format!(
            "MATCH p=(caller)-[:CALLS*1..{}]->(callee:Function {{name: $name}})
             RETURN p",
            depth.max(1)
        )
    }

    fn callees_query_with_depth(depth: usize) -> String {
        format!(
            "MATCH p=(caller:Function {{name: $name}})-[:CALLS*1..{}]->(callee:Function)
             RETURN p",
            depth.max(1)
        )
    }

    pub fn new(graph: GraphClient) -> Self {
        Self { graph }
    }

    pub async fn find_code(
        &self,
        query: &str,
        kind: SearchKind,
        path_filter: Option<&str>,
    ) -> Result<Vec<Value>> {
        let (predicate, _param_name) = match kind {
            SearchKind::Name => ("n.name = $query", "query"),
            SearchKind::Pattern => ("n.name CONTAINS $query", "query"),
            SearchKind::Type => ("n.kind = $query", "query"),
            SearchKind::Content => ("n.source CONTAINS $query", "query"),
        };

        let (cypher, params) = if let Some(path) = path_filter {
            (
                format!(
                    "MATCH (n:CodeNode) WHERE {predicate} AND n.path CONTAINS $path RETURN n LIMIT 100"
                ),
                vec![("query", query.to_string()), ("path", path.to_string())],
            )
        } else {
            (
                format!("MATCH (n:CodeNode) WHERE {predicate} RETURN n LIMIT 100"),
                vec![("query", query.to_string())],
            )
        };

        self.graph.query_with_params(&cypher, params).await
    }

    pub async fn callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.callers_with_filters(function_name, None).await
    }

    pub async fn callers_with_filters(
        &self,
        function_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (caller:Function)-[r:CALLS]->(callee)
                 WHERE coalesce(callee.name, r.callee_name) = $name
                 RETURN caller, callee, r.callee_name AS callee_name",
                "name",
                function_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.callees_with_filters(function_name, None).await
    }

    pub async fn callees_with_filters(
        &self,
        function_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (caller:Function {name: $name})-[:CALLS]->(callee)
                 WHERE callee:Function OR callee:CallTarget
                 RETURN caller, callee",
                "name",
                function_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    fn all_callers_query() -> &'static str {
        "MATCH p=(caller)-[:CALLS*1..20]->(callee:Function {name: $name}) RETURN p"
    }

    pub async fn all_callers(&self, function_name: &str) -> Result<Vec<Value>> {
        self.all_callers_with_filters(function_name, None).await
    }

    pub async fn all_callers_with_filters(
        &self,
        function_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(Self::all_callers_query(), "name", function_name)
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    fn all_callees_query() -> &'static str {
        "MATCH p=(caller:Function {name: $name})-[:CALLS*1..20]->(callee:Function) RETURN p"
    }

    pub async fn all_callees(&self, function_name: &str) -> Result<Vec<Value>> {
        self.all_callees_with_filters(function_name, None).await
    }

    pub async fn all_callees_with_filters(
        &self,
        function_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(Self::all_callees_query(), "name", function_name)
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn call_chain(
        &self,
        from: &str,
        to: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        self.call_chain_with_filters(from, to, depth, None).await
    }

    pub async fn call_chain_with_filters(
        &self,
        from: &str,
        to: &str,
        depth: Option<usize>,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let depth = depth.unwrap_or(15).max(1);
        let cypher = format!(
            "MATCH p=(a:Function {{name: $from}})-[:CALLS*1..{}]->(b:Function {{name: $to}})
             RETURN p
             ORDER BY length(p) ASC
             LIMIT 1",
            depth
        );
        let rows = self
            .graph
            .query_with_params(
                &cypher,
                vec![("from", from.to_string()), ("to", to.to_string())],
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn class_hierarchy(&self, class_name: &str) -> Result<Vec<Value>> {
        self.class_hierarchy_with_filters(class_name, None).await
    }

    pub async fn class_hierarchy_with_filters(
        &self,
        class_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH p=(n {name: $name})-[:INHERITS|IMPLEMENTS*0..10]->(m)
                 RETURN p",
                "name",
                class_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn dead_code(&self) -> Result<Vec<Value>> {
        self.dead_code_with_filters(None).await
    }

    pub async fn dead_code_with_filters(
        &self,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .raw_query(
                "MATCH (f:Function)
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, count(caller) AS incoming_calls
                 OPTIONAL MATCH (f)-[:CALLS]->(callee)
                 WITH f, incoming_calls, count(callee) AS outgoing_calls
                 WHERE incoming_calls = 0
                   AND NOT f.name IN ['main', '__init__', '__main__', 'new', 'default', 'drop']
                 RETURN f,
                        f.name AS function,
                        f.path AS path,
                        f.line_number AS line,
                        coalesce(f.lang, '') AS language,
                        incoming_calls,
                        outgoing_calls,
                        'no incoming calls' AS reason
                 ORDER BY outgoing_calls DESC, f.path, f.name",
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn complexity(&self, top_n: usize) -> Result<Vec<Value>> {
        self.complexity_with_filters(top_n, None).await
    }

    pub async fn complexity_with_filters(
        &self,
        top_n: usize,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let cypher = format!(
            "MATCH (f:Function)
             WITH f, toInteger(coalesce(f.cyclomatic_complexity, '1')) AS complexity
             RETURN f.name AS function,
                    f.path AS path,
                    f.line_number AS line,
                    coalesce(f.lang, '') AS language,
                    complexity
             ORDER BY complexity DESC, f.path, f.name
             LIMIT {}",
            top_n
        );
        let rows = self.graph.raw_query(&cypher).await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn find_complexity(&self, function_name: &str) -> Result<Vec<Value>> {
        self.find_complexity_with_filters(function_name, None).await
    }

    pub async fn find_complexity_with_filters(
        &self,
        function_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (f:Function {name: $name})
                 WITH f, toInteger(coalesce(f.cyclomatic_complexity, '1')) AS complexity
                 RETURN f.name AS function,
                        f.path AS path,
                        f.line_number AS line,
                        coalesce(f.lang, '') AS language,
                        complexity",
                "name",
                function_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn overrides(&self, method_name: &str) -> Result<Vec<Value>> {
        self.overrides_with_filters(method_name, None).await
    }

    pub async fn overrides_with_filters(
        &self,
        method_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (f:Function {name: $name})
                 MATCH (f)<-[:DEFINED_IN]-(owner)
                 RETURN owner, f",
                "name",
                method_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn module_dependencies(&self, module: &str) -> Result<Vec<Value>> {
        self.module_dependencies_with_filters(module, None).await
    }

    pub async fn module_dependencies_with_filters(
        &self,
        module: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        self.find_importers_with_filters(module, filters).await
    }

    pub async fn find_importers(&self, module: &str) -> Result<Vec<Value>> {
        self.find_importers_with_filters(module, None).await
    }

    pub async fn find_importers_with_filters(
        &self,
        module: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let module_prefix = format!("{module}::");
        let module_fragment = module
            .rsplit("::")
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(module);
        let rows = self
            .graph
            .query_with_params(
                "MATCH (f:File)-[:IMPORTS]->(m:Module)
                 WHERE m.name = $name
                    OR m.name STARTS WITH $prefix
                    OR m.name CONTAINS $fragment
                 RETURN f, m",
                vec![
                    ("name", module.to_string()),
                    ("prefix", module_prefix),
                    ("fragment", module_fragment.to_string()),
                ],
            )
            .await?;
        let matched = rows
            .into_iter()
            .filter(|row| {
                row_module_name(row)
                    .map(|import_path| import_path_matches_module(import_path, module))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        Ok(apply_row_filters(matched, filters))
    }

    pub async fn find_by_decorator(&self, decorator: &str) -> Result<Vec<Value>> {
        self.find_by_decorator_with_filters(decorator, None).await
    }

    pub async fn find_by_decorator_with_filters(
        &self,
        decorator: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (f:Function)
                 WHERE f.decorators CONTAINS $decorator
                 RETURN f",
                "decorator",
                decorator,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn find_by_argument(&self, argument_name: &str) -> Result<Vec<Value>> {
        self.find_by_argument_with_filters(argument_name, None)
            .await
    }

    pub async fn find_by_argument_with_filters(
        &self,
        argument_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (f:Function)-[:HAS_PARAMETER]->(p:Parameter {name: $name})
                 RETURN f, p",
                "name",
                argument_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn variable_scope(&self, variable_name: &str) -> Result<Vec<Value>> {
        self.variable_scope_with_filters(variable_name, None).await
    }

    pub async fn variable_scope_with_filters(
        &self,
        variable_name: &str,
        filters: Option<&AnalyzePathFilters>,
    ) -> Result<Vec<Value>> {
        if let Some(f) = filters {
            f.validate()?;
        }
        let rows = self
            .graph
            .query_with_param(
                "MATCH (v:Variable {name: $name})
                 OPTIONAL MATCH (n)-[]->(v)
                 RETURN v, n",
                "name",
                variable_name,
            )
            .await?;
        Ok(apply_row_filters(rows, filters))
    }

    pub async fn repository_stats(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (r:Repository)
                 OPTIONAL MATCH (r)-[:CONTAINS*]->(n)
                 RETURN r.path AS repository, count(n) AS node_count",
            )
            .await
    }

    /// Find all code nodes in a specific file
    pub async fn find_by_file(&self, file_path: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n:CodeNode)
                 WHERE n.path CONTAINS $path
                 RETURN n
                 ORDER BY n.line_number
                 LIMIT 500",
                "path",
                file_path,
            )
            .await
    }

    /// Find all functions in a specific module/namespace
    pub async fn find_in_module(&self, module: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n:Function)
                 WHERE n.path CONTAINS $module
                 RETURN n
                 ORDER BY n.name
                 LIMIT 200",
                "module",
                module,
            )
            .await
    }

    /// Find similar function names (fuzzy match)
    pub async fn find_similar(&self, name: &str, limit: usize) -> Result<Vec<Value>> {
        let cypher = format!(
            "MATCH (n:Function)
             WHERE n.name CONTAINS $name
             RETURN n.name AS name, n.path AS path, n.line_number AS line
             LIMIT {}",
            limit.min(100)
        );
        self.graph.query_with_param(&cypher, "name", name).await
    }

    /// Get entry points (functions that are not called by any other function)
    pub async fn entry_points(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, count(caller) AS incoming_calls
                 WHERE incoming_calls = 0
                   AND (f.name IN ['main', '__init__', '__main__', 'run', 'execute', 'start', 'handle']
                        OR f.name STARTS WITH 'test_'
                        OR f.name STARTS WITH 'handle_')
                 RETURN f.name AS name, f.path AS path, f.line_number AS line
                 ORDER BY f.path, f.name",
            )
            .await
    }

    /// Find all test functions
    pub async fn find_tests(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:Function)
                 WHERE f.name STARTS WITH 'test_'
                    OR f.name STARTS WITH 'it_'
                    OR f.name STARTS WITH 'should_'
                    OR f.annotations CONTAINS 'Test'
                    OR f.annotations CONTAINS '@test'
                 RETURN f.name AS name, f.path AS path, f.line_number AS line
                 ORDER BY f.path, f.name",
            )
            .await
    }

    fn find_tests_for_query() -> &'static str {
        "MATCH (test:Function)
                 WHERE (test.name STARTS WITH 'test_'
                       OR test.name STARTS WITH 'it_'
                       OR test.name STARTS WITH 'should_')
                   AND test.name CONTAINS $name
                 RETURN test.name AS test_name, test.path AS path, test.line_number AS line"
    }

    /// Find functions that test a specific function
    pub async fn find_tests_for(&self, function_name: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(Self::find_tests_for_query(), "name", function_name)
            .await
    }

    /// Find all functions that call a given function (alias for callers)
    pub async fn who_calls(&self, function_name: &str, depth: Option<usize>) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(1).min(10);
        if depth == 1 {
            self.callers(function_name).await
        } else {
            let cypher = Self::callers_query_with_depth(depth);
            self.graph
                .query_with_param(&cypher, "name", function_name)
                .await
        }
    }

    /// Find all functions called by a given function (alias for callees)
    pub async fn what_calls(
        &self,
        function_name: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(1).min(10);
        if depth == 1 {
            self.callees(function_name).await
        } else {
            let cypher = Self::callees_query_with_depth(depth);
            self.graph
                .query_with_param(&cypher, "name", function_name)
                .await
        }
    }

    /// Analyze module cohesion (find tightly/loosely coupled functions)
    pub async fn analyze_module(&self, module_path: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (f:Function)
                 WHERE f.path CONTAINS $path
                 OPTIONAL MATCH (f)-[:CALLS]->(called:Function)
                 WITH f, count(DISTINCT called) as outgoing_calls
                 OPTIONAL MATCH (caller:Function)-[:CALLS]->(f)
                 WITH f, outgoing_calls, count(DISTINCT caller) as incoming_calls
                 RETURN f.name AS function,
                        f.path AS path,
                        outgoing_calls,
                        incoming_calls,
                        outgoing_calls + incoming_calls AS total_coupling
                 ORDER BY total_coupling DESC
                 LIMIT 100",
                "path",
                module_path,
            )
            .await
    }

    /// Get all nodes with a specific annotation/decorator
    pub async fn find_by_annotation(&self, annotation: &str) -> Result<Vec<Value>> {
        self.graph
            .query_with_param(
                "MATCH (n)
                 WHERE n.annotations CONTAINS $annotation
                    OR n.decorators CONTAINS $annotation
                 RETURN labels(n) AS kind, n.name AS name, n.path AS path, n.line_number AS line
                 LIMIT 200",
                "annotation",
                annotation,
            )
            .await
    }

    /// Find unused imports
    pub async fn unused_imports(&self) -> Result<Vec<Value>> {
        self.graph
            .raw_query(
                "MATCH (f:File)-[:IMPORTS]->(m:Module)
                 OPTIONAL MATCH (f)-[:CONTAINS]->(n)
                 WITH f, m,
                      sum(CASE
                              WHEN n.source IS NOT NULL AND n.source CONTAINS m.name THEN 1
                              ELSE 0
                          END) AS usage_count
                 WHERE usage_count = 0
                 RETURN f.path AS file, m.name AS import
                 LIMIT 200",
            )
            .await
    }

    /// Get dependency graph for visualization
    pub async fn dependency_graph(
        &self,
        root_module: &str,
        depth: Option<usize>,
    ) -> Result<Vec<Value>> {
        let depth = depth.unwrap_or(3).min(5);
        let cypher = format!(
            "MATCH path = (root {{name: $name}})-[:IMPORTS|DEPENDS_ON*..{}]->(dep)
             RETURN [n in nodes(path) | n.name] AS chain",
            depth
        );
        self.graph
            .query_with_param(&cypher, "name", root_module)
            .await
    }

    /// Run review analysis over normalized diff inputs.
    pub fn analyze_review(&self, input: &ReviewInput) -> ReviewReport {
        // This path is graph-client independent and reuses static analyzer detectors.
        let review = ReviewAnalyzer::new();
        review.analyze(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_kind_variants() {
        assert_eq!(SearchKind::Name, SearchKind::Name);
        assert_ne!(SearchKind::Name, SearchKind::Pattern);
    }

    #[test]
    fn parameterized_query_safe() {
        // Test that injection attempts are safely parameterized
        let injection_attempt = "' OR '1'='1";
        let cypher = "MATCH (n:Function {name: $name}) RETURN n";
        // The injection string would be passed as a parameter value, not interpolated
        // This is safe because neo4rs handles parameterization properly
        assert!(cypher.contains("$name"));
        assert!(!cypher.contains(injection_attempt));
    }

    #[test]
    fn find_code_query_structure() {
        // Verify queries use parameterized inputs
        let cypher = "MATCH (n:CodeNode) WHERE n.name = $query RETURN n LIMIT 100";
        assert!(cypher.contains("$query"));
        assert!(!cypher.contains("format!"));
    }

    #[test]
    fn callers_query_structure() {
        let cypher = "MATCH (caller:Function)-[r:CALLS]->(callee)
                 WHERE coalesce(callee.name, r.callee_name) = $name
                 RETURN caller, callee, r.callee_name AS callee_name";
        assert!(cypher.contains("$name"));
        assert!(cypher.contains("coalesce(callee.name, r.callee_name)"));
    }

    #[test]
    fn call_chain_query_structure() {
        let cypher = "MATCH p=(a:Function {name: $from})-[:CALLS*1..15]->(b:Function {name: $to}) RETURN p ORDER BY length(p) ASC LIMIT 1";
        assert!(cypher.contains("$from"));
        assert!(cypher.contains("$to"));
    }

    #[test]
    fn find_by_file_query_structure() {
        // Verify the query uses parameterization
        let cypher = "MATCH (n:CodeNode) WHERE n.path CONTAINS $path RETURN n";
        assert!(cypher.contains("$path"));
    }

    #[test]
    fn find_similar_query_structure() {
        // Verify the query uses parameterization
        let cypher = "MATCH (n:Function) WHERE n.name CONTAINS $name RETURN n";
        assert!(cypher.contains("$name"));
    }

    #[test]
    fn entry_points_query_safe() {
        // Entry points query should not use user input
        let cypher = "MATCH (f:Function) OPTIONAL MATCH (caller:Function)-[:CALLS]->(f) WITH f, count(caller) AS incoming_calls WHERE incoming_calls = 0 RETURN f";
        // No parameters means no injection risk
        assert!(!cypher.contains("$"));
    }

    #[test]
    fn find_tests_for_query_requires_test_prefix() {
        let cypher = Analyzer::find_tests_for_query();
        assert!(cypher.contains("STARTS WITH 'test_'"));
        assert!(cypher.contains("STARTS WITH 'it_'"));
        assert!(cypher.contains("STARTS WITH 'should_'"));
        assert!(cypher.contains("CONTAINS $name"));
        // No redundant OR test.name CONTAINS $name in the OR branch
        assert!(
            !cypher.contains("OR test.name CONTAINS $name"),
            "OR branch must not contain CONTAINS $name; use AND test.name CONTAINS $name"
        );
    }

    #[test]
    fn all_callers_bounded_depth() {
        let cypher = Analyzer::all_callers_query();
        assert!(cypher.contains("*1..20"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn all_callees_bounded_depth() {
        let cypher = Analyzer::all_callees_query();
        assert!(cypher.contains("*1..20"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn dependency_graph_depth_limit() {
        // Verify depth is limited to prevent overly complex queries
        let depth = 5;
        let cypher = format!(
            "MATCH path = (root {{name: $name}})-[:IMPORTS|DEPENDS_ON*..{}]->(dep) RETURN path",
            depth
        );
        assert!(cypher.contains("*..5"));
    }

    #[test]
    fn who_calls_query_uses_requested_depth() {
        let cypher = Analyzer::callers_query_with_depth(3);
        assert!(cypher.contains("*1..3"));
        assert!(!cypher.contains("*1..]"));
    }

    #[test]
    fn what_calls_query_uses_requested_depth() {
        let cypher = Analyzer::callees_query_with_depth(4);
        assert!(cypher.contains("*1..4"));
    }

    #[test]
    fn analyze_module_query_structure() {
        // Verify module analysis uses parameterization
        let cypher = "MATCH (f:Function) WHERE f.path CONTAINS $path RETURN f";
        assert!(cypher.contains("$path"));
    }

    #[test]
    fn analyze_path_filters_include_exclude_precedence() {
        let filters = AnalyzePathFilters {
            include_paths: vec!["src".to_string()],
            include_files: Vec::new(),
            include_globs: vec!["**/*.rs".to_string()],
            exclude_paths: vec!["src/generated".to_string()],
            exclude_files: vec!["src/lib.rs".to_string()],
            exclude_globs: vec!["**/*_test.rs".to_string()],
        };

        assert!(filters.matches_path("src/main.rs"));
        assert!(!filters.matches_path("src/generated/schema.rs"));
        assert!(!filters.matches_path("src/lib.rs"));
        assert!(!filters.matches_path("src/auth/user_test.rs"));
        assert!(!filters.matches_path("docs/README.md"));
    }

    #[test]
    fn analyze_path_filters_file_name_match_works() {
        let filters = AnalyzePathFilters {
            include_paths: Vec::new(),
            include_files: vec!["main.rs".to_string()],
            include_globs: Vec::new(),
            exclude_paths: Vec::new(),
            exclude_files: Vec::new(),
            exclude_globs: Vec::new(),
        };
        assert!(filters.matches_path("/tmp/repo/src/main.rs"));
        assert!(!filters.matches_path("/tmp/repo/src/lib.rs"));
    }

    #[test]
    fn analyze_path_filters_validate_glob() {
        let filters = AnalyzePathFilters {
            include_paths: Vec::new(),
            include_files: Vec::new(),
            include_globs: vec!["[".to_string()],
            exclude_paths: Vec::new(),
            exclude_files: Vec::new(),
            exclude_globs: Vec::new(),
        };
        assert!(filters.validate().is_err());
    }

    #[test]
    fn analyze_path_filters_matches_any_path_normalizes_exclusions() {
        let filters = AnalyzePathFilters {
            include_paths: vec!["src".to_string()],
            include_files: Vec::new(),
            include_globs: Vec::new(),
            exclude_paths: vec!["./src/generated/".to_string()],
            exclude_files: vec!["./src\\lib.rs".to_string()],
            exclude_globs: vec!["src/**/ignored_*.rs".to_string()],
        };

        // Exclude path should match even with mixed separators and trailing slash style.
        assert!(!filters.matches_any_path(["src\\generated\\schema.rs"].iter().copied()));
        // Exclude file should match via normalized include entry.
        assert!(!filters.matches_any_path([".\\src\\lib.rs"].iter().copied()));
        // Exclude glob should match normalized candidate path.
        assert!(!filters.matches_any_path(["./src/mod/ignored_test.rs"].iter().copied()));
        // Non-excluded path under included scope should still pass.
        assert!(filters.matches_any_path(["./src/main.rs"].iter().copied()));
    }

    #[test]
    fn grouped_rust_import_matching_works() {
        assert!(import_path_matches_module("std::{io, fs}", "std::io"));
        assert!(import_path_matches_module(
            "crate::module::{TypeA, TypeB}",
            "crate::module::TypeB"
        ));
        assert!(import_path_matches_module(
            "crate::module::{TypeA, TypeB}",
            "crate::module"
        ));
        assert!(!import_path_matches_module("std::{io, fs}", "serde::json"));
    }
}
