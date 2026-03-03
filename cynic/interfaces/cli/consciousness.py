"""
CYNIC CLI " `consciousness` command (Ring 3 unified metathinking TUI dashboard).
"""
from __future__ import annotations

import time

from cynic.interfaces.cli.utils import (
    _CONSCIOUSNESS,
    _LOD_COLOR,
    _VERDICT_COLOR,
    _api_get,
    _read_json,
)


def cmd_consciousness() -> None:
    """
    TUI dashboard " unified metathinking output (Ring 3 KernelMirror).

    Reads ~/.cynic/consciousness.json (file) or GET /consciousness (API).
    Displays: overall health tier, subsystem breakdown, LLM routing stats,
    and a diff of what changed since the last judgment.

    Usage: python -m cynic.interfaces.cli consciousness
    """
    # Try API first, then file fallback
    data = _api_get("/consciousness")
    if data is None:
        data = _read_json(_CONSCIOUSNESS)
    if data is None:
        return

    mirror = data.get("mirror", {})
    float(data.get("uptime_s", 0.0))
    ts      = float(data.get("timestamp", 0.0))
    time.time() - ts if ts else 0.0

    # "" Header """"""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
    float(mirror.get("overall_health", 50.0))
    tier    = mirror.get("tier", "?")
    _VERDICT_COLOR.get(tier, "white")


    # "" QTable """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
    qt = mirror.get("qtable", {})
    if qt and not qt.get("error"):
        float(qt.get("coverage_pct", 0.0))
        int(qt.get("total_cells", 0))
        int(qt.get("total_updates", 0))

    # "" Axioms """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
    axioms = mirror.get("axioms", {})
    if axioms and not axioms.get("error"):
        atier = axioms.get("tier", "?")
        axioms.get("active_count", 0)
        axioms.get("total_signals", 0)
        {
            "TRANSCENDENT": "green", "AWAKENING": "cyan",
            "EMERGENCE": "yellow", "STIRRING": "orange", "DORMANT": "gray",
        }.get(atier, "white")
        # Per-axiom breakdown
        for _aname, ainfo in (axioms.get("axioms") or {}).items():
            if not isinstance(ainfo, dict):
                continue
            ainfo.get("state", "?")
            float(ainfo.get("maturity", 0.0))

    # "" LOD """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
    lod_data = mirror.get("lod", {})
    if lod_data and not lod_data.get("error"):
        lod_lvl = int(lod_data.get("current_lod", 0))
        lod_data.get("level_name", "?")
        _LOD_COLOR.get(lod_lvl, "white")

    # "" SAGE """""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""""
    sage = mirror.get("sage", {})
    if sage and sage.get("available"):
        float(sage.get("llm_activation_rate", 0.0))
        int(sage.get("heuristic_count", 0))
        int(sage.get("llm_count", 0))
        sage.get("temporal_mcts_active", False)

    # "" LLM Routing """"""""""""""""""""""""""""""""""""""""""""""""""""""""""
    routing = data.get("llm_routing", {})
    if routing and routing.get("available", True):
        float(routing.get("local_rate", 0.0))
        int(routing.get("total_routes", 0))
        int(routing.get("routes_to_local", 0))

    # "" Diff (what changed since last snapshot) """"""""""""""""""""""""""""""
    diff = data.get("diff", {})
    if diff:
        for _path, change in list(diff.items())[:8]:
            change.get("old", "?")
            change.get("new", "?")

