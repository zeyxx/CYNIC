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
            if consensus and consensus.confidence < 0.5:  # Low confidence in consensus
                remaining_budget = cell.budget_usd * (1.0 - PHI_INV_2)  # ~61.8% left
                if remaining_budget > 0.0001:  # $0.1 milli minimum
                    # Check LOD cap (if lod_controller available)
                    capped = ConsciousnessLevel.MACRO
                    if self.lod_controller is not None:
                        # Use the LOD controller's current level, capped at MACRO
                        current_lod = self.lod_controller.current
                        if current_lod.max_consciousness == "MACRO":
                            capped = ConsciousnessLevel.MACRO

                    if capped == ConsciousnessLevel.MACRO:
                        dog_votes = (
                            consensus.evidence.get("dog_votes", 0) if consensus.evidence else 0
                        )
                        logger.info(
                            "L2→L1 escalation: MICRO consensus weak (confidence=%.2f, dogs=%d) for cell %s",
                            consensus.confidence,
                            dog_votes,
                            cell.cell_id,
                        )
                        escalate_to_macro = True

            # Step 3: Fractal Axiom Scoring
            # Each active dog's score is used as input for the axiom architecture
            raw_scores = {j.dog_id: j.q_score for j in pipeline.dog_judgments}

            # Map dog IDs to Axioms they represent (best effort mapping)
            # In a full implementation, this mapping is dynamic
            {
                "FIDELITY": raw_scores.get("ANALYST", 50.0),
                "PHI": raw_scores.get("ARCHITECT", 50.0),
                "VERIFY": raw_scores.get("GUARDIAN", 50.0),
                "CULTURE": raw_scores.get("JANITOR", 50.0),
                "BURN": raw_scores.get("SCOUT", 50.0),
            }

            # Score each core axiom with fractal depth
            # Gather all coroutines and await them in parallel
            axiom_tasks = {
                axiom: self.axiom_arch.score_axiom_fractal(
                    axiom, context=cell.content, depth=1, max_depth=pipeline.fractal_depth
                )
                for axiom in ["FIDELITY", "PHI", "VERIFY", "CULTURE", "BURN"]
            }

            # Await all axiom scoring tasks in parallel
            axiom_scores = {}
            for axiom, task in axiom_tasks.items():
                axiom_scores[axiom] = await task

            # Final Q-Score is the geometric mean of axiom scores (The PHI Law)
            from cynic.kernel.core.phi import geometric_mean

            q_score_micro = geometric_mean(list(axiom_scores.values()))

            active_axioms = self.axiom_arch.active_axioms
            verdict = verdict_from_q_score(q_score_micro)
            total_cost = sum(j.cost_usd for j in pipeline.dog_judgments)

            # Consensus metrics
            consensus_votes = len(pipeline.dog_judgments) if consensus else 0
            consensus_quorum = 7  # Standard quorum for MICRO level
            consensus_reached = (
                consensus is not None and consensus.confidence >= PHI_INV and not escalate_to_macro
            )

            judgment = Judgment(
                cell=cell,
                q_score=q_score_micro,
                verdict=verdict.value,
                confidence=min(PHI_INV, MAX_CONFIDENCE),  # 61.8% at micro
                axiom_scores=axiom_scores,
                active_axioms=active_axioms,
                dog_votes=raw_scores,
                consensus_votes=consensus_votes,
                consensus_quorum=consensus_quorum,
                consensus_reached=consensus_reached,
                cost_usd=total_cost,
                duration_ms=pipeline.elapsed_ms(),
            )

            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution(
                "micro_cycle_complete", f"Q={q_score_micro:.1f} verdict={verdict.value}"
            )

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
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_micro", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )
