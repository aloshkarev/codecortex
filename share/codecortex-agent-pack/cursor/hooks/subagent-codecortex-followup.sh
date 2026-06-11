#!/usr/bin/env bash
# Advisory follow-up when CodeCortex subagent may have hit freshness limits.
set -euo pipefail

input=$(cat)
output=$(echo "$input" | jq -r '.output // .result // .message // empty' 2>/dev/null || true)

if [[ -z "$output" ]]; then
  echo '{}'
  exit 0
fi

if ! echo "$output" | grep -qiE 'blocked_freshness|freshness.*unknown|freshness.*stale|freshness.*partial'; then
  echo '{}'
  exit 0
fi

jq -n '{
  followup_message: "CodeCortex index may be stale. Consider delegating codecortex-indexer to repair (check_health, index_status, explain_index_freshness), then retry analysis or review."
}'
exit 0
