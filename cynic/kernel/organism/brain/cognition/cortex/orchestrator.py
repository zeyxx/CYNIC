"""
CYNIC Judge Orchestrator  Full 7-Step Cycle (Pipeline DAG).

Orchestrates the complete judgment pipeline:
  1. PERCEIVE   Cell is received (perception event)
  2. JUDGE      Dogs analyze, PHI-BFT consensus
  3. DECIDE     Governance approval/rejection
  4. ACT        Real-world effector execution
  5. LEARN      Q-Table reinforcement & Thompson Sampling
  6. ACCOUNT    Cost tracking & E-Score updates
  7. EMERGE     Pattern detection & self-improvement signals

This implementation uses a deterministic Pipeline DAG for total auditability.
"""

from __future__ import annotations

import logging
from typing import Any


from cynic.kernel.core.axioms import AxiomArchitecture
from cynic.kernel.core.consciousness import (
    ConsciousnessLevel,
)
from cynic.kernel.core.event_bus import (
    CoreEvent,
    Event,
)
from cynic.kernel.core.events_schema import (
    JudgmentFailedPayload,
)
from cynic.kernel.core.judgment import Cell, Judgment
from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import (
    execute_judgment_pipeline,
)
from cynic.kernel.organism.brain.cognition.cortex.pipeline import JudgmentPipeline
from cynic.kernel.organism.brain.cognition.neurons.base import AbstractDog

logger = logging.getLogger("cynic.kernel.brain.orchestrator")


class JudgeOrchestrator:
    """
    The 'Cortex' of CYNIC. Orchestrates the 7-step judgment cycle.
    Now follows a strict Pipeline DAG pattern for isolation and auditability.
    """

    def __init__(
        self,
        dogs: dict[str, AbstractDog],
        axiom_arch: AxiomArchitecture,
        cynic_dog: AbstractDog,
        bus: EventBus,
        residual_detector=None,
        gasdf_executor=None,
        state_manager=None,
        instance_id: str = "",
        llm_registry: Any | None = None,
        consciousness: Any | None = None,
        identity: Any | None = None,
        judgment_engine: Any | None = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.residual_detector = residual_detector
        self.gasdf_executor = gasdf_executor
        self.state_manager = state_manager
        self.instance_id = instance_id
        self.bus = bus
        self._consciousness = consciousness
        self.identity = identity
        self.judgment_engine = judgment_engine

        # Optional managers (injected via state.py or factory)
        self.axiom_monitor = None
        self.lod_controller = None
        self.context_compressor = None
        self.service_registry = None
        self.benchmark_registry = None
        self.escore_tracker = None
        self.decision_validator = None

        self._judgment_count = 0
        from cynic.kernel.organism.brain.cognition.cortex.circuit_breaker import (
            CircuitBreaker,
        )

        self._circuit_breaker = CircuitBreaker()

        from cynic.kernel.organism.brain.dialogue.agent import DialogueAgent

        self._dialogue_agent = DialogueAgent(llm_registry=llm_registry)

    async def run(
        self,
        cell: Cell,
        level: ConsciousnessLevel | None = None,
        budget_usd: float | None = None,
        fractal_depth: int = 1,
    ) -> Judgment:
        """
        Execute the deterministic 7-step judgment pipeline DAG.
        """
        # 1. Level Selection (Governance + Hardware)
        selected_level = level
        if selected_level is None:
            selected_level = await self._select_level(
                cell, budget_usd or cell.budget_usd
            )

        # 2. Survival Cap
        selected_level = self._apply_lod_cap(selected_level)

        # 3. Create Immutable Pipeline
        pipeline = JudgmentPipeline(
            cell=cell, level=selected_level, fractal_depth=fractal_depth
        )

        logger.info(
            f"[{pipeline.trace_id}]  STARTING DAG: {selected_level.name} | Cell: {cell.cell_id[:8]}"
        )

        # Check Circuit Breaker
        if not self._circuit_breaker.allow():
            logger.warning(
                f"[{pipeline.trace_id}]  Circuit Breaker OPEN - Suspending judgment."
            )
            raise RuntimeError("CircuitBreaker OPEN")

        try:
            # 4. Execute the DAG (Pure Pipeline)
            pipeline = await execute_judgment_pipeline(self, pipeline)

            judgment = pipeline.final_judgment
            if judgment is None:
                raise RuntimeError("Pipeline failed to produce a judgment")

            # 5. Post-Cycle Infrastructure (Reputation, Dialogue)
            self._judgment_count += 1
            if self._consciousness:
                self._consciousness.increment(selected_level)

            if self.escore_tracker:
                for dog_id, score in judgment.dog_votes.items():
                    self.escore_tracker.update_dimension(
                        f"agent:{dog_id}", "JUDGE", score / 100.0
                    )

            # Generate reasoning if missing
            if not judgment.reasoning:
                explanation = await self._dialogue_agent.explain_judgment(
                    judgment, question=str(cell.content)
                )
                if hasattr(judgment, "model_copy"):
                    judgment = judgment.model_copy(update={"reasoning": explanation})

            # Record success for SRE visibility
            self._circuit_breaker.record_success()

            # 6. Emit Result to Nervous System
            await self.bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_CREATED,
                    judgment.model_dump(),
                    source="orchestrator",
                )
            )

            logger.info(
                f"[{pipeline.trace_id}]  DAG COMPLETE: {judgment.verdict} (Q={judgment.q_score:.1f})"
            )
            return judgment

        except Exception as e:
            self._circuit_breaker.record_failure()
            logger.error(f"[{pipeline.trace_id}]  DAG CRASH: {e}", exc_info=True)
            await self.bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_FAILED,
                    JudgmentFailedPayload(cell_id=cell.cell_id, error=str(e)),
                    source="orchestrator",
                )
            )
            raise

    async def _select_level(self, cell: Cell, budget: float) -> ConsciousnessLevel:
        """Heuristic for choosing the consciousness level."""
        # Simplified: code gets MACRO, others get MICRO
        if cell.reality == "CODE":
            return ConsciousnessLevel.MACRO
        return ConsciousnessLevel.MICRO

    def _apply_lod_cap(self, level: ConsciousnessLevel) -> ConsciousnessLevel:
        """Cap level based on system health (LOD)."""
        if self.lod_controller and hasattr(self.lod_controller, "cap"):
            return self.lod_controller.cap(level)
        return level

    async def _act_phase(self, judgment: Judgment, pipeline: JudgmentPipeline) -> Any:
        """Internal callback for ActStage."""
        # Implement real action logic or delegate to motor system
        return None
