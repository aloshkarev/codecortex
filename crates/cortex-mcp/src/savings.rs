//! Persistent token-savings ledger for bounded MCP tools.

use cortex_core::tokens::{count_tokens, estimate_baseline_from_sample, tokenizer_name};
use cortex_core::{CortexConfig, McpConfig};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::contracts::{EnvelopeBuilder, TokenSavings};
use rmcp::model::CallToolResult;
use serde_json::Value;

const FLUSH_EVERY: usize = 20;

static LEDGER: Mutex<Option<SavingsLedger>> = Mutex::new(None);

/// In-process counters for the current MCP session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCounters {
    pub tokens_saved: u64,
    pub tokens_returned: u64,
    pub calls_counted: u64,
}

/// Persisted totals in `~/.cortex/savings.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavingsTotals {
    pub tokens_saved: u64,
    pub tokens_returned: u64,
    pub calls_counted: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
}

/// One append-only savings event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsEvent {
    pub ts_ms: u64,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub returned_tokens: u64,
    pub saved_tokens: u64,
    pub exact: bool,
}

/// Bucketed savings report for CLI display.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavingsBucket {
    pub tokens_saved: u64,
    pub tokens_returned: u64,
    pub calls_counted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsReport {
    pub totals: SavingsTotals,
    pub session: SessionCounters,
    pub today: SavingsBucket,
    pub last_7_days: SavingsBucket,
    pub all_time: SavingsBucket,
}

pub struct SavingsLedger {
    enabled: bool,
    dir: PathBuf,
    totals: SavingsTotals,
    session: SessionCounters,
    pending: Vec<SavingsEvent>,
}

impl SavingsLedger {
    pub fn new(enabled: bool, dir: PathBuf) -> Self {
        let totals = load_totals(&dir).unwrap_or_default();
        Self {
            enabled,
            dir,
            totals,
            session: SessionCounters::default(),
            pending: Vec::new(),
        }
    }

    pub fn session(&self) -> &SessionCounters {
        &self.session
    }

    pub fn record_call(
        &mut self,
        tool: &str,
        repo: Option<&str>,
        returned: u64,
        saved: u64,
        exact: bool,
    ) {
        if !self.enabled {
            return;
        }
        self.session.tokens_returned += returned;
        self.session.tokens_saved += saved;
        self.session.calls_counted += 1;

        self.totals.tokens_returned += returned;
        self.totals.tokens_saved += saved;
        self.totals.calls_counted += 1;
        self.totals.updated_at_ms = now_ms();

        self.pending.push(SavingsEvent {
            ts_ms: now_ms(),
            tool: tool.to_string(),
            repo: repo.map(str::to_string),
            returned_tokens: returned,
            saved_tokens: saved,
            exact,
        });

        if self.pending.len() >= FLUSH_EVERY {
            let _ = self.flush();
        }
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        let lock_path = self.dir.join("savings.lock");
        if !try_advisory_lock(&lock_path) {
            return Ok(());
        }
        let result = (|| {
            append_events(&self.dir, &self.pending)?;
            write_totals(&self.dir, &self.totals)?;
            Ok(())
        })();
        let _ = fs::remove_file(&lock_path);
        if result.is_ok() {
            self.pending.clear();
        }
        result
    }

    pub fn reset(&mut self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let lock_path = self.dir.join("savings.lock");
        if !try_advisory_lock(&lock_path) {
            return Ok(());
        }
        let result = (|| {
            let _ = fs::remove_file(self.dir.join("savings.json"));
            let _ = fs::remove_file(self.dir.join("savings.jsonl"));
            self.totals = SavingsTotals::default();
            self.session = SessionCounters::default();
            self.pending.clear();
            Ok(())
        })();
        let _ = fs::remove_file(&lock_path);
        result
    }
}

/// Initialize the process-wide ledger from config.
pub fn init_from_config(config: &CortexConfig) {
    let mut guard = LEDGER.lock().expect("savings ledger lock");
    *guard = Some(SavingsLedger::new(
        config.mcp.savings_enabled,
        savings_dir(),
    ));
}

fn ledger_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut SavingsLedger) -> R,
{
    let mut guard = LEDGER.lock().expect("savings ledger lock");
    if guard.is_none() {
        *guard = Some(SavingsLedger::new(true, savings_dir()));
    }
    f(guard.as_mut().expect("ledger initialized"))
}

/// Record a counted tool call into session + persistent ledger.
pub fn record_call(tool: &str, repo: Option<&str>, returned: u64, saved: u64, exact: bool) {
    ledger_mut(|ledger| ledger.record_call(tool, repo, returned, saved, exact));
}

/// Flush pending events to disk.
pub fn flush() {
    let _ = ledger_mut(|ledger| ledger.flush());
}

/// Reset persisted savings data.
pub fn reset() -> std::io::Result<()> {
    ledger_mut(|ledger| ledger.reset())
}

/// Build token savings metadata from returned payload and baseline sample.
pub fn compute_token_savings(
    returned_text: &str,
    baseline_total_chars: usize,
    baseline_sample: &str,
) -> TokenSavings {
    let (returned_tokens, returned_exact) = count_tokens(returned_text);
    let (baseline_tokens, baseline_estimated) =
        estimate_baseline_from_sample(baseline_total_chars, baseline_sample);
    let saved_tokens = baseline_tokens.saturating_sub(returned_tokens);
    let exact = returned_exact && !baseline_estimated;
    TokenSavings {
        returned_tokens,
        baseline_tokens,
        saved_tokens,
        baseline_estimated,
        tokenizer: tokenizer_name(exact).to_string(),
    }
}

/// Finish a conditional-fetch not-modified response and record avoided tokens.
pub fn finish_not_modified_response(
    enabled: bool,
    builder: EnvelopeBuilder,
    etag: &str,
    tool: &str,
    repo: Option<&str>,
    avoided_baseline_chars: usize,
    baseline_sample: &str,
) -> CallToolResult {
    let (baseline_tokens, _) =
        cortex_core::tokens::estimate_baseline_from_sample(avoided_baseline_chars, baseline_sample);
    if enabled {
        record_call(tool, repo, 0, baseline_tokens as u64, false);
    }
    let savings = TokenSavings {
        returned_tokens: 0,
        baseline_tokens,
        saved_tokens: baseline_tokens,
        baseline_estimated: true,
        tokenizer: cortex_core::tokens::tokenizer_name(false).to_string(),
    };
    builder.token_savings(savings).not_modified(etag)
}

/// Finish a bounded tool response with optional savings metadata + ledger write.
pub fn finish_counted_response(
    enabled: bool,
    builder: EnvelopeBuilder,
    data: Value,
    tool: &str,
    repo: Option<&str>,
    baseline_total_chars: usize,
    baseline_sample: &str,
) -> CallToolResult {
    let data_str = data.to_string();
    let savings = compute_token_savings(&data_str, baseline_total_chars, baseline_sample);
    if enabled {
        record_call(
            tool,
            repo,
            savings.returned_tokens as u64,
            savings.saved_tokens as u64,
            savings.tokenizer == "cl100k_base",
        );
    }
    builder.token_savings(savings).success(data)
}

/// Load a savings report from disk (for CLI).
pub fn load_report(session: Option<&SessionCounters>) -> SavingsReport {
    load_report_from(&savings_dir(), session)
}

/// Load a savings report from a specific ledger directory.
pub fn load_report_from(dir: &Path, session: Option<&SessionCounters>) -> SavingsReport {
    let totals = load_totals(dir).unwrap_or_default();
    let events = load_events(dir);
    let now = now_ms();
    let today_start = start_of_utc_day_ms(now);
    let week_start = now.saturating_sub(7 * 24 * 60 * 60 * 1000);

    let mut today = SavingsBucket::default();
    let mut last_7_days = SavingsBucket::default();
    for event in &events {
        if event.ts_ms >= week_start {
            accumulate_bucket(&mut last_7_days, event);
        }
        if event.ts_ms >= today_start {
            accumulate_bucket(&mut today, event);
        }
    }

    SavingsReport {
        all_time: SavingsBucket {
            tokens_saved: totals.tokens_saved,
            tokens_returned: totals.tokens_returned,
            calls_counted: totals.calls_counted,
        },
        totals: totals.clone(),
        session: session.cloned().unwrap_or_default(),
        today,
        last_7_days,
    }
}

fn accumulate_bucket(bucket: &mut SavingsBucket, event: &SavingsEvent) {
    bucket.tokens_saved += event.saved_tokens;
    bucket.tokens_returned += event.returned_tokens;
    bucket.calls_counted += 1;
}

pub fn savings_dir() -> PathBuf {
    std::env::var("HOME")
        .map(|home| PathBuf::from(home).join(".cortex"))
        .unwrap_or_else(|_| PathBuf::from(".cortex"))
}

pub fn savings_enabled(config: &McpConfig) -> bool {
    config.savings_enabled
}

fn load_totals(dir: &Path) -> std::io::Result<SavingsTotals> {
    let path = dir.join("savings.json");
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn write_totals(dir: &Path, totals: &SavingsTotals) -> std::io::Result<()> {
    let path = dir.join("savings.json");
    let data = serde_json::to_string_pretty(totals)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, data)
}

fn append_events(dir: &Path, events: &[SavingsEvent]) -> std::io::Result<()> {
    let path = dir.join("savings.jsonl");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for event in events {
        let line = serde_json::to_string(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(file, "{line}")?;
    }
    Ok(())
}

fn load_events(dir: &Path) -> Vec<SavingsEvent> {
    let path = dir.join("savings.jsonl");
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

fn try_advisory_lock(lock_path: &Path) -> bool {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
        .is_ok()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn start_of_utc_day_ms(now_ms: u64) -> u64 {
    let secs = now_ms / 1000;
    let day_secs = secs - (secs % 86_400);
    day_secs * 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn compute_token_savings_saved_is_non_negative() {
        let savings = compute_token_savings("short", 1000, "sample text for baseline");
        assert!(savings.saved_tokens <= savings.baseline_tokens);
        assert!(!savings.tokenizer.is_empty());
    }

    #[test]
    fn ledger_records_and_flushes() {
        let dir = TempDir::new().expect("tempdir");
        let mut ledger = SavingsLedger::new(true, dir.path().to_path_buf());
        ledger.record_call("get_skeleton", Some("/repo"), 100, 400, true);
        ledger.flush().expect("flush");
        assert!(dir.path().join("savings.json").exists());
        assert!(dir.path().join("savings.jsonl").exists());
        assert_eq!(ledger.session().calls_counted, 1);
        assert_eq!(ledger.session().tokens_saved, 400);
    }

    #[test]
    fn ledger_reset_clears_files() {
        let dir = TempDir::new().expect("tempdir");
        let mut ledger = SavingsLedger::new(true, dir.path().to_path_buf());
        ledger.record_call("get_skeleton", None, 10, 20, true);
        ledger.flush().expect("flush");
        ledger.reset().expect("reset");
        assert!(!dir.path().join("savings.json").exists());
        assert!(!dir.path().join("savings.jsonl").exists());
    }

    #[test]
    fn load_report_aggregates_events() {
        let dir = TempDir::new().expect("tempdir");
        let mut ledger = SavingsLedger::new(true, dir.path().to_path_buf());
        ledger.record_call("get_api_contract", None, 50, 150, true);
        ledger.flush().expect("flush");
        let report = load_report_from(dir.path(), Some(ledger.session()));
        assert_eq!(report.all_time.calls_counted, 1);
        assert_eq!(report.all_time.tokens_saved, 150);
    }
}
