"""
MacroCycleHandler — L1 MACRO consciousness cycle (full 7-step PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE).

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

import asyncio
import logging
import time
from typing import Any, Callable, Optional, TYPE_CHECKING

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Cell, Judgment
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_CONFIDENCE, MAX_Q_SCORE, phi_bound_score, fibonacci
from cynic.core.axioms import verdict_from_q_score
from cynic.cognition.neurons.base import DogId

if TYPE_CHECKING:
    from cynic.cognition.cortex.orchestrator import JudgmentPipeline
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import (
    PerceptionReceivedPayload,
    ResidualHighPayload,
)
from cynic.cognition.neurons.base import DogJudgment

logger = logging.getLogger("cynic.cognition.cortex.handlers.cycle_macro")


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
        dogs: dict[str, Any],
        axiom_arch: Any,
        cynic_dog: Optional[Any] = None,
        escore_tracker: Optional[Any] = None,
        lod_controller: Optional[Any] = None,
        axiom_monitor: Optional[Any] = None,
        context_compressor: Optional[Any] = None,
        act_phase_fn: Optional[Callable] = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.escore_tracker = escore_tracker
        self.lod_controller = lod_controller
        self.axiom_monitor = axiom_monitor
        self.context_compressor = context_compressor
        self.act_phase_fn = act_phase_fn

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute MACRO cycle for a cell.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            cell = pipeline.cell

            # STEP 1: PERCEIVE (already done — cell is the perception result)
            await get_core_bus().emit(Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                PerceptionReceivedPayload(reality=cell.reality, cell_id=cell.cell_id),
            ))

            # STEP 2: JUDGE — Dogs filtered by E-Score reputation (LOD↔EScore immune system)
            dog_items = list(self.dogs.items())  # [(dog_id, dog), ...]
            if self.escore_tracker is not None:
                # φ-threshold: GROWL_MIN = 38.2% — below this, Dog is unreliable
                GROWL_MIN = PHI_INV_2 * MAX_Q_SCORE   # 38.2
                MIN_ACTIVE = fibonacci(4)              # 3 — safety floor (never run fewer)

                passing = [
                    (did, d) for did, d in dog_items
                    if self.escore_tracker.get_score(f"agent:{did}") >= GROWL_MIN
                    or did == DogId.CYNIC  # Coordinator (PBFT) is never filtered
                ]

                if len(passing) >= MIN_ACTIVE:
                    skipped_n = len(dog_items) - len(passing)
                    if skipped_n > 0:
                        passing_ids = {did for did, _ in passing}
                        skipped_ids = [did for did, _ in dog_items if did not in passing_ids]
                        logger.info(
                            "EScore filter: bypassing %d/%d Dogs (E-Score < %.1f): %s",
                            skipped_n, len(dog_items), GROWL_MIN, skipped_ids,
                        )
                    dog_items = passing

            all_dogs = [d for _, d in dog_items]
            per_dog_budget = cell.budget_usd / max(len(all_dogs), 1)

            # R2: Holographic Mirror — pass organism health context to every Dog.
            organism_kwargs: dict[str, Any] = {
                "budget_usd": per_dog_budget,
                "active_dogs": len(all_dogs),
            }
            if self.lod_controller is not None:
                organism_kwargs["lod_level"] = int(self.lod_controller.current)
            if self.axiom_monitor is not None:
                organism_kwargs["active_axioms"] = self.axiom_monitor.active_count()

            # γ5: Memory injection — pass compressed CYNIC history
            if self.context_compressor is not None:
                compressed = self.context_compressor.get_compressed_context(budget=200)
                if compressed:
                    organism_kwargs["compressed_context"] = compressed

            tasks = [dog.analyze(cell, **organism_kwargs) for dog in all_dogs]
            dog_judgments_raw = await asyncio.gather(*tasks, return_exceptions=True)

            # Filter out errors gracefully
            pipeline.dog_judgments = [
                j for j in dog_judgments_raw
                if isinstance(j, DogJudgment)
            ]
            errors = [j for j in dog_judgments_raw if isinstance(j, Exception)]
            if errors:
                logger.warning("%d Dog(s) failed: %s", len(errors), errors)

            # STEP 2b: PBFT Consensus
            consensus = await self.cynic_dog.pbft_run(cell, pipeline.dog_judgments)
            pipeline.consensus = consensus

            # STEP 2c: Axiom scoring (full depth)
            q_scores_macro = [j.q_score for j in pipeline.dog_judgments]
            avg_q_macro = sum(q_scores_macro) / len(q_scores_macro) if q_scores_macro else 0.0
            consensus_strength = (consensus.votes / consensus.quorum) if consensus and consensus.quorum else 0.0
            axiom_result = self.axiom_arch.score_and_compute(
                domain=cell.reality,
                context=str(cell.content)[:500],
                fractal_depth=3,
                metrics={
                    "avg_dog_q": avg_q_macro / MAX_Q_SCORE,
                    "consensus_strength": consensus_strength,
                },
            )

            # Use consensus Q-Score if available, else axiom Q-Score
            final_q = consensus.final_q_score or axiom_result.q_score
            final_q = phi_bound_score(final_q)
            verdict = verdict_from_q_score(final_q)

            # Residual: unexplained variance between Dog votes
            if pipeline.dog_judgments:
                votes = [j.q_score for j in pipeline.dog_judgments]
                mean_v = sum(votes) / len(votes)
                variance = sum((v - mean_v) ** 2 for v in votes) / len(votes)
                residual = min(variance / (MAX_Q_SCORE ** 2), 1.0)
            else:
                residual = 0.0

            total_cost = sum(j.cost_usd for j in pipeline.dog_judgments)
            total_llm_calls = sum(1 for j in pipeline.dog_judgments if j.llm_id)

            judgment = Judgment(
                cell=cell,
                q_score=final_q,
                verdict=verdict.value,
                confidence=min(
                    consensus.final_confidence or axiom_result.q_score / MAX_Q_SCORE * PHI_INV,
                    MAX_CONFIDENCE,
                ),
                axiom_scores=axiom_result.axiom_scores,
                active_axioms=list(axiom_result.active_axioms),
                dog_votes={j.dog_id: j.q_score for j in pipeline.dog_judgments},
                consensus_votes=consensus.votes,
                consensus_quorum=consensus.quorum,
                consensus_reached=consensus.consensus,
                cost_usd=total_cost,
                llm_calls=total_llm_calls,
                residual_variance=residual,
                unnameable_detected=residual > PHI_INV,  # >61.8% residual = THE_UNNAMEABLE
                duration_ms=pipeline.elapsed_ms(),
            )
            pipeline.final_judgment = judgment

            # STEP 3: DECIDE + STEP 4: ACT — integrated into cycle
            if self.act_phase_fn is not None:
                action_result = await self.act_phase_fn(judgment, pipeline)
                if action_result:
                    pipeline.action_executed = True
                    pipeline.action_result = action_result
                else:
                    pipeline.action_executed = False

            # STEP 5: LEARN — handled in run() for all cycle levels

            # STEP 6: ACCOUNT — record cost
            pipeline.total_cost_usd = total_cost

            # STEP 7: EMERGE — detect if residual is emergent
            if judgment.unnameable_detected:
                await get_core_bus().emit(Event.typed(
                    CoreEvent.RESIDUAL_HIGH,
                    ResidualHighPayload(
                        cell_id=cell.cell_id,
                        residual_variance=residual,
                        judgment_id=judgment.judgment_id,
                    ),
                ))

            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution("macro_cycle_complete", f"Q={final_q:.1f} verdict={verdict.value}")

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=judgment,
                duration_ms=duration_ms,
                metadata={
                    "cell_id": cell.cell_id,
                    "level": "MACRO",
                    "verdict": verdict.value,
                    "consensus_reached": judgment.consensus_reached,
                },
            )
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_macro", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
