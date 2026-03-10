#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <session-id>"
  exit 1
fi

SESSION_ID="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOME_DIR="${HOME}/.codecortex-measurement"
LOG_DIR="${HOME_DIR}/logs"
LOG_FILE="${LOG_DIR}/mcp-${SESSION_ID}-$(date -u +%Y%m%dT%H%M%SZ).log"
MEASURE_PY="${SCRIPT_DIR}/codecortex_measure.py"

mkdir -p "${LOG_DIR}"

echo "Starting MCP capture for session: ${SESSION_ID}"
echo "Log file: ${LOG_FILE}"

set +e
cortex mcp start 2>&1 | tee "${LOG_FILE}"
MCP_EXIT=${PIPESTATUS[0]}
set -e

python3 "${MEASURE_PY}" snapshot --session-id "${SESSION_ID}" --mcp-log-path "${LOG_FILE}" || true

exit "${MCP_EXIT}"
