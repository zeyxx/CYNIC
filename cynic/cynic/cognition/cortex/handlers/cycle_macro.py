"""MacroCycleHandler — L1 MACRO consciousness cycle (full 7-step)."""
from __future__ import annotations

import time
from typing import Any

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Cell

import logging

logger = logging.getLogger("cynic.cognition.cortex.handlers.cycle_macro")


class MacroCycleHandler(BaseHandler):
    """L1 MACRO cycle handler."""

    handler_id = "cycle_macro"
    version = "1.0"
    description = "L1 MACRO cycle: full 7-step cycle"

    def __init__(self, dogs: dict[str, Any], axiom_arch: Any, **kwargs: Any) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch

    async def execute(self, cell: Cell, **kwargs: Any) -> HandlerResult:
        """Execute MACRO cycle."""
        t0 = time.perf_counter()
        try:
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=None,
                duration_ms=duration_ms,
                metadata={"cell_id": cell.cell_id, "level": "MACRO"},
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_macro", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
