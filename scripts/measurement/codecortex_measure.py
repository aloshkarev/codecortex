#!/usr/bin/env python3
"""CodeCortex measurement kit for daily ROI tracking.

Tracks:
- Baseline vs Cortex sessions
- Task outcomes (time, success, rework)
- Token usage imports from assistant/provider exports
- MCP readiness snapshots (diagnose/tools count + capture log path)

Output:
- Weekly/monthly style report with token/time savings and quality deltas
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import os
import sqlite3
import subprocess
import sys
import textwrap
import uuid
from pathlib import Path
from typing import Dict, Iterable, Optional, Tuple


DEFAULT_HOME = Path(
    os.environ.get("CODECORTEX_MEASURE_HOME", str(Path.home() / ".codecortex-measurement"))
)
DEFAULT_DB = DEFAULT_HOME / "measurements.db"
DEFAULT_LOG_DIR = DEFAULT_HOME / "logs"


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat()


def connect(db_path: Path) -> sqlite3.Connection:
    db_path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL;")
    conn.execute("PRAGMA foreign_keys=ON;")
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    conn.executescript(
        """
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            mode TEXT NOT NULL CHECK (mode IN ('baseline', 'cortex')),
            repo_path TEXT NOT NULL,
            assistant TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            notes TEXT
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
            task_key TEXT NOT NULL,
            category TEXT NOT NULL,
            minutes REAL NOT NULL CHECK (minutes >= 0),
            success INTEGER NOT NULL CHECK (success IN (0, 1)),
            rework INTEGER NOT NULL CHECK (rework IN (0, 1)),
            recorded_at TEXT NOT NULL,
            notes TEXT
        );

        CREATE TABLE IF NOT EXISTS token_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
            task_key TEXT,
            provider TEXT NOT NULL,
            model TEXT,
            prompt_tokens INTEGER NOT NULL CHECK (prompt_tokens >= 0),
            completion_tokens INTEGER NOT NULL CHECK (completion_tokens >= 0),
            total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
            source TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS mcp_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(session_id) ON DELETE CASCADE,
            captured_at TEXT NOT NULL,
            diagnose_overall_status TEXT,
            diagnose_payload_json TEXT,
            mcp_tools_count INTEGER,
            mcp_log_path TEXT,
            vector_read_enabled INTEGER,
            vector_write_enabled INTEGER
        );
        """
    )
    conn.commit()


def parse_bool(text: str) -> int:
    value = text.strip().lower()
    if value in {"1", "true", "yes", "y"}:
        return 1
    if value in {"0", "false", "no", "n"}:
        return 0
    raise ValueError(f"Invalid boolean value: {text}")


def read_env_flags(env_file: Optional[Path]) -> Tuple[Optional[int], Optional[int]]:
    if env_file is None or not env_file.exists():
        return (None, None)
    vector_read = None
    vector_write = None
    for raw in env_file.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip().strip('"').strip("'")
        if key == "CORTEX_FLAG_MCP_VECTOR_READ_ENABLED":
            vector_read = 1 if value in {"1", "true", "TRUE"} else 0
        elif key == "CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED":
            vector_write = 1 if value in {"1", "true", "TRUE"} else 0
    return (vector_read, vector_write)


def run_command_json(cmd: Iterable[str]) -> Optional[dict]:
    try:
        proc = subprocess.run(
            list(cmd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return None
    if proc.returncode != 0:
        return None
    try:
        return json.loads(proc.stdout.strip())
    except json.JSONDecodeError:
        return None


def run_command_lines(cmd: Iterable[str]) -> Optional[list[str]]:
    try:
        proc = subprocess.run(
            list(cmd),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return None
    if proc.returncode != 0:
        return None
    return [line for line in proc.stdout.splitlines() if line.strip()]


def cmd_init(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    init_db(conn)
    print(f"Initialized measurement DB: {args.db}")
    return 0


def cmd_session_start(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    init_db(conn)
    session_id = args.session_id or str(uuid.uuid4())
    conn.execute(
        """
        INSERT INTO sessions (session_id, mode, repo_path, assistant, started_at, notes)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        (
            session_id,
            args.mode,
            args.repo_path,
            args.assistant,
            utc_now(),
            args.notes,
        ),
    )
    conn.commit()
    print(session_id)
    return 0


def cmd_session_end(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    cur = conn.execute(
        "UPDATE sessions SET ended_at = ?, notes = COALESCE(?, notes) WHERE session_id = ?",
        (utc_now(), args.notes, args.session_id),
    )
    conn.commit()
    if cur.rowcount == 0:
        print(f"Session not found: {args.session_id}", file=sys.stderr)
        return 1
    print(f"Closed session: {args.session_id}")
    return 0


def cmd_task_log(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    init_db(conn)
    conn.execute(
        """
        INSERT INTO tasks (session_id, task_key, category, minutes, success, rework, recorded_at, notes)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            args.session_id,
            args.task_key,
            args.category,
            args.minutes,
            parse_bool(args.success),
            parse_bool(args.rework),
            utc_now(),
            args.notes,
        ),
    )
    conn.commit()
    print(f"Logged task {args.task_key} in session {args.session_id}")
    return 0


def required_int(row: Dict[str, str], keys: tuple[str, ...]) -> int:
    for key in keys:
        if key in row and row[key] != "":
            return int(row[key])
    raise ValueError(f"Missing required token column (tried: {', '.join(keys)})")


def cmd_tokens_import(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    init_db(conn)
    source = str(args.csv_path)
    inserted = 0
    with args.csv_path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames is None:
            print("CSV has no header", file=sys.stderr)
            return 1
        headers = {h.strip().lower(): h for h in reader.fieldnames if h}
        for raw in reader:
            row = {k.strip().lower(): (v.strip() if isinstance(v, str) else v) for k, v in raw.items() if k}
            prompt = required_int(
                row,
                ("prompt_tokens", "input_tokens", "prompt", "input"),
            )
            completion = required_int(
                row,
                ("completion_tokens", "output_tokens", "completion", "output"),
            )
            total = row.get("total_tokens")
            total_tokens = int(total) if total not in (None, "") else prompt + completion
            task_key = row.get("task_key") or row.get("task") or row.get("ticket")
            model = row.get("model", args.default_model)
            conn.execute(
                """
                INSERT INTO token_usage (
                    session_id, task_key, provider, model,
                    prompt_tokens, completion_tokens, total_tokens,
                    source, recorded_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    args.session_id,
                    task_key,
                    args.provider,
                    model,
                    prompt,
                    completion,
                    total_tokens,
                    source,
                    utc_now(),
                ),
            )
            inserted += 1
    conn.commit()
    print(f"Imported token rows: {inserted}")
    return 0


def cmd_snapshot(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    init_db(conn)

    diagnose = run_command_json(["cortex", "diagnose", "--format", "json"])
    tools = run_command_lines(["cortex", "mcp", "tools"])
    tool_count = len(tools) if tools is not None else None
    status = diagnose.get("overall_status") if isinstance(diagnose, dict) else None
    vector_read_enabled, vector_write_enabled = read_env_flags(args.env_file)

    conn.execute(
        """
        INSERT INTO mcp_snapshots (
            session_id, captured_at, diagnose_overall_status, diagnose_payload_json,
            mcp_tools_count, mcp_log_path, vector_read_enabled, vector_write_enabled
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            args.session_id,
            utc_now(),
            status,
            json.dumps(diagnose) if diagnose is not None else None,
            tool_count,
            str(args.mcp_log_path) if args.mcp_log_path else None,
            vector_read_enabled,
            vector_write_enabled,
        ),
    )
    conn.commit()
    print(
        json.dumps(
            {
                "session_id": args.session_id,
                "diagnose_overall_status": status,
                "mcp_tools_count": tool_count,
                "mcp_log_path": str(args.mcp_log_path) if args.mcp_log_path else None,
            },
            indent=2,
        )
    )
    return 0


def _aggregate(conn: sqlite3.Connection, where_sql: str, params: tuple) -> Dict[str, Dict[str, float]]:
    sql = f"""
    SELECT
        s.mode AS mode,
        COUNT(DISTINCT s.session_id) AS sessions,
        COUNT(DISTINCT t.id) AS tasks,
        COALESCE(SUM(t.minutes), 0) AS minutes_sum,
        COALESCE(AVG(t.minutes), 0) AS minutes_avg,
        COALESCE(AVG(t.success), 0) AS success_rate,
        COALESCE(AVG(t.rework), 0) AS rework_rate,
        COALESCE(SUM(u.prompt_tokens), 0) AS prompt_tokens,
        COALESCE(SUM(u.completion_tokens), 0) AS completion_tokens,
        COALESCE(SUM(u.total_tokens), 0) AS total_tokens
    FROM sessions s
    LEFT JOIN tasks t ON t.session_id = s.session_id
    LEFT JOIN token_usage u ON u.session_id = s.session_id
    WHERE {where_sql}
    GROUP BY s.mode
    """
    out: Dict[str, Dict[str, float]] = {}
    for row in conn.execute(sql, params):
        out[row["mode"]] = dict(row)
    return out


def pct_delta(baseline: float, treatment: float) -> Optional[float]:
    if baseline == 0:
        return None
    return ((baseline - treatment) / baseline) * 100.0


def cmd_report(args: argparse.Namespace) -> int:
    conn = connect(args.db)
    where = ["1=1"]
    params = []
    if args.since:
        where.append("s.started_at >= ?")
        params.append(args.since)
    if args.until:
        where.append("s.started_at <= ?")
        params.append(args.until)
    if args.repo_path:
        where.append("s.repo_path = ?")
        params.append(args.repo_path)

    agg = _aggregate(conn, " AND ".join(where), tuple(params))
    baseline = agg.get("baseline", {})
    cortex = agg.get("cortex", {})

    report = {
        "window": {"since": args.since, "until": args.until, "repo_path": args.repo_path},
        "baseline": baseline,
        "cortex": cortex,
        "kpis": {
            "token_saved_percent": pct_delta(
                baseline.get("total_tokens", 0.0),
                cortex.get("total_tokens", 0.0),
            ),
            "time_saved_percent": pct_delta(
                baseline.get("minutes_sum", 0.0),
                cortex.get("minutes_sum", 0.0),
            ),
            "success_rate_delta": cortex.get("success_rate", 0.0)
            - baseline.get("success_rate", 0.0),
            "rework_rate_delta": cortex.get("rework_rate", 0.0)
            - baseline.get("rework_rate", 0.0),
        },
    }

    if args.output == "json":
        print(json.dumps(report, indent=2))
        return 0

    def fmt(name: str, value: Optional[float], digits: int = 2) -> str:
        if value is None:
            return f"{name}: n/a"
        return f"{name}: {value:.{digits}f}"

    print("CodeCortex Measurement Report")
    print(f"Window: since={args.since or '-'} until={args.until or '-'}")
    if args.repo_path:
        print(f"Repo: {args.repo_path}")
    print("")
    print("Baseline:")
    print(
        textwrap.indent(
            json.dumps(baseline, indent=2) if baseline else "No baseline data",
            prefix="  ",
        )
    )
    print("")
    print("Cortex:")
    print(
        textwrap.indent(
            json.dumps(cortex, indent=2) if cortex else "No cortex data",
            prefix="  ",
        )
    )
    print("")
    print("KPIs:")
    print(f"  {fmt('token_saved_percent', report['kpis']['token_saved_percent'])}")
    print(f"  {fmt('time_saved_percent', report['kpis']['time_saved_percent'])}")
    print(f"  {fmt('success_rate_delta', report['kpis']['success_rate_delta'])}")
    print(f"  {fmt('rework_rate_delta', report['kpis']['rework_rate_delta'])}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="CodeCortex measurement toolkit")
    parser.add_argument(
        "--db",
        type=Path,
        default=DEFAULT_DB,
        help=f"SQLite DB path (default: {DEFAULT_DB})",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_init = sub.add_parser("init", help="Initialize SQLite schema")
    p_init.set_defaults(func=cmd_init)

    p_start = sub.add_parser("session-start", help="Start an experiment session")
    p_start.add_argument("--session-id", default=None, help="Optional session id (default: UUID)")
    p_start.add_argument("--mode", choices=("baseline", "cortex"), required=True)
    p_start.add_argument("--repo-path", required=True)
    p_start.add_argument("--assistant", default="cursor")
    p_start.add_argument("--notes", default=None)
    p_start.set_defaults(func=cmd_session_start)

    p_end = sub.add_parser("session-end", help="Close an experiment session")
    p_end.add_argument("--session-id", required=True)
    p_end.add_argument("--notes", default=None)
    p_end.set_defaults(func=cmd_session_end)

    p_task = sub.add_parser("task-log", help="Log task outcome inside a session")
    p_task.add_argument("--session-id", required=True)
    p_task.add_argument("--task-key", required=True, help="Task identifier (ticket or slug)")
    p_task.add_argument("--category", required=True, help="bugfix/refactor/feature/test/etc")
    p_task.add_argument("--minutes", required=True, type=float)
    p_task.add_argument("--success", required=True, help="true|false")
    p_task.add_argument("--rework", required=True, help="true|false")
    p_task.add_argument("--notes", default=None)
    p_task.set_defaults(func=cmd_task_log)

    p_import = sub.add_parser("tokens-import", help="Import token CSV rows for a session")
    p_import.add_argument("--session-id", required=True)
    p_import.add_argument("--csv-path", required=True, type=Path)
    p_import.add_argument("--provider", required=True, help="cursor/openai/anthropic/etc")
    p_import.add_argument("--default-model", default=None, help="Used when CSV has no model col")
    p_import.set_defaults(func=cmd_tokens_import)

    p_snap = sub.add_parser("snapshot", help="Capture MCP readiness snapshot")
    p_snap.add_argument("--session-id", required=True)
    p_snap.add_argument("--mcp-log-path", type=Path, default=None)
    p_snap.add_argument(
        "--env-file",
        type=Path,
        default=Path(".env.cortex"),
        help="Path to env file for vector flags (default: .env.cortex)",
    )
    p_snap.set_defaults(func=cmd_snapshot)

    p_report = sub.add_parser("report", help="Generate ROI report")
    p_report.add_argument("--since", default=None, help="ISO time lower bound")
    p_report.add_argument("--until", default=None, help="ISO time upper bound")
    p_report.add_argument("--repo-path", default=None)
    p_report.add_argument("--output", choices=("table", "json"), default="table")
    p_report.set_defaults(func=cmd_report)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
