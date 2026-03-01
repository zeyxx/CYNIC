"""
CYNIC benchmarks router — performance monitoring: auto-benchmark · probe · drift
"""
from __future__ import annotations

import logging
from typing import Any

from fastapi import APIRouter, Depends

from cynic.interfaces.api.state import AppContainer, get_app_container

logger = logging.getLogger("cynic.interfaces.api.server")

router_benchmarks = APIRouter(tags=["benchmarks"])


# ════════════════════════════════════════════════════════════════════════════
# GET /auto-benchmark/stats
# ════════════════════════════════════════════════════════════════════════════

@router_benchmarks.get("/auto-benchmark/stats")
async def auto_benchmark_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """AutoBenchmark probe stats — interval, runs, enabled flag (T09)."""
    state = container.organism
    if state.auto_benchmark is None:
        return {"enabled": False, "runs": 0, "interval_s": 0}
    return state.auto_benchmark.stats()


# ════════════════════════════════════════════════════════════════════════════
# POST /auto-benchmark/run
# ════════════════════════════════════════════════════════════════════════════

@router_benchmarks.post("/auto-benchmark/run")
async def auto_benchmark_run(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Trigger an immediate AutoBenchmark round (T09)."""
    state = container.organism
    if state.auto_benchmark is None:
        return {"completed": 0, "message": "auto_benchmark not initialised"}
    completed = await state.auto_benchmark.run_once()
    return {"completed": completed}


# ════════════════════════════════════════════════════════════════════════════
# GET /benchmark/probe-snapshot
# ════════════════════════════════════════════════════════════════════════════

@router_benchmarks.get("/benchmark/probe-snapshot")
async def benchmark_probe_snapshot(
    window: int = 10,
    source: str = "evolve",
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Rolling aggregate of the last N probe runs per probe (pass_rate, mean_q, std_q).

    Calls BenchmarkRegistry.snapshot() and writes a new benchmark_snapshots row.
    Returns {} when no DB pool is available (heuristic mode).
    """
    state = container.organism
    reg = getattr(state.orchestrator, "benchmark_registry", None)
    if reg is None:
        return {"available": False, "probes": {}}
    probes = await reg.snapshot(window_runs=window, source=source)
    return {"available": True, "window": window, "source": source, "probes": probes}


# ════════════════════════════════════════════════════════════════════════════
# GET /benchmark/drift-alerts
# ════════════════════════════════════════════════════════════════════════════

@router_benchmarks.get("/benchmark/drift-alerts")
async def benchmark_drift_alerts(
    threshold: float = 0.15,
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Detect probes whose pass_rate dropped >= threshold vs previous snapshot.

    severity: CRITICAL if delta >= 0.30, WARNING otherwise.
    Returns empty alerts list when no DB pool or insufficient snapshot history.
    """
    state = container.organism
    reg = getattr(state.orchestrator, "benchmark_registry", None)
    if reg is None:
        return {"available": False, "threshold": threshold, "alerts": []}
    alerts = await reg.drift_alerts(threshold=threshold)
    return {
        "available": True,
        "threshold": threshold,
        "alert_count": len(alerts),
        "alerts": alerts,
    }
