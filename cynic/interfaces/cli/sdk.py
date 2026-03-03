"""
CYNIC CLI " `sdk` command (recent Claude Code SDK sessions, L2).
"""
from __future__ import annotations

import json
import sys

from cynic.interfaces.cli.utils import (
    _SDK_SESSIONS_LOG,
    _api_get,
    _bar,
    _c,
)


def cmd_sdk() -> None:
    """
    Show recent Claude Code SDK sessions from JSONL persistence (L2).

    Sessions are written to ~/.cynic/sdk_sessions.jsonl after each SDK task.
    This command shows them without needing the server to be running.

    Usage: python -m cynic.interfaces.cli sdk [N]
      N: number of recent sessions to show (default 10)
    """
    args = sys.argv[2:]
    n = int(args[0]) if args and args[0].isdigit() else 10

    # Try API first (live view)
    data = _api_get(f"/act/telemetry?n={n}")

    if data is not None:
        sessions = data.get("sessions", [])
        stats = data.get("stats", {})
        api_ok = True
    else:
        # File fallback: read JSONL
        sessions = []
        try:
            with open(_SDK_SESSIONS_LOG, encoding="utf-8") as fh:
                all_lines = fh.readlines()
            for line in all_lines[-n:]:
                line = line.strip()
                if line:
                    try:
                        sessions.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except FileNotFoundError:
            pass
        stats = {}
        api_ok = False

    "API mode" if api_ok else _c("orange", "file mode (server offline)")

    if not sessions:
        return

    _VERDICT_COLOR = {"BARK": "red", "GROWL": "orange", "WAG": "yellow", "HOWL": "green"}

    for s in sessions:
        verdict   = s.get("output_verdict", "?")
        q_score   = float(s.get("output_q_score", 0))
        task      = (s.get("task") or "")[:60]
        s.get("task_type", "?")
        s.get("complexity", "?")
        float(s.get("total_cost_usd", 0))
        s.get("is_error", False)
        float(s.get("timestamp", 0))
        float(s.get("reward", 0))

        _VERDICT_COLOR.get(verdict, "white")

        _bar(q_score, max_score=100.0, width=8)

        if task:
            pass

    if stats:
        stats.get("count", 0)
        stats.get("error_rate", 0)
        stats.get("mean_q_score", 0)
        stats.get("total_cost_usd", 0)
