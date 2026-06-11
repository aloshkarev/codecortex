#!/usr/bin/env bash
# Suggest CodeCortex subagent when Task prompt matches common intents (advisory).
set -euo pipefail

input=$(cat)
prompt=$(echo "$input" | jq -r '.tool_input.prompt // .tool_input.description // .prompt // empty' 2>/dev/null || true)

if [[ -z "$prompt" ]]; then
  echo '{}'
  exit 0
fi

subagent=""
if echo "$prompt" | grep -qiE 'stale|reindex|freshness|vector-index|cortex index|index_status'; then
  subagent="codecortex-indexer"
elif echo "$prompt" | grep -qiE 'who calls|callers|callees|blast radius|dead code|complexity|hybrid search'; then
  subagent="codecortex-analyzer"
elif echo "$prompt" | grep -qiE 'review.*branch|PR review|against main|get_delta|structural diff'; then
  subagent="codecortex-pr-reviewer"
elif echo "$prompt" | grep -qiE 'patch plan|before edit|get_patch_context|plan.*(fix|feature|refactor)'; then
  subagent="codecortex-patch-planner"
fi

if [[ -z "$subagent" ]]; then
  echo '{}'
  exit 0
fi

jq -n \
  --arg subagent "$subagent" \
  '{
    additional_context: ("CodeCortex: this Task may fit subagent " + $subagent + " (see docs/agents/README.md). Ensure index_status freshness before impact-heavy work.")
  }'
exit 0
