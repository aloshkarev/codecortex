#!/usr/bin/env bash
# Suggest patch context before edits under crates/ (advisory only).
set -euo pipefail

input=$(cat)
path=$(echo "$input" | jq -r '
  .tool_input.path //
  .tool_input.file_path //
  .tool_input.target_file //
  .path //
  empty
' 2>/dev/null || true)

if [[ -z "$path" ]] || [[ "$path" != *"crates/"* ]]; then
  echo '{}'
  exit 0
fi

jq -n \
  --arg path "$path" \
  '{
    agent_message: ("CodeCortex: non-trivial edit under crates/. Prefer get_patch_context (user-codecortex) with include_paths scoped to the module, or delegate codecortex-patch-planner before broad file reads. Target: " + $path)
  }'
exit 0
