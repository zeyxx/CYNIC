"""BudgetManager Handler — Resource limits and LOD enforcement."""
from __future__ import annotations

import time
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.consciousness import ConsciousnessLevel

import logging

logger = logging.getLogger("cynic.cognition.cortex.handlers.budget_manager")


class BudgetManager(BaseHandler):
    """Budget and LOD management handler."""

    handler_id = "budget_manager"
    version = "1.0"
    description = "Resource limits: budget stress, LOD cap enforcement"

    def __init__(
        self,
        lod_controller: Optional[Any] = None,
        axiom_monitor: Optional[Any] = None,
        **kwargs: Any,
    ) -> None:
        self.lod_controller = lod_controller
        self.axiom_monitor = axiom_monitor
        self._budget_stress = False
        self._budget_exhausted = False

    async def execute(self, **kwargs: Any) -> HandlerResult:
        """Check budget and LOD status."""
        t0 = time.perf_counter()
        try:
            status = {
                "budget_stress": self._budget_stress,
                "budget_exhausted": self._budget_exhausted,
            }
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=status,
                duration_ms=duration_ms,
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("check_budget", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )

    def on_budget_warning(self) -> None:
        """Signal budget stress."""
        self._budget_stress = True
        logger.warning("Budget stress activated")

    def on_budget_exhausted(self) -> None:
        """Signal budget exhaustion."""
        self._budget_exhausted = True
        logger.error("Budget exhausted")

    def apply_lod_cap(self, level: ConsciousnessLevel) -> ConsciousnessLevel:
        """Apply LOD cap to consciousness level."""
        if not self.lod_controller:
            return level

        from cynic.cognition.cortex.lod import SurvivalLOD

        lod = self.lod_controller.current
        if lod >= SurvivalLOD.EMERGENCY:
            return ConsciousnessLevel.REFLEX
        if lod == SurvivalLOD.REDUCED and level == ConsciousnessLevel.MACRO:
            return ConsciousnessLevel.MICRO
        return level
