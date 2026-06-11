#!/usr/bin/env bash
# Inject skill binding when a CodeCortex subagent starts (advisory).
set -euo pipefail

input=$(cat)
name=$(echo "$input" | jq -r '.subagent_type // .agent_name // .name // empty' 2>/dev/null || true)

skill=""
if [[ -n "${CLAUDE_PLUGIN_ROOT:-}" ]]; then
  case "$name" in
    codecortex-indexer) skill="${CLAUDE_PLUGIN_ROOT}/skills/codecortex-indexing/SKILL.md" ;;
    codecortex-analyzer) skill="${CLAUDE_PLUGIN_ROOT}/skills/codecortex/SKILL.md" ;;
    codecortex-pr-reviewer|codecortex-patch-planner) skill="${CLAUDE_PLUGIN_ROOT}/skills/codecortex-workflows/SKILL.md" ;;
  esac
else
  case "$name" in
    codecortex-indexer) skill="docs/skills/codecortex-indexing/SKILL.md" ;;
    codecortex-analyzer) skill="docs/skills/codecortex/SKILL.md" ;;
    codecortex-pr-reviewer|codecortex-patch-planner) skill="docs/skills/codecortex-workflows/SKILL.md" ;;
  esac
fi

if [[ -z "$skill" ]]; then
  echo '{}'
  exit 0
fi

jq -n \
  --arg skill "$skill" \
  --arg name "$name" \
  '{
    permission: "allow",
    additional_context: ("CodeCortex subagent " + $name + ": read " + $skill + " first. Use user-codecortex MCP only. Return Status, Freshness, Findings, Handoff sections.")
  }'
exit 0
