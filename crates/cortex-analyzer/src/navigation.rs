use cortex_core::Result;
use cortex_graph::GraphClient;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionResult {
    pub name: String,
    pub qualified_name: Option<String>,
    pub kind: String,
    pub file_path: String,
    pub line_number: u32,
    pub confidence: DefinitionConfidence,
    pub source_preview: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionConfidence {
    Exact,
    ImportResolved,
    SameModule,
    NameOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResult {
    pub symbol_name: String,
    pub usage_kind: UsageKind,
    pub file_path: String,
    pub line_number: u32,
    pub context_name: String,
    pub source_snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    Call,
    Import,
    TypeReference,
    FieldAccess,
    Inheritance,
    Implementation,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickInfo {
    pub name: String,
    pub qualified_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub defined_in: DefinitionLocation,
    pub visibility: Option<String>,
    pub language: Option<String>,
    pub metrics: QuickInfoMetrics,
    pub parent_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinitionLocation {
    pub file_path: String,
    pub line_number: u32,
    pub module_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickInfoMetrics {
    pub usage_count: u32,
    pub caller_count: u32,
    pub callee_count: u32,
    pub complexity: Option<u32>,
    pub line_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchStructuralDiff {
    pub source_branch: String,
    pub target_branch: String,
    pub repository_path: String,
    pub added_symbols: Vec<SymbolDiffEntry>,
    pub removed_symbols: Vec<SymbolDiffEntry>,
    pub modified_symbols: Vec<ModifiedSymbolEntry>,
    pub impact: Vec<ImpactEntry>,
    pub summary: StructuralDiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDiffEntry {
    pub name: String,
    pub qualified_name: Option<String>,
    pub kind: String,
    pub file_path: String,
    pub line_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedSymbolEntry {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub source_line: u32,
    pub target_line: u32,
    pub change_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEntry {
    pub changed_symbol: String,
    pub affected_symbol: String,
    pub affected_file: String,
    pub relationship: String,
    pub impact_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralDiffSummary {
    pub total_added: usize,
    pub total_removed: usize,
    pub total_modified: usize,
    pub total_affected_callers: usize,
    pub affected_files: usize,
}

pub struct NavigationEngine {
    graph: GraphClient,
    repository_path: String,
    branch: Option<String>,
}

impl NavigationEngine {
    pub fn new(graph: GraphClient, repository_path: String, branch: Option<String>) -> Self {
        Self {
            graph,
            repository_path,
            branch,
        }
    }

    pub async fn go_to_definition(
        &self,
        symbol: &str,
        from_file: &str,
        _from_line: Option<u32>,
    ) -> Result<Vec<DefinitionResult>> {
        let mut results = self.find_by_qualified_name(symbol).await?;
        if !results.is_empty() {
            return Ok(results);
        }
        if !from_file.is_empty() {
            results = self.resolve_via_imports(symbol, from_file).await?;
            if !results.is_empty() {
                return Ok(results);
            }
            results = self.find_in_same_module(symbol, from_file).await?;
            if !results.is_empty() {
                return Ok(results);
            }
        }
        self.find_by_name_global(symbol).await
    }

    pub async fn find_usages(
        &self,
        symbol: &str,
        kind_filter: Option<UsageKind>,
    ) -> Result<Vec<UsageResult>> {
        let mut usages = Vec::new();
        if kind_filter.is_none() || kind_filter == Some(UsageKind::Call) {
            usages.extend(
                self.find_usages_for_rel(symbol, "CALLS", UsageKind::Call)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::Import) {
            usages.extend(
                self.find_usages_for_rel(symbol, "IMPORTS", UsageKind::Import)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::TypeReference) {
            usages.extend(
                self.find_usages_for_rel(symbol, "TYPE_REFERENCE", UsageKind::TypeReference)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::FieldAccess) {
            usages.extend(
                self.find_usages_for_rel(symbol, "FIELD_ACCESS", UsageKind::FieldAccess)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::Inheritance) {
            usages.extend(
                self.find_usages_for_rel(symbol, "INHERITS", UsageKind::Inheritance)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::Implementation) {
            usages.extend(
                self.find_usages_for_rel(symbol, "IMPLEMENTS", UsageKind::Implementation)
                    .await?,
            );
        }
        if kind_filter.is_none() || kind_filter == Some(UsageKind::Reference) {
            usages.extend(
                self.find_usages_for_rels(symbol, &["REFERENCES", "USES"], UsageKind::Reference)
                    .await?,
            );
        }
        dedup_usages(usages)
    }

    pub async fn quick_info(&self, symbol: &str) -> Result<Vec<QuickInfo>> {
        let branch_clause = self.branch_where("n");
        let cypher = format!(
            "MATCH (n:CodeNode {{name: $symbol}})
             WHERE n.repository_path = $repo
               {branch_clause}
             OPTIONAL MATCH (caller)-[:CALLS]->(n)
             WITH n, count(DISTINCT caller) AS caller_count
             OPTIONAL MATCH (n)-[:CALLS]->(callee)
             WITH n, caller_count, count(DISTINCT callee) AS callee_count
             OPTIONAL MATCH (user)-[:TYPE_REFERENCE|REFERENCES|USES|IMPORTS]->(n)
             WITH n, caller_count, callee_count, count(DISTINCT user) AS usage_count
             OPTIONAL MATCH (n)-[:MEMBER_OF]->(parent)
             RETURN n.name AS name,
                    n.qualified_name AS qualified_name,
                    n.kind AS kind,
                    coalesce(n.source, '') AS source,
                    n.docstring AS docstring,
                    n.path AS path,
                    n.line_number AS line,
                    coalesce(n.lang, '') AS language,
                    n.visibility AS visibility,
                    n.cyclomatic_complexity AS complexity,
                    caller_count,
                    callee_count,
                    usage_count,
                    parent.name AS parent_type
             ORDER BY n.path
             LIMIT 10"
        );
        let rows = self.query_symbol(&cypher, symbol).await?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                let source = r.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let signature = extract_signature_from_source(source);
                Some(QuickInfo {
                    name: r.get("name")?.as_str()?.to_string(),
                    qualified_name: r
                        .get("qualified_name")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    kind: r.get("kind")?.as_str()?.to_string(),
                    signature,
                    docstring: r
                        .get("docstring")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    defined_in: DefinitionLocation {
                        file_path: r.get("path")?.as_str()?.to_string(),
                        line_number: r.get("line")?.as_u64()? as u32,
                        module_path: r
                            .get("qualified_name")
                            .and_then(|v| v.as_str())
                            .and_then(|qn| qn.rsplit_once("::"))
                            .map(|(module, _)| module.to_string()),
                    },
                    visibility: r
                        .get("visibility")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    language: r.get("language").and_then(|v| v.as_str()).map(String::from),
                    metrics: QuickInfoMetrics {
                        usage_count: r.get("usage_count").and_then(|v| v.as_u64()).unwrap_or(0)
                            as u32,
                        caller_count: r.get("caller_count").and_then(|v| v.as_u64()).unwrap_or(0)
                            as u32,
                        callee_count: r.get("callee_count").and_then(|v| v.as_u64()).unwrap_or(0)
                            as u32,
                        complexity: r
                            .get("complexity")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32),
                        line_count: Some(source.lines().count() as u32),
                    },
                    parent_type: r
                        .get("parent_type")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })
            })
            .collect())
    }

    pub async fn branch_structural_diff(
        &self,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<BranchStructuralDiff> {
        let added = self
            .find_branch_only_symbols(source_branch, target_branch)
            .await?;
        let removed = self
            .find_branch_only_symbols(target_branch, source_branch)
            .await?;
        let modified = self
            .find_modified_symbols(source_branch, target_branch)
            .await?;

        let changed_symbols: Vec<&str> = modified
            .iter()
            .map(|m| m.name.as_str())
            .chain(removed.iter().map(|r| r.name.as_str()))
            .collect();

        let impact = self
            .find_affected_by_changes(&changed_symbols, target_branch)
            .await?;

        let affected_files: HashSet<&str> =
            impact.iter().map(|i| i.affected_file.as_str()).collect();
        let summary = StructuralDiffSummary {
            total_added: added.len(),
            total_removed: removed.len(),
            total_modified: modified.len(),
            total_affected_callers: impact.len(),
            affected_files: affected_files.len(),
        };

        Ok(BranchStructuralDiff {
            source_branch: source_branch.to_string(),
            target_branch: target_branch.to_string(),
            repository_path: self.repository_path.clone(),
            added_symbols: added,
            removed_symbols: removed,
            modified_symbols: modified,
            impact,
            summary,
        })
    }

    async fn find_by_qualified_name(&self, qualified_name: &str) -> Result<Vec<DefinitionResult>> {
        let branch_clause = self.branch_where("n");
        let cypher = format!(
            "MATCH (n:CodeNode)
             WHERE n.repository_path = $repo
               AND n.qualified_name = $symbol
               {branch_clause}
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line,
                    substring(coalesce(n.source, ''), 0, 200) AS preview
             LIMIT 10"
        );
        let rows = self.query_symbol(&cypher, qualified_name).await?;
        Ok(rows
            .iter()
            .filter_map(|r| self.parse_definition(r, DefinitionConfidence::Exact))
            .collect())
    }

    async fn resolve_via_imports(
        &self,
        symbol: &str,
        from_file: &str,
    ) -> Result<Vec<DefinitionResult>> {
        let branch_clause = self.branch_where("target");
        let cypher = format!(
            "MATCH (f:File {{path: $file_path}})-[:IMPORTS]->(m)
             WHERE f.repository_path = $repo
             WITH m.name AS imported_module
             MATCH (target:CodeNode {{name: $symbol}})
             WHERE target.repository_path = $repo
               AND target.path CONTAINS imported_module
               {branch_clause}
             RETURN target.name AS name, target.qualified_name AS qualified_name,
                    target.kind AS kind, target.path AS path,
                    target.line_number AS line,
                    substring(coalesce(target.source, ''), 0, 200) AS preview
             LIMIT 10"
        );
        let mut params = vec![
            ("repo", self.repository_path.clone()),
            ("file_path", from_file.to_string()),
            ("symbol", symbol.to_string()),
        ];
        if let Some(branch) = &self.branch {
            params.push(("branch", branch.clone()));
        }
        let rows = self.graph.query_with_params(&cypher, params).await?;
        Ok(rows
            .iter()
            .filter_map(|r| self.parse_definition(r, DefinitionConfidence::ImportResolved))
            .collect())
    }

    async fn find_in_same_module(
        &self,
        symbol: &str,
        from_file: &str,
    ) -> Result<Vec<DefinitionResult>> {
        let module_dir = from_file.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
        let branch_clause = self.branch_where("n");
        let cypher = format!(
            "MATCH (n:CodeNode {{name: $symbol}})
             WHERE n.repository_path = $repo
               AND n.path STARTS WITH $module_dir
               {branch_clause}
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line,
                    substring(coalesce(n.source, ''), 0, 200) AS preview
             ORDER BY n.path
             LIMIT 20"
        );
        let mut params = vec![
            ("repo", self.repository_path.clone()),
            ("symbol", symbol.to_string()),
            ("module_dir", module_dir.to_string()),
        ];
        if let Some(branch) = &self.branch {
            params.push(("branch", branch.clone()));
        }
        let rows = self.graph.query_with_params(&cypher, params).await?;
        Ok(rows
            .iter()
            .filter_map(|r| self.parse_definition(r, DefinitionConfidence::SameModule))
            .collect())
    }

    async fn find_by_name_global(&self, symbol: &str) -> Result<Vec<DefinitionResult>> {
        let branch_clause = self.branch_where("n");
        let cypher = format!(
            "MATCH (n:CodeNode {{name: $symbol}})
             WHERE n.repository_path = $repo
               {branch_clause}
               AND n.kind IN ['FUNCTION', 'METHOD', 'CLASS', 'STRUCT', 'TRAIT',
                              'INTERFACE', 'ENUM', 'TYPE_ALIAS', 'CONSTANT', 'VARIABLE']
             RETURN n.name AS name, n.qualified_name AS qualified_name, n.kind AS kind,
                    n.path AS path, n.line_number AS line,
                    substring(coalesce(n.source, ''), 0, 200) AS preview
             ORDER BY n.path
             LIMIT 20"
        );
        let rows = self.query_symbol(&cypher, symbol).await?;
        Ok(rows
            .iter()
            .filter_map(|r| self.parse_definition(r, DefinitionConfidence::NameOnly))
            .collect())
    }

    async fn find_usages_for_rel(
        &self,
        symbol: &str,
        relationship: &str,
        usage_kind: UsageKind,
    ) -> Result<Vec<UsageResult>> {
        self.find_usages_for_rels(symbol, &[relationship], usage_kind)
            .await
    }

    async fn find_usages_for_rels(
        &self,
        symbol: &str,
        relationships: &[&str],
        usage_kind: UsageKind,
    ) -> Result<Vec<UsageResult>> {
        let rels = relationships.join("|");
        let branch_clause = self.branch_where("source");
        let cypher = format!(
            "MATCH (source)-[:{rels}]->(target {{name: $symbol}})
             WHERE source.repository_path = $repo
               {branch_clause}
             RETURN source.name AS context_name, source.path AS file_path,
                    source.line_number AS line,
                    substring(coalesce(source.source, ''), 0, 150) AS snippet
             ORDER BY source.path, source.line_number
             LIMIT 500"
        );
        let rows = self.query_symbol(&cypher, symbol).await?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                Some(UsageResult {
                    symbol_name: symbol.to_string(),
                    usage_kind,
                    file_path: r.get("file_path")?.as_str()?.to_string(),
                    line_number: r.get("line")?.as_u64()? as u32,
                    context_name: r.get("context_name")?.as_str()?.to_string(),
                    source_snippet: r.get("snippet").and_then(|v| v.as_str()).map(String::from),
                })
            })
            .collect())
    }

    async fn find_branch_only_symbols(
        &self,
        branch_a: &str,
        branch_b: &str,
    ) -> Result<Vec<SymbolDiffEntry>> {
        let cypher = "MATCH (a:CodeNode)
             WHERE a.repository_path = $repo
               AND a.branch = $branch_a
               AND a.kind IN ['FUNCTION', 'METHOD', 'CLASS', 'STRUCT', 'TRAIT', 'ENUM']
               AND NOT EXISTS {
                 MATCH (b:CodeNode {name: a.name, kind: a.kind, branch: $branch_b})
                 WHERE b.repository_path = $repo
               }
             RETURN a.name AS name, a.qualified_name AS qualified_name,
                    a.kind AS kind, a.path AS path, a.line_number AS line
             ORDER BY a.path, a.name
             LIMIT 500";
        let rows = self
            .graph
            .query_with_params(
                cypher,
                vec![
                    ("repo", self.repository_path.clone()),
                    ("branch_a", branch_a.to_string()),
                    ("branch_b", branch_b.to_string()),
                ],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                Some(SymbolDiffEntry {
                    name: r.get("name")?.as_str()?.to_string(),
                    qualified_name: r
                        .get("qualified_name")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    kind: r.get("kind")?.as_str()?.to_string(),
                    file_path: r.get("path")?.as_str()?.to_string(),
                    line_number: r.get("line")?.as_u64()? as u32,
                })
            })
            .collect())
    }

    async fn find_modified_symbols(
        &self,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<Vec<ModifiedSymbolEntry>> {
        let cypher = "MATCH (s:CodeNode)
             WHERE s.repository_path = $repo
               AND s.branch = $source
               AND s.kind IN ['FUNCTION', 'METHOD', 'CLASS', 'STRUCT']
             MATCH (t:CodeNode {name: s.name, kind: s.kind, branch: $target})
             WHERE t.repository_path = $repo
               AND s.source <> t.source
             RETURN s.name AS name, s.kind AS kind, s.path AS path,
                    s.line_number AS source_line, t.line_number AS target_line,
                    s.source AS source_code, t.source AS target_code
             ORDER BY s.path, s.name
             LIMIT 200";
        let rows = self
            .graph
            .query_with_params(
                cypher,
                vec![
                    ("repo", self.repository_path.clone()),
                    ("source", source_branch.to_string()),
                    ("target", target_branch.to_string()),
                ],
            )
            .await?;
        Ok(rows
            .iter()
            .filter_map(|r| {
                let source_code = r.get("source_code").and_then(|v| v.as_str()).unwrap_or("");
                let target_code = r.get("target_code").and_then(|v| v.as_str()).unwrap_or("");
                let source_sig = extract_signature_from_source(source_code);
                let target_sig = extract_signature_from_source(target_code);
                let change_type = if source_sig != target_sig {
                    "signature"
                } else {
                    "body"
                };
                Some(ModifiedSymbolEntry {
                    name: r.get("name")?.as_str()?.to_string(),
                    kind: r.get("kind")?.as_str()?.to_string(),
                    file_path: r.get("path")?.as_str()?.to_string(),
                    source_line: r.get("source_line")?.as_u64()? as u32,
                    target_line: r.get("target_line")?.as_u64()? as u32,
                    change_type: change_type.to_string(),
                })
            })
            .collect())
    }

    async fn find_affected_by_changes(
        &self,
        symbol_names: &[&str],
        on_branch: &str,
    ) -> Result<Vec<ImpactEntry>> {
        if symbol_names.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_impacts = Vec::new();

        // Generate static parameter names once to avoid leaking memory
        // and because we can't use lazy_static without importing it.
        // We can just use an array of static strings.
        const PARAM_NAMES: [&str; 100] = [
            "sym_0", "sym_1", "sym_2", "sym_3", "sym_4", "sym_5", "sym_6", "sym_7", "sym_8", "sym_9",
            "sym_10", "sym_11", "sym_12", "sym_13", "sym_14", "sym_15", "sym_16", "sym_17", "sym_18", "sym_19",
            "sym_20", "sym_21", "sym_22", "sym_23", "sym_24", "sym_25", "sym_26", "sym_27", "sym_28", "sym_29",
            "sym_30", "sym_31", "sym_32", "sym_33", "sym_34", "sym_35", "sym_36", "sym_37", "sym_38", "sym_39",
            "sym_40", "sym_41", "sym_42", "sym_43", "sym_44", "sym_45", "sym_46", "sym_47", "sym_48", "sym_49",
            "sym_50", "sym_51", "sym_52", "sym_53", "sym_54", "sym_55", "sym_56", "sym_57", "sym_58", "sym_59",
            "sym_60", "sym_61", "sym_62", "sym_63", "sym_64", "sym_65", "sym_66", "sym_67", "sym_68", "sym_69",
            "sym_70", "sym_71", "sym_72", "sym_73", "sym_74", "sym_75", "sym_76", "sym_77", "sym_78", "sym_79",
            "sym_80", "sym_81", "sym_82", "sym_83", "sym_84", "sym_85", "sym_86", "sym_87", "sym_88", "sym_89",
            "sym_90", "sym_91", "sym_92", "sym_93", "sym_94", "sym_95", "sym_96", "sym_97", "sym_98", "sym_99",
        ];

        // Process in chunks to avoid queries with too many parameters
        for chunk in symbol_names.chunks(100) {
            // Build the WHERE IN clause dynamically since query_with_params doesn't support list directly
            let mut cypher =
                "MATCH (caller)-[r:CALLS|IMPORTS|INHERITS|IMPLEMENTS|TYPE_REFERENCE]->(target)
                 WHERE caller.repository_path = $repo
                   AND caller.branch = $branch
                   AND target.name IN ["
                    .to_string();

            let mut params = vec![
                ("repo", self.repository_path.clone()),
                ("branch", on_branch.to_string()),
            ];

            let mut param_markers = Vec::with_capacity(chunk.len());
            for i in 0..chunk.len() {
                param_markers.push(format!("${}", PARAM_NAMES[i]));
            }
            cypher.push_str(&param_markers.join(", "));
            cypher.push_str(
                "]
                 RETURN caller.name AS affected_symbol, caller.path AS affected_file,
                        type(r) AS relationship, target.name AS changed_symbol
                 ORDER BY caller.path
                 LIMIT 1000",
            );

            for (i, &name) in chunk.iter().enumerate() {
                params.push((PARAM_NAMES[i], name.to_string()));
            }

            let rows = self.graph.query_with_params(&cypher, params).await?;
            all_impacts.extend(rows.iter().filter_map(|r| {
                Some(ImpactEntry {
                    changed_symbol: r.get("changed_symbol")?.as_str()?.to_string(),
                    affected_symbol: r.get("affected_symbol")?.as_str()?.to_string(),
                    affected_file: r.get("affected_file")?.as_str()?.to_string(),
                    relationship: r.get("relationship")?.as_str()?.to_string(),
                    impact_level: "direct".to_string(),
                })
            }));
        }

        Ok(all_impacts)
    }

    fn parse_definition(
        &self,
        row: &Value,
        confidence: DefinitionConfidence,
    ) -> Option<DefinitionResult> {
        Some(DefinitionResult {
            name: row.get("name")?.as_str()?.to_string(),
            qualified_name: row
                .get("qualified_name")
                .and_then(|v| v.as_str())
                .map(String::from),
            kind: row.get("kind")?.as_str()?.to_string(),
            file_path: row.get("path")?.as_str()?.to_string(),
            line_number: row.get("line")?.as_u64()? as u32,
            confidence,
            source_preview: row
                .get("preview")
                .and_then(|v| v.as_str())
                .map(String::from),
        })
    }

    fn branch_where(&self, node_var: &str) -> String {
        match &self.branch {
            Some(_) => format!("AND {node_var}.branch = $branch"),
            None => String::new(),
        }
    }

    async fn query_symbol(&self, cypher: &str, symbol: &str) -> Result<Vec<Value>> {
        let mut params = vec![
            ("repo", self.repository_path.clone()),
            ("symbol", symbol.to_string()),
        ];
        if let Some(branch) = &self.branch {
            params.push(("branch", branch.clone()));
        }
        self.graph.query_with_params(cypher, params).await
    }
}

fn dedup_usages(mut usages: Vec<UsageResult>) -> Result<Vec<UsageResult>> {
    usages.sort_by(|a, b| {
        a.file_path
            .cmp(&b.file_path)
            .then_with(|| a.line_number.cmp(&b.line_number))
            .then_with(|| format!("{:?}", a.usage_kind).cmp(&format!("{:?}", b.usage_kind)))
    });
    usages.dedup_by(|a, b| {
        a.file_path == b.file_path
            && a.line_number == b.line_number
            && a.usage_kind == b.usage_kind
            && a.symbol_name == b.symbol_name
    });
    Ok(usages)
}

pub fn extract_signature_from_source(source: &str) -> Option<String> {
    let first_line = source.lines().find(|l| !l.trim().is_empty())?;
    if first_line.trim().is_empty() {
        return None;
    }
    let mut sig = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        sig.push_str(trimmed);
        sig.push(' ');
        if line.contains('{') || trimmed.ends_with(':') {
            break;
        }
    }
    let sig = sig.split('{').next().unwrap_or(&sig).trim().to_string();
    if sig.is_empty() { None } else { Some(sig) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_signature() {
        let src = "fn parse(input: &str) -> Result<()> {\n  Ok(())\n}";
        let sig = extract_signature_from_source(src);
        assert_eq!(sig.as_deref(), Some("fn parse(input: &str) -> Result<()>"));
    }

    #[test]
    fn test_extract_signature_multiline() {
        let src = "fn parse(\n    input: &str,\n    mode: Mode,\n) -> Result<()> {\n  Ok(())\n}";
        let sig = extract_signature_from_source(src).expect("signature");
        assert!(sig.contains("fn parse("));
        assert!(sig.contains("mode: Mode"));
        assert!(sig.contains("-> Result<()>"));
    }

    #[test]
    fn test_dedup_usages() {
        let usages = vec![
            UsageResult {
                symbol_name: "parse".to_string(),
                usage_kind: UsageKind::Call,
                file_path: "a.rs".to_string(),
                line_number: 10,
                context_name: "x".to_string(),
                source_snippet: None,
            },
            UsageResult {
                symbol_name: "parse".to_string(),
                usage_kind: UsageKind::Call,
                file_path: "a.rs".to_string(),
                line_number: 10,
                context_name: "x".to_string(),
                source_snippet: None,
            },
        ];
        let deduped = dedup_usages(usages).expect("dedup should succeed");
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn test_dedup_usages_keeps_different_kinds() {
        let usages = vec![
            UsageResult {
                symbol_name: "parse".to_string(),
                usage_kind: UsageKind::Call,
                file_path: "a.rs".to_string(),
                line_number: 10,
                context_name: "x".to_string(),
                source_snippet: None,
            },
            UsageResult {
                symbol_name: "parse".to_string(),
                usage_kind: UsageKind::TypeReference,
                file_path: "a.rs".to_string(),
                line_number: 10,
                context_name: "x".to_string(),
                source_snippet: None,
            },
        ];
        let deduped = dedup_usages(usages).expect("dedup should succeed");
        assert_eq!(deduped.len(), 2);
    }
}
