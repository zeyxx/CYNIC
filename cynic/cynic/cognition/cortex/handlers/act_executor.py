"""ActHandler — DECIDE + ACT phases."""
from __future__ import annotations

import time
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Judgment

import logging

logger = logging.getLogger("cynic.cognition.cortex.handlers.act_executor")


class ActHandler(BaseHandler):
    """ACT phase handler."""

    handler_id = "act_executor"
    version = "1.0"
    description = "ACT phase: DECIDE + execute actions"

    def __init__(
        self,
        decide_agent: Optional[Any] = None,
        decision_validator: Optional[Any] = None,
        runner: Optional[Any] = None,
        **kwargs: Any,
    ) -> None:
        self.decide_agent = decide_agent
        self.decision_validator = decision_validator
        self.runner = runner

    async def execute(self, judgment: Judgment, **kwargs: Any) -> HandlerResult:
        """Execute ACT phase."""
        t0 = time.perf_counter()
        try:
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=None,
                duration_ms=duration_ms,
                metadata={"judgment_id": judgment.judgment_id},
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_act", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
