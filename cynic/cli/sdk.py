"""
CYNIC CLI — `sdk` command (recent Claude Code SDK sessions, L2).
"""
from __future__ import annotations

import json
import sys

from cynic.cli.utils import (
    _SDK_SESSIONS_LOG,
    _api_get,
    _c, _bar, _ago,
)


def cmd_sdk() -> None:
    """
    Show recent Claude Code SDK sessions from JSONL persistence (L2).

    Sessions are written to ~/.cynic/sdk_sessions.jsonl after each SDK task.
    This command shows them without needing the server to be running.

    Usage: python -m cynic.cli sdk [N]
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

    print()
    mode_note = "API mode" if api_ok else _c("orange", "file mode (server offline)")
    print(_c("bold", f"  CYNIC SDK SESSIONS — last {len(sessions)}"))
    print(f"  {_c('dim', mode_note)}")
    print()

    if not sessions:
        print(_c("gray", "  *sniff* No SDK sessions recorded yet."))
        print(_c("dim", "  Sessions appear after running /act/execute."))
        print()
        return

    _VERDICT_COLOR = {"BARK": "red", "GROWL": "orange", "WAG": "yellow", "HOWL": "green"}

    for s in sessions:
        verdict   = s.get("output_verdict", "?")
        q_score   = float(s.get("output_q_score", 0))
        task      = (s.get("task") or "")[:60]
        t_type    = s.get("task_type", "?")
        complexity = s.get("complexity", "?")
        cost      = float(s.get("total_cost_usd", 0))
        is_error  = s.get("is_error", False)
        ts        = float(s.get("timestamp", 0))
        reward    = float(s.get("reward", 0))

        vc = _VERDICT_COLOR.get(verdict, "white")
        ec = "red" if is_error else "green"

        q_bar = _bar(q_score, max_score=100.0, width=8)

        print(
            f"  {_c(vc, verdict):<8}"
            f"  Q={_c(vc, q_bar)}"
            f"  {_c(ec, 'ERR' if is_error else 'OK ')}"
            f"  {t_type:<8}"
            f"  {complexity:<8}"
            f"  ${cost:.4f}"
            f"  r={reward:.3f}"
            f"  {_ago(ts)}"
        )
        if task:
            print(f"    {_c('dim', task)}")
    print()

    if stats:
        count = stats.get("count", 0)
        err_r = stats.get("error_rate", 0)
        mean_q = stats.get("mean_q_score", 0)
        total_c = stats.get("total_cost_usd", 0)
        print(
            _c("dim",
               f"  sessions={count}"
               f"  error_rate={err_r:.1%}"
               f"  mean_Q={mean_q:.1f}"
               f"  total_cost=${total_c:.4f}"
            )
        )
        print()
