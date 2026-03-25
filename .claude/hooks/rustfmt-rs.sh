#!/usr/bin/env bash
# CYNIC — PostToolUse: auto-format .rs files after Edit/Write (Layer 3)
# Fast (<1s). Sync. Only fires on .rs files.
set -uo pipefail
INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[[ "$FILE" == *.rs ]] || exit 0
rustfmt "$FILE" 2>&1 || echo "WARNING: rustfmt failed on $FILE" >&2
