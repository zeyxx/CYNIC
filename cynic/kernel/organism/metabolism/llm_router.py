"""
CYNIC LLM Router " Ring 4: Q-Table driven model selection

Routes SDK tasks between Primary (Brain), Fast (Nerves), and Local (Spine)
models based on accumulated Q-Table confidence and system configuration.

Bootstrap Philosophy:
  Phase 1 (cold start): All tasks ' Primary Model (build Q-Table)
  Phase 2 (warming):    Simple/trivial ' Fast Model (save cost)
  Phase 3 (hot):        80%+ tasks ' Fast/Local (maximum savings)

Safety: NEVER route complex/review/unknown tasks automatically unless in Sovereign mode.
Always returns a RoutingDecision " callers decide whether to act on it.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Any

from cynic.kernel.core.phi import PHI_INV

logger = logging.getLogger("cynic.kernel.organism.metabolism.llm_router")

# Task types safe for local/cheap routing (CYNIC has seen enough examples)
_SIMPLE_TASK_TYPES = frozenset({"debug", "refactor", "test", "explain", "write"})

# Complexity levels eligible for cheap routing
_CHEAP_COMPLEXITIES = frozenset({"trivial", "simple"})

# Model tiers
MODEL_SONNET = "claude-sonnet-4-5-20251001"  # Default: full capability
MODEL_HAIKU = "claude-haiku-4-5-20251001"  # Cheap: fast, sufficient for simple tasks

# Minimum visits before we trust the Q-Table for routing
_MIN_VISITS_TO_ROUTE: int = 3


@dataclass
class RoutingDecision:
    """
    LLM routing recommendation from Q-Table analysis.
    """

    recommended_model: str
    route_to_local: bool
    confidence: float
    reason: str
    task_type: str
    complexity: str


class LLMRouter:
    """
    Routes SDK tasks to appropriate LLM tier based on Q-Table confidence,
    Sovereign Mode (Local-First), or Operator Overrides (Slow Mode).
    """

    def __init__(self) -> None:
        self._total_routes: int = 0
        self._routes_to_local: int = 0
        self._routes_to_full: int = 0

        # Pull configuration dynamically
        from cynic.config import CynicConfig

        try:
            from cynic.kernel.core.container import get_container

            self.config = get_container().get(CynicConfig)
        except Exception:
            self.config = CynicConfig.from_env()

        # Sovereignty Check: Do we have any cloud keys?
        self.sovereign_mode = not any(
            [self.config.anthropic_api_key, self.config.google_api_key]
        )

        if self.sovereign_mode:
            logger.warning(
                "Sovereign Mode Active: No cloud API keys found. Defaulting to Local-First."
            )

    def route(
        self,
        state_key: str,
        qtable: Any,
        task_type: str,
        complexity: str,
    ) -> RoutingDecision:
        """Determine optimal model, prioritizing operator overrides and sovereignty."""

        # 0. Operator Override: Deep Thought / Slow Mode
        # If the human forces deep thought, always use the primary "Brain"
        if getattr(self.config, "force_slow_mode", False):
            self._routes_to_full += 1
            return RoutingDecision(
                recommended_model=self.config.llm_primary_model,
                route_to_local=False,
                confidence=1.0,
                reason="Operator override: Deep Thought / Slow Mode active.",
                task_type=task_type,
                complexity=complexity,
            )

        # 1. Sovereign override: If no keys, always use local
        if self.sovereign_mode:
            self._routes_to_local += 1
            return RoutingDecision(
                recommended_model=self.config.llm_local_model,
                route_to_local=True,
                confidence=1.0,
                reason="Sovereign override: Local-First enforcement (no cloud keys).",
                task_type=task_type,
                complexity=complexity,
            )

        # Get Q-Table confidence for this state
        confidence = qtable.confidence(state_key)

        # Gate 1: Minimum confidence threshold ( = 0.618)
        if confidence < PHI_INV:
            self._routes_to_full += 1
            return RoutingDecision(
                recommended_model=self.config.llm_primary_model,
                route_to_local=False,
                confidence=confidence,
                reason=f"Cold start - confidence {confidence:.3f} <  ({PHI_INV:.3f})",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 2: Task type must be in known-simple set
        if task_type not in _SIMPLE_TASK_TYPES:
            self._routes_to_full += 1
            return RoutingDecision(
                recommended_model=self.config.llm_primary_model,
                route_to_local=False,
                confidence=confidence,
                reason=f"Task '{task_type}' requires full capability (not in simple set)",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 3: Complexity must be trivial or simple
        if complexity not in _CHEAP_COMPLEXITIES:
            self._routes_to_full += 1
            return RoutingDecision(
                recommended_model=self.config.llm_primary_model,
                route_to_local=False,
                confidence=confidence,
                reason=f"Complexity '{complexity}' too high for cheap routing",
                task_type=task_type,
                complexity=complexity,
            )

        # Gate 4: Visit count sanity " need minimum data
        best_action = qtable.exploit(state_key)
        entry = qtable._table.get(state_key, {}).get(best_action)
        visits = entry.visits if entry is not None else 0
        if visits < _MIN_VISITS_TO_ROUTE:
            self._routes_to_full += 1
            return RoutingDecision(
                recommended_model=self.config.llm_primary_model,
                route_to_local=False,
                confidence=confidence,
                reason=f"Insufficient data: {visits} visits < {_MIN_VISITS_TO_ROUTE} minimum",
                task_type=task_type,
                complexity=complexity,
            )

        # All gates passed ' route to Fast Model (or Local)
        self._total_routes += 1
        self._routes_to_local += 1

        # Decide between Fast Cloud or Local based on complexity
        target_model = (
            self.config.llm_local_model
            if complexity == "trivial"
            else self.config.llm_fast_model
        )

        logger.info(
            "LLM_ROUTE: %s/%s ' %s (conf=%.3f, visits=%d)",
            task_type,
            complexity,
            target_model,
            confidence,
            visits,
        )
        return RoutingDecision(
            recommended_model=target_model,
            route_to_local=(target_model == self.config.llm_local_model),
            confidence=confidence,
            reason=(
                f"Q-Table confident (conf={confidence:.3f}, visits={visits}): "
                f"'{task_type}/{complexity}' ' {target_model} (optimization)"
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
