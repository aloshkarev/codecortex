#!/usr/bin/env bash
# Nightly MCP audit: smoke on disposable fixture + semantic oracles on full repo.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE="${CORTEX_AUDIT_FIXTURE:-$(mktemp -d /tmp/cortex-audit-XXXXXX)}"
SEMANTIC_REPO="${CORTEX_SEMANTIC_REPO:-$ROOT}"
export CORTEX_AUDIT_REPO="${CORTEX_AUDIT_REPO:-$FIXTURE}"
export CORTEX_AUDIT_PROFILE=nightly
export CORTEX_AUDIT_SKIP_LONG=0
export CORTEX_AUDIT_SKIP_DESTRUCTIVE=0
export CORTEX_AUDIT_A2A_CHAIN=1
export CORTEX_BIN="${CORTEX_BIN:-$ROOT/target/release/cortex-cli}"

if [[ ! -x "$CORTEX_BIN" ]]; then
  echo "Building cortex-cli release binary..." >&2
  (cd "$ROOT" && cargo build -p cortex-cli --release)
fi

# Minimal tree for smoke long/destructive tools on fixture
mkdir -p "$FIXTURE/crates/cortex-mcp/src"
cp "$ROOT/crates/cortex-mcp/src/handler.rs" "$FIXTURE/crates/cortex-mcp/src/handler.rs" 2>/dev/null || true
cp "$ROOT/Cargo.toml" "$FIXTURE/Cargo.toml" 2>/dev/null || true

export CORTEX_AUDIT_SOURCE="$FIXTURE/crates/cortex-mcp/src/handler.rs"
export CORTEX_AUDIT_FLOW_FROM="${CORTEX_AUDIT_FLOW_FROM:-resolve_project_context}"
export CORTEX_AUDIT_FLOW_TO="${CORTEX_AUDIT_FLOW_TO:-build_symbol_resolver}"

echo "Nightly smoke fixture: $FIXTURE" >&2
python3 "$ROOT/scripts/mcp_tool_audit.py" --a2a-chain --nightly
SMOKE_EXIT=$?

echo "Nightly semantic repo: $SEMANTIC_REPO" >&2
echo "Vector-index (semantic repo)..." >&2
"$CORTEX_BIN" vector-index "$SEMANTIC_REPO"

echo "Semantic audit (profile=nightly)..." >&2
CORTEX_SEMANTIC_REPO="$SEMANTIC_REPO" CORTEX_SEMANTIC_PROFILE=nightly CORTEX_BIN="$CORTEX_BIN" \
  python3 "$ROOT/scripts/mcp_semantic_audit.py" --profile nightly --repo "$SEMANTIC_REPO" --skip-preflight --a2a-chain
SEMANTIC_EXIT=$?

# Merge ledgers for triage
python3 - <<'PY' "$ROOT" || true
import json, sys
from pathlib import Path
root = Path(sys.argv[1])
smoke = root / "target" / "mcp-audit-ledger.json"
semantic = root / "target" / "mcp-semantic-ledger.json"
out = root / "target" / "mcp-full-audit.json"
merged = {"smoke": None, "semantic": None}
if smoke.is_file():
    merged["smoke"] = json.loads(smoke.read_text())
if semantic.is_file():
    merged["semantic"] = json.loads(semantic.read_text())
if merged["smoke"] or merged["semantic"]:
    out.write_text(json.dumps(merged, indent=2) + "\n")
    print(f"Wrote {out}", file=sys.stderr)
PY

echo "Fixture left at $FIXTURE (set CORTEX_AUDIT_FIXTURE to reuse)" >&2
EXIT=$((SMOKE_EXIT | SEMANTIC_EXIT))
exit $EXIT
