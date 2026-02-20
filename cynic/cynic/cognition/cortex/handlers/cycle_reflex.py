"""
ReflexCycleHandler — L3 REFLEX consciousness cycle (fast, non-LLM).

Extracted from JudgeOrchestrator._cycle_reflex().

Responsibility:
- Run only GUARDIAN + ANALYST + JANITOR + CYNIC dogs
- Majority vote (no PBFT)
- Fast path (<10ms)
"""
from __future__ import annotations

import logging
import time
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Cell, Judgment

logger = logging.getLogger("cynic.cognition.cortex.handlers.cycle_reflex")


class ReflexCycleHandler(BaseHandler):
    """
    L3 REFLEX cycle handler.

    Injects:
    - dogs: dict[str, AbstractDog] (all dogs)
    - axiom_arch: AxiomArchitecture
    - consciousness_state: ConsciousnessState
    """

    handler_id = "cycle_reflex"
    version = "1.0"
    description = "L3 REFLEX cycle: non-LLM dogs, fast path"

    def __init__(
        self,
        dogs: dict[str, Any],
        axiom_arch: Any,
        consciousness_state: Optional[Any] = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.consciousness_state = consciousness_state

    async def execute(self, cell: Cell, **kwargs: Any) -> HandlerResult:
        """
        Execute REFLEX cycle for a cell.

        Args:
            cell: The Cell being judged

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            # TODO: Extract _cycle_reflex logic from orchestrator.py
            # For now, return placeholder
            duration_ms = (time.perf_counter() - t0) * 1000

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=None,  # Would be Judgment
                duration_ms=duration_ms,
                metadata={"cell_id": cell.cell_id, "level": "REFLEX"},
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_reflex", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
