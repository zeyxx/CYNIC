#!/usr/bin/env bash
# CYNIC — Branch enforcement gate (PreToolUse / BeforeTool)
# Blocks Edit/Write on main/master branch. Forces branch-before-edit discipline.
# CHAOS baseline: 51% compliance — needs mechanical enforcement.
#
# Works for both Claude Code (exit 2 = block) and Gemini CLI (JSON decision: deny).
set -uo pipefail

INPUT=$(cat)

# Detect CLI
HOOK_EVENT=$(echo "$INPUT" | jq -r '.hook_event_name // empty' 2>/dev/null || true)
IS_GEMINI=false
if [[ "$HOOK_EVENT" == "BeforeTool" ]] || [[ -n "${GEMINI_PROJECT_DIR:-}" ]]; then
    IS_GEMINI=true
fi

PROJECT_DIR="${GEMINI_PROJECT_DIR:-${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}}"

# Get current branch
BRANCH=$(git -C "$PROJECT_DIR" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

# Allow if not on main/master
if [[ "$BRANCH" != "main" && "$BRANCH" != "master" ]]; then
    if [[ "$IS_GEMINI" == "true" ]]; then
        echo '{}'
    fi
    exit 0
fi

# On main — check if this is an Edit/Write operation
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)

# Only gate edit tools
case "$TOOL_NAME" in
    Edit|Write|edit_file|write_file|replace_in_file|apply_diff) ;;
    *)
        if [[ "$IS_GEMINI" == "true" ]]; then
            echo '{}'
        fi
        exit 0
        ;;
esac

# Extract target path
TARGET=$(echo "$INPUT" | jq -r '.tool_input.file_path // .tool_input.path // empty' 2>/dev/null || true)

# Allow edits to operational files on main (hooks, memory, agents, config)
case "$TARGET" in
    */.claude/hooks/*|*/.gemini/hooks/*|*/.claude/memory/*|*/.claude/agents/*|*/MEMORY.md|*/.claude/settings*|*/.gemini/settings*)
        if [[ "$IS_GEMINI" == "true" ]]; then
            echo '{}'
        fi
        exit 0
        ;;
esac

# BLOCK: editing code on main
REASON="BRANCH GUARD: You are on '${BRANCH}'. Create a feature branch before editing code. Run: git checkout -b <type>/<scope>-\$(date +%Y-%m-%d)-\$(head -c4 /dev/urandom | xxd -p)"

if [[ "$IS_GEMINI" == "true" ]]; then
    jq -n --arg reason "$REASON" '{
        hookSpecificOutput: {
            decision: "deny",
            reason: $reason
        },
        systemMessage: $reason
    }'
else
    echo "$REASON" >&2
    exit 2
fi
