"""
CYNIC CLI — `consciousness` command (Ring 3 unified metathinking TUI dashboard).
"""
from __future__ import annotations

import time

from cynic.cli.utils import (
    _CONSCIOUSNESS,
    _api_get, _read_json,
    _c, _bar, _format_s, _verdict_str,
    _VERDICT_COLOR, _LOD_COLOR,
)


def cmd_consciousness() -> None:
    """
    TUI dashboard — unified metathinking output (Ring 3 KernelMirror).

    Reads ~/.cynic/consciousness.json (file) or GET /consciousness (API).
    Displays: overall health tier, subsystem breakdown, LLM routing stats,
    and a diff of what changed since the last judgment.

    Usage: python -m cynic.cli consciousness
    """
    # Try API first, then file fallback
    data = _api_get("/consciousness")
    source = "API"
    if data is None:
        data = _read_json(_CONSCIOUSNESS)
        source = "file"
    if data is None:
        print()
        print(_c("red", "  *GROWL* No consciousness data found."))
        print(_c("dim", "  Start the kernel: python -m cynic.api"))
        print(_c("dim", f"  Expected: {_CONSCIOUSNESS}"))
        print()
        return

    mirror = data.get("mirror", {})
    uptime  = float(data.get("uptime_s", 0.0))
    ts      = float(data.get("timestamp", 0.0))
    age_s   = time.time() - ts if ts else 0.0

    # ── Header ──────────────────────────────────────────────────────────────
    health  = float(mirror.get("overall_health", 50.0))
    tier    = mirror.get("tier", "?")
    tier_col = _VERDICT_COLOR.get(tier, "white")

    print()
    print(_c("bold", "  ╔══════════════════════════════════════════════════════╗"))
    print(_c("bold", "  ║           CYNIC CONSCIOUSNESS DASHBOARD              ║"))
    print(_c("bold", "  ╚══════════════════════════════════════════════════════╝"))
    print()
    print(
        f"  {_verdict_str(tier, health)}"
        f"  {_bar(health)}  health={health:.1f}"
        f"  uptime={_format_s(uptime)}"
    )
    print(
        _c("dim",
           f"  source={source}  age={age_s:.0f}s ago"
        )
    )
    print()

    # ── QTable ───────────────────────────────────────────────────────────────
    qt = mirror.get("qtable", {})
    if qt and not qt.get("error"):
        cov  = float(qt.get("coverage_pct", 0.0))
        cells = int(qt.get("total_cells", 0))
        updates = int(qt.get("total_updates", 0))
        print(
            f"  {_c('cyan', 'QTABLE')}"
            f"  coverage={_bar(cov)}  {cov:.1f}%"
            f"  cells={cells}/343"
            f"  updates={updates}"
        )

    # ── Axioms ───────────────────────────────────────────────────────────────
    axioms = mirror.get("axioms", {})
    if axioms and not axioms.get("error"):
        atier = axioms.get("tier", "?")
        active = axioms.get("active_count", 0)
        total_sigs = axioms.get("total_signals", 0)
        atier_col = {
            "TRANSCENDENT": "green", "AWAKENING": "cyan",
            "EMERGENCE": "yellow", "STIRRING": "orange", "DORMANT": "gray",
        }.get(atier, "white")
        print(
            f"  {_c('cyan', 'AXIOMS')}"
            f"  tier={_c(atier_col, atier)}"
            f"  active={active}  signals={total_sigs}"
        )
        # Per-axiom breakdown
        for aname, ainfo in (axioms.get("axioms") or {}).items():
            if not isinstance(ainfo, dict):
                continue
            astate  = ainfo.get("state", "?")
            mat     = float(ainfo.get("maturity", 0.0))
            acol    = "green" if astate == "ACTIVE" else ("orange" if astate == "MATURING" else "gray")
            print(
                f"    {_c(acol, aname[:12]):<18}"
                f"  {_bar(mat * 100, width=8)}"
                f"  {astate}"
            )

    # ── LOD ─────────────────────────────────────────────────────────────────
    lod_data = mirror.get("lod", {})
    if lod_data and not lod_data.get("error"):
        lod_lvl = int(lod_data.get("current_lod", 0))
        lod_name = lod_data.get("level_name", "?")
        lod_col  = _LOD_COLOR.get(lod_lvl, "white")
        print(
            f"  {_c('cyan', 'LOD')}"
            f"  level={_c(lod_col, str(lod_lvl))}"
            f"  {lod_name}"
        )

    # ── SAGE ─────────────────────────────────────────────────────────────────
    sage = mirror.get("sage", {})
    if sage and sage.get("available"):
        llm_rate  = float(sage.get("llm_activation_rate", 0.0))
        heur      = int(sage.get("heuristic_count", 0))
        llm_ct    = int(sage.get("llm_count", 0))
        temporal  = sage.get("temporal_mcts_active", False)
        sage_col  = "green" if temporal else "orange"
        print(
            f"  {_c('cyan', 'SAGE')}"
            f"  llm_rate={_bar(llm_rate * 100, width=6)}"
            f"  {llm_rate:.0%}"
            f"  temporal={'ON' if temporal else 'OFF'}"
            f"  {_c(sage_col, f'heur={heur} llm={llm_ct}')}"
        )

    # ── LLM Routing ──────────────────────────────────────────────────────────
    routing = data.get("llm_routing", {})
    if routing and routing.get("available", True):
        local_r  = float(routing.get("local_rate", 0.0))
        total_r  = int(routing.get("total_routes", 0))
        local_ct = int(routing.get("routes_to_local", 0))
        print(
            f"  {_c('cyan', 'LLM_ROUTER')}"
            f"  local_rate={_bar(local_r * 100, width=6)}"
            f"  {local_r:.0%}"
            f"  haiku={local_ct}/{total_r}"
        )

    # ── Diff (what changed since last snapshot) ──────────────────────────────
    diff = data.get("diff", {})
    if diff:
        print()
        print(_c("bold", "  CHANGES since last snapshot:"))
        for path, change in list(diff.items())[:8]:
            old = change.get("old", "?")
            new = change.get("new", "?")
            print(f"    {_c('dim', path)}: {old} → {_c('yellow', str(new))}")

    print()
