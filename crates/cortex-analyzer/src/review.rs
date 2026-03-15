use crate::{
    AnalyzePathFilters, CodeSmell, RefactoringEngine, RefactoringRecommendation, Severity,
    SmellDetector,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewLineRange {
    pub start_line: u32,
    pub end_line: u32,
}

impl ReviewLineRange {
    pub fn contains(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewFileInput {
    pub path: String,
    pub source: String,
    pub changed_ranges: Vec<ReviewLineRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewInput {
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub filters: AnalyzePathFilters,
    pub min_severity: Severity,
    pub max_findings: Option<usize>,
    pub files: Vec<ReviewFileInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewSmellFinding {
    pub file_path: String,
    pub line_number: u32,
    pub severity: Severity,
    pub smell_type: String,
    pub symbol_name: String,
    pub message: String,
    pub in_changed_lines: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewRefactorFinding {
    pub file_path: String,
    pub line_number: u32,
    pub priority: String,
    pub technique: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewSummary {
    pub files_considered: usize,
    pub files_analyzed: usize,
    pub smell_findings_total: usize,
    pub smell_findings_returned: usize,
    pub refactoring_total: usize,
    pub refactoring_returned: usize,
    pub by_severity: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewReport {
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub summary: ReviewSummary,
    pub smells: Vec<ReviewSmellFinding>,
    pub refactorings: Vec<ReviewRefactorFinding>,
}

#[derive(Debug, Default, Clone)]
pub struct ReviewAnalyzer {
    smell_detector: SmellDetector,
    refactoring_engine: RefactoringEngine,
}

impl ReviewAnalyzer {
    pub fn new() -> Self {
        Self {
            smell_detector: SmellDetector::new(),
            refactoring_engine: RefactoringEngine::new(),
        }
    }

    pub fn analyze(&self, input: &ReviewInput) -> ReviewReport {
        let files_considered = input.files.len();
        let mut files_analyzed = 0usize;
        let mut smells = Vec::new();
        let mut canonical_smells = Vec::new();

        for file in &input.files {
            if !input.filters.matches_path(&file.path) {
                continue;
            }

            files_analyzed += 1;
            let detected = self.smell_detector.detect(&file.source, &file.path);
            let filtered = self.filter_smells_for_review(&detected, file, input.min_severity);
            canonical_smells.extend(filtered.iter().map(|(smell, _)| smell.clone()));
            smells.extend(filtered.into_iter().map(|(_, finding)| finding));
        }

        smells.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.line_number.cmp(&b.line_number))
        });

        let smells_total = smells.len();
        let mut smells = if let Some(limit) = input.max_findings {
            if smells.len() > limit {
                smells.into_iter().take(limit).collect()
            } else {
                smells
            }
        } else {
            smells
        };

        let mut by_severity = BTreeMap::new();
        for smell in &smells {
            *by_severity
                .entry(smell.severity.to_string())
                .or_insert(0usize) += 1;
        }

        let recommendations = self.refactoring_engine.prioritize(
            self.refactoring_engine
                .generate_recommendations(&canonical_smells),
        );
        let refactoring_total = recommendations.len();

        let mut location_by_smell: HashMap<String, (String, u32)> = HashMap::new();
        for smell in &canonical_smells {
            location_by_smell
                .entry(smell.smell_type.to_string())
                .or_insert((smell.file_path.clone(), smell.line_number));
        }

        let mut refactorings = recommendations
            .iter()
            .map(|rec| Self::to_refactor_finding(rec, &location_by_smell))
            .collect::<Vec<_>>();
        refactorings.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.line_number.cmp(&b.line_number))
                .then_with(|| a.technique.cmp(&b.technique))
        });

        if let Some(limit) = input.max_findings {
            if refactorings.len() > limit {
                refactorings.truncate(limit);
            }
            if smells.len() > limit {
                smells.truncate(limit);
            }
        }

        ReviewReport {
            base_ref: input.base_ref.clone(),
            head_ref: input.head_ref.clone(),
            summary: ReviewSummary {
                files_considered,
                files_analyzed,
                smell_findings_total: smells_total,
                smell_findings_returned: smells.len(),
                refactoring_total,
                refactoring_returned: refactorings.len(),
                by_severity,
            },
            smells,
            refactorings,
        }
    }

    fn filter_smells_for_review(
        &self,
        smells: &[CodeSmell],
        file: &ReviewFileInput,
        min_severity: Severity,
    ) -> Vec<(CodeSmell, ReviewSmellFinding)> {
        smells
            .iter()
            .filter(|smell| smell.severity >= min_severity)
            .filter_map(|smell| {
                let in_changed_lines = if file.changed_ranges.is_empty() {
                    true
                } else {
                    file.changed_ranges
                        .iter()
                        .any(|r| r.contains(smell.line_number))
                };
                if !in_changed_lines {
                    return None;
                }
                Some((
                    smell.clone(),
                    ReviewSmellFinding {
                        file_path: smell.file_path.clone(),
                        line_number: smell.line_number,
                        severity: smell.severity,
                        smell_type: smell.smell_type.to_string(),
                        symbol_name: smell.symbol_name.clone(),
                        message: smell.message.clone(),
                        in_changed_lines,
                    },
                ))
            })
            .collect()
    }

    fn to_refactor_finding(
        rec: &RefactoringRecommendation,
        location_by_smell: &HashMap<String, (String, u32)>,
    ) -> ReviewRefactorFinding {
        let (file_path, line_number) = location_by_smell
            .get(rec.smell_type.to_string().as_str())
            .cloned()
            .unwrap_or_default();
        ReviewRefactorFinding {
            file_path,
            line_number,
            priority: rec.priority.to_string(),
            technique: rec.technique.to_string(),
            description: rec.description.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_line_filter_keeps_only_overlapping_smells() {
        let analyzer = ReviewAnalyzer::new();
        let input = ReviewInput {
            base_ref: Some("main".to_string()),
            head_ref: Some("feature".to_string()),
            filters: AnalyzePathFilters::default(),
            min_severity: Severity::Info,
            max_findings: None,
            files: vec![ReviewFileInput {
                path: "src/app.rs".to_string(),
                source: r#"
fn long_one() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    let f = 6;
    let g = 7;
    let h = 8;
    let i = 9;
    let j = 10;
    let k = 11;
    let l = 12;
    let m = 13;
    let n = 14;
    let o = 15;
    let p = 16;
    let q = 17;
    let r = 18;
    let s = 19;
    let t = 20;
    let u = 21;
    let v = 22;
    let w = 23;
    let x = 24;
    let y = 25;
    let z = 26;
    let aa = 27;
    let ab = 28;
    let ac = 29;
    let ad = 30;
    let ae = 31;
    let af = 32;
    let ag = 33;
    let ah = 34;
    let ai = 35;
    let aj = 36;
    let ak = 37;
    let al = 38;
    let am = 39;
    let an = 40;
    let ao = 41;
    let ap = 42;
    let aq = 43;
    let ar = 44;
    let as_ = 45;
    let at = 46;
    let au = 47;
    let av = 48;
    let aw = 49;
    let ax = 50;
    let ay = 51;
}
"#
                .to_string(),
                changed_ranges: vec![ReviewLineRange {
                    start_line: 2,
                    end_line: 60,
                }],
            }],
        };
        let report = analyzer.analyze(&input);
        assert!(report.summary.files_analyzed > 0);
        assert!(report.smells.iter().all(|s| s.in_changed_lines));
    }
}
