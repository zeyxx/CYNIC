"""
CYNIC CLI — `review`, `watch`, `feedback` commands + `_print_action` helper.
"""
from __future__ import annotations

import sys
import time

from cynic.cli.utils import (
    _API,
    _api_get, _api_post,
    _pending_actions, _file_set_status,
    _c, _bar, _ago,
    _ATYPE_COLOR, _PRIORITY_COLOR, _VERDICT_COLOR,
)


def _print_action(action: dict, index: int, total: int) -> None:
    """Render one proposed action for the review screen."""
    action_id = action.get("action_id", "?")
    atype     = action.get("action_type", "?")
    verdict   = action.get("verdict", "?")
    priority  = action.get("priority", 3)
    desc      = action.get("description", "")
    prompt    = action.get("prompt", "")[:120]
    ts        = float(action.get("proposed_at", 0))

    atype_col    = _ATYPE_COLOR.get(atype, "white")
    priority_col = _PRIORITY_COLOR.get(priority, "white")
    v_col        = _VERDICT_COLOR.get(verdict, "white")

    print()
    print(_c("bold", f"  [{index}/{total}]  {action_id}  {_ago(ts)}"))
    print(
        f"  {_c(atype_col, atype)}"
        f"  priority={_c(priority_col, str(priority))}"
        f"  verdict={_c(v_col, verdict)}"
    )
    print(f"  {desc}")
    if prompt.strip():
        print(f"  {_c('gray', prompt[:100] + ('…' if len(prompt) >= 100 else ''))}")


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
        print()
        print(_c("green", "  *tail wag* No pending actions — queue is empty."))
        print()
        return

    # Sort by priority (1=critical first), then by age
    pending.sort(key=lambda a: (a.get("priority", 3), a.get("proposed_at", 0)))

    print()
    print(_c("bold", f"  CYNIC ACTION REVIEW — {len(pending)} pending"))
    mode_note = "API mode" if api_available else _c("orange", "file mode (server offline)")
    print(f"  {_c('dim', mode_note)}  [a]ccept  [r]eject  [s]kip  [q]uit")

    accepted = rejected = skipped = 0

    for i, action in enumerate(pending, 1):
        action_id = action.get("action_id", "?")
        _print_action(action, i, len(pending))
        print()

        try:
            raw = input("  choice > ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print()
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
                print(_c("green", "  *tail wag* Accepted"))
                accepted += 1
            else:
                print(_c("red", "  *GROWL* Failed to accept — check server"))
        elif raw == "r":
            if api_available:
                result = _api_post(f"/actions/{action_id}/reject")
                ok = result is not None and result.get("rejected")
            else:
                ok = _file_set_status(action_id, "REJECTED")
            if ok:
                print(_c("orange", "  *head tilt* Rejected"))
                rejected += 1
            else:
                print(_c("red", "  *GROWL* Failed to reject — check server"))
        else:
            print(_c("dim", "  skipped"))
            skipped += 1

    print()
    print(
        f"  *yawn* Done.  accepted={_c('green', str(accepted))}"
        f"  rejected={_c('orange', str(rejected))}"
        f"  skipped={_c('dim', str(skipped))}"
    )
    print()


def cmd_watch() -> None:
    """
    Poll for new pending actions every N seconds.
    Notifies when the queue grows. Ctrl+C to stop.

    Usage: python -m cynic.cli watch [interval_seconds]
    Default interval: 10s.
    """
    args = sys.argv[2:]
    try:
        interval = int(args[0]) if args else 10
    except ValueError:
        interval = 10

    print()
    print(_c("bold", f"  *sniff* CYNIC WATCH — polling every {interval}s  (Ctrl+C to stop)"))
    print(_c("dim",  f"  Run 'python -m cynic.cli review' when actions appear"))
    print()

    last_count = -1
    last_ids: set = set()

    while True:
        try:
            pending, api_ok = _pending_actions()
        except Exception:
            pending, api_ok = [], False

        current_ids = {a.get("action_id") for a in pending}
        new_ids     = current_ids - last_ids
        count       = len(pending)

        if new_ids and last_ids:
            # New actions arrived
            for action_id in new_ids:
                action = next((a for a in pending if a.get("action_id") == action_id), {})
                atype  = action.get("action_type", "?")
                desc   = action.get("description", "")[:60]
                col    = _ATYPE_COLOR.get(atype, "white")
                print(
                    f"  *ears perk* [{_c(col, atype)}] {action_id}  {_c('dim', desc)}"
                )
            print(f"  → {count} pending  run 'cynic.cli review' to process")
        elif count != last_count:
            if count == 0 and last_count > 0:
                print(f"  *tail wag* Queue cleared — all actions processed")
            elif last_count >= 0:
                status_col = "green" if count == 0 else "yellow"
                print(f"  {_c(status_col, str(count))} pending actions  {_c('dim', _ago(time.time()))}")

        last_count = count
        last_ids   = current_ids

        try:
            time.sleep(interval)
        except KeyboardInterrupt:
            print()
            print(_c("dim", "  *yawn* Watch stopped."))
            print()
            break


def cmd_feedback() -> None:
    """
    Rate the last kernel judgment (1=bad … 5=good).

    Usage: python -m cynic.cli feedback [1-5]
    Requires server to be running.

    Example:
      python -m cynic.cli feedback 4   → reward 0.70 → QTable update
    """
    args = sys.argv[2:]
    if not args:
        print()
        print(_c("bold", "  Usage: python -m cynic.cli feedback [1-5]"))
        print()
        print("  Rates the last judgment seen by the kernel.")
        print("  Reward mapping (φ-aligned):")
        for rating, reward in [(1, 0.10), (2, 0.30), (3, 0.50), (4, 0.70), (5, 0.90)]:
            bar = _bar(reward * 100, max_score=100.0, width=8)
            print(f"    {rating}/5 → reward {reward:.2f}  {bar}")
        print()
        sys.exit(1)

    try:
        rating = int(args[0])
    except ValueError:
        print(_c("red", f"*GROWL* Rating must be 1-5, got: {args[0]}"))
        sys.exit(1)

    if not (1 <= rating <= 5):
        print(_c("red", f"*GROWL* Rating out of range: {rating} (must be 1-5)"))
        sys.exit(1)

    result = _api_post("/feedback", {"rating": rating})
    if result is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)

    msg     = result.get("message", "")
    q_value = result.get("q_value", 0.0)
    reward  = result.get("reward", 0.0)
    action  = result.get("action", "?")
    sk      = result.get("state_key", "")[:40]

    reward_col = "green" if reward >= 0.6 else ("cyan" if reward >= 0.4 else "orange")
    print()
    print(_c("bold", f"  *tail wag* Feedback: {rating}/5"))
    print(f"  reward={_c(reward_col, f'{reward:.2f}')}  Q[{action}]={q_value:.4f}")
    print(f"  {_c('dim', sk)}")
    if msg:
        print(f"  {_c('dim', msg[:80])}")
    print()
