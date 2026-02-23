"""
CYNIC CLI — `execute` command (L1 closure: accept + fire action).
"""
from __future__ import annotations

import sys

from cynic.cli.utils import (
    _PORT,
    _api_post,
    _c,
)


def cmd_execute() -> None:
    """
    Accept and immediately execute a pending action proposal (L1 closure).

    Accept marks the action ACCEPTED and fires ACT_REQUESTED → the runner
    executes it autonomously via Claude Code. This closes the full
    Machine→Actions loop without a web UI.

    Usage: python -m cynic.cli execute <action_id>
      action_id: 8-char hex prefix from `cynic.cli review`
    """
    args = sys.argv[2:]
    if not args:
        print()
        print(_c("red", "  *GROWL* Missing action_id."))
        print(_c("dim", "  Usage: python -m cynic.cli execute <action_id>"))
        print(_c("dim", "  Run `python -m cynic.cli review` to see pending actions."))
        print()
        return

    action_id = args[0]
    data = _api_post(f"/actions/{action_id}/accept")

    print()
    if data is None:
        print(_c("red", f"  *GROWL* Could not reach server. Is it running on port {_PORT}?"))
        print(_c("dim", "  Start: python -m cynic.api"))
        print()
        return

    action = data.get("action", {})
    executing = data.get("executing", False)
    desc = action.get("description", "?")[:80]
    prompt_preview = (action.get("prompt") or "")[:100]

    if executing:
        print(_c("green", f"  *tail wag* Action {action_id} ACCEPTED → executing"))
        print(f"  {_c('bold', desc)}")
        if prompt_preview:
            print(f"  prompt: {_c('dim', prompt_preview)}")
        print()
        print(_c("dim", "  Claude Code runner spawned. Check `cynic.cli sdk` for result."))
    else:
        print(_c("yellow", f"  *ears perk* Action {action_id} ACCEPTED (no prompt to execute)"))
        print(f"  {_c('dim', desc)}")
    print()
