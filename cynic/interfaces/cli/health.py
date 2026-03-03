"""
CYNIC CLI " `health`, `lod`, `loops` commands.
"""
from __future__ import annotations

import sys

from cynic.interfaces.cli.utils import (
    _LOOPS,
    _api_get,
    _bar,
)


def cmd_health() -> None:
    data = _api_get("/health")
    if data is None:
        sys.exit(1)
    data.get("status", "?")
    data.get("uptime_s", 0)


def cmd_lod() -> None:
    data = _api_get("/lod")
    if data is None:
        sys.exit(1)
    data.get("current_lod", 0)


def cmd_loops() -> None:
    for _loop_name, (pct, _note) in _LOOPS.items():
        _bar(pct, max_score=100.0, width=12)
