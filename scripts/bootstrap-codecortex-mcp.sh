#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <repo-path>"
  exit 1
fi

REPO_PATH="$1"

echo "[1/4] Running health check"
cortex doctor

echo "[2/4] Building graph index for: ${REPO_PATH}"
cortex index "${REPO_PATH}"

echo "[3/4] Building vector index for: ${REPO_PATH}"
cortex vector-index "${REPO_PATH}"

echo "[4/4] Starting MCP server (stdio)"
exec cortex mcp start
