"""
CYNIC CLI " `execute` command (L1 closure: accept + fire action).
"""

from __future__ import annotations

import sys

from cynic.interfaces.cli.utils import (
    _api_post,
)


def cmd_execute() -> None:
    """
    Accept and immediately execute a pending action proposal (L1 closure).

    Accept marks the action ACCEPTED and fires ACT_REQUESTED ' the runner
    executes it autonomously via Claude Code. This closes the full
    Machine'Actions loop without a web UI.

    Usage: python -m cynic.interfaces.cli execute <action_id>
      action_id: 8-char hex prefix from `cynic.interfaces.cli review`
    """
    args = sys.argv[2:]
    if not args:
        return

    action_id = args[0]
    data = _api_post(f"/actions/{action_id}/accept")

    if data is None:
        return

    action = data.get("action", {})
    executing = data.get("executing", False)
    action.get("description", "?")[:80]
    prompt_preview = (action.get("prompt") or "")[:100]

    if executing:
        if prompt_preview:
            pass
    else:
        pass
