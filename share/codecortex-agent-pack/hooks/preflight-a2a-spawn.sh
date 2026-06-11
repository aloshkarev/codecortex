#!/usr/bin/env bash
# Suggest A2A spawn before long multi-tool MCP chains (advisory only).
set -euo pipefail

input=$(cat)
server=$(echo "$input" | jq -r '.server // .mcp_server // empty' 2>/dev/null || true)
tool=$(echo "$input" | jq -r '.tool_name // .tool // empty' 2>/dev/null || true)

if echo "$server" | grep -qi 'codecortex'; then
  if echo "$tool" | grep -qiE 'get_impact_graph|analyze_code_relationships|branch_structural_diff|pr_review|find_patterns'; then
    jq -n '{
      permission: "allow",
      agent_message: "Multi-step review or impact trace: prefer a single cortex_a2a_spawn_session (consensus_review, impact_review, or pr_review) with wait_for_completion or subscribe_url — avoid chaining many graph tools in the host context. See docs/A2A.md and codecortex-a2a rule."
    }'
    exit 0
  fi
fi

echo '{ "permission": "allow" }'
exit 0
