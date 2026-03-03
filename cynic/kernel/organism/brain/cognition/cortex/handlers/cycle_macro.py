"""
MacroCycleHandler " L1 MACRO consciousness cycle (full 7-step PERCEIVE'JUDGE'DECIDE'ACT'LEARN'ACCOUNT'EMERGE).

Extracted from JudgeOrchestrator._cycle_macro().

Responsibility:
- Run ALL Dogs (11 dogs, filtered by E-Score reputation)
- Full PBFT consensus (quorum required)
- Axiom scoring at full depth (fractal_depth=3)
- Execute actions (DECIDE + ACT integrated)
- Emit all event signals (PERCEPTION_RECEIVED, RESIDUAL_HIGH)
- ~160 LOC canonical cycle
"""

from __future__ import annotations

import logging
import time
from collections.abc import Callable
from typing import TYPE_CHECKING, Any

from cynic.kernel.core.event_bus import EventBus
from cynic.kernel.organism.brain.cognition.cortex.handlers.base import (
    BaseHandler,
    HandlerResult,
)

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import (
        JudgmentPipeline,
    )


logger = logging.getLogger(
    "cynic.kernel.organism.brain.cognition.cortex.handlers.cycle_macro"
)


class MacroCycleHandler(BaseHandler):
    """
    L1 MACRO cycle handler.

    Injects:
    - dogs: dict[str, AbstractDog] (all dogs)
    - axiom_arch: AxiomArchitecture
    - cynic_dog: CynicDog (for PBFT coordination)
    - escore_tracker: EScoreTracker (optional, for reputation filtering)
    - lod_controller: LODController (optional, for health context)
    - axiom_monitor: AxiomMonitor (optional, for active axiom context)
    - context_compressor: ContextCompressor (optional, for memory injection)
    - act_phase_fn: Callable (for DECIDE + ACT step)
    """

    handler_id = "cycle_macro"
    version = "1.0"
    description = "L1 MACRO cycle: full 7-step cycle with E-Score filtering, PBFT, and action execution"

    def __init__(
        self,
        bus: EventBus,
        dogs: dict[str, Any],
        axiom_arch: Any,
        cynic_dog: Any | None = None,
        escore_tracker: Any | None = None,
        lod_controller: Any | None = None,
        axiom_monitor: Any | None = None,
        context_compressor: Any | None = None,
        act_phase_fn: Callable | None = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.escore_tracker = escore_tracker
        self.lod_controller = lod_controller
        self.axiom_monitor = axiom_monitor
        self.context_compressor = context_compressor
        self.act_phase_fn = act_phase_fn
        self.bus = bus

    async def _act_phase(self, judgment: Any, pipeline: Any) -> Any:
        """Call injected act phase logic."""
        if self.act_phase_fn:
            return await self.act_phase_fn(judgment, pipeline)
        return None

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute MACRO cycle for a cell via modular 7-step pipeline.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import (
                execute_judgment_pipeline,
            )

            # The orchestrator is needed by the stages to access Dogs, monitors etc.
            # We assume the caller (Composer) or the pipeline has access to it.
            # In our current architecture, the registry handlers are initialized with
            # references to these components.

            # We create a shim orchestrator if needed, or pass self if we have the dogs.
            # Since MacroCycleHandler already has dogs and monitors, it can act as the stage context.
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
                metadata={
                    "cell_id": pipeline.cell.cell_id,
                    "level": "MACRO",
                    "verdict": judgment.verdict,
                    "consensus_reached": judgment.consensus_reached,
                },
            )
        except Exception as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
