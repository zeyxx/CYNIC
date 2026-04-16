#!/usr/bin/env bash
# CYNIC — PostToolUse: auto-format .rs files after Edit/Write (Layer 3)
# Fast (<1s). Sync. Only fires on .rs files.
#
# WHY --edition 2024: all CYNIC crates declare edition = "2024" (cynic-kernel,
# cynic-node). Edition 2024 changed rustfmt's default import sort from
# case-insensitive to ASCII (uppercase before lowercase). Without this flag,
# standalone rustfmt defaults to 2015 and reorders imports differently than
# `cargo fmt` used by the pre-commit gate, causing staged edits to fail
# `cargo fmt --check` with spurious whitespace/ordering diffs.
set -uo pipefail
INPUT=$(cat)
FILE=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[[ "$FILE" == *.rs ]] || exit 0
rustfmt --edition 2024 "$FILE" 2>&1 || echo "WARNING: rustfmt failed on $FILE" >&2
