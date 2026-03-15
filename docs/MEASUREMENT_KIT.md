# CodeCortex Measurement Kit

This kit measures real day-to-day impact of CodeCortex indexing/vector retrieval on:

- token usage
- task time
- success and rework rate

It is designed for practical A/B operation in Cursor workflows.

## What it includes

- `scripts/measurement/codecortex_measure.py`
  - SQLite-backed tracker and report generator
- `scripts/measurement/start_mcp_capture.sh`
  - starts `cortex mcp start`, writes logs, stores MCP snapshot
- `scripts/measurement/token_usage.template.csv`
  - import template for token usage exports

## Quick start (5 minutes)

Single bootstrap command:

```bash
make measure-bootstrap
```

This initializes the DB, creates a session, and prints the exact next commands.

1) Initialize DB:

```bash
python3 scripts/measurement/codecortex_measure.py init
```

2) Start a session:

```bash
SESSION_ID=$(python3 scripts/measurement/codecortex_measure.py session-start \
  --mode cortex \
  --repo-path "$PWD" \
  --assistant cursor)
echo "$SESSION_ID"
```

3) Run MCP with capture:

```bash
./scripts/measurement/start_mcp_capture.sh "$SESSION_ID"
```

4) Log tasks as you complete them:

```bash
python3 scripts/measurement/codecortex_measure.py task-log \
  --session-id "$SESSION_ID" \
  --task-key "TASK-001" \
  --category "bugfix" \
  --minutes 22 \
  --success true \
  --rework false
```

5) Import token usage CSV (from Cursor/provider export):

```bash
python3 scripts/measurement/codecortex_measure.py tokens-import \
  --session-id "$SESSION_ID" \
  --csv-path ./token-usage.csv \
  --provider cursor
```

6) Close session:

```bash
python3 scripts/measurement/codecortex_measure.py session-end --session-id "$SESSION_ID"
```

7) Generate report:

```bash
python3 scripts/measurement/codecortex_measure.py report
```

## A/B experiment setup

Use crossover schedule:

- Day 1: `baseline` mode (vector flags off)
- Day 2: `cortex` mode (vector flags on)
- Repeat

Start sessions with:

```bash
python3 scripts/measurement/codecortex_measure.py session-start --mode baseline --repo-path "$PWD"
python3 scripts/measurement/codecortex_measure.py session-start --mode cortex --repo-path "$PWD"
```

Set these in `.env.cortex` for baseline runs:

```bash
CORTEX_FLAG_MCP_VECTOR_READ_ENABLED=0
CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED=0
```

Enable for treatment runs:

```bash
CORTEX_FLAG_MCP_VECTOR_READ_ENABLED=1
CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED=1
```

## CSV requirements for token import

Required columns (at minimum):

- `prompt_tokens` (or `input_tokens`)
- `completion_tokens` (or `output_tokens`)

Optional columns:

- `total_tokens` (auto-calculated if missing)
- `task_key`
- `model`

Use the template:

`scripts/measurement/token_usage.template.csv`

## KPI definitions

- `token_saved_percent = (baseline_total_tokens - cortex_total_tokens) / baseline_total_tokens * 100`
- `time_saved_percent = (baseline_minutes_sum - cortex_minutes_sum) / baseline_minutes_sum * 100`
- `success_rate_delta = cortex_success_rate - baseline_success_rate`
- `rework_rate_delta = cortex_rework_rate - baseline_rework_rate`

## Operational notes

- DB path default: `~/.codecortex-measurement/measurements.db`
- MCP logs default dir: `~/.codecortex-measurement/logs`
- `snapshot` captures:
  - `cortex diagnose --format json` status
  - `cortex mcp tools` count
  - vector feature flags from `.env.cortex`

If `diagnose` fails (for example DB auth issue), snapshot still stores partial data.

## Production-readiness checklist

- Track at least 30 tasks in each mode before interpreting savings.
- Keep task mix balanced (bugfix/refactor/feature/test).
- Keep assistant/model settings stable during experiment windows.
- Record rework honestly for quality control.
- Review weekly using `report` output in JSON and table formats.
