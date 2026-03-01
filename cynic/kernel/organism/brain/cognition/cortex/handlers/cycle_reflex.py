"""
ReflexCycleHandler — L3 REFLEX consciousness cycle (fast, non-LLM).

Extracted from JudgeOrchestrator._cycle_reflex().

Responsibility:
- Run only GUARDIAN + ANALYST + JANITOR + CYNIC dogs
- Majority vote (no PBFT)
- Fast path (<10ms)
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import TYPE_CHECKING, Any

from cynic.kernel.core.axioms import verdict_from_q_score
from cynic.kernel.core.consciousness import ConsciousnessLevel, dogs_for_level
from cynic.kernel.core.exceptions import CynicError
from cynic.kernel.core.judgment import Judgment
from cynic.kernel.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE, PHI_INV_2, phi_bound_score
from cynic.kernel.organism.brain.cognition.cortex.handlers.base import BaseHandler, HandlerResult

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgmentPipeline

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.handlers.cycle_reflex")


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
        consciousness_state: Any | None = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.consciousness_state = consciousness_state

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute REFLEX cycle for a cell via canonical 7-step pipeline.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import execute_judgment_pipeline
            
            pipeline.level = ConsciousnessLevel.REFLEX
            pipeline = await execute_judgment_pipeline(self, pipeline)
            
            judgment = pipeline.final_judgment
            if judgment is None:
                raise RuntimeError("7-step pipeline failed to produce a judgment")

            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=judgment,
                duration_ms=duration_ms,
                metadata={"cell_id": pipeline.cell.cell_id, "level": "REFLEX", "verdict": judgment.verdict},
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
