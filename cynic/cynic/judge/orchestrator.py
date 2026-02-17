"""
CYNIC Judge Orchestrator — Full 7-Step Cycle

Orchestrates the complete judgment pipeline:
  1. PERCEIVE  → Cell is received (perception event)
  2. JUDGE     → Dogs analyze, PBFT consensus
  3. DECIDE    → Governance approval/rejection
  4. ACT       → Execute approved actions
  5. LEARN     → Update Q-Table, Thompson, EWC
  6. ACCOUNT   → Record cost, E-Score update
  7. EMERGE    → Detect patterns, residual, emergence

Consciousness levels dictate which path:
  L3 REFLEX → skip to JUDGE with non-LLM Dogs only
  L2 MICRO  → JUDGE with voting Dogs, skip ACT
  L1 MACRO  → Full 7-step cycle
  L4 META   → Full cycle + organism evolution

This is the HEART of CYNIC. Every judgment flows through here.
"""
from __future__ import annotations

import logging
import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.core.phi import (
    MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_INV_2,
    phi_bound_score, LEARNING_RATE, fibonacci,
)
from cynic.core.consciousness import (
    ConsciousnessLevel, ConsciousnessState, get_consciousness, dogs_for_level,
)
from cynic.core.judgment import Cell, Judgment, ConsensusResult
from cynic.core.axioms import AxiomArchitecture, Verdict, verdict_from_q_score
from cynic.core.event_bus import (
    get_core_bus, Event, CoreEvent,
)
from cynic.dogs.base import AbstractDog, DogJudgment, DogId
from cynic.dogs.cynic_dog import CynicDog

logger = logging.getLogger("cynic.judge")

# Maximum Dogs to run in parallel at L1 MACRO (F(7)=13 > 11, so all 11)
MAX_PARALLEL_DOGS = 11


@dataclass
class JudgmentPipeline:
    """
    Context for one complete judgment pipeline execution.

    Tracks timing, cost, and intermediate results for all 7 steps.
    """
    cell: Cell
    level: ConsciousnessLevel = ConsciousnessLevel.MACRO
    pipeline_id: str = field(default_factory=lambda: str(uuid.uuid4()))
    started_at: float = field(default_factory=time.time)

    # Step results
    dog_judgments: List[DogJudgment] = field(default_factory=list)
    consensus: Optional[ConsensusResult] = None
    final_judgment: Optional[Judgment] = None

    # Costs
    total_cost_usd: float = 0.0
    total_latency_ms: float = 0.0

    def elapsed_ms(self) -> float:
        return (time.time() - self.started_at) * 1000


class JudgeOrchestrator:
    """
    The Judge Orchestrator — runs the 7-step CYNIC cycle.

    Usage:
      orchestrator = JudgeOrchestrator(dogs, axiom_arch, cynic_dog)
      judgment = await orchestrator.run(cell, level=ConsciousnessLevel.MACRO)
    """

    def __init__(
        self,
        dogs: Dict[str, AbstractDog],
        axiom_arch: AxiomArchitecture,
        cynic_dog: CynicDog,
    ) -> None:
        self.dogs = dogs        # {dog_id: AbstractDog}
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self._judgment_count = 0
        self._consciousness = get_consciousness()

    # ── STEP 0: Entry Point ────────────────────────────────────────────────

    async def run(
        self,
        cell: Cell,
        level: Optional[ConsciousnessLevel] = None,
        budget_usd: Optional[float] = None,
    ) -> Judgment:
        """
        Run the complete judgment cycle for a Cell.

        Level auto-selected if None (based on budget and timer health).
        """
        level = level or self._select_level(cell, budget_usd or cell.budget_usd)
        pipeline = JudgmentPipeline(cell=cell, level=level)

        # Emit JUDGMENT_REQUESTED
        await get_core_bus().emit(Event(
            type=CoreEvent.JUDGMENT_REQUESTED,
            payload={"cell_id": cell.cell_id, "reality": cell.reality, "level": level.name},
        ))

        timer = self._consciousness.timers.get(level.name)
        if timer:
            timer.start()

        try:
            # Route to appropriate cycle based on consciousness level
            if level == ConsciousnessLevel.REFLEX:
                judgment = await self._cycle_reflex(pipeline)
            elif level == ConsciousnessLevel.MICRO:
                judgment = await self._cycle_micro(pipeline)
            else:  # MACRO (L1) — full 7-step cycle
                judgment = await self._cycle_macro(pipeline)

            if timer:
                elapsed = timer.stop()
                pipeline.total_latency_ms = elapsed

            # Emit JUDGMENT_CREATED
            self._judgment_count += 1
            self._consciousness.increment(level)
            await get_core_bus().emit(Event(
                type=CoreEvent.JUDGMENT_CREATED,
                payload=judgment.to_dict(),
            ))

            # Emit LEARNING_EVENT for ALL cycles (REFLEX/MICRO/MACRO).
            # Was incorrectly placed inside _cycle_macro only — Q-Learning never fired.
            await get_core_bus().emit(Event(
                type=CoreEvent.LEARNING_EVENT,
                payload={
                    "judgment_id": judgment.judgment_id,
                    "state_key": cell.state_key(),
                    "action": judgment.verdict,
                    "reward": judgment.q_score / MAX_Q_SCORE,
                    "loop_name": "JUDGE_ORCHESTRATOR",
                },
            ))

            # STEP 5 (LEARN): Feed Scholar its outcome — builds similarity memory.
            # ScholarDog.learn() is separate from analyze() to avoid feedback contamination.
            scholar = self.dogs.get(DogId.SCHOLAR)  # type: ignore[assignment]
            if scholar is not None:
                cell_text = cell.content or cell.state_key()
                scholar.learn(
                    cell_text=cell_text,
                    q_score=judgment.q_score,
                    cell_id=cell.cell_id,
                    reality=cell.reality,
                )

            return judgment

        except Exception as e:
            logger.error("Judgment pipeline failed: %s", e, exc_info=True)
            if timer:
                timer.stop()
            await get_core_bus().emit(Event(
                type=CoreEvent.JUDGMENT_FAILED,
                payload={"cell_id": cell.cell_id, "error": str(e)},
            ))
            raise

    # ── LEVEL SELECTION ────────────────────────────────────────────────────

    def _select_level(self, cell: Cell, budget_usd: float) -> ConsciousnessLevel:
        """Auto-select consciousness level based on budget and cell metadata."""
        suggested = self._consciousness.should_downgrade(budget_usd)
        if suggested:
            return suggested

        # Use cell's own consciousness gradient to guide level selection
        if cell.consciousness <= 1:
            return ConsciousnessLevel.REFLEX
        elif cell.consciousness <= 3:
            return ConsciousnessLevel.MICRO
        else:
            return ConsciousnessLevel.MACRO

    # ── L3 REFLEX CYCLE (<10ms, non-LLM only) ─────────────────────────────

    async def _cycle_reflex(self, pipeline: JudgmentPipeline) -> Judgment:
        """
        L3 REFLEX cycle: non-LLM Dogs only, no consensus, fast path.

        Uses GUARDIAN + ANALYST + JANITOR + CYNIC(PBFT coordinator).
        No full PBFT — just independent votes, majority wins.
        """
        cell = pipeline.cell
        reflex_dog_ids = dogs_for_level(ConsciousnessLevel.REFLEX)
        active_dogs = [d for did, d in self.dogs.items() if did in reflex_dog_ids]

        # Run all reflex Dogs in parallel
        import asyncio
        tasks = [dog.analyze(cell, budget_usd=cell.budget_usd) for dog in active_dogs]
        dog_judgments: List[DogJudgment] = await asyncio.gather(*tasks, return_exceptions=False)
        pipeline.dog_judgments = dog_judgments

        # Simple majority vote (no full PBFT at L3)
        q_scores = [j.q_score for j in dog_judgments]
        avg_q = sum(q_scores) / len(q_scores) if q_scores else 0.0

        # Axiom scoring (fast-path: only active core axioms)
        axiom_result = self.axiom_arch.score_and_compute(
            domain=cell.reality,
            context=str(cell.content)[:500],   # truncate for speed
            fractal_depth=1,
            metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
        )

        verdict = verdict_from_q_score(axiom_result.q_score)
        total_cost = sum(j.cost_usd for j in dog_judgments)

        return Judgment(
            cell=cell,
            q_score=axiom_result.q_score,
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

    # ── L2 MICRO CYCLE (~500ms, Dog voting) ───────────────────────────────

    async def _cycle_micro(self, pipeline: JudgmentPipeline) -> Judgment:
        """
        L2 MICRO cycle: all available Dogs vote, fast PBFT.

        Includes LLM Dogs at reduced budget. No ACT phase.
        """
        import asyncio
        cell = pipeline.cell
        micro_dog_ids = dogs_for_level(ConsciousnessLevel.MICRO)
        active_dogs = [d for did, d in self.dogs.items() if did in micro_dog_ids]

        micro_budget = cell.budget_usd * PHI_INV_2  # 38.2% of total budget

        tasks = [
            dog.analyze(cell, budget_usd=micro_budget / max(len(active_dogs), 1))
            for dog in active_dogs
        ]
        dog_judgments = await asyncio.gather(*tasks, return_exceptions=False)
        pipeline.dog_judgments = list(dog_judgments)

        # PBFT consensus
        consensus = await self.cynic_dog.pbft_run(cell, pipeline.dog_judgments)
        pipeline.consensus = consensus

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

        return Judgment(
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

    # ── L1 MACRO CYCLE (~2.85s, full 7-step) ──────────────────────────────

    async def _cycle_macro(self, pipeline: JudgmentPipeline) -> Judgment:
        """
        L1 MACRO cycle: full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE.

        This is the canonical CYNIC judgment cycle.
        """
        import asyncio
        cell = pipeline.cell

        # STEP 1: PERCEIVE (already done — cell is the perception result)
        await get_core_bus().emit(Event(
            type=CoreEvent.PERCEPTION_RECEIVED,
            payload={"cell_id": cell.cell_id, "reality": cell.reality},
        ))

        # STEP 2: JUDGE — all 11 Dogs analyze in parallel
        all_dogs = list(self.dogs.values())
        per_dog_budget = cell.budget_usd / max(len(all_dogs), 1)
        tasks = [dog.analyze(cell, budget_usd=per_dog_budget) for dog in all_dogs]
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

        # STEP 5: LEARN — handled in run() for all cycle levels

        # STEP 6: ACCOUNT — emit cost event
        pipeline.total_cost_usd = total_cost

        # STEP 7: EMERGE — detect if residual is emergent
        if judgment.unnameable_detected:
            await get_core_bus().emit(Event(
                type=CoreEvent.RESIDUAL_HIGH,
                payload={
                    "cell_id": cell.cell_id,
                    "residual_variance": residual,
                    "judgment_id": judgment.judgment_id,
                },
            ))

        return judgment

    def stats(self) -> Dict[str, Any]:
        return {
            "judgments_total": self._judgment_count,
            "dogs_active": len(self.dogs),
            "consciousness": self._consciousness.to_dict(),
        }
