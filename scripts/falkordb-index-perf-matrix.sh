#!/usr/bin/env bash
# FalkorDB indexing performance matrix.
#
# Usage:
#   RUN_DOCKER_INTEGRATION=1 ./scripts/falkordb-index-perf-matrix.sh [REPO_PATH]
#
# Requires Docker. Writes markdown to:
#   docs/superpowers/specs/2026-05-29-falkordb-index-perf-results.md

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RESULTS="${ROOT}/docs/superpowers/specs/2026-05-29-falkordb-index-perf-results.md"
REPO="${1:-${ROOT}/crates/cortex-parser/tests/fixtures/sample_project_rust}"
WORK_HOME="$(mktemp -d)"
mkdir -p "$WORK_HOME/.cortex"

if [[ "${RUN_DOCKER_INTEGRATION:-}" != "1" ]]; then
  echo "Set RUN_DOCKER_INTEGRATION=1 to run (starts FalkorDB via Docker)." >&2
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker required." >&2
  exit 1
fi

cargo build -p cortex-cli --manifest-path "$ROOT/Cargo.toml" >/dev/null
CLI="$ROOT/target/debug/cortex-cli"
[[ -x "$CLI" ]] || CLI="$ROOT/target/release/cortex-cli"

falkordb_port() {
  docker inspect -f '{{(index (index .NetworkSettings.Ports "6379/tcp") 0).HostPort}}' codecortex-falkordb-perf 2>/dev/null || echo ""
}

ensure_falkordb() {
  if docker ps -a --format '{{.Names}}' | grep -qx codecortex-falkordb-perf; then
    docker start codecortex-falkordb-perf >/dev/null 2>&1 || true
  else
    docker run -d --name codecortex-falkordb-perf -p 6379:6379 falkordb/falkordb:latest >/dev/null
  fi
  sleep 2
}

write_config() {
  local uri="$1"
  local batch="$2"
  local source_cap="$3"
  local pool="${4:-1}"
  cat >"$WORK_HOME/.cortex/config.toml" <<EOF
backend_type = "falkordb"
falkordb_uri = "$uri"
falkordb_password = ""
falkordb_graph = "codecortex_perf"
max_batch_size = $batch
falkordb_unwind_batch_max = $batch
falkordb_write_pool_size = $pool
graph_node_source_max_bytes = $source_cap
[vector]
store_type = "json"
store_path = "$WORK_HOME/.cortex/vectors"
EOF
}

run_index_to() {
  local out="$1"
  local err
  err="$(mktemp)"
  RUST_LOG=error HOME="$WORK_HOME" CORTEX_INDEX_PROFILE=1 CORTEX_FALKORDB_PROFILE=1 \
    "$CLI" index "$REPO" --force --format json >"$out.raw" 2>"$err" || true
  grep '^{' "$out.raw" 2>/dev/null | tail -1 >"$out" || true
  rm -f "$out.raw" "$err"
}

report_jq() {
  local json="$1"
  local filter="$2"
  echo "$json" | jq -r "$filter" 2>/dev/null || echo "n/a"
}

report_field() {
  local json="$1"
  local field="$2"
  report_jq "$json" ".${field} // .[0].${field}"
}

append_row() {
  local label="$1"
  local backend="$2"
  local batch="$3"
  local source_cap="$4"
  local jtmp="$5"
  local json
  json="$(cat "$jtmp" 2>/dev/null || true)"
  local dur flush edges bolts maxb lockf resolve resolve_frac
  dur="$(report_field "$json" duration_secs)"
  flush="$(report_field "$json" phase_edge_flush_secs)"
  edges="$(report_field "$json" edges_flushed)"
  bolts="$(report_field "$json" edge_flush_bolt_executions)"
  maxb="$(report_jq "$json" '.falkordb_profile.query_bytes_max // .[0].falkordb_profile.query_bytes_max // "n/a"')"
  lockf="$(report_jq "$json" '.falkordb_profile.lock_wait_fraction // .[0].falkordb_profile.lock_wait_fraction // "n/a"')"
  resolve="$(report_field "$json" phase_resolve_call_targets_secs)"
  resolve_frac="n/a"
  if [[ "$resolve" != "n/a" && "$dur" != "n/a" ]] && awk "BEGIN {exit !($dur > 0)}" 2>/dev/null; then
    resolve_frac="$(awk "BEGIN {printf \"%.3f\", $resolve / $dur}")"
  fi
  local eps="n/a"
  if [[ "$flush" != "n/a" && "$edges" != "n/a" ]] && awk "BEGIN {exit !($flush > 0)}" 2>/dev/null; then
    eps="$(awk "BEGIN {printf \"%.1f\", $edges / $flush}")"
  fi
  echo "| $label | $backend | $batch | $source_cap | $dur | $resolve | $resolve_frac | $flush | $edges | $eps | $bolts | $maxb | $lockf |" >>"$RESULTS"
}

{
  echo "# FalkorDB index performance results"
  echo ""
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "Repo: \`$REPO\`"
  echo ""
  echo "| case | backend | batch | source_cap | duration_secs | phase_resolve | resolve_frac | phase_edge_flush | edges | edges_per_sec | bolt_exec | falkor_max_query_bytes | falkor_lock_wait_frac |"
  echo "|------|---------|-------|------------|---------------|---------------|--------------|------------------|-------|---------------|-----------|------------------------|----------------------|"
} >"$RESULTS"

AUDIT_DIR="${ROOT}/audit/index-perf"
mkdir -p "$AUDIT_DIR"

run_profile_index() {
  local profile="$1"
  local out="$2"
  local err
  err="$(mktemp)"
  RUST_LOG=error HOME="$WORK_HOME" CORTEX_INDEX_PROFILE=1 CORTEX_FALKORDB_PROFILE=1 \
    "$CLI" index "$REPO" --force --profile "$profile" --format json >"$out.raw" 2>"$err" || true
  grep '^{' "$out.raw" 2>/dev/null | tail -1 >"$out" || true
  rm -f "$out.raw" "$err"
}

ensure_falkordb
FK_PORT="$(falkordb_port)"
FK_PORT="${FK_PORT:-6379}"

# Profile sweep (uses CortexConfig defaults via env; minimal TOML)
write_config "falkor://127.0.0.1:${FK_PORT}" 4096 65536 4
jtmp="$(mktemp)" && run_profile_index conservative "$jtmp" && append_row "profile_conservative" falkordb profile conservative "$jtmp" && cp "$jtmp" "$AUDIT_DIR/small-conservative.json" 2>/dev/null || true && rm -f "$jtmp"
jtmp="$(mktemp)" && run_profile_index highspeed "$jtmp" && append_row "profile_highspeed" falkordb profile highspeed "$jtmp" && cp "$jtmp" "$AUDIT_DIR/small-highspeed.json" 2>/dev/null || true && rm -f "$jtmp"

# Baseline FalkorDB
write_config "falkor://127.0.0.1:${FK_PORT}" 2048 262144 1
jtmp="$(mktemp)" && run_index_to "$jtmp" && append_row "falkordb_baseline" falkordb 2048 262144 "$jtmp" && rm -f "$jtmp"

# Batch sweep (FalkorDB)
for batch in 1024 4096; do
  write_config "falkor://127.0.0.1:${FK_PORT}" "$batch" 262144 1
  jtmp="$(mktemp)" && run_index_to "$jtmp" && append_row "falkordb_b${batch}" falkordb "$batch" 262144 "$jtmp" && rm -f "$jtmp"
done

# Source cap sweep
for cap in 0 32768; do
  write_config "falkor://127.0.0.1:${FK_PORT}" 2048 "$cap" 1
  jtmp="$(mktemp)" && run_index_to "$jtmp" && append_row "falkordb_src${cap}" falkordb 2048 "$cap" "$jtmp" && rm -f "$jtmp"
done

echo "" >>"$RESULTS"
echo "Analyze: \`cortex index-report analyze --file report.json\`" >>"$RESULTS"
echo "See [hypothesis report](2026-05-29-falkordb-index-perf-analysis.md)." >>"$RESULTS"

echo "Wrote $RESULTS"
