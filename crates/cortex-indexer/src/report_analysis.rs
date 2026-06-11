//! Interpret [`crate::IndexReport`] JSON for performance triage.

use crate::IndexReport;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Derived throughput metrics from an index run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexDerivedKpis {
    pub edges_per_sec: Option<f64>,
    pub sec_per_bolt: Option<f64>,
    pub edges_per_bolt: Option<f64>,
    pub expected_chunks: Option<u64>,
    pub bolt_multiplier: Option<f64>,
    pub symbols_per_file: Option<f64>,
    pub edges_per_file: Option<f64>,
    pub ms_per_call_target: Option<f64>,
}

/// One row in the phase breakdown table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseRow {
    pub name: String,
    pub seconds: f64,
    pub pct_of_total: f64,
}

/// Heuristic flags for common bottlenecks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexHeuristics {
    pub graph_write_bound: bool,
    pub force_branch_deferred_replay: bool,
    pub rel_type_splitting: bool,
    pub call_target_upsert_bound: bool,
    pub messages: Vec<String>,
}

/// Full analysis output (CLI / JSON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexReportAnalysis {
    pub phases: Vec<PhaseRow>,
    pub kpis: IndexDerivedKpis,
    pub heuristics: IndexHeuristics,
    pub summary: String,
}

/// `max_batch_size` used for chunk/bolt multiplier (default 2048).
pub fn analyze_report(report: &IndexReport, max_batch_size: usize) -> IndexReportAnalysis {
    let phases = phase_rows(report);
    let kpis = derived_kpis(report, max_batch_size);
    let heuristics = heuristics(report, &kpis);
    let summary = format_summary(report, &phases, &kpis, &heuristics);
    IndexReportAnalysis {
        phases,
        kpis,
        heuristics,
        summary,
    }
}

fn phase_rows(report: &IndexReport) -> Vec<PhaseRow> {
    let total = report.duration_secs.max(1e-9);
    let mut rows = vec![
        ("edge_flush", report.phase_edge_flush_secs),
        ("deferred_node_write", report.phase_deferred_node_write_secs),
        ("call_targets", report.phase_call_targets_secs),
        ("parse_loop_wall", report.phase_parse_loop_wall_secs),
        ("branch_delete", report.phase_branch_delete_secs),
        ("incremental_cleanup", report.phase_incremental_cleanup_secs),
        ("preflight", report.phase_preflight_secs),
        (
            "resolve_call_targets",
            report.phase_resolve_call_targets_secs,
        ),
        (
            "resolve_type_references",
            report.phase_resolve_type_references_secs,
        ),
        (
            "resolve_field_accesses",
            report.phase_resolve_field_accesses_secs,
        ),
        ("promotion", report.phase_promotion_secs),
        ("skip_guard", report.phase_skip_guard_secs),
        ("unattributed", report.phase_unattributed_secs),
    ];
    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    rows.into_iter()
        .filter(|(_, s)| *s >= 0.001)
        .map(|(name, seconds)| PhaseRow {
            name: name.to_string(),
            seconds,
            pct_of_total: (seconds / total) * 100.0,
        })
        .collect()
}

pub fn derived_kpis(report: &IndexReport, max_batch_size: usize) -> IndexDerivedKpis {
    let batch = max_batch_size.max(1);
    let edges_per_sec = if report.phase_edge_flush_secs > 0.0 {
        Some(report.edges_flushed as f64 / report.phase_edge_flush_secs)
    } else {
        None
    };
    let sec_per_bolt = if report.edge_flush_bolt_executions > 0 {
        Some(report.phase_edge_flush_secs / report.edge_flush_bolt_executions as f64)
    } else {
        None
    };
    let edges_per_bolt = if report.edge_flush_bolt_executions > 0 {
        Some(report.edges_flushed as f64 / report.edge_flush_bolt_executions as f64)
    } else {
        None
    };
    let expected_chunks = if report.edges_flushed > 0 {
        Some((report.edges_flushed as usize).div_ceil(batch) as u64)
    } else {
        None
    };
    let bolt_multiplier = match (expected_chunks, report.edge_flush_bolt_executions) {
        (Some(chunks), bolts) if chunks > 0 && bolts > 0 => Some(bolts as f64 / chunks as f64),
        _ => None,
    };
    let symbols_per_file = if report.indexed_files > 0 {
        Some(report.symbol_count as f64 / report.indexed_files as f64)
    } else {
        None
    };
    let edges_per_file = if report.indexed_files > 0 {
        Some(report.edges_flushed as f64 / report.indexed_files as f64)
    } else {
        None
    };
    let ms_per_call_target = if report.call_targets_upserted > 0 {
        Some(report.phase_call_targets_secs * 1000.0 / report.call_targets_upserted as f64)
    } else {
        None
    };
    IndexDerivedKpis {
        edges_per_sec,
        sec_per_bolt,
        edges_per_bolt,
        expected_chunks,
        bolt_multiplier,
        symbols_per_file,
        edges_per_file,
        ms_per_call_target,
    }
}

fn heuristics(report: &IndexReport, kpis: &IndexDerivedKpis) -> IndexHeuristics {
    let mut messages = Vec::new();
    let graph_write_bound =
        report.duration_secs > 0.0 && report.phase_edge_flush_secs / report.duration_secs > 0.5;
    if graph_write_bound {
        messages.push(
            "Graph write bound: phase_edge_flush_secs is >50% of duration_secs (tune batch_size, falkordb_write_pool_size, graph indexes).".into(),
        );
    }
    if let Some(fp) = &report.falkordb_profile {
        if fp.query_count > 0 && fp.lock_wait_fraction > 0.15 {
            messages.push(format!(
                "FalkorDB serial lock wait {:.0}% of query wall time ({} queries); consider a write connection pool.",
                fp.lock_wait_fraction * 100.0,
                fp.query_count
            ));
        }
        if fp.query_bytes_max > 512 * 1024 {
            messages.push(format!(
                "FalkorDB max inline query {:.1} MiB — reduce graph_node_source_max_bytes or max_batch_size.",
                fp.query_bytes_max as f64 / (1024.0 * 1024.0)
            ));
        }
    }
    let force_branch_deferred_replay = report.phase_deferred_node_write_secs > 0.0;
    if force_branch_deferred_replay {
        messages.push(
            "Deferred node replay active (force+branch): phase_deferred_node_write_secs > 0; prefer incremental without --force, --wipe-branch-first, or subsystem includes.".into(),
        );
    }
    let deferred_fraction = if report.duration_secs > 0.0 {
        report.phase_deferred_node_write_secs / report.duration_secs
    } else {
        0.0
    };
    if deferred_fraction > 0.25 {
        messages.push(format!(
            "Deferred replay {:.0}% of wall time ({:.1}s read, {:.1}s collect, {:.1}s write; spill {} MiB)",
            deferred_fraction * 100.0,
            report.deferred_spill_read_secs,
            report.deferred_collect_secs,
            report.deferred_write_nodes_secs,
            report.deferred_spill_bytes / (1024 * 1024),
        ));
    }
    let rel_type_splitting = kpis.bolt_multiplier.is_some_and(|m| m > 3.0);
    if rel_type_splitting {
        messages.push(format!(
            "Relationship-type splitting: bolt_multiplier ≈ {:.1} (many UNWIND calls per chunk).",
            kpis.bolt_multiplier.unwrap_or(0.0)
        ));
    }
    let call_target_upsert_bound = kpis.ms_per_call_target.is_some_and(|ms| ms > 10.0);
    if call_target_upsert_bound {
        messages.push(format!(
            "Call-target upsert slow: {:.1} ms/target.",
            kpis.ms_per_call_target.unwrap_or(0.0)
        ));
    }
    IndexHeuristics {
        graph_write_bound,
        force_branch_deferred_replay,
        rel_type_splitting,
        call_target_upsert_bound,
        messages,
    }
}

fn format_summary(
    report: &IndexReport,
    phases: &[PhaseRow],
    kpis: &IndexDerivedKpis,
    heuristics: &IndexHeuristics,
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Index performance analysis (duration {:.1}s, {} files, {} edges)",
        report.duration_secs, report.indexed_files, report.edges_flushed
    );
    let _ = writeln!(out, "\nPhases (wall clock, descending):");
    for row in phases.iter().take(8) {
        let _ = writeln!(
            out,
            "  {:<24} {:>10.2}s  {:>5.1}%",
            row.name, row.seconds, row.pct_of_total
        );
    }
    let _ = writeln!(out, "\nDerived KPIs:");
    if let Some(v) = kpis.edges_per_sec {
        let _ = writeln!(out, "  edges_per_sec:      {v:.1}");
    }
    if let Some(v) = kpis.sec_per_bolt {
        let _ = writeln!(
            out,
            "  sec_per_bolt:       {v:.2}  (graph write executions; not Bolt on Grafeo)"
        );
    }
    if let Some(v) = kpis.edges_per_bolt {
        let _ = writeln!(out, "  edges_per_bolt:     {v:.0}");
    }
    if let Some(v) = kpis.bolt_multiplier {
        let _ = writeln!(
            out,
            "  bolt_multiplier:    {v:.2}  (executions / expected_chunks)"
        );
    }
    if let Some(v) = kpis.ms_per_call_target {
        let _ = writeln!(out, "  ms_per_call_target: {v:.1}");
    }
    if !heuristics.messages.is_empty() {
        let _ = writeln!(out, "\nFlags:");
        for msg in &heuristics.messages {
            let _ = writeln!(out, "  - {msg}");
        }
    }
    if report.edge_spill_read_secs > 0.0 || report.edge_spill_bolt_secs > 0.0 {
        let _ = writeln!(out, "\nEdge flush profile:");
        let _ = writeln!(
            out,
            "  read/deserialize: {:.2}s  bolt: {:.2}s",
            report.edge_spill_read_secs, report.edge_spill_bolt_secs
        );
        for entry in &report.edge_spill_bolt_by_rel_type {
            let _ = writeln!(
                out,
                "    {}: {} executions, {:.2}s",
                entry.rel_type, entry.bolt_executions, entry.seconds
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IndexReport;
    use crate::incremental::IndexRunMode;
    use cortex_core::IndexFreshness;

    fn baseline_report() -> IndexReport {
        IndexReport {
            scanned_files: 143,
            indexed_files: 143,
            skipped_files: 0,
            ignored_file_count: 0,
            resolved_calls: 0,
            build_systems: vec!["cargo".into()],
            compile_commands_loaded: 0,
            include_paths_count: 0,
            duration_secs: 4337.1,
            timed_out: false,
            branch: Some("main".into()),
            commit_hash: Some("abc".into()),
            symbol_count: 14388,
            skipped_reason: None,
            mode: IndexRunMode::Full,
            deleted_files: 143,
            tombstoned_files: 0,
            cache_hits: 0,
            cache_misses: 143,
            cache_metadata_hits: 0,
            phase_parse_secs: 2.08,
            phase_incremental_graph_secs: 0.0,
            phase_node_write_secs: 0.0,
            phase_preflight_secs: 0.01,
            phase_parse_loop_wall_secs: 2.13,
            phase_incremental_cleanup_secs: 0.0,
            phase_branch_delete_secs: 0.15,
            phase_deferred_node_write_secs: 309.9,
            deferred_spill_read_secs: 0.0,
            deferred_collect_secs: 0.0,
            deferred_write_nodes_secs: 0.0,
            deferred_spill_bytes: 0,
            phase_call_targets_secs: 95.9,
            call_targets_upserted: 2517,
            phase_edge_flush_secs: 3928.9,
            edges_flushed: 57956,
            edge_flush_bolt_executions: 207,
            phase_resolve_call_targets_secs: 0.007,
            phase_resolve_type_references_secs: 0.008,
            phase_resolve_field_accesses_secs: 0.008,
            phase_promotion_secs: 0.04,
            phase_skip_guard_secs: 0.0,
            phase_unattributed_secs: 0.01,
            truncated: false,
            max_files_cap: None,
            freshness: IndexFreshness::Fresh,
            edge_spill_read_secs: 0.0,
            edge_spill_bolt_secs: 0.0,
            edge_spill_bolt_by_rel_type: Vec::new(),
            falkordb_profile: None,
        }
    }

    #[test]
    fn baseline_kpis_match_appendix() {
        let kpis = derived_kpis(&baseline_report(), 2048);
        assert!(kpis.edges_per_sec.unwrap() > 14.0 && kpis.edges_per_sec.unwrap() < 16.0);
        assert!(kpis.sec_per_bolt.unwrap() > 18.0 && kpis.sec_per_bolt.unwrap() < 20.0);
        assert!(kpis.bolt_multiplier.unwrap() > 7.0);
    }

    #[test]
    fn graph_write_bound_flag() {
        let analysis = analyze_report(&baseline_report(), 2048);
        assert!(analysis.heuristics.graph_write_bound);
        assert!(analysis.heuristics.force_branch_deferred_replay);
    }

    /// Post parallel-resolve: resolve phase should be a modest fraction of total wall time.
    #[test]
    fn resolve_call_targets_phase_fraction_gate() {
        let mut report = baseline_report();
        report.duration_secs = 73.0;
        report.phase_resolve_call_targets_secs = 20.0;
        let frac = report.phase_resolve_call_targets_secs / report.duration_secs;
        assert!(
            frac < 0.35,
            "resolve fraction {frac} should stay below 0.35 after parallel chunked resolve"
        );
    }
}
