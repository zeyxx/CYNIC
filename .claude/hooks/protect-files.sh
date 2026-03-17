#!/usr/bin/env bash
# CYNIC — PreToolUse hook
# Blocks edits to sensitive files and detects secret patterns in commands.
# Reliable enforcement layer (deny rules for Read/Edit have known bugs).
set -euo pipefail

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty')
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# ── Protect sensitive files from Edit/Write ──
if [[ "$TOOL_NAME" == "Edit" || "$TOOL_NAME" == "Write" || "$TOOL_NAME" == "MultiEdit" ]]; then
    case "$FILE_PATH" in
        */.ssh/*)
            echo "BLOCKED: cannot edit SSH keys ($FILE_PATH)" >&2; exit 2 ;;
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            echo "BLOCKED: cannot edit secret config ($FILE_PATH)" >&2; exit 2 ;;
        */.env|*/.env.*)
            echo "BLOCKED: cannot edit env files ($FILE_PATH)" >&2; exit 2 ;;
        */.git/hooks/*)
            echo "BLOCKED: cannot modify git hooks — edit manually ($FILE_PATH)" >&2; exit 2 ;;
        */.claude/settings.local.json)
            echo "BLOCKED: cannot self-modify settings — edit manually ($FILE_PATH)" >&2; exit 2 ;;
    esac
fi

# ── Protect sensitive files from Read ──
if [[ "$TOOL_NAME" == "Read" ]]; then
    case "$FILE_PATH" in
        */.config/cynic/env|*/.config/cynic/llama-api-key)
            echo "BLOCKED: cannot read secret config ($FILE_PATH)" >&2; exit 2 ;;
        */.ssh/id_*|*/.ssh/known_hosts)
            echo "BLOCKED: cannot read SSH keys ($FILE_PATH)" >&2; exit 2 ;;
    esac
fi

# ── Detect secrets in Bash commands ──
if [[ "$TOOL_NAME" == "Bash" && -n "$COMMAND" ]]; then
    # Check for real secret values being echoed/written
    if echo "$COMMAND" | grep -qiE '(AIzaSy[A-Za-z0-9_-]{30}|hf_[A-Za-z0-9]{30}|sk-[A-Za-z0-9]{40})'; then
        echo "BLOCKED: command contains what looks like a real API key" >&2; exit 2
    fi
fi

# All clear
exit 0
