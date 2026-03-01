"""
Judgment Stages — Modular 7-step CYNIC cycle.

Extracts each step of the judgment pipeline into a pluggable stage class.
This lets Track C override individual stages for empirical testing.

Usage:
    stages = [PerceiveStage, JudgeStage, DecideStage, ActStage, LearnStage, AccountStage, EmergeStage]
    for stage_cls in stages:
        pipeline = await stage_cls(orchestrator).execute(pipeline)
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from typing import Any

from cynic.kernel.core.axioms import verdict_from_q_score
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus
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

# Maximum Dogs to run in parallel
MAX_PARALLEL_DOGS = 11


class JudgmentStage(ABC):
    """Abstract stage in the 7-step judgment cycle.

    Each stage receives a pipeline context, processes it, and returns updated context.
    Stages are composable — one stage's output is the next stage's input.
    """

    def __init__(self, orchestrator: Any) -> None:
        """Initialize stage with orchestrator (gives access to Dogs, managers, etc)."""
        self.orchestrator = orchestrator

    @abstractmethod
    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        """Execute this stage.

        Args:
            pipeline: Current judgment pipeline context

        Returns:
            Updated pipeline context (may be same object or new one)
        """
        ...


class PerceiveStage(JudgmentStage):
    """STEP 1: PERCEIVE — Cell is received.

    Emits perception event to signal cycle start.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        cell = pipeline.cell
        await self.orchestrator.bus.emit(
            Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                PerceptionReceivedPayload(reality=cell.reality, cell_id=cell.cell_id),
            )
        )
        logger.debug("PerceiveStage: %s received (reality=%s)", cell.cell_id[:8], cell.reality)
        return pipeline


class JudgeStage(JudgmentStage):
    """STEP 2: JUDGE — Dogs analyze, PBFT consensus, axiom scoring.

    Dogs are filtered by E-Score reputation. All dogs vote. Consensus reached.
    Axiom architecture scores the judgment. Final Q-score computed.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        cell = pipeline.cell
        orch = self.orchestrator
        level = pipeline.level or ConsciousnessLevel.MACRO

        # REFLEX (L3): non-LLM Dogs only, no full PBFT, no E-Score filtering needed
        if level == ConsciousnessLevel.REFLEX:
            from cynic.kernel.core.consciousness import dogs_for_level
            reflex_dog_ids = dogs_for_level(ConsciousnessLevel.REFLEX)
            active_dogs = [d for did, d in orch.dogs.items() if did in reflex_dog_ids]

            tasks = [dog.analyze(cell, budget_usd=cell.budget_usd) for dog in active_dogs]
            dog_judgments = await asyncio.gather(*tasks, return_exceptions=False)
            pipeline.dog_judgments = dog_judgments

            # Simple majority vote (no full PBFT at L3)
            q_scores = [j.q_score for j in dog_judgments]
            avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0

            # Veto logic
            dog_veto = any(j.veto for j in dog_judgments)
            hard_veto = cell.risk >= 1.0 and cell.analysis == "ACT"
            veto = hard_veto or dog_veto
            final_q = 0.0 if veto else phi_bound_score(avg_q)

            axiom_result = await orch.axiom_arch.score_and_compute(
                domain=cell.reality,
                context=str(cell.content)[:500],
                fractal_depth=1,
                metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
            )

            judgment = Judgment(
                cell=cell,
                q_score=final_q,
                verdict=verdict_from_q_score(final_q).value,
                confidence=min(PHI_INV_2, MAX_CONFIDENCE),
                axiom_scores=axiom_result.axiom_scores,
                active_axioms=list(axiom_result.active_axioms),
                dog_votes={j.dog_id: j.q_score for j in dog_judgments},
                consensus_votes=len(dog_judgments),
                consensus_quorum=3,
                consensus_reached=len(dog_judgments) >= 3,
                cost_usd=sum(j.cost_usd for j in dog_judgments),
                llm_calls=0,
                duration_ms=pipeline.elapsed_ms(),
            )
            pipeline.final_judgment = judgment
            return pipeline

        # MICRO (L2) & MACRO (L1) logic (unified with reputation filtering)
        dog_items = list(orch.dogs.items())
        
        # Filter for MICRO if needed
        if level == ConsciousnessLevel.MICRO:
            from cynic.kernel.core.consciousness import dogs_for_level
            micro_ids = dogs_for_level(ConsciousnessLevel.MICRO)
            dog_items = [(did, d) for did, d in dog_items if did in micro_ids]

        if orch.escore_tracker is not None:
            GROWL_MIN = PHI_INV_2 * MAX_Q_SCORE
            MIN_ACTIVE = fibonacci(4)

            passing = [
                (did, d)
                for did, d in dog_items
                if orch.escore_tracker.get_score(f"agent:{did}") >= GROWL_MIN or did == DogId.CYNIC
            ]

            if len(passing) >= MIN_ACTIVE:
                dog_items = passing

        all_dogs = [d for _, d in dog_items]
        
        # Budget scaling
        effective_budget = cell.budget_usd
        if level == ConsciousnessLevel.MICRO:
            effective_budget *= PHI_INV_2
            
        per_dog_budget = effective_budget / max(len(all_dogs), 1)

        organism_kwargs: dict[str, Any] = {
            "budget_usd": per_dog_budget,
            "active_dogs": len(all_dogs),
        }
        if orch.lod_controller is not None:
            organism_kwargs["lod_level"] = int(orch.lod_controller.current)
        if orch.axiom_monitor is not None:
            organism_kwargs["active_axioms"] = orch.axiom_monitor.active_count()
        if orch.context_compressor is not None:
            compressed = orch.context_compressor.get_compressed_context(budget=200)
            if compressed:
                organism_kwargs["compressed_context"] = compressed

        tasks = [dog.analyze(cell, **organism_kwargs) for dog in all_dogs]
        dog_judgments_raw = await asyncio.gather(*tasks, return_exceptions=True)

        pipeline.dog_judgments = [j for j in dog_judgments_raw if isinstance(j, DogJudgment)]
        
        # phi-BFT Consensus
        consensus = await orch.cynic_dog.phi_bft_run(cell, pipeline.dog_judgments)
        pipeline.consensus = consensus

        # Axiom scoring
        depth = 3 if level == ConsciousnessLevel.MACRO else 2
        q_scores = [j.q_score for j in pipeline.dog_judgments]
        avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0
        
        axiom_result = await orch.axiom_arch.score_and_compute(
            domain=cell.reality,
            context=str(cell.content)[:500],
            fractal_depth=depth,
            metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
        )

        final_q = consensus.final_q_score or axiom_result.q_score
        final_q = phi_bound_score(final_q)
        
        # Residual variance
        if pipeline.dog_judgments:
            votes = [j.q_score for j in pipeline.dog_judgments]
            mean_v = sum(votes) / len(votes)
            variance = sum((v - mean_v) ** 2 for v in votes) / len(votes)
            residual = min(variance / (MAX_Q_SCORE**2), 1.0)
        else:
            residual = 0.0

        judgment = Judgment(
            cell=cell,
            q_score=final_q,
            verdict=verdict_from_q_score(final_q).value,
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
            cost_usd=sum(j.cost_usd for j in pipeline.dog_judgments),
            llm_calls=sum(1 for j in pipeline.dog_judgments if j.llm_id),
            residual_variance=residual,
            unnameable_detected=residual > PHI_INV,
            duration_ms=pipeline.elapsed_ms(),
        )
        pipeline.final_judgment = judgment
        pipeline.total_cost_usd = judgment.cost_usd
        return pipeline


class DecideStage(JudgmentStage):
    """STEP 3: DECIDE — Governance approval/rejection.

    Validates judgment against policies. May reject high-risk decisions.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            return pipeline

        # Skip DECIDE for REFLEX and MICRO (not actionable by default)
        level = pipeline.level or ConsciousnessLevel.MACRO
        if level in (ConsciousnessLevel.REFLEX, ConsciousnessLevel.MICRO):
            return pipeline

        orch = self.orchestrator
        if orch.decision_validator is not None:
            decision = orch.decision_validator.validate(judgment)
            pipeline.decision = decision
        return pipeline


class ActStage(JudgmentStage):
    """STEP 4: ACT — Execute approved actions.

    Calls act_phase if judgment approved. Records action result.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            return pipeline

        orch = self.orchestrator
        
        # We need to execute the action if one is recommended and approved.
        # Instead of calling private orch._act_phase, we'll implement the 
        # delegator logic here or ensure the orchestrator has the method.
        
        if hasattr(orch, "_act_phase"):
            action_result = await orch._act_phase(judgment, pipeline)
            if action_result:
                pipeline.action_executed = True
                pipeline.action_result = action_result
            else:
                pipeline.action_executed = False
        else:
            # Fallback for handlers acting as orchestrator context
            logger.debug("ActStage: context has no _act_phase implementation")
            pipeline.action_executed = False

        logger.debug("ActStage: action_executed=%s", pipeline.action_executed)
        return pipeline


class LearnStage(JudgmentStage):
    """STEP 5: LEARN — Update Q-Table, Thompson, EWC.

    Updates learning models from judgment outcome.
    (Actual learning logic is in orchestrator.run() post-cycle.)
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        # Learning happens in orchestrator.run() after cycle complete
        # This stage is a placeholder for future empirical learning hooks
        logger.debug("LearnStage: placeholder (learning done in orchestrator.run)")
        return pipeline


class EmpiricalLearnStage(JudgmentStage):
    """STEP 5: LEARN (empirical) — Directly inject Q-table learning signal.

    Used by Track C to test learning pipeline without waiting for orchestrator.run().
    Injects LearningSignal directly into orchestrator.qtable.

    This is the concrete learning implementation for synthetic testing.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            return pipeline

        orch = self.orchestrator

        # Build state key from cell (same format as real orchestrator)
        # Format: "{reality}:{analysis}:PRESENT:0" (LOD = 0 for MACRO)
        state_key = f"{judgment.cell.reality}:{judgment.cell.analysis}:PRESENT:0"

        # Construct learning signal
        from cynic.kernel.organism.brain.learning.qlearning import LearningSignal

        signal = LearningSignal(
            state_key=state_key,
            action=judgment.verdict.value,  # "BARK", "GROWL", "WAG", "HOWL"
            reward=judgment.q_score / MAX_Q_SCORE,  # Normalize to [0, 1]
            judgment_id=judgment.judgment_id,
            loop_name="EMPIRICAL",
        )

        # Update Q-table if available
        if hasattr(orch, "qtable") and orch.qtable is not None:
            orch.qtable.update(signal)
            pipeline.learning_applied = True
            logger.debug(
                "EmpiricalLearnStage: Q-table updated (state=%s, action=%s, reward=%.3f)",
                state_key,
                signal.action,
                signal.reward,
            )
        else:
            logger.warning("EmpiricalLearnStage: orchestrator has no qtable")

        return pipeline


class AccountStage(JudgmentStage):
    """STEP 6: ACCOUNT — Record cost, E-Score update.

    Tracks cost by reality/dog. Updates reputation scores.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            return pipeline

        # Cost already tracked in pipeline.total_cost_usd
        # E-Score updates happen in orchestrator post-cycle

        logger.debug(
            "AccountStage: cost_usd=%.6f, dog_votes=%d",
            judgment.cost_usd,
            len(judgment.dog_votes),
        )
        return pipeline


class EmergeStage(JudgmentStage):
    """STEP 7: EMERGE — Detect patterns, residual, emergence.

    If residual high (>φ⁻¹), emit emergence signal.
    Feeds into ResidualDetector and axiom evolution.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            return pipeline

        # If residual is high, emit emergence signal
        if judgment.unnameable_detected:
            await self.orchestrator.bus.emit(
                Event.typed(
                    CoreEvent.RESIDUAL_HIGH,
                    ResidualHighPayload(
                        cell_id=judgment.cell.cell_id,
                        residual_variance=judgment.residual_variance,
                        judgment_id=judgment.judgment_id,
                    ),
                )
            )
            logger.info(
                "EmergeStage: high residual detected (%.3f > φ⁻¹)",
                judgment.residual_variance,
            )

        return pipeline


# ════════════════════════════════════════════════════════════════════════════
# Pipeline Executor — Chains all 7 stages
# ════════════════════════════════════════════════════════════════════════════


async def execute_judgment_pipeline(
    orchestrator: Any,
    pipeline: JudgmentPipeline,
    stages: list[type[JudgmentStage]] | None = None,
) -> JudgmentPipeline:
    """Execute complete 7-step judgment pipeline.

    Args:
        orchestrator: JudgeOrchestrator instance
        pipeline: JudgmentPipeline context
        stages: List of stage classes to execute (default: all 7)

    Returns:
        Final pipeline context with all results
    """
    if stages is None:
        stages = [
            PerceiveStage,
            JudgeStage,
            DecideStage,
            ActStage,
            LearnStage,
            AccountStage,
            EmergeStage,
        ]

    for stage_cls in stages:
        stage = stage_cls(orchestrator)
        pipeline = await stage.execute(pipeline)

    return pipeline
