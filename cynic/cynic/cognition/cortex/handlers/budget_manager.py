"""
BudgetManager Handler — ACCOUNT phase resource management.

Extracted from JudgeOrchestrator budget methods + LevelSelector.

Responsibility:
- Calculate axiom-based budget multiplier (γ3 loop)
- React to BUDGET_WARNING / BUDGET_EXHAUSTED signals
- Apply LOD cap enforcement to consciousness levels
- Track stress state for level selection
"""
from __future__ import annotations

import logging
import time
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.phi import PHI, PHI_INV, PHI_INV_2

logger = logging.getLogger("cynic.cognition.cortex.handlers.budget_manager")


class BudgetManager(BaseHandler):
    """
    ACCOUNT phase budget manager.

    Injects:
    - axiom_monitor: AxiomMonitor (optional, for γ3 multiplier)
    - lod_controller: LODController (optional, for emergency enforcement)
    """

    handler_id = "budget_manager"
    version = "1.0"
    description = "ACCOUNT: budget multiplier (γ3), stress enforcement, LOD cap"

    def __init__(
        self,
        axiom_monitor: Optional[Any] = None,
        lod_controller: Optional[Any] = None,
        **kwargs: Any,
    ) -> None:
        self.axiom_monitor = axiom_monitor
        self.lod_controller = lod_controller
        self._budget_stress = False
        self._budget_exhausted = False

    async def execute(self, budget_usd: float = 0.0, **kwargs: Any) -> HandlerResult:
        """
        Execute budget status check and multiplier calculation.

        Args:
            budget_usd: Current budget (optional, for context)

        Returns:
            HandlerResult with budget status and multiplier
        """
        t0 = time.perf_counter()
        try:
            multiplier = self.compute_axiom_budget_multiplier()
            status = {
                "budget_stress": self._budget_stress,
                "budget_exhausted": self._budget_exhausted,
                "axiom_multiplier": round(multiplier, 3),
                "effective_budget": round(budget_usd * multiplier, 6) if budget_usd else 0.0,
            }
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution("budget_check", f"stress={self._budget_stress} exhausted={self._budget_exhausted}")

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=status,
                duration_ms=duration_ms,
                metadata={
                    "multiplier": multiplier,
                    "stress_active": self._budget_stress,
                    "exhausted": self._budget_exhausted,
                },
            )
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_budget", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )

    def compute_axiom_budget_multiplier(self) -> float:
        """
        Compute budget multiplier from emergent axiom health (γ3 loop).

        Active axioms signal a healthy, coordinated organism — it can afford
        deeper judgment (MACRO). Dormant axioms signal stress — conserve budget.

        Multiplier table (φ-derived):
            0 active axioms → PHI_INV_2 = 0.382  (stressed  → REFLEX/MICRO)
            1 active axiom  → PHI_INV   = 0.618  (stirring  → MICRO)
            2 active axioms → 1.0                (balanced  → MACRO)
            3 active axioms → PHI       = 1.618  (healthy   → deeper MACRO)
            4 active axioms → PHI²      = 2.618  (peak      → max depth)

        Formula: PHI ** (active_count - 2)  →  range [0.382, 2.618]

        Returns:
            Budget multiplier in range [0.382, 2.618]
        """
        if self.axiom_monitor is None:
            return 1.0
        active = self.axiom_monitor.active_count()
        return PHI ** (active - 2)

    def on_budget_warning(self) -> None:
        """
        React to BUDGET_WARNING — cap future judgments at MICRO level.

        Called by state.py when AccountAgent emits BUDGET_WARNING (38.2% budget left).
        Prevents Ollama MACRO calls while budget is low.
        """
        if not self._budget_stress:
            self._budget_stress = True
            logger.warning(
                "*GROWL* Budget stress: capping judgment level at MICRO "
                "(no MACRO/Ollama until budget resets)"
            )

    def on_budget_exhausted(self) -> None:
        """
        React to BUDGET_EXHAUSTED — cap future judgments at REFLEX level.

        Called by state.py when AccountAgent emits BUDGET_EXHAUSTED (budget=0).
        Zero LLM calls — pure heuristic until session resets.
        """
        if not self._budget_exhausted:
            self._budget_exhausted = True
            logger.error(
                "*GROWL* Budget exhausted: forcing REFLEX-only mode "
                "(zero LLM calls)"
            )

    def reset_budget_state(self) -> None:
        """Reset stress and exhaustion flags when budget refills."""
        self._budget_stress = False
        self._budget_exhausted = False
        logger.info("Budget state reset")

    def apply_lod_cap(self, level: ConsciousnessLevel) -> ConsciousnessLevel:
        """
        Apply LOD cap to consciousness level (B2 fix).

        Enforces system health constraints on depth selection.
        Single enforcement point for both auto-selected and explicit levels.

        Args:
            level: Consciousness level to cap

        Returns:
            Capped consciousness level
        """
        if self.lod_controller is None:
            return level

        from cynic.cognition.cortex.lod import SurvivalLOD

        lod = self.lod_controller.current
        if lod >= SurvivalLOD.EMERGENCY:
            if level != ConsciousnessLevel.REFLEX:
                logger.warning(
                    "LOD cap: %s → REFLEX (LOD=%s, system under stress)",
                    level.name,
                    lod.name,
                )
            return ConsciousnessLevel.REFLEX
        if lod == SurvivalLOD.REDUCED and level == ConsciousnessLevel.MACRO:
            logger.info("LOD cap: MACRO → MICRO (LOD=REDUCED)")
            return ConsciousnessLevel.MICRO
        return level
