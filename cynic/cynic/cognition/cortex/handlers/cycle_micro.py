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
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Cell, Judgment, JudgmentPipeline
from cynic.core.consciousness import ConsciousnessLevel, dogs_for_level
from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_CONFIDENCE, MAX_Q_SCORE
from cynic.core.axioms import verdict_from_q_score

logger = logging.getLogger("cynic.cognition.cortex.handlers.cycle_micro")


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
        cynic_dog: Optional[Any] = None,
        lod_controller: Optional[Any] = None,
    ) -> None:
        self.dogs = dogs
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.lod_controller = lod_controller

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute MICRO cycle for a cell.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
            metadata['escalate_to'] = 'cycle_macro' if consensus fails and escalation needed
        """
        t0 = time.perf_counter()
        try:
            cell = pipeline.cell

            # Get all MICRO dogs
            micro_dog_ids = dogs_for_level(ConsciousnessLevel.MICRO)
            active_dogs = [d for did, d in self.dogs.items() if did in micro_dog_ids]

            # Reduced budget: 38.2% of total
            micro_budget = cell.budget_usd * PHI_INV_2
            per_dog_budget = micro_budget / max(len(active_dogs), 1)

            # Run all MICRO dogs in parallel
            tasks = [dog.analyze(cell, budget_usd=per_dog_budget) for dog in active_dogs]
            dog_judgments = await asyncio.gather(*tasks, return_exceptions=False)
            pipeline.dog_judgments = list(dog_judgments)

            # PBFT consensus (only if cynic_dog available)
            consensus = None
            if self.cynic_dog is not None:
                consensus = await self.cynic_dog.pbft_run(cell, pipeline.dog_judgments)
                pipeline.consensus = consensus

            # Check if escalation needed
            escalate_to_macro = False
            if consensus and not consensus.consensus:
                remaining_budget = cell.budget_usd * (1.0 - PHI_INV_2)  # ~61.8% left
                if remaining_budget > 0.0001:  # $0.1 milli minimum
                    # Check LOD cap (if lod_controller available)
                    capped = ConsciousnessLevel.MACRO
                    if self.lod_controller is not None:
                        capped = self.lod_controller._apply_lod_cap(ConsciousnessLevel.MACRO)

                    if capped == ConsciousnessLevel.MACRO:
                        logger.info(
                            "L2→L1 escalation: MICRO consensus failed (%d/%d votes) for cell %s",
                            consensus.votes, consensus.quorum, cell.cell_id,
                        )
                        escalate_to_macro = True

            # Axiom scoring at medium depth
            q_scores_micro = [j.q_score for j in pipeline.dog_judgments]
            avg_q_micro = sum(q_scores_micro) / len(q_scores_micro) if q_scores_micro else 0.0
            axiom_result = self.axiom_arch.score_and_compute(
                domain=cell.reality,
                context=str(cell.content)[:500],
                fractal_depth=2,
                metrics={"avg_dog_q": avg_q_micro / MAX_Q_SCORE},
            )

            verdict = verdict_from_q_score(axiom_result.q_score)
            total_cost = sum(j.cost_usd for j in pipeline.dog_judgments)

            judgment = Judgment(
                cell=cell,
                q_score=axiom_result.q_score,
                verdict=verdict.value,
                confidence=min(PHI_INV, MAX_CONFIDENCE),  # 61.8% at micro
                axiom_scores=axiom_result.axiom_scores,
                active_axioms=list(axiom_result.active_axioms),
                dog_votes={j.dog_id: j.q_score for j in pipeline.dog_judgments},
                consensus_votes=consensus.votes if consensus else 0,
                consensus_quorum=consensus.quorum if consensus else 7,
                consensus_reached=consensus.consensus if consensus else False,
                cost_usd=total_cost,
                duration_ms=pipeline.elapsed_ms(),
            )

            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution("micro_cycle_complete", f"Q={axiom_result.q_score:.1f} verdict={verdict.value}")

            metadata = {
                "cell_id": cell.cell_id,
                "level": "MICRO",
                "verdict": verdict.value,
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
            self._log_error("execute_micro", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
