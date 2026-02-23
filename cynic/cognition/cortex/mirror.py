"""
CYNIC KernelMirror — Ring 3 Self-Reflection Surface

A unified snapshot of ALL kernel subsystems, enabling CYNIC to observe
its own state in a single structured view. Designed to power:

  1. /mirror API endpoint — full kernel introspection in one call
  2. CONSCIOUSNESS signal source — high-quality self-model triggers A10
  3. Temporal diff — detect what changed between two snapshots

Subsystems aggregated:
  - QTable: matrix coverage (7×7×7), learning stats, top states
  - AxiomMonitor: A6-A11 tier, maturity scores, activation counts
  - LODController: current LOD, health dimensions
  - AccountAgent: budget ledger, cost by reality/dog
  - EScoreTracker: per-dog reputation scores (7 dimensions)
  - ResidualDetector: spike counts, STABLE_HIGH / RISING signals
  - SAGE: temporal MCTS vs heuristic ratio (LLM activation rate)
  - Dogs: judgment counts, hit rates, latency profiles

φ-integration:
  - overall_health [0, 100] — geometric mean of subsystem health indicators
  - If overall_health ≥ WAG_MIN (61.8) → signals A10 CONSCIOUSNESS
"""
from __future__ import annotations

import time
import logging
from math import exp, log
from typing import Any, Dict


from cynic.core.phi import WAG_MIN, GROWL_MIN, MAX_Q_SCORE
from cynic.core.formulas import AXIOM_MATURITY_WINDOW_SIZE

logger = logging.getLogger("cynic.cognition.cortex.mirror")

# Rolling window for diff history (F(8)=21 snapshots) — imported from formulas.py
_DIFF_WINDOW: int = AXIOM_MATURITY_WINDOW_SIZE  # 21


class KernelMirror:
    """
    Unified self-reflection surface for the CYNIC kernel.

    Aggregates stats from all subsystems into a single snapshot dict.
    Provides diff() to surface changes between consecutive snapshots.
    Computes overall_health score as geometric mean of key indicators.

    Usage:
        mirror = KernelMirror(state)
        snap = mirror.snapshot()
        health = snap["overall_health"]   # [0, 100] — how well is CYNIC doing?
        tier = snap["tier"]               # DORMANT/STIRRING/ACTIVE/TRANSCENDENT
    """

    def __init__(self) -> None:
        self._prev_snapshot: Optional[dict[str, Any]] = None
        self._penultimate_snapshot: Optional[dict[str, Any]] = None  # N-2 snapshot for diff
        self._snapshot_count: int = 0
        self._created_at: float = time.time()

    def snapshot(self, state: Any) -> dict[str, Any]:
        """
        Build a full kernel snapshot from the current KernelState.

        Args:
            state: KernelState object from cynic.api.state

        Returns:
            Dict with all subsystem stats + overall_health + tier.
        """
        snap: dict[str, Any] = {
            "snapshot_id": self._snapshot_count,
            "timestamp": round(time.time(), 3),
            "uptime_s": round(time.time() - self._created_at, 1),
        }

        # QTable: 7×7×7 materialization coverage
        if hasattr(state, "qtable") and state.qtable is not None:
            try:
                snap["qtable"] = {
                    **state.qtable.stats(),
                    **state.qtable.matrix_stats(),
                    "top_states": state.qtable.top_states(n=3),
                }
            except CynicError as exc:
                snap["qtable"] = {"error": str(exc)}

        # AxiomMonitor: A6-A11 emergence tier
        if hasattr(state, "axiom_monitor") and state.axiom_monitor is not None:
            try:
                snap["axioms"] = state.axiom_monitor.dashboard()
            except CynicError as exc:
                snap["axioms"] = {"error": str(exc)}

        # LODController: graceful degradation level
        if hasattr(state, "lod_controller") and state.lod_controller is not None:
            try:
                snap["lod"] = state.lod_controller.status()
            except CynicError as exc:
                snap["lod"] = {"error": str(exc)}

        # AccountAgent: cost ledger
        if hasattr(state, "account_agent") and state.account_agent is not None:
            try:
                snap["account"] = state.account_agent.stats()
            except CynicError as exc:
                snap["account"] = {"error": str(exc)}

        # EScoreTracker: per-dog reputation
        if hasattr(state, "escore_tracker") and state.escore_tracker is not None:
            try:
                snap["escore"] = {
                    **state.escore_tracker.stats(),
                    "top": [
                        {"entity": eid, "score": round(sc, 3)}
                        for eid, sc in state.escore_tracker.top_entities(5)
                    ],
                }
            except CynicError as exc:
                snap["escore"] = {"error": str(exc)}

        # ResidualDetector: emergence signals
        if hasattr(state, "residual_detector") and state.residual_detector is not None:
            try:
                snap["residual"] = state.residual_detector.stats()
            except CynicError as exc:
                snap["residual"] = {"error": str(exc)}

        # SAGE: temporal MCTS activation rate
        snap["sage"] = self._sage_stats(state)

        # Dogs: aggregate judgment telemetry
        snap["dogs"] = self._dog_stats(state)

        # Handler Topology: Tier 3 architecture self-awareness
        if hasattr(state, "_handler_registry") and state._handler_registry is not None:
            try:
                snap["handler_topology"] = state._handler_registry.introspect()
            except EventBusError as exc:
                snap["handler_topology"] = {"error": str(exc)}

        # Decisions: DecideAgent Ring-2 stats
        if hasattr(state, "decide_agent") and state.decide_agent is not None:
            try:
                snap["decide"] = state.decide_agent.stats()
            except EventBusError as exc:
                snap["decide"] = {"error": str(exc)}

        # Compute overall_health and tier
        snap["overall_health"] = self._compute_health(snap)
        snap["tier"] = self._health_tier(snap["overall_health"])

        # Keep penultimate for diff comparison (N-2 → allows diff after N snapshot)
        self._penultimate_snapshot = self._prev_snapshot
        self._prev_snapshot = snap
        self._snapshot_count += 1
        return snap

    def diff(self, current: dict[str, Any]) -> dict[str, Any]:
        """
        Compute a delta between the penultimate snapshot and current.

        After N calls to snapshot(), diff(current) compares the N-1 snapshot
        (stored as penultimate) against `current` (N). This lets callers call:
            snap1 = mirror.snapshot(state1)
            snap2 = mirror.snapshot(state2)
            d = mirror.diff(snap2)   # shows what changed between snap1 and snap2

        Returns:
            Dict of changed paths → {"old": ..., "new": ...}
            Empty dict if fewer than 2 snapshots have been taken.
        """
        if self._penultimate_snapshot is None:
            return {}
        return _deep_diff(self._penultimate_snapshot, current)

    # ── Subsystem extractors ───────────────────────────────────────────────

    @staticmethod
    def _sage_stats(state: Any) -> dict[str, Any]:
        """Extract SAGE temporal MCTS activation ratio."""
        from cynic.cognition.neurons.base import DogId
        # Use canonical path: state.orchestrator.dogs[SAGE]
        orch = getattr(state, "orchestrator", None)
        if orch is None:
            return {"available": False}
        dogs = getattr(orch, "dogs", None)
        if not isinstance(dogs, dict):
            return {"available": False}

        sage = dogs.get(DogId.SAGE)
        if sage is None:
            return {"available": False}

        heuristic = int(getattr(sage, "_heuristic_count", 0) or 0)
        llm = int(getattr(sage, "_llm_count", 0) or 0)
        total = heuristic + llm
        llm_rate = (llm / total) if total > 0 else 0.0
        return {
            "available": True,
            "heuristic_count": heuristic,
            "llm_count": llm,
            "total_judgments": total,
            "llm_activation_rate": round(llm_rate, 3),
            "temporal_mcts_active": llm > 0,
        }

    @staticmethod
    def _dog_stats(state: Any) -> dict[str, Any]:
        """Aggregate judgment telemetry across all Dogs."""
        orch = getattr(state, "orchestrator", None)
        if orch is None:
            return {}
        dogs = getattr(orch, "dogs", {})
        result = {}
        for dog_id, dog in dogs.items():
            result[str(dog_id)] = {
                "judgment_count": getattr(dog, "_judgment_count", 0),
                "avg_latency_ms": round(getattr(dog, "avg_latency_ms", 0.0), 2),
                "veto_count": getattr(dog, "_veto_count", 0),
            }
        return result

    # ── Health scoring ─────────────────────────────────────────────────────

    @staticmethod
    def _compute_health(snap: dict[str, Any]) -> float:
        """
        Compute overall_health [0, 100] as geometric mean of key indicators.

        Indicators:
          1. QTable coverage_pct (normalized to [0, 100])
          2. Axiom tier score: DORMANT=0, STIRRING=25, EMERGENCE=50,
             AWAKENING=75, TRANSCENDENT=100
          3. LOD health: LOD_0=100, LOD_1=75, LOD_2=38, LOD_3=10
          4. SAGE LLM activation (0=heuristic-only, 100=always temporal)

        Geometric mean keeps score honest — one bad indicator drags the whole.
        """
        scores: list[float] = []

        # 1. QTable coverage
        qt = snap.get("qtable", {})
        if isinstance(qt, dict) and "coverage_pct" in qt:
            scores.append(max(qt["coverage_pct"], 0.1))

        # 2. Axiom tier
        axioms = snap.get("axioms", {})
        tier_scores = {
            "DORMANT": 0.1, "EMERGENCE": 50.0, "STIRRING": 25.0,
            "AWAKENING": 75.0, "TRANSCENDENT": 100.0,
        }
        if isinstance(axioms, dict) and "tier" in axioms:
            scores.append(tier_scores.get(axioms["tier"], 25.0))

        # 3. LOD health
        lod = snap.get("lod", {})
        lod_scores = {0: 100.0, 1: 75.0, 2: 38.2, 3: 10.0}
        if isinstance(lod, dict) and "current_lod" in lod:
            scores.append(lod_scores.get(lod["current_lod"], 50.0))

        # 4. SAGE LLM rate (temporal path active = wisdom mode)
        sage = snap.get("sage", {})
        if isinstance(sage, dict) and sage.get("available"):
            llm_rate = sage.get("llm_activation_rate", 0.0)
            # Map: 0→25 (heuristic-only still useful), 1→100 (full temporal)
            scores.append(25.0 + 75.0 * llm_rate)

        if not scores:
            return 50.0  # Unknown — neutral

        # Geometric mean
        log_sum = sum(log(max(s, 0.1)) for s in scores)
        geo_mean = exp(log_sum / len(scores))
        return round(min(geo_mean, MAX_Q_SCORE), 1)

    @staticmethod
    def _health_tier(health: float) -> str:
        """Map overall_health score to a descriptive tier."""
        if health >= 82.0:
            return "HOWL"      # All systems optimal
        if health >= WAG_MIN:
            return "WAG"       # Good health, some gaps
        if health >= GROWL_MIN:
            return "GROWL"     # Degraded — attention needed
        return "BARK"          # Critical — intervention required


def _deep_diff(
    old: dict[str, Any],
    new: dict[str, Any],
    prefix: str = "",
) -> dict[str, Any]:
    """
    Recursively compute leaf-level diff between two nested dicts.

    Returns {path: {"old": v, "new": v}} for changed scalar values only.
    Ignores "snapshot_id" and "timestamp" (always different).
    """
    changes: dict[str, Any] = {}
    _SKIP_KEYS = {"snapshot_id", "timestamp"}

    all_keys = set(old.keys()) | set(new.keys())
    for key in all_keys:
        if key in _SKIP_KEYS:
            continue
        path = f"{prefix}.{key}" if prefix else key
        old_val = old.get(key)
        new_val = new.get(key)

        if isinstance(old_val, dict) and isinstance(new_val, dict):
            sub_diff = _deep_diff(old_val, new_val, path)
            changes.update(sub_diff)
        elif old_val != new_val and old_val is not None and new_val is not None:
            changes[path] = {"old": old_val, "new": new_val}

    return changes
