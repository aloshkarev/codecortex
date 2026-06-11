#!/usr/bin/env bash
# Advisory session context for CodeCortex (never blocks).
set -euo pipefail

cat <<'EOF'
{
  "additional_context": "CodeCortex (user-codecortex MCP): Discover → Act → Verify. Session preflight: manage_codecortex(action=assess) or check_health + index_status. First-time repo: manage_codecortex(action=bootstrap, install_agent_pack=true) or workspace_setup(install_agent_pack=true). Before non-trivial edits in crates/: get_patch_context or codecortex-patch-planner. Skills: .cursor/skills/codecortex*. Subagents: .cursor/agents/codecortex-*. Rules: .cursor/rules/codecortex-*.mdc. Resource: codecortex://guide/agent-pack-bootstrap. Do not claim high impact when freshness is stale, partial, or unknown."
}
EOF
exit 0
