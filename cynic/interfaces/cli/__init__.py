"""
CYNIC CLI â€" Terminal status dashboard + human feedback loop (L3).

Usage:
  python -m cynic.interfaces.cli            â' full status (default)
  python -m cynic.interfaces.cli status     â' same
  python -m cynic.interfaces.cli health     â' quick health check only
  python -m cynic.interfaces.cli lod        â' LOD level only
  python -m cynic.interfaces.cli loops      â' 4 feedback loop completion matrix
  python -m cynic.interfaces.cli review     â' interactive accept/reject pending actions (L3)
  python -m cynic.interfaces.cli watch      â' live poll: notify when new actions arrive (L3)
  python -m cynic.interfaces.cli feedback N â' rate last judgment 1-5 (L3)
  python -m cynic.interfaces.cli probes    â' self-improvement proposals from L4 SelfProber
  python -m cynic.interfaces.cli execute ID â' accept + execute a pending action (L1 closure)
  python -m cynic.interfaces.cli sdk [N]  â' last N SDK sessions (L2 bidirectional loop)
  python -m cynic.interfaces.cli consciousness â' TUI dashboard (Ring 3 unified metathinking)
  python -m cynic.interfaces.cli dashboard     â' Health Dashboard (8 breathing checks + 7Ã-7 matrix)
  python -m cynic.interfaces.cli chat          â' interactive coding assistant (CYNIC Code)
  python -m cynic.interfaces.cli perceive-watch â' real git PERCEIVE + JUDGE loop (Phase 2)

Reads from (fastest path â€" no server needed):
  ~/.cynic/guidance.json        â' last judgment verdict/Q/dogs
  ~/.cynic/session-latest.json  â' session checkpoint info
  ~/.cynic/pending_actions.json â' proposed action queue

Optionally queries (falls back gracefully if server is down):
  http://localhost:PORT/health
  http://localhost:PORT/lod
  http://localhost:PORT/act/telemetry
  http://localhost:PORT/actions
  http://localhost:PORT/feedback
  http://localhost:PORT/self-probes
  http://localhost:PORT/act/telemetry

Ï-bound: confidence never shown above 61.8%.
"""
from __future__ import annotations

import sys

from cynic.interfaces.cli.actions import cmd_feedback, cmd_review, cmd_watch
from cynic.interfaces.cli.battles import cmd_battles
from cynic.interfaces.cli.chat import cmd_chat
from cynic.interfaces.cli.consciousness import cmd_consciousness
from cynic.interfaces.cli.dashboard import cmd_dashboard
from cynic.interfaces.cli.execute import cmd_execute
from cynic.interfaces.cli.full_loop import cmd_full_loop
from cynic.interfaces.cli.health import cmd_health, cmd_lod, cmd_loops
from cynic.interfaces.cli.perceive_watch import cmd_perceive_watch
from cynic.interfaces.cli.probes import cmd_probes
from cynic.interfaces.cli.sdk import cmd_sdk
from cynic.interfaces.cli.status import cmd_status
from cynic.interfaces.cli.tui import cmd_tui

# Re-export PORT/API constants for any consumer that might need them
from cynic.interfaces.cli.utils import _API, _PORT, _c  # noqa: F401

__all__ = [
    "cmd_status",
    "cmd_health",
    "cmd_lod",
    "cmd_loops",
    "cmd_review",
    "cmd_watch",
    "cmd_feedback",
    "cmd_probes",
    "cmd_execute",
    "cmd_sdk",
    "cmd_consciousness",
    "cmd_tui",
    "cmd_chat",
    "cmd_perceive_watch",
    "cmd_full_loop",
    "cmd_battles",
    "cmd_dashboard",
    "main",
]


def main() -> None:
    args = sys.argv[1:]
    cmd  = args[0] if args else "status"

    dispatch = {
        "status":        cmd_status,
        "health":        cmd_health,
        "lod":           cmd_lod,
        "loops":         cmd_loops,
        "review":        cmd_review,
        "watch":         cmd_watch,
        "feedback":      cmd_feedback,
        "probes":        cmd_probes,
        "execute":       cmd_execute,
        "sdk":           cmd_sdk,
        "consciousness": cmd_consciousness,
        "tui":           cmd_tui,
        "chat":          cmd_chat,
        "perceive-watch": cmd_perceive_watch,
        "full-loop":     cmd_full_loop,
        "battles":       cmd_battles,
        "dashboard":     cmd_dashboard,
    }

    fn = dispatch.get(cmd)
    if fn is None:
        sys.exit(1)

    fn()


if __name__ == "__main__":
    main()
