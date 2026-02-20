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
from typing import Any, Optional, TYPE_CHECKING

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.judgment import Cell, Judgment
from cynic.core.consciousness import ConsciousnessLevel, dogs_for_level
from cynic.core.phi import phi_bound_score, PHI_INV_2, MAX_CONFIDENCE, MAX_Q_SCORE
from cynic.core.axioms import verdict_from_q_score

if TYPE_CHECKING:
    from cynic.cognition.cortex.orchestrator import JudgmentPipeline

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

    async def execute(self, pipeline: JudgmentPipeline, **kwargs: Any) -> HandlerResult:
        """
        Execute REFLEX cycle for a cell.

        Args:
            pipeline: JudgmentPipeline with cell and context

        Returns:
            HandlerResult with Judgment in output
        """
        t0 = time.perf_counter()
        try:
            cell = pipeline.cell

            # Get reflex dogs only: GUARDIAN + ANALYST + JANITOR + CYNIC
            reflex_dog_ids = dogs_for_level(ConsciousnessLevel.REFLEX)
            active_dogs = [d for did, d in self.dogs.items() if did in reflex_dog_ids]

            # Run all reflex Dogs in parallel
            tasks = [dog.analyze(cell, budget_usd=cell.budget_usd) for dog in active_dogs]
            dog_judgments = await asyncio.gather(*tasks, return_exceptions=False)
            pipeline.dog_judgments = dog_judgments

            # Simple majority vote (no full PBFT at L3)
            q_scores = [j.q_score for j in dog_judgments]
            avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0

            # GUARDIAN veto: any dog can force Q=0 (immune system override)
            dog_veto = any(j.veto for j in dog_judgments)

            # Hard veto: cell explicitly declared as risk=1.0 + analysis=ACT
            hard_veto = cell.risk >= 1.0 and cell.analysis == "ACT"

            veto = hard_veto or dog_veto

            # At REFLEX level, dog heuristics ARE the score
            final_q = 0.0 if veto else phi_bound_score(avg_q)

            # Axiom scoring for active_axioms tracking and emergent activation
            axiom_result = self.axiom_arch.score_and_compute(
                domain=cell.reality,
                context=str(cell.content)[:500],
                fractal_depth=1,
                metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
            )

            verdict = verdict_from_q_score(final_q)
            total_cost = sum(j.cost_usd for j in dog_judgments)

            judgment = Judgment(
                cell=cell,
                q_score=final_q,
                verdict=verdict.value,
                confidence=min(PHI_INV_2, MAX_CONFIDENCE),  # 38.2% — low confidence at reflex
                axiom_scores=axiom_result.axiom_scores,
                active_axioms=list(axiom_result.active_axioms),
                dog_votes={j.dog_id: j.q_score for j in dog_judgments},
                consensus_votes=len(dog_judgments),
                consensus_quorum=3,  # lower bar at L3
                consensus_reached=len(dog_judgments) >= 3,
                cost_usd=total_cost,
                llm_calls=0,
                duration_ms=pipeline.elapsed_ms(),
            )

            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_execution("reflex_cycle_complete", f"Q={final_q:.1f} verdict={verdict.value}")

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=judgment,
                duration_ms=duration_ms,
                metadata={"cell_id": cell.cell_id, "level": "REFLEX", "verdict": verdict.value},
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
