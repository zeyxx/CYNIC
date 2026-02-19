"""
CYNIC LLM Router — Ring 4: Q-Table driven model selection

Routes SDK tasks between expensive (Claude Sonnet) and cheap (Claude Haiku
or local Ollama) based on accumulated Q-Table confidence.

Bootstrap Philosophy:
  Phase 1 (cold start): All tasks → Claude Sonnet (build Q-Table)
  Phase 2 (warming):    Simple/trivial → Claude Haiku (save cost)
  Phase 3 (hot):        80%+ tasks → Haiku/Ollama (maximum savings)

Routing criteria (φ-derived):
  - confidence ≥ PHI_INV (0.618): Q-Table has seen enough data
  - task_type in SIMPLE_TYPES: debug, refactor, test, explain, write
  - complexity in {trivial, simple}: few tools, predictable
  → route to HAIKU (cheaper, faster, sufficient for known patterns)

Safety: NEVER route complex/review/unknown tasks automatically.
Always returns a RoutingDecision — callers decide whether to act on it.
"""
from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Any


from cynic.core.phi import PHI_INV, PHI_INV_2

logger = logging.getLogger("cynic.act.llm_router")

# Task types safe for local/cheap routing (CYNIC has seen enough examples)
_SIMPLE_TASK_TYPES = frozenset({"debug", "refactor", "test", "explain", "write"})

# Complexity levels eligible for cheap routing
_CHEAP_COMPLEXITIES = frozenset({"trivial", "simple"})

# Model tiers
MODEL_SONNET = "claude-sonnet-4-5-20251001"   # Default: full capability
MODEL_HAIKU  = "claude-haiku-4-5-20251001"    # Cheap: fast, sufficient for simple tasks

# Minimum visits before we trust the Q-Table for routing
_MIN_VISITS_TO_ROUTE: int = 3


@dataclass
class RoutingDecision:
    """
    LLM routing recommendation from Q-Table analysis.

    Attributes:
        recommended_model:  Which model to use for the next task.
        route_to_local:     True if downgrading from Sonnet to Haiku/cheaper.
        confidence:         Q-Table confidence for this state [0, 0.618].
        reason:             Human-readable routing explanation.
        task_type:          Detected task type that influenced decision.
        complexity:         Detected complexity that influenced decision.
    """
    recommended_model: str
    route_to_local: bool
    confidence: float
    reason: str
    task_type: str
    complexity: str


class LLMRouter:
    """
    Routes SDK tasks to appropriate LLM tier based on Q-Table confidence.

    Designed to be called after each SDK result to determine whether the
    NEXT task of this type should use a cheaper model.

    Usage:
        router = LLMRouter()
        decision = router.route("SDK:claude-sonnet:debug:trivial", qtable, "debug", "trivial")
        if decision.route_to_local:
            # Send set_model to switch Claude to Haiku
            await send({"type": "set_model", "model": decision.recommended_model})
    """

    def __init__(self) -> None:
        self._total_routes: int = 0
        self._routes_to_local: int = 0
        self._routes_to_full: int = 0

    def route(
        self,
        state_key: str,
        qtable: Any,
        task_type: str,
        complexity: str,
    ) -> RoutingDecision:
        """
        Determine optimal model for the given task type and complexity.

        Args:
            state_key:  Q-Table state key (e.g. "SDK:claude-sonnet:debug:trivial")
            qtable:     QTable instance — source of confidence scores
            task_type:  Task type from classify_task() (debug/refactor/test/...)
            complexity: Task complexity from estimate_complexity() (trivial/simple/...)

        Returns:
            RoutingDecision with recommended model + reason.
        """
        # Get Q-Table confidence for this state
        confidence = qtable.confidence(state_key)

        # Gate 1: Minimum confidence threshold (φ⁻¹ = 0.618)
        if confidence < PHI_INV:
            return RoutingDecision(
                recommended_model=MODEL_SONNET,
                route_to_local=False,
                confidence=confidence,
                reason=f"Cold start — confidence {confidence:.3f} < φ⁻¹ ({PHI_INV:.3f})",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 2: Task type must be in known-simple set
        if task_type not in _SIMPLE_TASK_TYPES:
            return RoutingDecision(
                recommended_model=MODEL_SONNET,
                route_to_local=False,
                confidence=confidence,
                reason=f"Task '{task_type}' requires full capability (not in simple set)",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 3: Complexity must be trivial or simple
        if complexity not in _CHEAP_COMPLEXITIES:
            return RoutingDecision(
                recommended_model=MODEL_SONNET,
                route_to_local=False,
                confidence=confidence,
                reason=f"Complexity '{complexity}' too high for cheap routing",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 4: Visit count sanity — need minimum data
        best_action = qtable.exploit(state_key)
        entry = qtable._table.get(state_key, {}).get(best_action)
        visits = entry.visits if entry is not None else 0
        if visits < _MIN_VISITS_TO_ROUTE:
            return RoutingDecision(
                recommended_model=MODEL_SONNET,
                route_to_local=False,
                confidence=confidence,
                reason=f"Insufficient data: {visits} visits < {_MIN_VISITS_TO_ROUTE} minimum",
                task_type=task_type,
                complexity=complexity,
            )

        # All gates passed → route to Haiku
        self._total_routes += 1
        self._routes_to_local += 1
        logger.info(
            "LLM_ROUTE: %s/%s → Haiku (conf=%.3f, visits=%d)",
            task_type, complexity, confidence, visits,
        )
        return RoutingDecision(
            recommended_model=MODEL_HAIKU,
            route_to_local=True,
            confidence=confidence,
            reason=(
                f"Q-Table confident (conf={confidence:.3f}, visits={visits}): "
                f"'{task_type}/{complexity}' → Haiku (save ~75% cost)"
            ),
            task_type=task_type,
            complexity=complexity,
        )

    def stats(self) -> dict:
        """Stats for /sdk/routing endpoint."""
        total = max(self._total_routes, 1)
        return {
            "total_routes": self._total_routes,
            "routes_to_local": self._routes_to_local,
            "routes_to_full": self._routes_to_full,
            "local_rate": round(self._routes_to_local / total, 3),
            "phi_threshold": PHI_INV,
            "min_visits": _MIN_VISITS_TO_ROUTE,
            "simple_task_types": sorted(_SIMPLE_TASK_TYPES),
            "cheap_complexities": sorted(_CHEAP_COMPLEXITIES),
        }
