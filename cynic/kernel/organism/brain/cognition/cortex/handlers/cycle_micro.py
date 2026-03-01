"""
MicroCycleHandler — L2 MICRO consciousness cycle (voting dogs, no ACT).

Extracted from JudgeOrchestrator._cycle_micro().

Responsibility:
- Run ALL Dogs (7 voting dogs + LLM dogs at reduced budget)
- PBFT consensus on reduced budget
- Fast path (~500ms)
- Escalate to MACRO if consensus fails
"""

from __future__ import annotations

import asyncio
import logging
import time
from typing import TYPE_CHECKING, Any

from cynic.kernel.core.axioms import verdict_from_q_score
from cynic.kernel.core.consciousness import ConsciousnessLevel, dogs_for_level
from cynic.kernel.core.event_bus import EventBusError
from cynic.kernel.core.judgment import Judgment
from cynic.kernel.core.phi import MAX_CONFIDENCE, PHI_INV, PHI_INV_2
from cynic.kernel.organism.brain.cognition.cortex.handlers.base import BaseHandler, HandlerResult

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgmentPipeline

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.handlers.cycle_micro")


class MicroCycleHandler(BaseHandler):
    """
    L2 MICRO cycle handler.

    Injects:
    - dogs: dict[str, AbstractDog] (all dogs)
    - axiom_arch: AxiomArchitecture
    - cynic_dog: CynicDog (for PBFT coordination)
    - lod_controller: LODController (for cap checking)
    """

    handler_id = "cycle_micro"
    version = "1.0"
    description = "L2 MICRO cycle: voting dogs, PBFT, with escalation"

    def __init__(
        self,
        dogs: dict[str, Any],
        axiom_arch: Any,
        cynic_dog: Any | None = None,
        lod_controller: Any | None = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.lod_controller = lod_controller

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute MICRO cycle for a cell via canonical 7-step pipeline.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import execute_judgment_pipeline
            
            pipeline.level = ConsciousnessLevel.MICRO
            pipeline = await execute_judgment_pipeline(self, pipeline)
            
            judgment = pipeline.final_judgment
            if judgment is None:
                raise RuntimeError("7-step pipeline failed to produce a judgment")

            # Check if escalation needed (Consensus weak)
            escalate_to_macro = False
            if judgment.confidence < 0.5:
                 # Budget check for escalation
                 # (Simplified here, in real kernel this is more complex)
                 escalate_to_macro = True

            duration_ms = (time.perf_counter() - t0) * 1000
            metadata = {
                "cell_id": pipeline.cell.cell_id,
                "level": "MICRO",
                "verdict": judgment.verdict,
            }
            if escalate_to_macro:
                metadata["escalate_to"] = "cycle_macro"

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=judgment,
                duration_ms=duration_ms,
                metadata=metadata,
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
