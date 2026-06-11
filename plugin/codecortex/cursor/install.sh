#!/usr/bin/env bash
# Install CodeCortex Cursor skills, agents, hooks, rules, and project MCP config.
set -euo pipefail

CURSOR_PACK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "${CURSOR_PACK}/.." && pwd)"
PROJECT_ROOT="$(pwd)"
USE_SYMLINK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --symlink) USE_SYMLINK=1; shift ;;
    -h|--help)
      echo "Usage: install.sh [--symlink]"
      echo "  Installs from ${PLUGIN_ROOT} into ${PROJECT_ROOT}/.cursor/"
      exit 0
      ;;
    *) echo "unknown option: $1" >&2; exit 1 ;;
  esac
done

if [[ ! -f "${CURSOR_PACK}/hooks.json" ]]; then
  echo "error: missing ${CURSOR_PACK}/hooks.json" >&2
  exit 1
fi

install_tree() {
  local src="$1"
  local dest="$2"
  local name
  name="$(basename "$src")"
  local target="${dest}/${name}"
  if [[ -e "$target" ]]; then
    echo "warning: ${target} exists — skipping (use MCP workspace_setup with overwrite to replace)"
    return 0
  fi
  if [[ "$USE_SYMLINK" -eq 1 ]]; then
    ln -s "$src" "$target"
  elif [[ -d "$src" ]]; then
    cp -a "$src" "$target"
  else
    cp -a "$src" "$target"
  fi
  echo "  installed ${target}"
}

mkdir -p "${PROJECT_ROOT}/.cursor/skills" "${PROJECT_ROOT}/.cursor/agents"
mkdir -p "${PROJECT_ROOT}/.cursor/hooks" "${PROJECT_ROOT}/.cursor/rules"

echo "Installing skills from ${PLUGIN_ROOT}/skills/"
for d in "${PLUGIN_ROOT}/skills/"*/; do
  [[ -d "$d" ]] || continue
  install_tree "$d" "${PROJECT_ROOT}/.cursor/skills"
done

echo "Installing agents from ${PLUGIN_ROOT}/agents/"
for f in "${PLUGIN_ROOT}/agents/"*.md; do
  [[ -f "$f" ]] || continue
  install_tree "$f" "${PROJECT_ROOT}/.cursor/agents"
done

echo "Installing hooks to ${PROJECT_ROOT}/.cursor/hooks/"
cp -a "${CURSOR_PACK}/hooks/"*.sh "${PROJECT_ROOT}/.cursor/hooks/"
chmod +x "${PROJECT_ROOT}/.cursor/hooks/"*.sh

if [[ -f "${PROJECT_ROOT}/.cursor/hooks.json" ]]; then
  echo "warning: backing up existing hooks.json to hooks.json.bak.codecortex"
  cp "${PROJECT_ROOT}/.cursor/hooks.json" "${PROJECT_ROOT}/.cursor/hooks.json.bak.codecortex"
fi
cp "${CURSOR_PACK}/hooks.json" "${PROJECT_ROOT}/.cursor/hooks.json"

echo "Installing rules to ${PROJECT_ROOT}/.cursor/rules/"
for f in "${CURSOR_PACK}/rules/"*.mdc; do
  [[ -f "$f" ]] || continue
  dest="${PROJECT_ROOT}/.cursor/rules/$(basename "$f")"
  if [[ -f "$dest" ]]; then
    echo "warning: ${dest} exists — skipping"
  else
    cp -a "$f" "$dest"
    echo "  installed ${dest}"
  fi
done

REPO_CWD="${PROJECT_ROOT}"
CORTEX_CMD="${CORTEX_MCP_COMMAND:-cortex}"
MCP_JSON="$(cat <<EOF
{
  "mcpServers": {
    "codecortex": {
      "command": "${CORTEX_CMD}",
      "args": ["mcp", "start"],
      "cwd": "${REPO_CWD}"
    }
  }
}
EOF
)"

mkdir -p "${PROJECT_ROOT}/.cursor"
for out in "${PROJECT_ROOT}/.cursor/mcp.json" "${PROJECT_ROOT}/mcp.json"; do
  if [[ -f "$out" ]]; then
    echo "warning: ${out} exists — skipping (see docs/INTEGRATION.md)"
  else
    printf '%s\n' "$MCP_JSON" > "$out"
    echo "  wrote ${out}"
  fi
done

echo "Done. Global MCP: ~/.cursor/mcp.json (see docs/INTEGRATION.md)."
echo "MCP tools: manage_codecortex (assess), workspace_setup (install_agent_pack=true)."
