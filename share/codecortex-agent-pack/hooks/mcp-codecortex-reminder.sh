#!/usr/bin/env bash
# Remind to use CodeCortex MCP for repo-structure questions (advisory only).
set -euo pipefail

input=$(cat)
server=$(echo "$input" | jq -r '.server // .mcp_server // empty' 2>/dev/null || true)
tool=$(echo "$input" | jq -r '.tool_name // .tool // empty' 2>/dev/null || true)

# Already using CodeCortex
if echo "$server" | grep -qi 'codecortex'; then
  echo '{ "permission": "allow" }'
  exit 0
fi

# Heuristic: graph/structure tools on other servers — nudge only
if echo "$tool" | grep -qiE 'search|grep|find|codebase'; then
  jq -n '{
    permission: "allow",
    agent_message: "For callers, impact, tests around a change, or patch context, prefer user-codecortex MCP (check_health, index_status, recommend_tools) instead of broad search when the repo is indexed."
  }'
  exit 0
fi

echo '{ "permission": "allow" }'
exit 0
