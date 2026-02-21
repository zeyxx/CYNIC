"""
LevelSelector Handler — Consciousness level selection logic.

Extracted from JudgeOrchestrator._select_level() and related budget/LOD methods.

Responsibilities:
- Enforce LOD (system health) cap
- Enforce budget stress caps
- Use ConsciousnessScheduler if available
- Fall back to cell consciousness gradient
- React to budget warning/exhausted signals
"""
from __future__ import annotations

import logging
import time
from typing import Any, Optional

from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.judgment import Cell
from cynic.core.phi import PHI_INV
from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult

logger = logging.getLogger("cynic.cognition.cortex.handlers.level_selector")


class LevelSelector(BaseHandler):
    """
    Selects consciousness level for a judgment.

    Injects:
    - lod_controller: LODController (optional, for system health cap)
    - consciousness_scheduler: ConsciousnessScheduler (optional, for blended escalation)
    - axiom_monitor: AxiomMonitor (optional, for axiom health signals)
    - escore_tracker: EScoreTracker (optional, for E-score)
    """

    handler_id = "level_selector"
    version = "1.0"
    description = "Consciousness level selection with LOD + budget enforcement"

    def __init__(
        self,
        lod_controller: Optional[Any] = None,
        consciousness_scheduler: Optional[Any] = None,
        axiom_monitor: Optional[Any] = None,
        escore_tracker: Optional[Any] = None,
    ) -> None:
        self.lod_controller = lod_controller
        self.consciousness_scheduler = consciousness_scheduler
        self.axiom_monitor = axiom_monitor
        self.escore_tracker = escore_tracker
        self._budget_stress = False
        self._budget_exhausted = False

    async def execute(
        self,
        cell: Cell,
        budget_usd: float,
        current_level: Optional[ConsciousnessLevel] = None,
        **kwargs: Any,
    ) -> HandlerResult:
        """
        Select consciousness level for a cell.

        Args:
            cell: The Cell being judged
            budget_usd: Budget available for this judgment
            current_level: Current level (for hysteresis, optional)

        Returns:
            HandlerResult with level in output
        """
        t0 = time.perf_counter()
        try:
            level = await self.select_level(cell, budget_usd)
            duration_ms = (time.perf_counter() - t0) * 1000

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=level,
                duration_ms=duration_ms,
                metadata={
                    "level_name": level.name,
                    "budget_usd": budget_usd,
                    "cell_consciousness": cell.consciousness,
                },
            )
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("select_level", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )

    async def select_level(
        self, cell: Cell, budget_usd: float
    ) -> ConsciousnessLevel:
        """
        Auto-select consciousness level based on budget and cell metadata.

        Priority order (with LOD cap enforced at each step):
        1. LOD enforcement (system health caps depth) — absolute priority
        2. Budget enforcement (stress/exhausted caps depth)
        3. ConsciousnessScheduler (blended axiom + e_score + oracle) if available
        4. Cell's own consciousness gradient (fallback)

        All selected levels are capped by apply_lod_cap() before return.
        """
        selected_level = None

        # Step 1: LOD enforcement (health→JUDGE loop): system health caps depth first
        # This is absolute priority — a crashed system can't afford Ollama regardless
        if self.lod_controller is not None:
            from cynic.cognition.cortex.lod import SurvivalLOD

            lod = self.lod_controller.current
            if lod >= SurvivalLOD.EMERGENCY:
                logger.warning("LOD EMERGENCY → REFLEX (system under stress)")
                return ConsciousnessLevel.REFLEX
            if lod == SurvivalLOD.REDUCED:
                logger.info("LOD REDUCED: capping at MICRO")
                consciousness = get_consciousness()
                suggested = consciousness.should_downgrade(budget_usd)
                if suggested == ConsciousnessLevel.REFLEX:
                    return ConsciousnessLevel.REFLEX
                return ConsciousnessLevel.MICRO

        # Step 2: Budget enforcement (ACCOUNT→JUDGE loop): stressed budget caps depth
        if self._budget_exhausted:
            logger.error("Budget exhausted: forcing REFLEX-only mode")
            return ConsciousnessLevel.REFLEX
        if self._budget_stress:
            logger.warning("Budget stress: capping at MICRO (no Ollama calls)")
            consciousness = get_consciousness()
            suggested = consciousness.should_downgrade(budget_usd)
            if suggested == ConsciousnessLevel.REFLEX:
                return ConsciousnessLevel.REFLEX
            return ConsciousnessLevel.MICRO

        # Step 3: ConsciousnessScheduler (Task #8: blended escalation policy)
        # Uses axiom maturity, E-Score, oracle confidence for intelligent selection
        if self.consciousness_scheduler is not None:
            try:
                selected_level = await self.consciousness_scheduler.select_level(cell)
                logger.debug(f"ConsciousnessScheduler selected {selected_level.name}")
            except CynicError as e:
                logger.warning(f"ConsciousnessScheduler failed, falling back: {e}")
                selected_level = None

        # Step 4: Legacy fallback if scheduler unavailable or failed
        if selected_level is None:
            consciousness = get_consciousness()
            suggested = consciousness.should_downgrade(budget_usd)
            if suggested:
                selected_level = suggested
            else:
                # Final fallback: cell's own consciousness gradient
                if cell.consciousness <= 1:
                    selected_level = ConsciousnessLevel.REFLEX
                elif cell.consciousness <= 3:
                    selected_level = ConsciousnessLevel.MICRO
                else:
                    selected_level = ConsciousnessLevel.MACRO

        # Step 5: Apply LOD cap to final selection
        # Single enforcement point: ensures LOD cap is applied consistently
        # (even if ConsciousnessScheduler bypasses internal LOD checks)
        final_level = self.apply_lod_cap(selected_level)

        if final_level != selected_level:
            logger.info(f"LOD cap enforced: {selected_level.name} → {final_level.name}")

        return final_level

    def apply_lod_cap(self, level: ConsciousnessLevel) -> ConsciousnessLevel:
        """
        Enforce LOD cap on any level — explicit or auto-selected (B2 fix).

        _select_level() already enforces this for the auto-select path, but:
          1. run(cell, level=MACRO) bypasses _select_level entirely.
          2. _cycle_micro escalation calls _cycle_macro directly (no level check).
        This method is the single enforcement point for both cases.

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
                    level.name, lod.name,
                )
            return ConsciousnessLevel.REFLEX
        if lod == SurvivalLOD.REDUCED and level == ConsciousnessLevel.MACRO:
            logger.info("LOD cap: MACRO → MICRO (LOD=REDUCED)")
            return ConsciousnessLevel.MICRO
        return level

    def set_budget_stress(self, stressed: bool) -> None:
        """Signal budget stress (caps at MICRO)."""
        self._budget_stress = stressed
        logger.warning(
            "*GROWL* Budget stress: capping judgment level at MICRO "
            "(no MACRO/Ollama until budget resets)"
        ) if stressed else logger.info("Budget stress cleared")

    def set_budget_exhausted(self, exhausted: bool) -> None:
        """Signal budget exhaustion (caps at REFLEX)."""
        self._budget_exhausted = exhausted
        logger.error(
            "*GROWL* Budget exhausted: forcing REFLEX-only mode "
            "(zero LLM calls)"
        ) if exhausted else logger.info("Budget exhaustion cleared")
