#!/usr/bin/env bash
# CYNIC — PreToolUse hook (security only)
# 1. Blocks edits to sensitive files
# 2. Detects secret patterns in commands
# Coord claims: see coord-claim.sh (scoped via `if` in settings.json)
set -euo pipefail

INPUT=$(cat)

block() {
    echo "BLOCKED: $*" >&2
    exit 2
}

# Guard: jq is required to parse hook payloads safely. Missing jq = no enforceable policy.
if ! command -v jq &>/dev/null; then
    block "jq is required for protect-files.sh"
fi

TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null || true)

# ── Protect sensitive files from Edit/Write ──
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" ]]; then
    case "$FILE_PATH" in
        */.ssh/*)
            block "cannot edit SSH keys ($FILE_PATH)" ;;
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            block "cannot edit secret config ($FILE_PATH)" ;;
        */.env|*/.env.*)
            block "cannot edit env files ($FILE_PATH)" ;;
        # .git/hooks and settings.local.json: no longer blocked.
        # Git hooks need maintenance. settings.local.json = permissions only.
    esac
fi

# ── Protect sensitive files from Read ──
if [[ "$TOOL_NAME" == "Read" ]]; then
    case "$FILE_PATH" in
        */.cynic-env|*/.config/cynic/env|*/.config/cynic/llama-api-key)
            block "cannot read secret config ($FILE_PATH)" ;;
        */.ssh/id_*|*/.ssh/known_hosts)
            block "cannot read SSH keys ($FILE_PATH)" ;;
    esac
fi

# ── Detect secrets in Bash commands ──
if [[ "$TOOL_NAME" == "Bash" && -n "$COMMAND" ]]; then
    if echo "$COMMAND" | grep -qiE '(AIzaSy[A-Za-z0-9_-]{30}|hf_[A-Za-z0-9]{30}|sk-[A-Za-z0-9]{40})'; then
        block "command contains what looks like a real API key"
    fi
fi

# Coord claims moved to coord-claim.sh (scoped via `if` in settings.json)

# All clear
exit 0
