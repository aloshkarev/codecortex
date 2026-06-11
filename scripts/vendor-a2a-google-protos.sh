#!/usr/bin/env bash
# Refresh vendored google.api / google.protobuf files for docs/a2a.proto codegen.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_DIR="$ROOT/docs/google/api"
PB_DIR="$ROOT/docs/google/protobuf"
GOOGLEAPIS="https://raw.githubusercontent.com/googleapis/googleapis/master"
PROTOBUF="https://raw.githubusercontent.com/protocolbuffers/protobuf/main/src/google/protobuf"

mkdir -p "$API_DIR" "$PB_DIR"

fetch() {
  local url="$1"
  local dest="$2"
  if [[ -f "$dest" ]] && [[ "${FORCE:-0}" != "1" ]]; then
    echo "skip (exists): $dest"
    return 0
  fi
  echo "fetch: $dest"
  curl -fsSL --connect-timeout 15 --max-time 120 "$url" -o "$dest"
}

for f in field_behavior.proto http.proto annotations.proto client.proto launch_stage.proto; do
  fetch "$GOOGLEAPIS/google/api/$f" "$API_DIR/$f"
done

# Only vendor messages referenced directly by docs/a2a.proto. descriptor/duration stay on
# the system protoc include path (main-branch descriptor.proto breaks older protoc).
for f in empty.proto struct.proto timestamp.proto; do
  fetch "$PROTOBUF/$f" "$PB_DIR/$f"
done

echo "Vendored protos under docs/google/ ($(find "$ROOT/docs/google" -name '*.proto' | wc -l) files)"
