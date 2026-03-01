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
from typing import Any, Optional

from cynic.kernel.core.phi import (
    MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_INV_2,
    phi_bound_score, fibonacci,
)
from cynic.kernel.core.axioms import verdict_from_q_score
from cynic.kernel.core.judgment import Cell, Judgment
from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.kernel.core.events_schema import (
    PerceptionReceivedPayload,
    ResidualHighPayload,
)
from cynic.kernel.core.exceptions import CynicError
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
        await get_core_bus().emit(Event.typed(
            CoreEvent.PERCEPTION_RECEIVED,
            PerceptionReceivedPayload(reality=cell.reality, cell_id=cell.cell_id),
        ))
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

        # E-Score filter: skip unreliable Dogs (but keep CYNIC coordinator)
        dog_items = list(orch.dogs.items())
        if orch.escore_tracker is not None:
            GROWL_MIN = PHI_INV_2 * MAX_Q_SCORE  # 38.2
            MIN_ACTIVE = fibonacci(4)  # 3 — safety floor

            passing = [
                (did, d) for did, d in dog_items
                if orch.escore_tracker.get_score(f"agent:{did}") >= GROWL_MIN
                or did == DogId.CYNIC
            ]

            if len(passing) >= MIN_ACTIVE:
                skipped_n = len(dog_items) - len(passing)
                if skipped_n > 0:
                    passing_ids = {did for did, _ in passing}
                    skipped_ids = [did for did, _ in dog_items if did not in passing_ids]
                    logger.info(
                        "JudgeStage: EScore filter bypassing %d/%d Dogs: %s",
                        skipped_n, len(dog_items), skipped_ids,
                    )
                dog_items = passing

        all_dogs = [d for _, d in dog_items]
        per_dog_budget = cell.budget_usd / max(len(all_dogs), 1)

        # R2: Organism context for Dogs (health, LOD, axioms, memory)
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

        # Run all Dogs in parallel
        tasks = [dog.analyze(cell, **organism_kwargs) for dog in all_dogs]
        dog_judgments_raw = await asyncio.gather(*tasks, return_exceptions=True)

        # Filter errors gracefully
        pipeline.dog_judgments = [
            j for j in dog_judgments_raw
            if isinstance(j, DogJudgment)
        ]
        errors = [j for j in dog_judgments_raw if isinstance(j, Exception)]
        if errors:
            logger.warning("JudgeStage: %d Dog(s) failed: %s", len(errors), errors)

        # PBFT Consensus
        consensus = await orch.cynic_dog.pbft_run(cell, pipeline.dog_judgments)
        pipeline.consensus = consensus

        # Axiom scoring
        q_scores = [j.q_score for j in pipeline.dog_judgments]
        avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0
        consensus_strength = (consensus.votes / consensus.quorum) if consensus and consensus.quorum else 0.0

        axiom_result = orch.axiom_arch.score_and_compute(
            domain=cell.reality,
            context=str(cell.content)[:500],
            fractal_depth=3,
            metrics={
                "avg_dog_q": avg_q / MAX_Q_SCORE,
                "consensus_strength": consensus_strength,
            },
        )

        # Final Q-Score: consensus if available, else axiom score
        final_q = consensus.final_q_score or axiom_result.q_score
        final_q = phi_bound_score(final_q)
        verdict = verdict_from_q_score(final_q)

        # Residual variance: unexplained variance between Dog votes
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
            unnameable_detected=residual > PHI_INV,  # >61.8% = THE_UNNAMEABLE
            duration_ms=pipeline.elapsed_ms(),
        )
        pipeline.final_judgment = judgment
        pipeline.total_cost_usd = total_cost

        logger.debug(
            "JudgeStage: judgment complete (Q=%.1f, verdict=%s, residual=%.3f)",
            final_q, verdict.value, residual,
        )
        return pipeline


class DecideStage(JudgmentStage):
    """STEP 3: DECIDE — Governance approval/rejection.

    Validates judgment against policies. May reject high-risk decisions.
    """

    async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
        judgment = pipeline.final_judgment
        if judgment is None:
            logger.warning("DecideStage: no judgment to decide on")
            return pipeline

        orch = self.orchestrator
        if orch.decision_validator is not None:
            # Validate judgment (may block, may pass)
            decision = orch.decision_validator.validate(judgment)
            pipeline.decision = decision
            logger.debug(
                "DecideStage: validation decision=%s (approved=%s)",
                type(decision).__name__, decision.approved if hasattr(decision, 'approved') else "N/A",
            )
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
        # Use existing _act_phase method from orchestrator
        action_result = await orch._act_phase(judgment, pipeline)
        if action_result:
            pipeline.action_executed = True
            pipeline.action_result = action_result
        else:
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
        if hasattr(orch, 'qtable') and orch.qtable is not None:
            orch.qtable.update(signal)
            pipeline.learning_applied = True
            logger.debug(
                "EmpiricalLearnStage: Q-table updated (state=%s, action=%s, reward=%.3f)",
                state_key, signal.action, signal.reward,
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

        orch = self.orchestrator
        # Cost already tracked in pipeline.total_cost_usd
        # E-Score updates happen in orchestrator post-cycle

        logger.debug(
            "AccountStage: cost_usd=%.6f, dog_votes=%d",
            judgment.cost_usd, len(judgment.dog_votes),
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
            await get_core_bus().emit(Event.typed(
                CoreEvent.RESIDUAL_HIGH,
                ResidualHighPayload(
                    cell_id=judgment.cell.cell_id,
                    residual_variance=judgment.residual_variance,
                    judgment_id=judgment.judgment_id,
                ),
            ))
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
    stages: Optional[list[type[JudgmentStage]]] = None,
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
