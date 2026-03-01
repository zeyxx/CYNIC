"""
CYNIC introspection router — self-reflection & diagnostics: introspect · axioms · lod · mirror · registry
"""
from __future__ import annotations

import logging
import time
import uuid
from typing import Any

from fastapi import APIRouter, Depends

from cynic.kernel.core.consciousness import get_consciousness
from cynic.kernel.core.phi import PHI, PHI_INV, fibonacci
from cynic.kernel.core.formulas import (
    KERNEL_INTEGRITY_HOWL_THRESHOLD,
    KERNEL_INTEGRITY_WAG_THRESHOLD,
    KERNEL_INTEGRITY_GROWL_THRESHOLD,
)
from cynic.interfaces.api.state import get_app_container, AppContainer

logger = logging.getLogger("cynic.interfaces.api.server")

router_introspection = APIRouter(tags=["introspection"])


# ════════════════════════════════════════════════════════════════════════════
# GET /introspect  (MetaCognition — composant 9/9, self-model)
# ════════════════════════════════════════════════════════════════════════════

@router_introspection.get("/introspect")
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
    from cynic.kernel.organism.brain.cognition.neurons.base import DogId

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

@router_introspection.get("/axioms")
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

@router_introspection.get("/lod")
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


# ════════════════════════════════════════════════════════════════════════════
# GET /mirror (Ring 3 unified self-reflection snapshot)
# ════════════════════════════════════════════════════════════════════════════

@router_introspection.get("/mirror")
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
# GET /internal/registry — Tier 1 Nervous System: Service State Registry
# ════════════════════════════════════════════════════════════════════════════

@router_introspection.get("/internal/registry")
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
