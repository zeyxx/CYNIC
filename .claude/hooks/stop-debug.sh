#!/bin/bash
set -euo pipefail

# Log each Stop hook execution
HOOKS=(
  "./.claude/hooks/askesis-claude-end.sh"
  "./.claude/hooks/session-stop.sh"
  "./.claude/hooks/dream-trigger.sh"
  "./.claude/hooks/exercise-scheduler.sh"
)

DEBUG_LOG="/tmp/stop-hooks-debug.log"
echo "=== Stop hooks execution at $(date -Is) ===" >> "$DEBUG_LOG"

for hook in "${HOOKS[@]}"; do
  echo "Running: $hook" >> "$DEBUG_LOG"
  if bash -x "$hook" < /dev/null >> "$DEBUG_LOG" 2>&1; then
    echo "  ✓ exit 0" >> "$DEBUG_LOG"
  else
    EXIT=$?
    echo "  ✗ exit $EXIT" >> "$DEBUG_LOG"
  fi
done

echo "Debug log: $DEBUG_LOG"
tail -30 "$DEBUG_LOG"
