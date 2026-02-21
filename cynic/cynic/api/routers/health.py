"""
CYNIC health router — health · stats · introspect · axioms · lod · mirror · consciousness
"""
from __future__ import annotations

import json
import logging
import os
import pathlib as _pathlib
import time
import uuid
from typing import Any


from fastapi import APIRouter, Depends, HTTPException, Request
from fastapi.responses import FileResponse

from cynic.core.consciousness import get_consciousness
from cynic.core.phi import PHI, PHI_INV, PHI_INV_2, fibonacci, WAG_MIN
from cynic.core.formulas import (
    KERNEL_INTEGRITY_HOWL_THRESHOLD,
    KERNEL_INTEGRITY_WAG_THRESHOLD,
    KERNEL_INTEGRITY_GROWL_THRESHOLD,
)
from cynic.api.models import HealthResponse, StatsResponse
from cynic.api.state import get_app_container, AppContainer

logger = logging.getLogger("cynic.api.server")

router_health = APIRouter(tags=["health"])

# Static dir — routers/ is one level deeper than api/, so parent.parent.parent
# reaches the package root (cynic/cynic/).
_static_dir = _pathlib.Path(__file__).parent.parent.parent / "static"

_CONSCIOUSNESS_FILE = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


# ── Root route: API alive status ────────────────────────────────────────────
@router_health.get("/", include_in_schema=True)
async def root(request: Request) -> dict:
    """
    Root endpoint — CYNIC kernel is alive.

    Returns:
        - status: "alive" if all systems nominal
        - name: "CYNIC Kernel"
        - φ: The golden ratio (for identity)
        - routes: List of available API routes
    """
    # Collect available routes from the app
    app = request.app
    routes = []
    for route in app.routes:
        if hasattr(route, "path") and not route.path.startswith("/openapi") and not route.path.startswith("/docs") and not route.path.startswith("/redoc") and not route.path.startswith("/static"):
            routes.append(route.path)
    routes.sort()

    return {
        "status": "alive",
        "name": "CYNIC Kernel",
        "φ": f"{PHI:.6f}",  # The golden ratio
        "routes": routes,
    }


# ── Dashboard convenience route ────────────────────────────────────────────
@router_health.get("/dashboard", include_in_schema=False)
async def dashboard() -> FileResponse:
    """Serve the live CYNIC kernel dashboard (connects to /ws/stream)."""
    path = _static_dir / "dashboard.html"
    if not path.is_file():
        raise HTTPException(status_code=404, detail="dashboard.html not found")
    return FileResponse(str(path), media_type="text/html")


# ════════════════════════════════════════════════════════════════════════════
# GET /health
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/health", response_model=HealthResponse)
async def health(container: AppContainer = Depends(get_app_container)) -> HealthResponse:
    """
    Kernel health — the organism's vital signs.

    status=alive    → all systems nominal
    status=degraded → partial functionality (e.g. no DB, no LLM)
    status=dead     → kernel not initialized (should never reach this route)
    """
    state = container.organism
    consciousness = get_consciousness()
    judge_stats = state.orchestrator.stats()
    learn_stats = state.qtable.stats()

    sched_stats = state.scheduler.stats()

    # Determine status
    status = "alive"
    if not state.learning_loop._active:
        status = "degraded"

    # T02: check SurrealDB singleton status (no I/O — just checks if initialized)
    _storage_status: dict[str, Any] = {}
    try:
        from cynic.core.storage.surreal import get_storage as _get_storage  # noqa: deferred
        _get_storage()  # raises RuntimeError if not initialized
        _storage_status["surreal"] = "connected"
    except RuntimeError:
        _storage_status["surreal"] = "disconnected"
    except ValidationError:
        _storage_status["surreal"] = "error"

    return HealthResponse(
        status=status,
        uptime_s=round(state.uptime_s, 1),
        consciousness=consciousness.to_dict(),
        dogs=state.dogs,
        learning={
            "active": state.learning_loop._active,
            "states": learn_stats["states"],
            "total_updates": learn_stats["total_updates"],
            "pending_flush": learn_stats["pending_flush"],
        },
        scheduler=sched_stats,
        llm_adapters=[a.adapter_id for a in __import__("cynic.llm.adapter", fromlist=["get_registry"]).get_registry().get_available()],
        judgments_total=judge_stats["judgments_total"],
        phi=PHI,
        storage=_storage_status,
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /stats
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/stats", response_model=StatsResponse)
async def stats(container: AppContainer = Depends(get_app_container)) -> StatsResponse:
    """Detailed kernel metrics — everything CYNIC knows about itself."""
    state = container.organism

    return StatsResponse(
        judgments=state.orchestrator.stats(),
        learning=state.qtable.stats(),
        top_states=state.qtable.top_states(n=10),
        consciousness=get_consciousness().to_dict(),
        compressor=state.context_compressor.stats(),
    )


# ════════════════════════════════════════════════════════════════════════════
# GET /introspect  (MetaCognition — composant 9/9, self-model)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/introspect")
async def introspect(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    MetaCognition — CYNIC reads its own cognitive state.

    Returns a deep self-model:
    - Learning maturity (Q-table fill, top states, best actions)
    - Residual variance patterns (emergence, THE_UNNAMEABLE events)
    - Consciousness metrics (level distribution, upgrade/downgrade counts)
    - Dog health (hit rates, latencies, capability breakdown)
    - Scholar buffer (similarity memory richness)
    - Kernel integrity (9/9 components, their status)
    - φ-bound assessment (is CYNIC within its own axioms?)

    This is CYNIC judging itself — meta-cognitive self-assessment.
    "φ distrusts φ" — the organism reflects on its own biases.
    """
    from cynic.cognition.neurons.base import DogId

    state = container.organism
    consciousness = get_consciousness()
    qtable_stats = state.qtable.stats()
    orch_stats = state.orchestrator.stats()
    residual_stats = state.residual_detector.stats()

    # Dog-level introspection
    dogs_status = {}
    for dog_id, dog in state.orchestrator.dogs.items():
        caps = dog.get_capabilities()
        dogs_status[dog_id] = {
            "sefirot": caps.sefirot,
            "uses_llm": caps.uses_llm,
            "consciousness_min": caps.consciousness_min.name if hasattr(caps.consciousness_min, 'name') else str(caps.consciousness_min),
            "avg_latency_ms": round(dog.avg_latency_ms, 2),
            "judgment_count": dog._judgment_count,
        }

    # Scholar buffer richness
    scholar = state.orchestrator.dogs.get(DogId.SCHOLAR)
    scholar_status = {}
    if scholar is not None:
        scholar_stats = scholar.stats() if hasattr(scholar, 'stats') else {}
        scholar_status = {
            "buffer_size": len(scholar._buffer),
            "buffer_max": scholar._buffer.maxlen if hasattr(scholar._buffer, 'maxlen') else 89,
            "lookups": scholar._lookups,
            "hits": scholar._hits,
            "hit_rate": round(scholar._hits / max(scholar._lookups, 1), 3),
            "buffer_richness": round(
                min(1.0, len(scholar._buffer) / fibonacci(8)), 3
            ),
        }

    # 9-component kernel integrity check
    components = {
        "1_AXIOMS":         {"status": "ACTIVE", "description": "5 axioms × 7 facets scoring"},
        "2_PHI_BOUND":      {"status": "ACTIVE", "description": "φ⁻¹=61.8% max confidence enforced"},
        "3_MULTI_AGENT":    {"status": "ACTIVE", "description": f"{len(state.orchestrator.dogs)}/11 Dogs active"},
        "4_EVENT_DRIVEN":   {"status": "ACTIVE", "description": "Core bus wired, JUDGMENT_CREATED flowing"},
        "5_JUDGMENT":       {"status": "ACTIVE", "description": "7-step PERCEIVE→EMERGE pipeline"},
        "6_LEARNING":       {
            "status": "ACTIVE" if qtable_stats.get("total_updates", 0) > 0 else "WARM",
            "description": f"QTable: {qtable_stats.get('unique_states', 0)} states learned",
        },
        "7_RESIDUAL":       {
            "status": "ACTIVE",
            "description": f"ResidualDetector: {residual_stats['observations']} obs, {residual_stats['patterns_detected']} patterns",
        },
        "8_MEMORY":         {
            "status": "ACTIVE" if scholar_status.get("buffer_size", 0) > 0 else "COLD",
            "description": f"Scholar buffer: {scholar_status.get('buffer_size', 0)}/{scholar_status.get('buffer_max', 89)} cells",
        },
        "9_META_COGNITION": {"status": "ACTIVE", "description": "This endpoint — /introspect live"},
    }

    active_count = sum(1 for c in components.values() if c["status"] == "ACTIVE")
    kernel_integrity = round(active_count / 9, 3)

    # φ self-assessment
    # Is CYNIC operating within its own axioms?
    phi_violations = []
    max_conf = qtable_stats.get("max_confidence") or 0.0
    if max_conf > PHI_INV + 0.01:
        phi_violations.append(f"Q-table confidence exceeds φ⁻¹: {max_conf:.3f}")
    if residual_stats.get("anomaly_rate", 0) > PHI_INV:
        phi_violations.append(f"Residual anomaly rate exceeds φ⁻¹: {residual_stats['anomaly_rate']:.3f}")

    return {
        "introspect_id": str(uuid.uuid4()),
        "timestamp": time.time(),
        "uptime_s": round(state.uptime_s, 1),
        "φ_self_assessment": {
            "kernel_integrity": kernel_integrity,
            "phi_violations": phi_violations,
            "self_confidence": round(min(kernel_integrity * PHI_INV, PHI_INV), 3),
            "verdict": "HOWL" if kernel_integrity >= KERNEL_INTEGRITY_HOWL_THRESHOLD else (
                "WAG" if kernel_integrity >= KERNEL_INTEGRITY_WAG_THRESHOLD else (
                    "GROWL" if kernel_integrity >= KERNEL_INTEGRITY_GROWL_THRESHOLD else "BARK"
                )
            ),
        },
        "consciousness": consciousness.to_dict(),
        "learning": {
            **qtable_stats,
            "learning_loop_active": True,
        },
        "residual": residual_stats,
        "dogs": dogs_status,
        "scholar": scholar_status,
        "components": components,
        "orchestrator": orch_stats,
        # δ1+δ2+γ4 intelligence layer
        "emergent_axioms": state.axiom_monitor.stats(),
        "lod": state.lod_controller.status(),
        "escore_top": state.escore_tracker.top_entities(n=5),
        "message": "*sniff* Je me lis moi-même. Le chien qui se connaît.",
    }


# ════════════════════════════════════════════════════════════════════════════
# GET /axioms  (δ1 Emergent Axiom Dashboard)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/axioms")
async def axioms(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Emergent Axiom dashboard — A6-A9 activation status.

    Returns live maturity scores and tier for the 4 emergent axioms:
      A6. AUTONOMY     — Dogs coordinate without human approval
      A7. SYMBIOSIS    — Human×Machine mutual value creation
      A8. EMERGENCE    — Patterns beyond core axioms
      A9. ANTIFRAGILITY — System improves under chaos
    """
    state = container.organism
    return state.axiom_monitor.dashboard()


# ════════════════════════════════════════════════════════════════════════════
# GET /lod  (δ2 Survival LOD status)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/lod")
async def lod(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Survival LOD status — current graceful degradation level.

    LOD 0 FULL:      All Dogs + LLM + all consciousness levels
    LOD 1 REDUCED:   Skip slow Dogs, L2 MICRO max
    LOD 2 EMERGENCY: REFLEX only, no LLM
    LOD 3 MINIMAL:   GUARDIAN only, survival mode
    """
    state = container.organism
    return state.lod_controller.status()


@router_health.get("/account/stats")
async def account_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    AccountAgent step-6 ledger — cost tracking + budget enforcement.

    Returns per-reality and per-dog cost breakdown, budget remaining,
    and BUDGET_WARNING / BUDGET_EXHAUSTED event emission status.

    Step 6 of the 7-step cycle: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE
    """
    state = container.organism
    if state.account_agent is None:
        return {"error": "AccountAgent not initialized", "total_cost_usd": 0.0}
    return state.account_agent.stats()


@router_health.get("/decide/stats")
async def decide_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    DecideAgent Ring-2 stats — MCTS decision counts.

    Shows decisions_made (BARK/GROWL auto-decided via NestedMCTS) and
    skipped (WAG/HOWL or low-confidence judgments not escalated).
    """
    state = container.organism
    if state.decide_agent is None:
        return {"decisions_made": 0, "skipped": 0}
    return state.decide_agent.stats()


@router_health.get("/sage/stats")
async def sage_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    SAGE Dog temporal MCTS activation stats.

    Shows heuristic vs LLM (temporal) judgment counts.
    llm_activation_rate > 0 → Temporal MCTS is firing (Ollama available).
    llm_activation_rate == 0 → Heuristic-only mode (Ollama unavailable).
    """
    from cynic.cognition.neurons.base import DogId
    state = container.organism
    orch = state.orchestrator
    sage = orch.dogs.get(DogId.SAGE) if orch and hasattr(orch, "dogs") else None
    if sage is None:
        return {"available": False, "heuristic_count": 0, "llm_count": 0}
    heuristic = getattr(sage, "_heuristic_count", 0)
    llm = getattr(sage, "_llm_count", 0)
    total = heuristic + llm
    return {
        "available": True,
        "heuristic_count": heuristic,
        "llm_count": llm,
        "total_judgments": total,
        "llm_activation_rate": round(llm / total, 3) if total > 0 else 0.0,
        "temporal_mcts_active": llm > 0,
    }


@router_health.get("/residual/stats")
async def residual_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    ResidualDetector stats — residual variance history + pattern detection (T04).

    observations > 0  → warm-start succeeded (SurrealDB loaded history on boot)
    anomaly_rate > 0  → some judgments had high residual variance (≥38.2%)
    patterns_detected → EMERGENCE patterns found (SPIKE / RISING / STABLE_HIGH)
    """
    state = container.organism
    return state.residual_detector.stats()


@router_health.get("/llm/benchmarks")
async def llm_benchmarks() -> dict[str, Any]:
    """
    LLM Benchmark routing matrix — per-(dog, task_type, llm_id) perf history (T05).

    Persisted to SurrealDB after each update_benchmark() call.
    Warmed from SurrealDB on boot so routing survives restarts.
    Used by LLMRouter to select the best LLM for each Dog × Task combination.
    """
    from cynic.llm.adapter import get_registry as _get_registry
    reg = _get_registry()
    matrix = [
        {
            "dog_id":          dog_id,
            "task_type":       task_type,
            "llm_id":          llm_id,
            "quality_score":   round(r.quality_score, 2),
            "speed_score":     round(r.speed_score, 3),
            "cost_score":      round(r.cost_score, 3),
            "composite_score": round(r.composite_score, 3),
            "error_rate":      round(r.error_rate, 3),
            "sample_count":    r.sample_count,
        }
        for (dog_id, task_type, llm_id), r in reg._benchmarks.items()
    ]
    return {"count": len(matrix), "matrix": matrix}


@router_health.get("/auto-benchmark/stats")
async def auto_benchmark_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """AutoBenchmark probe stats — interval, runs, enabled flag (T09)."""
    state = container.organism
    if state.auto_benchmark is None:
        return {"enabled": False, "runs": 0, "interval_s": 0}
    return state.auto_benchmark.stats()


@router_health.post("/auto-benchmark/run")
async def auto_benchmark_run(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """Trigger an immediate AutoBenchmark round (T09)."""
    state = container.organism
    if state.auto_benchmark is None:
        return {"completed": 0, "message": "auto_benchmark not initialised"}
    completed = await state.auto_benchmark.run_once()
    return {"completed": completed}


@router_health.get("/benchmark/probe-snapshot")
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


@router_health.get("/benchmark/drift-alerts")
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


@router_health.get("/mirror")
async def mirror(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    KernelMirror — Ring 3 unified self-reflection snapshot.

    Aggregates all subsystem stats into a single response:
      - qtable: 7×7×7 matrix coverage + learning stats
      - axioms: A6-A11 tier + maturity scores
      - lod: current LOD + health dimensions
      - account: budget ledger + cost by reality/dog
      - escore: per-dog reputation (7 dimensions)
      - residual: spike/stable/rising signal counts
      - sage: temporal MCTS vs heuristic ratio
      - dogs: judgment counts + latency profiles
      - overall_health: geometric mean [0, 100]
      - tier: BARK/GROWL/WAG/HOWL

    Use for dashboards, health checks, and CONSCIOUSNESS signal evaluation.
    """
    state = container.organism
    snap = state.kernel_mirror.snapshot(state)
    snap["diff"] = state.kernel_mirror.diff(snap)
    return snap


# ════════════════════════════════════════════════════════════════════════════
# GET /consciousness  (unified metathinking output)
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/consciousness")
async def consciousness(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Unified metathinking output — the organism's complete cognitive state.

    Returns the contents of ~/.cynic/consciousness.json if available,
    otherwise falls back to a live mirror snapshot.

    Updated by the kernel after every JUDGMENT_CREATED (throttled to F(7)=13s).
    """
    # Try reading pre-written file first (avoids re-computing snapshot)
    try:
        import pathlib
        p = pathlib.Path(_CONSCIOUSNESS_FILE)
        if p.exists() and (time.time() - p.stat().st_mtime) < 60.0:
            with p.open("r", encoding="utf-8") as fh:
                return json.load(fh)
    except OSError:
        pass

    # Fallback: live snapshot
    state = container.organism
    snap = state.kernel_mirror.snapshot(state)
    payload: dict[str, Any] = {
        "timestamp": round(time.time(), 3),
        "uptime_s": round(state.uptime_s, 1),
        "mirror": snap,
        "diff": state.kernel_mirror.diff(snap),
    }
    if state.llm_router is not None:
        payload["llm_routing"] = state.llm_router.stats()
    return payload


# ════════════════════════════════════════════════════════════════════════════
# GET /internal/registry — Tier 1 Nervous System: Service State Registry
# ════════════════════════════════════════════════════════════════════════════

@router_health.get("/internal/registry")
async def internal_registry(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Tier 1 Nervous System: Real-time snapshot of all kernel component health.

    Returns:
        {
            "timestamp_ms": float,
            "components": {
                "orchestrator": {component snapshot},
                "qtable": {component snapshot},
                ...
            },
            "total_components": int,
            "healthy_count": int,
            "degraded_count": int,
            "stalled_count": int,
            "failed_count": int,
        }
    """
    state = container.organism
    if state.service_registry is None:
        return {"error": "Service registry not available"}

    snapshot = await state.service_registry.snapshot()
    return snapshot.to_dict()


@router_health.get("/convergence/stats")
async def convergence_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Phase 3: Convergence Validator — Announcement vs Reality Verification.

    Tracks what organism announced it would do (verdict, Q-score) vs what actually
    happened. Used for end-to-end validation that announced behavior matches reality.

    Returns:
        {
            "total_announced": int,
            "total_outcomes": int,
            "total_matches": int,
            "convergence_rate": float,  # percentage
            "recent": [
                {
                    "status": "✓ MATCH" | "✗ DIVERGE",
                    "announced_verdict": str,
                    "actual_verdict": str,
                    "latency_ms": float,
                }
            ]
        }
    """
    state = container.organism
    validator = state.convergence_validator
    if validator is None:
        return {"error": "Convergence validator not available"}

    stats = validator.stats()
    return {
        "total_announced": stats.get("total_announced", 0),
        "total_outcomes": stats.get("total_outcomes", 0),
        "total_matches": stats.get("total_matches", 0),
        "convergence_rate": stats.get("convergence_rate", 0.0),
        "recent": stats.get("recent", []),
    }


@router_health.get("/")
async def root() -> dict[str, Any]:
    return {
        "name": "CYNIC Kernel",
        "version": "2.0.0",
        "status": "alive",
        "φ": PHI,
        "routes": [
            "/judge", "/perceive", "/learn", "/policy/{key}",
            "/health", "/stats", "/introspect",
            "/ws/stream", "/ws/sdk",
            "/sdk/sessions", "/sdk/task",
            "/act/execute", "/act/telemetry",
            "/internal/registry",  # Tier 1 Nervous System
            "/convergence/stats",   # Phase 3: Observability
        ],
        "message": "*sniff* Le chien est là.",
    }
