"""
CYNIC CLI — `review`, `watch`, `feedback` commands + `_print_action` helper.
"""
from __future__ import annotations

import sys
import time

from cynic.interfaces.cli.utils import (
    _ATYPE_COLOR,
    _PRIORITY_COLOR,
    _VERDICT_COLOR,
    _api_post,
    _bar,
    _c,
    _file_set_status,
    _pending_actions,
)


def _print_action(action: dict, index: int, total: int) -> None:
    """Render one proposed action for the review screen."""
    action.get("action_id", "?")
    atype     = action.get("action_type", "?")
    verdict   = action.get("verdict", "?")
    priority  = action.get("priority", 3)
    action.get("description", "")
    prompt    = action.get("prompt", "")[:120]
    float(action.get("proposed_at", 0))

    _ATYPE_COLOR.get(atype, "white")
    _PRIORITY_COLOR.get(priority, "white")
    _VERDICT_COLOR.get(verdict, "white")

    if prompt.strip():
        pass


def cmd_review() -> None:
    """
    Interactive review of pending proposed actions.

    Shows each PENDING action one at a time.
    Reads from API if server is running; falls back to direct file read.
    Writes accept/reject via API if available; falls back to file mode.

    Keys: [a]ccept  [r]eject  [s]kip  [q]uit
    """
    pending, api_available = _pending_actions()

    if not pending:
        return

    # Sort by priority (1=critical first), then by age
    pending.sort(key=lambda a: (a.get("priority", 3), a.get("proposed_at", 0)))

    "API mode" if api_available else _c("orange", "file mode (server offline)")

    accepted = rejected = skipped = 0

    for i, action in enumerate(pending, 1):
        action_id = action.get("action_id", "?")
        _print_action(action, i, len(pending))

        try:
            raw = input("  choice > ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            break

        if raw == "q":
            break
        elif raw == "a":
            if api_available:
                result = _api_post(f"/actions/{action_id}/accept")
                ok = result is not None and result.get("accepted")
            else:
                ok = _file_set_status(action_id, "ACCEPTED")
            if ok:
                accepted += 1
            else:
                pass
        elif raw == "r":
            if api_available:
                result = _api_post(f"/actions/{action_id}/reject")
                ok = result is not None and result.get("rejected")
            else:
                ok = _file_set_status(action_id, "REJECTED")
            if ok:
                rejected += 1
            else:
                pass
        else:
            skipped += 1



def cmd_watch() -> None:
    """
    Poll for new pending actions every N seconds.
    Notifies when the queue grows. Ctrl+C to stop.

    Usage: python -m cynic.interfaces.cli watch [interval_seconds]
    Default interval: 10s.
    """
    args = sys.argv[2:]
    try:
        interval = int(args[0]) if args else 10
    except ValueError:
        interval = 10


    last_count = -1
    last_ids: set = set()

    while True:
        try:
            pending, api_ok = _pending_actions()
        except CynicError:
            pending, _api_ok = [], False

        current_ids = {a.get("action_id") for a in pending}
        new_ids     = current_ids - last_ids
        count       = len(pending)

        if new_ids and last_ids:
            # New actions arrived
            for action_id in new_ids:
                action = next((a for a in pending if a.get("action_id") == action_id), {})
                atype  = action.get("action_type", "?")
                action.get("description", "")[:60]
                _ATYPE_COLOR.get(atype, "white")
        elif count != last_count:
            if count == 0 and last_count > 0:
                pass
            elif last_count >= 0:
                pass

        last_count = count
        last_ids   = current_ids

        try:
            time.sleep(interval)
        except KeyboardInterrupt:
            break


def cmd_feedback() -> None:
    """
    Rate the last kernel judgment (1=bad … 5=good).

    Usage: python -m cynic.interfaces.cli feedback [1-5]
    Requires server to be running.

    Example:
      python -m cynic.interfaces.cli feedback 4   → reward 0.70 → QTable update
    """
    args = sys.argv[2:]
    if not args:
        for rating, reward in [(1, 0.10), (2, 0.30), (3, 0.50), (4, 0.70), (5, 0.90)]:
            _bar(reward * 100, max_score=100.0, width=8)
        sys.exit(1)

    try:
        rating = int(args[0])
    except ValueError:
        sys.exit(1)

    if not (1 <= rating <= 5):
        sys.exit(1)

    result = _api_post("/feedback", {"rating": rating})
    if result is None:
        sys.exit(1)

    msg     = result.get("message", "")
    result.get("q_value", 0.0)
    reward  = result.get("reward", 0.0)
    result.get("action", "?")
    result.get("state_key", "")[:40]

    if msg:
        pass
