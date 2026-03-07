use crate::registry::ProjectRegistry;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cortex_core::{CortexError, GitOperations, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::interval;

const JOB_PENDING: &str = "pending";
const JOB_RUNNING: &str = "running";
const JOB_COMPLETED: &str = "completed";
const JOB_FAILED: &str = "failed";
const MAX_JOB_ATTEMPTS: i64 = 3;
const BASE_RETRY_BACKOFF_SECS: i64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobMode {
    Full,
    IncrementalDiff,
}

impl JobMode {
    fn from_raw(raw: &str) -> Self {
        if raw == "incremental_diff" {
            JobMode::IncrementalDiff
        } else {
            JobMode::Full
        }
    }
}

impl std::fmt::Display for JobMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobMode::Full => write!(f, "full"),
            JobMode::IncrementalDiff => write!(f, "incremental_diff"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexJobRequest {
    pub repository_path: String,
    pub branch: String,
    pub commit_hash: String,
    pub mode: JobMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexJob {
    pub id: String,
    pub dedupe_key: String,
    pub repository_path: String,
    pub branch: String,
    pub commit_hash: String,
    pub mode: JobMode,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueResult {
    pub job: IndexJob,
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchRegistration {
    pub project_path: String,
    pub enabled: bool,
    pub last_branch: Option<String>,
    pub last_commit_hash: Option<String>,
    pub last_observed_at: Option<String>,
    pub last_event_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBranchHealth {
    pub project_path: String,
    pub branch: String,
    pub indexed_commit_hash: Option<String>,
    pub current_commit_hash: Option<String>,
    pub is_stale: bool,
    pub last_indexed_at: Option<String>,
    pub last_observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonPaths {
    pub root_dir: PathBuf,
    pub pid_path: PathBuf,
    pub socket_path: PathBuf,
    pub db_path: PathBuf,
    pub log_path: PathBuf,
}

impl DaemonPaths {
    pub fn default_paths() -> Self {
        if let Ok(root) = std::env::var("CORTEX_DAEMON_ROOT")
            && !root.trim().is_empty()
        {
            return Self::from_root(root);
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let primary_root = PathBuf::from(home).join(".cortex").join("daemon");
        if std::fs::create_dir_all(&primary_root).is_ok() {
            return Self::from_root(primary_root);
        }

        let fallback = std::env::temp_dir().join("cortex-daemon");
        Self::from_root(fallback)
    }

    pub fn from_root<P: AsRef<Path>>(root: P) -> Self {
        let root_dir = root.as_ref().to_path_buf();
        Self {
            pid_path: root_dir.join("daemon.pid"),
            socket_path: root_dir.join("daemon.sock"),
            db_path: root_dir.join("daemon.db"),
            log_path: root_dir.join("daemon.log"),
            root_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub paths: DaemonPaths,
    pub last_heartbeat: Option<String>,
    pub queue: HashMap<String, usize>,
    pub watched_projects: usize,
    pub store_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonMetrics {
    pub counters: HashMap<String, i64>,
}

fn io_err<E: std::fmt::Display>(err: E) -> CortexError {
    CortexError::Io(err.to_string())
}

fn db_err<E: std::fmt::Display>(err: E) -> CortexError {
    CortexError::Database(err.to_string())
}

fn ensure_layout(paths: &DaemonPaths) -> Result<()> {
    std::fs::create_dir_all(&paths.root_dir)?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let mut rows = stmt.query([]).map_err(db_err)?;
    while let Some(row) = rows.next().map_err(db_err)? {
        let name: String = row.get(1).map_err(db_err)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_column(conn: &Connection, table: &str, column: &str, ddl: &str) -> Result<()> {
    if !table_has_column(conn, table, column)? {
        let sql = format!("ALTER TABLE {table} ADD COLUMN {ddl}");
        conn.execute(&sql, []).map_err(db_err)?;
    }
    Ok(())
}

fn open_db(paths: &DaemonPaths) -> Result<Connection> {
    ensure_layout(paths)?;
    let conn = Connection::open(&paths.db_path).map_err(db_err)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode=WAL;
        PRAGMA synchronous=NORMAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS daemon_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS index_jobs (
            id TEXT PRIMARY KEY,
            dedupe_key TEXT NOT NULL UNIQUE,
            repository_path TEXT NOT NULL,
            branch TEXT NOT NULL,
            commit_hash TEXT NOT NULL,
            mode TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT,
            error_text TEXT,
            updated_at TEXT,
            attempts INTEGER NOT NULL DEFAULT 0,
            worker_id TEXT,
            next_attempt_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_index_jobs_status ON index_jobs(status);
        CREATE INDEX IF NOT EXISTS idx_index_jobs_repo_branch ON index_jobs(repository_path, branch);

        CREATE TABLE IF NOT EXISTS project_branch_health (
            project_path TEXT NOT NULL,
            branch TEXT NOT NULL,
            indexed_commit_hash TEXT,
            current_commit_hash TEXT,
            is_stale INTEGER NOT NULL DEFAULT 1,
            last_indexed_at TEXT,
            last_observed_at TEXT NOT NULL,
            PRIMARY KEY(project_path, branch)
        );

        CREATE TABLE IF NOT EXISTS watched_projects (
            project_path TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 1,
            last_branch TEXT,
            last_commit_hash TEXT,
            last_observed_at TEXT,
            last_event_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS daemon_metrics (
            metric_key TEXT PRIMARY KEY,
            metric_value INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )
    .map_err(db_err)?;

    // Migration safety for previously created DB files.
    ensure_column(&conn, "index_jobs", "updated_at", "updated_at TEXT")?;
    ensure_column(
        &conn,
        "index_jobs",
        "attempts",
        "attempts INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(&conn, "index_jobs", "worker_id", "worker_id TEXT")?;
    ensure_column(
        &conn,
        "index_jobs",
        "next_attempt_at",
        "next_attempt_at TEXT",
    )?;

    Ok(conn)
}

fn write_pid(paths: &DaemonPaths, pid: u32) -> Result<()> {
    ensure_layout(paths)?;
    std::fs::write(&paths.pid_path, pid.to_string())?;
    Ok(())
}

fn remove_pid(paths: &DaemonPaths) -> Result<()> {
    if paths.pid_path.exists() {
        std::fs::remove_file(&paths.pid_path)?;
    }
    Ok(())
}

fn read_pid(paths: &DaemonPaths) -> Result<Option<u32>> {
    if !paths.pid_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&paths.pid_path)?;
    let pid = raw
        .trim()
        .parse::<u32>()
        .map_err(|e| CortexError::Runtime(e.to_string()))?;
    Ok(Some(pid))
}

fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn upsert_heartbeat(conn: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        INSERT INTO daemon_meta(key, value, updated_at)
        VALUES ('last_heartbeat', ?1, ?1)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
        ",
        params![now],
    )
    .map_err(db_err)?;
    Ok(())
}

fn increment_metric(conn: &Connection, key: &str, delta: i64) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        INSERT INTO daemon_metrics(metric_key, metric_value, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(metric_key)
        DO UPDATE SET metric_value = daemon_metrics.metric_value + excluded.metric_value,
                      updated_at = excluded.updated_at
        ",
        params![key, delta, now],
    )
    .map_err(db_err)?;
    Ok(())
}

fn queue_counts(conn: &Connection) -> Result<HashMap<String, usize>> {
    let mut counts = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT status, COUNT(*) FROM index_jobs GROUP BY status")
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            let status: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((status, count as usize))
        })
        .map_err(db_err)?;
    for row in rows {
        let (status, count) = row.map_err(db_err)?;
        counts.insert(status, count);
    }
    Ok(counts)
}

fn watched_project_count(conn: &Connection) -> Result<usize> {
    let count = conn
        .query_row(
            "SELECT COUNT(*) FROM watched_projects WHERE enabled = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(db_err)?;
    Ok(count as usize)
}

#[derive(Debug, Clone)]
struct ProjectPolicySnapshot {
    index_only: Vec<String>,
    exclude_patterns: Vec<String>,
    max_parallel_index_jobs: usize,
}

impl Default for ProjectPolicySnapshot {
    fn default() -> Self {
        Self {
            index_only: Vec::new(),
            exclude_patterns: Vec::new(),
            max_parallel_index_jobs: 1,
        }
    }
}

fn project_policy_for_path(project_path: &str) -> ProjectPolicySnapshot {
    let registry = ProjectRegistry::new();
    let path = PathBuf::from(project_path);
    let Some(project) = registry.get_project(&path) else {
        return ProjectPolicySnapshot::default();
    };
    ProjectPolicySnapshot {
        index_only: project.config.index_only,
        exclude_patterns: project.config.exclude_patterns,
        max_parallel_index_jobs: project.config.max_parallel_index_jobs.max(1),
    }
}

fn branch_allowed(policy: &ProjectPolicySnapshot, branch: &str) -> bool {
    if policy.index_only.is_empty() {
        return true;
    }
    policy.index_only.iter().any(|b| b == branch)
}

fn read_last_heartbeat(conn: &Connection) -> Result<Option<String>> {
    let mut stmt = conn
        .prepare("SELECT value FROM daemon_meta WHERE key = 'last_heartbeat' LIMIT 1")
        .map_err(db_err)?;
    match stmt.query_row([], |row| row.get::<_, String>(0)) {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(db_err(err)),
    }
}

fn read_metrics(conn: &Connection) -> Result<DaemonMetrics> {
    let mut stmt = conn
        .prepare(
            "
            SELECT metric_key, metric_value
            FROM daemon_metrics
            ORDER BY metric_key ASC
            ",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(db_err)?;

    let mut counters = HashMap::new();
    for row in rows {
        let (key, value) = row.map_err(db_err)?;
        counters.insert(key, value);
    }
    Ok(DaemonMetrics { counters })
}

fn heartbeat_is_recent(last_heartbeat: Option<&str>, max_age_secs: i64) -> bool {
    let Some(raw) = last_heartbeat else {
        return false;
    };
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(raw) else {
        return false;
    };
    let age = Utc::now().signed_duration_since(ts.with_timezone(&Utc));
    age.num_seconds() >= 0 && age.num_seconds() <= max_age_secs
}

fn load_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexJob> {
    let mode_raw: String = row.get(5)?;
    Ok(IndexJob {
        id: row.get(0)?,
        dedupe_key: row.get(1)?,
        repository_path: row.get(2)?,
        branch: row.get(3)?,
        commit_hash: row.get(4)?,
        mode: JobMode::from_raw(&mode_raw),
        status: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn enqueue_index_job_conn(conn: &Connection, request: &IndexJobRequest) -> Result<EnqueueResult> {
    let dedupe_key = format!(
        "{}::{}::{}::{}",
        request.repository_path, request.branch, request.commit_hash, request.mode
    );
    let now = Utc::now().to_rfc3339();

    let mut stmt = conn
        .prepare(
            "
            SELECT id, dedupe_key, repository_path, branch, commit_hash, mode, status, created_at
            FROM index_jobs
            WHERE dedupe_key = ?1
            LIMIT 1
            ",
        )
        .map_err(db_err)?;
    let existing = stmt
        .query_row(params![dedupe_key.clone()], load_job_row)
        .optional()
        .map_err(db_err)?;

    if let Some(mut job) = existing {
        if job.status == JOB_PENDING || job.status == JOB_RUNNING {
            return Ok(EnqueueResult {
                job,
                deduplicated: true,
            });
        }

        conn.execute(
            "
            UPDATE index_jobs
            SET status = ?2,
                created_at = ?3,
                updated_at = ?3,
                started_at = NULL,
                finished_at = NULL,
                error_text = NULL,
                worker_id = NULL,
                next_attempt_at = NULL
            WHERE id = ?1
            ",
            params![job.id, JOB_PENDING, now],
        )
        .map_err(db_err)?;
        job.status = JOB_PENDING.to_string();
        job.created_at = now;
        return Ok(EnqueueResult {
            job,
            deduplicated: false,
        });
    }

    let job_id = format!("idx-{}", Utc::now().timestamp_millis());
    conn.execute(
        "
        INSERT INTO index_jobs(
            id, dedupe_key, repository_path, branch, commit_hash, mode, status,
            created_at, updated_at, attempts, next_attempt_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0, NULL)
        ",
        params![
            job_id,
            dedupe_key,
            request.repository_path,
            request.branch,
            request.commit_hash,
            request.mode.to_string(),
            JOB_PENDING,
            now
        ],
    )
    .map_err(db_err)?;

    Ok(EnqueueResult {
        job: IndexJob {
            id: job_id,
            dedupe_key: format!(
                "{}::{}::{}::{}",
                request.repository_path, request.branch, request.commit_hash, request.mode
            ),
            repository_path: request.repository_path.clone(),
            branch: request.branch.clone(),
            commit_hash: request.commit_hash.clone(),
            mode: request.mode.clone(),
            status: JOB_PENDING.to_string(),
            created_at: now,
        },
        deduplicated: false,
    })
}

fn claim_next_pending_job(conn: &mut Connection, worker_id: &str) -> Result<Option<IndexJob>> {
    let tx = conn.unchecked_transaction().map_err(db_err)?;
    let mut stmt = tx
        .prepare(
            "
            SELECT id, dedupe_key, repository_path, branch, commit_hash, mode, status, created_at
            FROM index_jobs
            WHERE status = ?1
              AND (next_attempt_at IS NULL OR next_attempt_at <= ?2)
            ORDER BY COALESCE(next_attempt_at, created_at) ASC, created_at ASC
            LIMIT 1
            ",
        )
        .map_err(db_err)?;

    let next = stmt
        .query_row(params![JOB_PENDING, Utc::now().to_rfc3339()], load_job_row)
        .optional()
        .map_err(db_err)?;
    drop(stmt);

    let Some(mut job) = next else {
        tx.commit().map_err(db_err)?;
        return Ok(None);
    };

    let now = Utc::now().to_rfc3339();
    tx.execute(
        "
        UPDATE index_jobs
        SET status = ?2,
            started_at = COALESCE(started_at, ?3),
            updated_at = ?3,
            attempts = attempts + 1,
            worker_id = ?4,
            next_attempt_at = NULL
        WHERE id = ?1
        ",
        params![job.id, JOB_RUNNING, now, worker_id],
    )
    .map_err(db_err)?;
    tx.commit().map_err(db_err)?;

    if let Ok(created_ts) = DateTime::parse_from_rfc3339(&job.created_at) {
        let wait = Utc::now().signed_duration_since(created_ts.with_timezone(&Utc));
        if wait.num_milliseconds() > 0 {
            increment_metric(conn, "queue_wait_ms_total", wait.num_milliseconds())?;
            increment_metric(conn, "queue_wait_samples", 1)?;
        }
    }

    job.status = JOB_RUNNING.to_string();
    Ok(Some(job))
}

fn mark_job_completed(conn: &Connection, job_id: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        UPDATE index_jobs
        SET status = ?2, finished_at = ?3, updated_at = ?3, error_text = NULL
        WHERE id = ?1
        ",
        params![job_id, JOB_COMPLETED, now],
    )
    .map_err(db_err)?;
    Ok(())
}

fn mark_job_failed(conn: &Connection, job_id: &str, error_text: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        UPDATE index_jobs
        SET status = ?2, finished_at = ?3, updated_at = ?3, error_text = ?4, next_attempt_at = NULL
        WHERE id = ?1
        ",
        params![job_id, JOB_FAILED, now, error_text],
    )
    .map_err(db_err)?;
    Ok(())
}

fn error_retryable(error_text: &str) -> bool {
    let text = error_text.to_ascii_lowercase();
    [
        "connection error",
        "couldn't connect",
        "timed out",
        "timeout",
        "temporarily unavailable",
        "operation not permitted",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn schedule_job_retry(conn: &Connection, job_id: &str, error_text: &str) -> Result<bool> {
    let attempts: i64 = conn
        .query_row(
            "SELECT attempts FROM index_jobs WHERE id = ?1 LIMIT 1",
            params![job_id],
            |row| row.get(0),
        )
        .map_err(db_err)?;

    if attempts >= MAX_JOB_ATTEMPTS {
        return Ok(false);
    }

    let exp = (attempts.saturating_sub(1)).clamp(0, 8);
    let backoff_secs = (BASE_RETRY_BACKOFF_SECS * (1_i64 << exp)).min(300);
    let next_attempt = (Utc::now() + ChronoDuration::seconds(backoff_secs)).to_rfc3339();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        UPDATE index_jobs
        SET status = ?2,
            updated_at = ?3,
            finished_at = NULL,
            error_text = ?4,
            worker_id = NULL,
            next_attempt_at = ?5
        WHERE id = ?1
        ",
        params![job_id, JOB_PENDING, now, error_text, next_attempt],
    )
    .map_err(db_err)?;

    increment_metric(conn, "retried_jobs", 1)?;
    Ok(true)
}

fn recover_stale_running_jobs(conn: &Connection) -> Result<usize> {
    let now = Utc::now().to_rfc3339();
    let updated = conn
        .execute(
            "
            UPDATE index_jobs
            SET status = ?1, updated_at = ?2, worker_id = NULL
            WHERE status = ?3
            ",
            params![JOB_PENDING, now, JOB_RUNNING],
        )
        .map_err(db_err)?;
    Ok(updated)
}

fn update_branch_health_observation(
    conn: &Connection,
    project_path: &str,
    branch: &str,
    current_commit_hash: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let indexed_commit: Option<String> = conn
        .query_row(
            "
            SELECT indexed_commit_hash
            FROM project_branch_health
            WHERE project_path = ?1 AND branch = ?2
            LIMIT 1
            ",
            params![project_path, branch],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_err)?
        .flatten();

    let is_stale = indexed_commit
        .as_ref()
        .map(|indexed| indexed != current_commit_hash)
        .unwrap_or(true);

    conn.execute(
        "
        INSERT INTO project_branch_health(
            project_path, branch, indexed_commit_hash, current_commit_hash,
            is_stale, last_indexed_at, last_observed_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)
        ON CONFLICT(project_path, branch) DO UPDATE SET
            current_commit_hash = excluded.current_commit_hash,
            is_stale = excluded.is_stale,
            last_observed_at = excluded.last_observed_at,
            indexed_commit_hash = COALESCE(project_branch_health.indexed_commit_hash, excluded.indexed_commit_hash)
        ",
        params![
            project_path,
            branch,
            indexed_commit,
            current_commit_hash,
            if is_stale { 1 } else { 0 },
            now
        ],
    )
    .map_err(db_err)?;

    Ok(())
}

fn update_branch_health_indexed(
    conn: &Connection,
    project_path: &str,
    branch: &str,
    commit_hash: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "
        INSERT INTO project_branch_health(
            project_path, branch, indexed_commit_hash, current_commit_hash,
            is_stale, last_indexed_at, last_observed_at
        )
        VALUES (?1, ?2, ?3, ?3, 0, ?4, ?4)
        ON CONFLICT(project_path, branch) DO UPDATE SET
            indexed_commit_hash = excluded.indexed_commit_hash,
            current_commit_hash = excluded.current_commit_hash,
            is_stale = 0,
            last_indexed_at = excluded.last_indexed_at,
            last_observed_at = excluded.last_observed_at
        ",
        params![project_path, branch, commit_hash, now],
    )
    .map_err(db_err)?;
    Ok(())
}

fn discover_git_head(path: &str) -> Option<(String, String)> {
    let root = Path::new(path);
    let git = GitOperations::new(root);
    if !git.is_git_repo() {
        return None;
    }
    let branch = git.get_current_branch().ok()?;
    let commit = git.get_current_commit().ok()?;
    Some((branch, commit))
}

fn poll_watches_and_enqueue(conn: &Connection) -> Result<usize> {
    let mut stmt = conn
        .prepare(
            "
            SELECT project_path, last_branch, last_commit_hash
            FROM watched_projects
            WHERE enabled = 1
            ORDER BY project_path ASC
            ",
        )
        .map_err(db_err)?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(db_err)?;

    let mut queued = 0usize;
    for row in rows {
        let (project_path, last_branch, last_commit_hash) = row.map_err(db_err)?;
        let now = Utc::now().to_rfc3339();

        if let Some((branch, commit_hash)) = discover_git_head(&project_path) {
            let policy = project_policy_for_path(&project_path);
            let commit_changed = last_commit_hash
                .as_ref()
                .map(|c| c != &commit_hash)
                .unwrap_or(true);
            let branch_changed = last_branch.as_ref().map(|b| b != &branch).unwrap_or(false);

            conn.execute(
                "
                UPDATE watched_projects
                SET last_branch = ?2,
                    last_commit_hash = ?3,
                    last_observed_at = ?4,
                    updated_at = ?4
                WHERE project_path = ?1
                ",
                params![project_path, branch, commit_hash, now],
            )
            .map_err(db_err)?;

            update_branch_health_observation(conn, &project_path, &branch, &commit_hash)?;

            if branch_changed {
                increment_metric(conn, "branch_switch_count", 1)?;
            }

            if commit_changed {
                if !branch_allowed(&policy, &branch) {
                    increment_metric(conn, "policy_skipped_jobs", 1)?;
                    continue;
                }
                let mode = if last_commit_hash.is_some() {
                    JobMode::IncrementalDiff
                } else {
                    JobMode::Full
                };
                let request = IndexJobRequest {
                    repository_path: project_path.clone(),
                    branch,
                    commit_hash,
                    mode,
                };
                let result = enqueue_index_job_conn(conn, &request)?;
                if !result.deduplicated {
                    queued += 1;
                    conn.execute(
                        "
                        UPDATE watched_projects
                        SET last_event_at = ?2,
                            updated_at = ?2
                        WHERE project_path = ?1
                        ",
                        params![project_path, now],
                    )
                    .map_err(db_err)?;
                }
            }
        } else {
            conn.execute(
                "
                UPDATE watched_projects
                SET last_observed_at = ?2,
                    updated_at = ?2
                WHERE project_path = ?1
                ",
                params![project_path, now],
            )
            .map_err(db_err)?;
        }
    }

    if queued > 0 {
        increment_metric(conn, "watch_enqueued_jobs", queued as i64)?;
    }

    Ok(queued)
}

fn trim_error_text(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    if !stderr.is_empty() {
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    if text.trim().is_empty() && !stdout.is_empty() {
        text.push_str(&String::from_utf8_lossy(stdout));
    }

    let cleaned = text.trim();
    if cleaned.is_empty() {
        return "index command failed without output".to_string();
    }

    const MAX_LEN: usize = 1500;
    if cleaned.len() > MAX_LEN {
        format!("{}...", &cleaned[..MAX_LEN])
    } else {
        cleaned.to_string()
    }
}

fn process_next_pending_job(conn: &mut Connection, executable: &Path) -> Result<Option<IndexJob>> {
    let worker_id = format!("pid:{}", std::process::id());
    let Some(job) = claim_next_pending_job(conn, &worker_id)? else {
        return Ok(None);
    };
    let policy = project_policy_for_path(&job.repository_path);

    let running_for_repo: i64 = conn
        .query_row(
            "
            SELECT COUNT(*)
            FROM index_jobs
            WHERE repository_path = ?1 AND status = ?2
            ",
            params![job.repository_path, JOB_RUNNING],
            |row| row.get(0),
        )
        .map_err(db_err)?;
    if (running_for_repo as usize) >= policy.max_parallel_index_jobs {
        let now = Utc::now().to_rfc3339();
        let next_attempt = (Utc::now() + ChronoDuration::seconds(2)).to_rfc3339();
        conn.execute(
            "
            UPDATE index_jobs
            SET status = ?2,
                updated_at = ?3,
                started_at = NULL,
                attempts = CASE WHEN attempts > 0 THEN attempts - 1 ELSE 0 END,
                worker_id = NULL,
                next_attempt_at = ?4
            WHERE id = ?1
            ",
            params![job.id, JOB_PENDING, now, next_attempt],
        )
        .map_err(db_err)?;
        increment_metric(conn, "policy_throttled_jobs", 1)?;
        return Ok(Some(job));
    }

    if let Some((current_branch, current_commit)) = discover_git_head(&job.repository_path)
        && (current_branch != job.branch || current_commit != job.commit_hash)
    {
        update_branch_health_observation(
            conn,
            &job.repository_path,
            &current_branch,
            &current_commit,
        )?;
        mark_job_failed(
            conn,
            &job.id,
            &format!(
                "branch/commit changed before indexing (expected {}/{} got {}/{})",
                job.branch, job.commit_hash, current_branch, current_commit
            ),
        )?;
        increment_metric(conn, "dropped_jobs", 1)?;
        return Ok(Some(job));
    }

    let started = Instant::now();
    let mut command = Command::new(executable);
    command
        .arg("--format")
        .arg("json")
        .arg("index")
        .arg(&job.repository_path)
        .env("CORTEX_DAEMON_BYPASS_QUEUE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !policy.exclude_patterns.is_empty() {
        command.env(
            "CORTEX_INDEX_EXCLUDE_PATTERNS",
            policy.exclude_patterns.join("\n"),
        );
    }
    if matches!(job.mode, JobMode::Full) {
        command.arg("--force");
        command.arg("--mode").arg("full");
    } else {
        command.arg("--mode").arg("incremental-diff");
    }
    let output = command.output().map_err(io_err)?;

    if output.status.success() {
        mark_job_completed(conn, &job.id)?;
        update_branch_health_indexed(conn, &job.repository_path, &job.branch, &job.commit_hash)?;
        increment_metric(conn, "completed_jobs", 1)?;
        increment_metric(
            conn,
            "index_duration_ms_total",
            started.elapsed().as_millis() as i64,
        )?;
    } else {
        let error_text = trim_error_text(&output.stdout, &output.stderr);
        if error_retryable(&error_text) && schedule_job_retry(conn, &job.id, &error_text)? {
            increment_metric(conn, "retryable_failures", 1)?;
        } else {
            mark_job_failed(conn, &job.id, &error_text)?;
            increment_metric(conn, "failed_jobs", 1)?;
        }
    }

    Ok(Some(job))
}

pub fn daemon_status(paths: &DaemonPaths) -> Result<DaemonStatus> {
    let pid = read_pid(paths)?;
    let (queue, last_heartbeat, watched_projects, store_error) = match open_db(paths) {
        Ok(conn) => (
            queue_counts(&conn)?,
            read_last_heartbeat(&conn)?,
            watched_project_count(&conn)?,
            None::<String>,
        ),
        Err(err) => (HashMap::new(), None, 0usize, Some(err.to_string())),
    };
    let heartbeat_recent = heartbeat_is_recent(last_heartbeat.as_deref(), 12);
    let running = match pid {
        Some(p) => heartbeat_recent || (last_heartbeat.is_none() && is_process_alive(p)),
        None => false,
    };

    Ok(DaemonStatus {
        running,
        pid,
        paths: paths.clone(),
        last_heartbeat,
        queue,
        watched_projects,
        store_error,
    })
}

pub fn start_background(paths: &DaemonPaths, executable: &Path) -> Result<DaemonStatus> {
    let status = daemon_status(paths)?;
    if status.running {
        return Err(CortexError::AlreadyExists(format!(
            "daemon already running with pid {}",
            status.pid.unwrap_or_default()
        )));
    }

    ensure_layout(paths)?;
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log_path)
        .map_err(io_err)?;
    let log_err = log_file.try_clone().map_err(io_err)?;

    Command::new(executable)
        .arg("daemon")
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(io_err)?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        let status = daemon_status(paths)?;
        if status.running {
            return Ok(status);
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    daemon_status(paths)
}

pub fn stop_daemon(paths: &DaemonPaths) -> Result<DaemonStatus> {
    let pid = read_pid(paths)?;
    let Some(pid) = pid else {
        return daemon_status(paths);
    };

    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if !is_process_alive(pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let _ = remove_pid(paths);
    daemon_status(paths)
}

pub async fn run_daemon(paths: &DaemonPaths) -> Result<()> {
    let mut conn = open_db(paths)?;
    write_pid(paths, std::process::id())?;
    upsert_heartbeat(&conn)?;
    let recovered = recover_stale_running_jobs(&conn)?;
    if recovered > 0 {
        increment_metric(&conn, "recovered_stale_jobs", recovered as i64)?;
    }

    let executable = std::env::current_exe().map_err(io_err)?;

    let mut ticker = interval(Duration::from_secs(2));
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(io_err)?;
    #[cfg(unix)]
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(io_err)?;

    #[cfg(unix)]
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                upsert_heartbeat(&conn)?;
                let poll_started = Instant::now();
                let _ = poll_watches_and_enqueue(&conn)?;
                increment_metric(&conn, "watch_poll_count", 1)?;
                increment_metric(&conn, "watch_poll_ms_total", poll_started.elapsed().as_millis() as i64)?;

                let process_started = Instant::now();
                let _ = process_next_pending_job(&mut conn, &executable)?;
                increment_metric(&conn, "process_tick_ms_total", process_started.elapsed().as_millis() as i64)?;
            }
            _ = sigterm.recv() => {
                break;
            }
            _ = sigint.recv() => {
                break;
            }
        }
    }

    #[cfg(not(unix))]
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                upsert_heartbeat(&conn)?;
                let poll_started = Instant::now();
                let _ = poll_watches_and_enqueue(&conn)?;
                increment_metric(&conn, "watch_poll_count", 1)?;
                increment_metric(&conn, "watch_poll_ms_total", poll_started.elapsed().as_millis() as i64)?;

                let process_started = Instant::now();
                let _ = process_next_pending_job(&mut conn, &executable)?;
                increment_metric(&conn, "process_tick_ms_total", process_started.elapsed().as_millis() as i64)?;
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }

    remove_pid(paths)?;
    Ok(())
}

pub fn enqueue_index_job(paths: &DaemonPaths, request: &IndexJobRequest) -> Result<EnqueueResult> {
    let conn = open_db(paths)?;
    enqueue_index_job_conn(&conn, request)
}

pub fn list_index_jobs(paths: &DaemonPaths, limit: usize) -> Result<Vec<IndexJob>> {
    let conn = open_db(paths)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, dedupe_key, repository_path, branch, commit_hash, mode, status, created_at
            FROM index_jobs
            ORDER BY created_at DESC
            LIMIT ?1
            ",
        )
        .map_err(db_err)?;

    let rows = stmt
        .query_map(params![limit as i64], load_job_row)
        .map_err(db_err)?;

    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(row.map_err(db_err)?);
    }
    Ok(jobs)
}

pub fn register_watch<P: AsRef<Path>>(
    paths: &DaemonPaths,
    project_path: P,
) -> Result<WatchRegistration> {
    let conn = open_db(paths)?;
    let canonical = project_path
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| project_path.as_ref().to_path_buf());
    let path_text = canonical.display().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "
        INSERT INTO watched_projects(
            project_path, enabled, last_branch, last_commit_hash,
            last_observed_at, last_event_at, created_at, updated_at
        )
        VALUES (?1, 1, NULL, NULL, NULL, NULL, ?2, ?2)
        ON CONFLICT(project_path) DO UPDATE SET
            enabled = 1,
            updated_at = excluded.updated_at
        ",
        params![path_text, now],
    )
    .map_err(db_err)?;

    let mut watches = list_watches(paths)?;
    watches
        .drain(..)
        .find(|w| w.project_path == canonical.display().to_string())
        .ok_or_else(|| {
            CortexError::Runtime("watch registration not found after insert".to_string())
        })
}

pub fn unregister_watch<P: AsRef<Path>>(paths: &DaemonPaths, project_path: P) -> Result<bool> {
    let conn = open_db(paths)?;
    let canonical = project_path
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| project_path.as_ref().to_path_buf());
    let updated = conn
        .execute(
            "
            UPDATE watched_projects
            SET enabled = 0,
                updated_at = ?2
            WHERE project_path = ?1
            ",
            params![canonical.display().to_string(), Utc::now().to_rfc3339()],
        )
        .map_err(db_err)?;
    Ok(updated > 0)
}

pub fn list_watches(paths: &DaemonPaths) -> Result<Vec<WatchRegistration>> {
    let conn = open_db(paths)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT project_path, enabled, last_branch, last_commit_hash, last_observed_at, last_event_at
            FROM watched_projects
            WHERE enabled = 1
            ORDER BY project_path ASC
            ",
        )
        .map_err(db_err)?;

    let rows = stmt
        .query_map([], |row| {
            Ok(WatchRegistration {
                project_path: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                last_branch: row.get(2)?,
                last_commit_hash: row.get(3)?,
                last_observed_at: row.get(4)?,
                last_event_at: row.get(5)?,
            })
        })
        .map_err(db_err)?;

    let mut watches = Vec::new();
    for row in rows {
        watches.push(row.map_err(db_err)?);
    }
    Ok(watches)
}

pub fn project_branch_health<P: AsRef<Path>>(
    paths: &DaemonPaths,
    project_path: P,
) -> Result<Vec<ProjectBranchHealth>> {
    let conn = open_db(paths)?;
    let canonical = project_path
        .as_ref()
        .canonicalize()
        .unwrap_or_else(|_| project_path.as_ref().to_path_buf());
    let project = canonical.display().to_string();

    let mut stmt = conn
        .prepare(
            "
            SELECT project_path, branch, indexed_commit_hash, current_commit_hash,
                   is_stale, last_indexed_at, last_observed_at
            FROM project_branch_health
            WHERE project_path = ?1
            ORDER BY branch ASC
            ",
        )
        .map_err(db_err)?;

    let rows = stmt
        .query_map(params![project], |row| {
            Ok(ProjectBranchHealth {
                project_path: row.get(0)?,
                branch: row.get(1)?,
                indexed_commit_hash: row.get(2)?,
                current_commit_hash: row.get(3)?,
                is_stale: row.get::<_, i64>(4)? != 0,
                last_indexed_at: row.get(5)?,
                last_observed_at: row.get(6)?,
            })
        })
        .map_err(db_err)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(db_err)?);
    }
    Ok(items)
}

pub fn daemon_metrics(paths: &DaemonPaths) -> Result<DaemonMetrics> {
    let conn = open_db(paths)?;
    read_metrics(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn daemon_paths_from_root() {
        let paths = DaemonPaths::from_root("/tmp/cortex-daemon-test");
        assert!(paths.db_path.ends_with("daemon.db"));
        assert!(paths.pid_path.ends_with("daemon.pid"));
    }

    #[test]
    fn enqueue_job_deduplicates() {
        let dir = tempdir().unwrap();
        let paths = DaemonPaths::from_root(dir.path());

        let request = IndexJobRequest {
            repository_path: "/repo".to_string(),
            branch: "main".to_string(),
            commit_hash: "abc123".to_string(),
            mode: JobMode::IncrementalDiff,
        };

        let first = enqueue_index_job(&paths, &request).unwrap();
        let second = enqueue_index_job(&paths, &request).unwrap();
        assert!(!first.deduplicated);
        assert!(second.deduplicated);
        assert_eq!(first.job.id, second.job.id);
    }

    #[test]
    fn watch_registration_roundtrip() {
        let dir = tempdir().unwrap();
        let paths = DaemonPaths::from_root(dir.path());

        let watch_path = dir.path().join("repo");
        std::fs::create_dir_all(&watch_path).unwrap();

        let registration = register_watch(&paths, &watch_path).unwrap();
        assert!(registration.enabled);

        let listed = list_watches(&paths).unwrap();
        assert_eq!(listed.len(), 1);

        let removed = unregister_watch(&paths, &watch_path).unwrap();
        assert!(removed);

        let listed_after = list_watches(&paths).unwrap();
        assert!(listed_after.is_empty());
    }

    #[test]
    fn list_jobs_returns_inserted_jobs() {
        let dir = tempdir().unwrap();
        let paths = DaemonPaths::from_root(dir.path());

        enqueue_index_job(
            &paths,
            &IndexJobRequest {
                repository_path: "/repo".to_string(),
                branch: "main".to_string(),
                commit_hash: "abc123".to_string(),
                mode: JobMode::Full,
            },
        )
        .unwrap();

        let jobs = list_index_jobs(&paths, 10).unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, JOB_PENDING);
    }

    #[test]
    fn enqueue_requeues_failed_job_with_same_dedupe_key() {
        let dir = tempdir().unwrap();
        let paths = DaemonPaths::from_root(dir.path());
        let request = IndexJobRequest {
            repository_path: "/repo".to_string(),
            branch: "main".to_string(),
            commit_hash: "abc123".to_string(),
            mode: JobMode::Full,
        };

        let first = enqueue_index_job(&paths, &request).unwrap();
        let conn = open_db(&paths).unwrap();
        conn.execute(
            "UPDATE index_jobs SET status = ?2, finished_at = ?3 WHERE id = ?1",
            params![first.job.id, JOB_FAILED, Utc::now().to_rfc3339()],
        )
        .unwrap();

        let second = enqueue_index_job(&paths, &request).unwrap();
        assert!(!second.deduplicated);
        assert_eq!(first.job.id, second.job.id);
        assert_eq!(second.job.status, JOB_PENDING);
    }

    #[test]
    fn throttled_job_does_not_consume_attempt_or_started_at() {
        let dir = tempdir().unwrap();
        let paths = DaemonPaths::from_root(dir.path());
        let mut conn = open_db(&paths).unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "
            INSERT INTO index_jobs(
                id, dedupe_key, repository_path, branch, commit_hash, mode, status,
                created_at, updated_at, attempts, started_at, worker_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 1, ?8, ?9)
            ",
            params![
                "running-job",
                "/repo::main::running::full",
                "/repo",
                "main",
                "running",
                JobMode::Full.to_string(),
                JOB_RUNNING,
                now,
                "pid:existing"
            ],
        )
        .unwrap();
        conn.execute(
            "
            INSERT INTO index_jobs(
                id, dedupe_key, repository_path, branch, commit_hash, mode, status,
                created_at, updated_at, attempts, next_attempt_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, 0, NULL)
            ",
            params![
                "pending-job",
                "/repo::main::pending::full",
                "/repo",
                "main",
                "pending",
                JobMode::Full.to_string(),
                JOB_PENDING,
                now
            ],
        )
        .unwrap();

        let fake_executable = dir.path().join("missing-executable");
        let claimed = process_next_pending_job(&mut conn, &fake_executable).unwrap();
        assert!(claimed.is_some());

        let (attempts, started_at, status): (i64, Option<String>, String) = conn
            .query_row(
                "SELECT attempts, started_at, status FROM index_jobs WHERE id = ?1",
                params!["pending-job"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(attempts, 0);
        assert_eq!(started_at, None);
        assert_eq!(status, JOB_PENDING);
    }
}
