#!/usr/bin/env bash
# Sync plugin/codecortex from canonical docs/ and .cursor/ assets in 64-codecortex.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${PLUGIN_ROOT}/../.." && pwd)"

SKILLS_SRC="${REPO_ROOT}/docs/skills"
AGENTS_SRC="${REPO_ROOT}/docs/agents"
HOOKS_SRC="${REPO_ROOT}/docs/cursor/hooks"
RULES_SRC="${REPO_ROOT}/.cursor/rules"
RULES_INDEX_SRC="${REPO_ROOT}/docs/cursor/RULES-INDEX.md"

echo "Syncing CodeCortex plugin from ${REPO_ROOT}"

# Skills (preserve plugin-only codecortex-setup)
SETUP_TMP=""
if [[ -d "${PLUGIN_ROOT}/skills/codecortex-setup" ]]; then
  SETUP_TMP="$(mktemp -d)"
  cp -a "${PLUGIN_ROOT}/skills/codecortex-setup" "${SETUP_TMP}/"
fi
rm -rf "${PLUGIN_ROOT}/skills"
mkdir -p "${PLUGIN_ROOT}/skills"
for d in codecortex codecortex-indexing codecortex-workflows; do
  cp -a "${SKILLS_SRC}/${d}" "${PLUGIN_ROOT}/skills/"
done
if [[ -n "${SETUP_TMP}" ]]; then
  cp -a "${SETUP_TMP}/codecortex-setup" "${PLUGIN_ROOT}/skills/"
  rm -rf "${SETUP_TMP}"
fi

# Agents (plugin-relative paths in body)
mkdir -p "${PLUGIN_ROOT}/agents"
for f in "${AGENTS_SRC}"/codecortex-*.md; do
  [[ -f "$f" ]] || continue
  sed \
    -e 's|docs/skills/|skills/|g' \
    -e 's|docs/agents/|agents/|g' \
    -e 's|docs/cursor/|cursor/|g' \
    -e 's|symlinked at `.cursor/agents/|packaged in `agents/|g' \
    -e 's|symlinked for Cursor at `.cursor/skills/|packaged in `skills/|g' \
    "$f" > "${PLUGIN_ROOT}/agents/$(basename "$f")"
  if ! grep -q 'CLAUDE_PLUGIN_ROOT' "${PLUGIN_ROOT}/agents/$(basename "$f")"; then
    sed -i '/^You are the CodeCortex/a\
\
When installed as a Claude Code plugin, skill paths live under `${CLAUDE_PLUGIN_ROOT}/skills/`.' "${PLUGIN_ROOT}/agents/$(basename "$f")" 2>/dev/null || true
  fi
done

# Hook scripts (Claude + Cursor copies)
mkdir -p "${PLUGIN_ROOT}/hooks" "${PLUGIN_ROOT}/cursor/hooks"
cp -a "${HOOKS_SRC}/"*.sh "${PLUGIN_ROOT}/hooks/"
cp -a "${HOOKS_SRC}/"*.sh "${PLUGIN_ROOT}/cursor/hooks/"
chmod +x "${PLUGIN_ROOT}/hooks/"*.sh "${PLUGIN_ROOT}/cursor/hooks/"*.sh

# Claude Code hooks.json
cat > "${PLUGIN_ROOT}/hooks/hooks.json" <<'EOF'
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-codecortex-context.sh"
      }
    ],
    "subagentStart": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/subagent-codecortex-inject.sh",
        "matcher": "codecortex-indexer|codecortex-analyzer|codecortex-pr-reviewer|codecortex-patch-planner"
      }
    ],
    "preToolUse": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/preflight-before-edit.sh",
        "matcher": "Write|StrReplace"
      }
    ],
    "postToolUse": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/after-task-codecortex.sh",
        "matcher": "Task"
      }
    ],
    "subagentStop": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/subagent-codecortex-followup.sh",
        "matcher": "codecortex-indexer|codecortex-analyzer|codecortex-pr-reviewer|codecortex-patch-planner"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/mcp-codecortex-reminder.sh"
      }
    ]
  }
}
EOF

# Cursor hooks.json (installed to .cursor/hooks.json)
cat > "${PLUGIN_ROOT}/cursor/hooks.json" <<'EOF'
{
  "version": 1,
  "hooks": {
    "sessionStart": [
      {
        "command": ".cursor/hooks/session-codecortex-context.sh"
      }
    ],
    "subagentStart": [
      {
        "command": ".cursor/hooks/subagent-codecortex-inject.sh",
        "matcher": "codecortex-indexer|codecortex-analyzer|codecortex-pr-reviewer|codecortex-patch-planner"
      }
    ],
    "preToolUse": [
      {
        "command": ".cursor/hooks/preflight-before-edit.sh",
        "matcher": "Write|StrReplace"
      }
    ],
    "postToolUse": [
      {
        "command": ".cursor/hooks/after-task-codecortex.sh",
        "matcher": "Task"
      }
    ],
    "subagentStop": [
      {
        "command": ".cursor/hooks/subagent-codecortex-followup.sh",
        "matcher": "codecortex-indexer|codecortex-analyzer|codecortex-pr-reviewer|codecortex-patch-planner"
      }
    ],
    "beforeMCPExecution": [
      {
        "command": ".cursor/hooks/mcp-codecortex-reminder.sh"
      }
    ]
  }
}
EOF

# Cursor rules
mkdir -p "${PLUGIN_ROOT}/cursor/rules"
cp -a "${RULES_SRC}"/codecortex-*.mdc "${PLUGIN_ROOT}/cursor/rules/"
for f in "${PLUGIN_ROOT}/cursor/rules/"*.mdc; do
  sed -i \
    -e 's|docs/skills/|skills/|g' \
    -e 's|docs/agents/|agents/|g' \
    -e 's|docs/cursor/|cursor/|g' \
    -e 's|../../AGENTS.md|plugin README + AGENTS.md in repo|g' \
    "$f" 2>/dev/null || true
done

cp -a "${RULES_INDEX_SRC}" "${PLUGIN_ROOT}/cursor/RULES-INDEX.md"
sed -i \
  -e 's|docs/skills/|skills/|g' \
  -e 's|docs/agents/|agents/|g' \
  -e 's|`.cursor/rules/`|plugin cursor/rules/ (install to .cursor/rules/)|g' \
  "${PLUGIN_ROOT}/cursor/RULES-INDEX.md" 2>/dev/null || true

# Optional share mirror for installed cortex binary (../share/codecortex-agent-pack)
SHARE_DST="${REPO_ROOT}/share/codecortex-agent-pack"
echo "Refreshing share mirror at ${SHARE_DST}"
rm -rf "${SHARE_DST}"
mkdir -p "${SHARE_DST}"
rsync -a --exclude 'scripts' "${PLUGIN_ROOT}/" "${SHARE_DST}/" 2>/dev/null \
  || cp -a "${PLUGIN_ROOT}/." "${SHARE_DST}/"

echo "Sync complete: ${PLUGIN_ROOT}"
