"""
Judgment Stages — Modular 7-step CYNIC cycle (DAG Stages).

Each stage is a pure transition that returns a NEW evolved pipeline instance.
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from typing import Any

from cynic.kernel.core.axioms import verdict_from_q_score
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.events_schema import (
    PerceptionReceivedPayload,
    ResidualHighPayload,
)
from cynic.kernel.core.judgment import Judgment
from cynic.kernel.core.phi import (
    MAX_CONFIDENCE,
    MAX_Q_SCORE,
    PHI_INV,
    PHI_INV_2,
    fibonacci,
    phi_bound_score,
)
from cynic.kernel.organism.brain.cognition.cortex.pipeline import JudgmentPipeline
from cynic.kernel.organism.brain.cognition.neurons.base import DogId, DogJudgment

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.stages")


class JudgmentStage(ABC):
    """Abstract stage in the 7-step judgment cycle (Functional pattern)."""

    def __init__(self, orchestrator: Any) -> None:
        self.orchestrator = orchestrator

    @abstractmethod
    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        """Process and return a NEW evolved pipeline context."""
        ...


class PerceiveStage(JudgmentStage):
    """STEP 1: PERCEIVE — Signal cycle start."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        cell = pipeline.cell
        await self.orchestrator.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                PerceptionReceivedPayload(reality=cell.reality, cell_id=cell.cell_id),
            )
        )
        logger.debug(f"[{pipeline.trace_id}] Stage 1: Perceive (reality={cell.reality})")
        return pipeline.evolve()


class JudgeStage(JudgmentStage):
    """STEP 2: JUDGE — Dogs analysis & consensus."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        cell = pipeline.cell
        orch = self.orchestrator
        level = pipeline.level

        # Filter Dogs based on level and E-Score
        dog_items = list(orch.dogs.items())
        from cynic.kernel.core.consciousness import dogs_for_level
        active_ids = dogs_for_level(level)
        dog_items = [(did, d) for did, d in dog_items if did in active_ids]

        if orch.escore_tracker is not None:
            GROWL_MIN = PHI_INV_2 * MAX_Q_SCORE
            dog_items = [
                (did, d) for did, d in dog_items 
                if orch.escore_tracker.get_score(f"agent:{did}") >= GROWL_MIN or did == DogId.CYNIC
            ]

        all_dogs = [d for _, d in dog_items]
        effective_budget = cell.budget_usd * (PHI_INV_2 if level == ConsciousnessLevel.MICRO else 1.0)
        per_dog_budget = effective_budget / max(len(all_dogs), 1)

        tasks = [dog.analyze(cell, budget_usd=per_dog_budget) for dog in all_dogs]
        results = await asyncio.gather(*tasks, return_exceptions=True)
        valid_judgments = [j for j in results if isinstance(j, DogJudgment)]

        # Consensus & Axioms
        consensus = await orch.cynic_dog.phi_bft_run(cell, valid_judgments)
        
        q_scores = [j.q_score for j in valid_judgments]
        avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0
        depth = 1 if level == ConsciousnessLevel.REFLEX else (2 if level == ConsciousnessLevel.MICRO else 3)
        
        axiom_result = await orch.axiom_arch.score_and_compute(
            domain=cell.reality,
            context=str(cell.content)[:500],
            fractal_depth=depth,
            metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
        )

        final_q = phi_bound_score(consensus.final_q_score or axiom_result.q_score)
        
        judgment = Judgment(
            cell=cell,
            q_score=final_q,
            verdict=verdict_from_q_score(final_q).value,
            confidence=min(consensus.final_confidence or PHI_INV_2, MAX_CONFIDENCE),
            axiom_scores=axiom_result.axiom_scores,
            active_axioms=list(axiom_result.active_axioms),
            dog_votes={j.dog_id: j.q_score for j in valid_judgments},
            consensus_votes=consensus.votes,
            consensus_quorum=consensus.quorum,
            consensus_reached=consensus.consensus,
            cost_usd=sum(j.cost_usd for j in valid_judgments),
            duration_ms=pipeline.elapsed_ms(),
        )

        logger.debug(f"[{pipeline.trace_id}] Stage 2: Judge (Q={final_q:.1f} Verdict={judgment.verdict})")
        return pipeline.evolve(
            dog_judgments=tuple(valid_judgments),
            consensus=consensus,
            final_judgment=judgment,
            total_cost_usd=judgment.cost_usd
        )


class DecideStage(JudgmentStage):
    """STEP 3: DECIDE — Governance validation."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        if pipeline.final_judgment is None or pipeline.level in (ConsciousnessLevel.REFLEX, ConsciousnessLevel.MICRO):
            return pipeline.evolve()

        if self.orchestrator.decision_validator:
            decision = self.orchestrator.decision_validator.validate(pipeline.final_judgment)
            logger.debug(f"[{pipeline.trace_id}] Stage 3: Decide (Approved={decision.approved})")
            return pipeline.evolve(decision=decision)
        
        return pipeline.evolve()


class ActStage(JudgmentStage):
    """STEP 4: ACT — Effector execution."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        if pipeline.final_judgment is None:
            return pipeline

        orch = self.orchestrator
        if hasattr(orch, "_act_phase"):
            result = await orch._act_phase(pipeline.final_judgment, pipeline)
            logger.debug(f"[{pipeline.trace_id}] Stage 4: Act (Executed={result is not None})")
            return pipeline.evolve(action_executed=result is not None, action_result=result)
        
        return pipeline.evolve()


class LearnStage(JudgmentStage):
    """STEP 5: LEARN — Placeholder for future hooks."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        # Real learning is in Orchestrator.run post-cycle for now
        return pipeline.evolve()


class AccountStage(JudgmentStage):
    """STEP 6: ACCOUNT — Cost tracking."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        # pipeline.total_cost_usd is already updated in JudgeStage
        return pipeline.evolve()


class EmergeStage(JudgmentStage):
    """STEP 7: EMERGE — Detect patterns."""

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        if pipeline.final_judgment and pipeline.final_judgment.unnameable_detected:
            await self.orchestrator.bus.emit(
                Event.typed(
                    CoreEvent.RESIDUAL_HIGH,
                    ResidualHighPayload(
                        cell_id=pipeline.cell.cell_id,
                        residual_variance=pipeline.final_judgment.residual_variance,
                        judgment_id=pipeline.final_judgment.judgment_id,
                    ),
                )
            )
            logger.info(f"[{pipeline.trace_id}] Stage 7: Emerge (THE_UNNAMEABLE detected)")
        return pipeline.evolve()


async def execute_judgment_pipeline(
    orchestrator: Any,
    pipeline: JudgmentPipeline,
    stages: list[type[JudgmentStage]] | None = None,
) -> JudgmentPipeline:
    """Execute complete 7-step judgment pipeline DAG."""
    if stages is None:
        stages = [
            PerceiveStage, JudgeStage, DecideStage, 
            ActStage, LearnStage, AccountStage, EmergeStage
        ]

    current = pipeline
    for stage_cls in stages:
        stage = stage_cls(orchestrator)
        current = await stage.execute(current)

    return current
