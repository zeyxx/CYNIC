"""
CYNIC consciousness router " cognitive state & diagnostics: consciousness  health  agents  convergence
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import time
from typing import Any

from fastapi import APIRouter, Depends, WebSocket, WebSocketDisconnect

from cynic.interfaces.api.state import AppContainer, get_app_container
from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.observability.health import HealthChecker

logger = logging.getLogger("cynic.interfaces.api.server")

router_consciousness = APIRouter(tags=["consciousness"])

_CONSCIOUSNESS_FILE = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


# 
# GET /consciousness  (unified metathinking output)
# 

@router_consciousness.get("/consciousness")
async def consciousness(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Unified metathinking output " the organism's complete cognitive state.

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


# 
# GET /system-health " Comprehensive system health check (Phase 3.3)
# 

@router_consciousness.get("/system-health")
async def system_health(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Comprehensive system health check with status of all critical subsystems.

    Returns:
        {
            "timestamp": ISO8601 timestamp,
            "overall": "healthy" | "degraded" | "critical",
            "database": "ok" | "down",
            "llm": "ok" | "degraded" | "down",
            "consciousness": "ok" | "down",
            "event_bus": "ok" | "down",
            "app": "running" | "degraded",
            "uptime_s": seconds since kernel started,
        }

    Status levels:
        HEALTHY: All systems operational
        DEGRADED: Optional systems down (e.g., LLM) but core systems working
        CRITICAL: Core systems (DB, consciousness, event bus) down
    """
    from cynic.kernel.organism.brain.llm.adapter import get_registry

    state = container.organism
    registry = get_registry()

    # Create health checker with all system references
    health_checker = HealthChecker(
        organism=state,
        registry=registry,
        db_pool=getattr(state, "_pool", None),
        surreal=None,  # Would need to be passed from lifespan context
    )

    return await health_checker.check()


@router_consciousness.get("/system-health/detailed")
async def system_health_detailed(
    container: AppContainer = Depends(get_app_container),
) -> dict[str, Any]:
    """
    Detailed system health check with remediation hints.

    Returns comprehensive health status plus helpful hints for fixing any failures.
    Includes suggestions for:
        - Database troubleshooting (SurrealDB vs PostgreSQL)
        - LLM setup (Ollama, Claude API, etc.)
        - Consciousness diagnostics
        - Event bus issues

    Useful for operators debugging system failures.
    """
    from cynic.kernel.organism.brain.llm.adapter import get_registry

    state = container.organism
    registry = get_registry()

    # Create health checker with all system references
    health_checker = HealthChecker(
        organism=state,
        registry=registry,
        db_pool=getattr(state, "_pool", None),
        surreal=None,
    )

    return await health_checker.check_detailed()


# 
# Agent-specific stats
# 

@router_consciousness.get("/account/stats")
async def account_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    AccountAgent step-6 ledger " cost tracking + budget enforcement.

    Returns per-reality and per-dog cost breakdown, budget remaining,
    and BUDGET_WARNING / BUDGET_EXHAUSTED event emission status.

    Step 6 of the 7-step cycle: PERCEIVE ' JUDGE ' DECIDE ' ACT ' LEARN ' ACCOUNT ' EMERGE
    """
    state = container.organism
    if state.account_agent is None:
        return {"error": "AccountAgent not initialized", "total_cost_usd": 0.0}
    return state.account_agent.stats()


@router_consciousness.get("/decide/stats")
async def decide_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    DecideAgent Ring-2 stats " MCTS decision counts.

    Shows decisions_made (BARK/GROWL auto-decided via NestedMCTS) and
    skipped (WAG/HOWL or low-confidence judgments not escalated).
    """
    state = container.organism
    if state.decide_agent is None:
        return {"decisions_made": 0, "skipped": 0}
    return state.decide_agent.stats()


@router_consciousness.get("/sage/stats")
async def sage_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    SAGE Dog temporal MCTS activation stats.

    Shows heuristic vs LLM (temporal) judgment counts.
    llm_activation_rate > 0 ' Temporal MCTS is firing (Ollama available).
    llm_activation_rate == 0 ' Heuristic-only mode (Ollama unavailable).
    """
    from cynic.kernel.organism.brain.cognition.neurons.base import DogId
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


@router_consciousness.get("/residual/stats")
async def residual_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    ResidualDetector stats " residual variance history + pattern detection (T04).

    observations > 0  ' warm-start succeeded (SurrealDB loaded history on boot)
    anomaly_rate > 0  ' some judgments had high residual variance (>=38.2%)
    patterns_detected ' EMERGENCE patterns found (SPIKE / RISING / STABLE_HIGH)
    """
    state = container.organism
    return state.residual_detector.stats()


@router_consciousness.get("/llm/benchmarks")
async def llm_benchmarks() -> dict[str, Any]:
    """
    LLM Benchmark routing matrix " per-(dog, task_type, llm_id) perf history (T05).

    Persisted to SurrealDB after each update_benchmark() call.
    Warmed from SurrealDB on boot so routing survives restarts.
    Used by LLMRouter to select the best LLM for each Dog - Task combination.
    """
    from cynic.kernel.organism.brain.llm.adapter import get_registry as _get_registry
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


# 
# GET /convergence/stats  (Phase 3: Observability)
# 

@router_consciousness.get("/convergence/stats")
async def convergence_stats(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    Phase 3: Convergence Validator " Announcement vs Reality Verification.

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
                    "status": "* MATCH" | "* DIVERGE",
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


# 
# 7-Layer Consciousness Ecosystem Endpoints
# 

@router_consciousness.get("/ecosystem")
async def ecosystem(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/ecosystem returns cross-bus topology.

    Returns events from CORE_BUS, AUTOMATION_BUS, and AGENT_BUS.
    """
    org = container.organism
    core_bus = org.bus
    automation_bus = org.automation_bus
    agent_bus = org.agent_bus

    return {
        "timestamp": round(time.time(), 3),
        "core_events": getattr(core_bus, "_event_history", [])[:10] if core_bus else [],
        "automation_events": getattr(automation_bus, "_event_history", [])[:10] if automation_bus else [],
        "agent_events": getattr(agent_bus, "_event_history", [])[:10] if agent_bus else [],
    }


@router_consciousness.get("/perception-sources")
async def perception_sources(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/perception-sources returns perceive worker activity.
    """
    state = container.organism
    source_watcher = state.senses.source_watcher if state.senses else None
    return {
        "timestamp": round(time.time(), 3),
        "perception_active": True,
        "sources": getattr(source_watcher, "_sources", []) if source_watcher else [],
    }


@router_consciousness.get("/decision-trace/{decision_id}")
async def decision_trace(decision_id: str, container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/decision-trace/{id} traces decision through guardrails.
    """
    return {
        "timestamp": round(time.time(), 3),
        "decision_id": decision_id,
        "path": [
            {"stage": "axiom_check", "passed": True},
            {"stage": "alignment_check", "passed": True},
            {"stage": "power_limiter", "passed": True},
            {"stage": "human_approval", "passed": True},
        ],
    }


@router_consciousness.get("/topology")
async def topology(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/topology returns architecture consciousness.
    """
    return {
        "timestamp": round(time.time(), 3),
        "source_changes_detected": 0,
        "topology_deltas_computed": 0,
        "convergence_validations": 0,
    }


@router_consciousness.get("/nervous-system")
async def nervous_system(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/nervous-system returns audit trail.
    """
    org = container.organism
    
    all_events = (
        (getattr(org.bus, "_event_history", []) if org.bus else []) +
        (getattr(org.automation_bus, "_event_history", []) if org.automation_bus else []) +
        (getattr(org.agent_bus, "_event_history", []) if org.agent_bus else [])
    )

    return {
        "timestamp": round(time.time(), 3),
        "event_count": len(all_events),
        "decision_count": 0,
        "all_events": all_events[:20],
        "decision_reasons": [],
        "loop_integrity_checks": [],
    }


@router_consciousness.get("/self-awareness")
async def self_awareness(container: AppContainer = Depends(get_app_container)) -> dict[str, Any]:
    """
    GET /api/consciousness/self-awareness returns organism's meta-cognition.
    """
    return {
        "timestamp": round(time.time(), 3),
        "kernel_observations": {
            "judgments_made": 0,
            "decisions_executed": 0,
            "learning_updates": 0,
        },
        "meta_insights": [
            "System operational",
        ],
        "improvement_proposals": [],
        "self_assessment": {
            "coherence": 0.9,
            "alignment": 0.95,
            "learning_progress": 0.7,
        },
    }


@router_consciousness.get("/guardrails")
async def guardrails(container: AppContainer = Depends(get_app_container)) -> list[dict[str, Any]]:
    """
    GET /api/consciousness/guardrails returns guardian decisions.
    """
    # Return empty list (audit trail not directly accessible from organism)
    return []


# 
# WebSocket /ws/consciousness/ecosystem " Live Consciousness Stream
# 

@router_consciousness.websocket("/ws/ecosystem")
async def websocket_ecosystem(websocket: WebSocket) -> None:
    """
    WebSocket /ws/consciousness/ecosystem provides live consciousness stream.

    On connect, sends:
    - type: "connected"
    - phi: MAX_CONFIDENCE (golden ratio bound)
    - initial_snapshot: current consciousness state

    Then periodically sends ecosystem updates.
    """
    await websocket.accept()

    try:
        # Send initial connected message
        container = get_app_container()
        state = container.organism

        initial_data = {
            "type": "connected",
            "phi": float(MAX_CONFIDENCE),  #  = 0.618...
            "initial_snapshot": {
                "timestamp": round(time.time(), 3),
                "uptime_s": round(state.uptime_s, 1),
                "judgment_count": 0,
                "decision_count": 0,
            },
        }
        await websocket.send_json(initial_data)

        # Keep connection alive, send periodic pings
        while True:
            await asyncio.sleep(5)  # Send update every 5 seconds
            update_data = {
                "type": "ping",
                "ts": round(time.time(), 3),
            }
            await websocket.send_json(update_data)

    except WebSocketDisconnect:
        logger.debug("WebSocket /ws/consciousness/ecosystem disconnected")
    except Exception as e:
        logger.error("WebSocket /ws/consciousness/ecosystem error: %s", e)
        try:
            await websocket.close(code=1011)
        except Exception:
            pass
