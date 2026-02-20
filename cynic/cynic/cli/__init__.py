"""
CYNIC CLI — Terminal status dashboard + human feedback loop (L3).

Usage:
  python -m cynic.cli            → full status (default)
  python -m cynic.cli status     → same
  python -m cynic.cli health     → quick health check only
  python -m cynic.cli lod        → LOD level only
  python -m cynic.cli loops      → 4 feedback loop completion matrix
  python -m cynic.cli review     → interactive accept/reject pending actions (L3)
  python -m cynic.cli watch      → live poll: notify when new actions arrive (L3)
  python -m cynic.cli feedback N → rate last judgment 1-5 (L3)
  python -m cynic.cli probes    → self-improvement proposals from L4 SelfProber
  python -m cynic.cli execute ID → accept + execute a pending action (L1 closure)
  python -m cynic.cli sdk [N]  → last N SDK sessions (L2 bidirectional loop)
  python -m cynic.cli consciousness → TUI dashboard (Ring 3 unified metathinking)
  python -m cynic.cli chat          → interactive coding assistant (CYNIC Code)
  python -m cynic.cli perceive-watch → real git PERCEIVE + JUDGE loop (Phase 2)

Reads from (fastest path — no server needed):
  ~/.cynic/guidance.json        → last judgment verdict/Q/dogs
  ~/.cynic/session-latest.json  → session checkpoint info
  ~/.cynic/pending_actions.json → proposed action queue

Optionally queries (falls back gracefully if server is down):
  http://localhost:PORT/health
  http://localhost:PORT/lod
  http://localhost:PORT/act/telemetry
  http://localhost:PORT/actions
  http://localhost:PORT/feedback
  http://localhost:PORT/self-probes
  http://localhost:PORT/act/telemetry

φ-bound: confidence never shown above 61.8%.
"""
from __future__ import annotations

import sys

from cynic.cli.status import cmd_status
from cynic.cli.health import cmd_health, cmd_lod, cmd_loops
from cynic.cli.actions import cmd_review, cmd_watch, cmd_feedback
from cynic.cli.probes import cmd_probes
from cynic.cli.execute import cmd_execute
from cynic.cli.sdk import cmd_sdk
from cynic.cli.consciousness import cmd_consciousness
from cynic.cli.tui import cmd_tui
from cynic.cli.chat import cmd_chat
from cynic.cli.perceive_watch import cmd_perceive_watch
from cynic.cli.utils import _c

# Re-export PORT/API constants for any consumer that might need them
from cynic.cli.utils import _PORT, _API  # noqa: F401

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
    }

    fn = dispatch.get(cmd)
    if fn is None:
        print(f"*head tilt* Unknown command: {cmd}")
        print(f"  Available: {', '.join(dispatch)}")
        sys.exit(1)

    fn()


if __name__ == "__main__":
    main()
