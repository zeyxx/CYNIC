"""
LevelSelector Handler — Consciousness level selection logic.

Extracted from JudgeOrchestrator._select_level() and related budget/LOD methods.

Responsibilities:
- Enforce LOD (system health) cap
- Enforce budget stress caps
- Use ConsciousnessScheduler if available
- Fall back to cell consciousness gradient
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
        except Exception as e:
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

        Priority order:
        1. LOD enforcement (system health caps depth)
        2. Budget enforcement (stress/exhausted caps depth)
        3. ConsciousnessScheduler (blended axiom + e_score + oracle) if available
        4. Cell's own consciousness gradient (fallback)
        """
        # LOD enforcement (health→JUDGE loop): system health caps depth first
        if self.lod_controller is not None:
            from cynic.cognition.cortex.lod import SurvivalLOD

            lod = self.lod_controller.current
            if lod >= SurvivalLOD.EMERGENCY:
                logger.debug("LOD EMERGENCY → REFLEX")
                return ConsciousnessLevel.REFLEX
            if lod == SurvivalLOD.REDUCED:
                logger.debug("LOD REDUCED → MICRO (at most)")
                consciousness = get_consciousness()
                suggested = consciousness.should_downgrade(budget_usd)
                if suggested == ConsciousnessLevel.REFLEX:
                    return ConsciousnessLevel.REFLEX
                return ConsciousnessLevel.MICRO

        # Budget enforcement (ACCOUNT→JUDGE loop): stressed budget caps depth
        if self._budget_exhausted:
            logger.debug("Budget exhausted → REFLEX")
            return ConsciousnessLevel.REFLEX
        if self._budget_stress:
            logger.debug("Budget stress → MICRO (at most)")
            consciousness = get_consciousness()
            suggested = consciousness.should_downgrade(budget_usd)
            if suggested == ConsciousnessLevel.REFLEX:
                return ConsciousnessLevel.REFLEX
            return ConsciousnessLevel.MICRO

        # ConsciousnessScheduler (Task #8: blended escalation policy)
        if self.consciousness_scheduler is not None:
            try:
                level = await self.consciousness_scheduler.select_level(cell)
                logger.debug(f"ConsciousnessScheduler selected {level.name}")
                return level
            except Exception as e:
                logger.warning(f"ConsciousnessScheduler failed: {e}")
                # Fall through to legacy logic

        # Legacy fallback: budget-based downgrade
        consciousness = get_consciousness()
        suggested = consciousness.should_downgrade(budget_usd)
        if suggested:
            logger.debug(f"Budget downgrade → {suggested.name}")
            return suggested

        # Use cell's own consciousness gradient to guide level selection
        if cell.consciousness <= 1:
            return ConsciousnessLevel.REFLEX
        elif cell.consciousness <= 3:
            return ConsciousnessLevel.MICRO
        else:
            return ConsciousnessLevel.MACRO

    def set_budget_stress(self, stressed: bool) -> None:
        """Signal budget stress (caps at MICRO)."""
        self._budget_stress = stressed
        logger.info(f"Budget stress: {stressed}")

    def set_budget_exhausted(self, exhausted: bool) -> None:
        """Signal budget exhaustion (caps at REFLEX)."""
        self._budget_exhausted = exhausted
        logger.info(f"Budget exhausted: {exhausted}")
