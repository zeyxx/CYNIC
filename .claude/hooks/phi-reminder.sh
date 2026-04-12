#!/usr/bin/env bash
# UserPromptSubmit hook — phi-inverse reminder injected at every user turn.
# Zero model calls. Outputs additionalContext as JSON.

cat << 'CONTEXT'
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "φ⁻¹ REMINDER: Max confidence 0.618 on any claim. Label epistemic status: observed (probed), deduced (from observed), inferred (pattern), conjecture (hypothesis). If you cannot state what would falsify your conclusion, you do not have a conclusion."
  }
}
CONTEXT
