"""
CYNIC CLI — `health`, `lod`, `loops` commands.
"""
from __future__ import annotations

import sys

from cynic.cli.utils import (
    _API,
    _api_get,
    _c, _bar, _format_s, _lod_str,
    _LOOPS,
)


def cmd_health() -> None:
    data = _api_get("/health")
    if data is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)
    status = data.get("status", "?")
    uptime = data.get("uptime_s", 0)
    col = "green" if status == "alive" else "orange"
    print(f"*sniff* {_c(col, status.upper())}  uptime={_format_s(uptime)}  dogs={len(data.get('dogs',[]))}")


def cmd_lod() -> None:
    data = _api_get("/lod")
    if data is None:
        print(_c("red", f"*GROWL* Server unreachable at {_API}"))
        sys.exit(1)
    lod_val = data.get("current_lod", 0)
    print(_lod_str(lod_val) + f"  {_c('dim', data.get('description',''))}")


def cmd_loops() -> None:
    print()
    print(_c("bold", "  CYNIC FEEDBACK LOOPS — completion matrix"))
    print()
    for loop_name, (pct, note) in _LOOPS.items():
        bar = _bar(pct, max_score=100.0, width=12)
        col = "green" if pct >= 80 else ("cyan" if pct >= 61 else ("yellow" if pct >= 42 else "red"))
        print(f"  {_c(col, bar)} {pct:3d}%  {_c('bold', loop_name)}")
        print(f"         {_c('dim', note)}")
    print()
