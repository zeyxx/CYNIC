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
    MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_INV_2, PHI,
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
from cynic.judge.circuit_breaker import CircuitBreaker, CircuitState

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
        residual_detector=None,
    ) -> None:
        self.dogs = dogs        # {dog_id: AbstractDog}
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.residual_detector = residual_detector  # Optional[ResidualDetector]
        self.benchmark_registry = None  # Optional[BenchmarkRegistry] — set via state.py
        self.escore_tracker = None  # Optional[EScoreTracker] — injected via state.py
        self.axiom_monitor = None  # Optional[AxiomMonitor] — γ3: axiom health → budget multiplier
        self._judgment_count = 0
        self._consciousness = get_consciousness()
        # evolve() history — last F(8)=21 META cycles
        self._evolve_history: List[Dict[str, Any]] = []
        # Circuit breaker — prevents cascade failures (topology M1)
        self._circuit_breaker = CircuitBreaker()

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
        # γ3: scale budget by axiom health before level selection
        effective_budget = (budget_usd or cell.budget_usd) * self._axiom_budget_multiplier()
        level = level or self._select_level(cell, effective_budget)
        pipeline = JudgmentPipeline(cell=cell, level=level)

        # Circuit breaker — fast-fail when cascade failure detected (topology M1)
        if not self._circuit_breaker.allow():
            cb = self._circuit_breaker
            logger.warning(
                "CircuitBreaker %s — fast-failing judgment %s (failures=%d)",
                cb.state.value, cell.cell_id, cb.failure_count,
            )
            await get_core_bus().emit(Event(
                type=CoreEvent.JUDGMENT_FAILED,
                payload={
                    "cell_id": cell.cell_id,
                    "error": "circuit_open",
                    "circuit_state": cb.state.value,
                    "failure_count": cb.failure_count,
                },
            ))
            raise RuntimeError(
                f"CircuitBreaker OPEN — pipeline suspended "
                f"({cb.failure_count} consecutive failures)"
            )

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

            # Emit JUDGMENT_CREATED (enriched with cell context for DecideAgent)
            self._judgment_count += 1
            self._consciousness.increment(level)
            jc_payload = judgment.to_dict()
            jc_payload["state_key"] = cell.state_key()
            jc_payload["reality"] = cell.reality   # needed by guidance.json writer
            jc_payload["content_preview"] = str(cell.content or "")[:200]
            jc_payload["context"] = cell.context or ""
            await get_core_bus().emit(Event(
                type=CoreEvent.JUDGMENT_CREATED,
                payload=jc_payload,
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

            # STEP 7 (EMERGE): ResidualDetector observes judgment variance.
            # Synchronous observe() is authoritative; async event handler is secondary.
            if self.residual_detector is not None:
                self.residual_detector.observe(judgment)

            # Circuit breaker: successful judgment — reset failure counter
            self._circuit_breaker.record_success()

            return judgment

        except Exception as e:
            logger.error("Judgment pipeline failed: %s", e, exc_info=True)
            if timer:
                timer.stop()
            # Circuit breaker: record failure — may open circuit after threshold
            self._circuit_breaker.record_failure()
            await get_core_bus().emit(Event(
                type=CoreEvent.JUDGMENT_FAILED,
                payload={"cell_id": cell.cell_id, "error": str(e)},
            ))
            raise

    # ── γ3: AXIOM → BUDGET MULTIPLIER ─────────────────────────────────────

    def _axiom_budget_multiplier(self) -> float:
        """
        Compute budget multiplier from emergent axiom health (γ3 loop).

        Active axioms signal a healthy, coordinated organism — it can afford
        deeper judgment (MACRO). Dormant axioms signal stress — conserve budget.

        Multiplier table (φ-derived):
            0 active axioms → PHI_INV_2 = 0.382  (stressed  → REFLEX/MICRO)
            1 active axiom  → PHI_INV   = 0.618  (stirring  → MICRO)
            2 active axioms → 1.0                (balanced  → MACRO)
            3 active axioms → PHI       = 1.618  (healthy   → deeper MACRO)
            4 active axioms → PHI²      = 2.618  (peak      → max depth)

        Formula: PHI ** (active_count - 2)  →  range [0.382, 2.618]
        """
        if self.axiom_monitor is None:
            return 1.0
        active = self.axiom_monitor.active_count()
        return PHI ** (active - 2)

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

        # GUARDIAN veto: any dog can force Q=0 (immune system override)
        dog_veto = any(j.veto for j in dog_judgments)

        # Hard veto: a Cell explicitly declared as risk=1.0 + analysis=ACT
        # is absolutely dangerous by construction — no ML needed to detect it.
        # This is not over-riding GUARDIAN; it IS the GUARDIAN rule for declared danger.
        hard_veto = cell.risk >= 1.0 and cell.analysis == "ACT"

        veto = hard_veto or dog_veto

        # At REFLEX level, dog heuristics ARE the score.
        # The axiom facet scorer defaults to 50.0 (no LLM) → always produces 30.9
        # regardless of content. Dog outputs (GUARDIAN anomaly detection, JANITOR
        # AST analysis, etc.) are the actual signal at this level.
        final_q = 0.0 if veto else phi_bound_score(avg_q)

        # Axiom scoring kept for active_axioms tracking and emergent activation only.
        axiom_result = self.axiom_arch.score_and_compute(
            domain=cell.reality,
            context=str(cell.content)[:500],
            fractal_depth=1,
            metrics={"avg_dog_q": avg_q / MAX_Q_SCORE},
        )

        verdict = verdict_from_q_score(final_q)
        total_cost = sum(j.cost_usd for j in dog_judgments)

        return Judgment(
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

        # L2 → L1 ESCALATION: If consensus failed at MICRO, upgrade to full MACRO cycle.
        # Law: A failed quorum means the Dogs disagree — need more analysis, not less.
        # Budget guard: Only escalate if remaining budget can cover MACRO overhead.
        if not consensus.consensus:
            remaining_budget = cell.budget_usd * (1.0 - PHI_INV_2)  # ~61.8% left
            if remaining_budget > 0.0001:  # $0.1 milli minimum
                logger.info(
                    "L2→L1 escalation: MICRO consensus failed (%d/%d votes) for cell %s → MACRO",
                    consensus.votes, consensus.quorum, cell.cell_id,
                )
                pipeline.level = ConsciousnessLevel.MACRO
                return await self._cycle_macro(pipeline)

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

    # ── L4 META EVOLUTION ─────────────────────────────────────────────────────

    async def evolve(self) -> Dict[str, Any]:
        """
        L4 META: Self-benchmark via 5 canonical probe cells.

        Runs each probe at REFLEX level (no LLM, <200ms total), compares
        q_scores against expected ranges, detects regression vs previous call.

        Called by DogScheduler._meta_evolve() every ~4 hours.
        Emits CoreEvent.META_CYCLE with results summary.

        Returns summary dict:
          pass_rate, pass_count, total, regression, results[]
        """
        import asyncio as _asyncio
        from cynic.judge.probes import PROBE_CELLS, ProbeResult

        results: List[ProbeResult] = []

        for probe in PROBE_CELLS:
            t0 = time.time()
            try:
                judgment = await self.run(
                    probe["cell"],
                    level=ConsciousnessLevel.REFLEX,
                )
                elapsed = (time.time() - t0) * 1000
                passed = probe["min_q"] <= judgment.q_score <= probe["max_q"]
                results.append(ProbeResult(
                    name=probe["name"],
                    q_score=judgment.q_score,
                    verdict=judgment.verdict,
                    expected_min=probe["min_q"],
                    expected_max=probe["max_q"],
                    passed=passed,
                    duration_ms=elapsed,
                ))
            except Exception as exc:
                elapsed = (time.time() - t0) * 1000
                logger.warning("evolve() probe %s failed: %s", probe["name"], exc)
                results.append(ProbeResult(
                    name=probe["name"],
                    q_score=0.0,
                    verdict="BARK",
                    expected_min=probe["min_q"],
                    expected_max=probe["max_q"],
                    passed=False,
                    duration_ms=elapsed,
                    error=str(exc),
                ))

        pass_count = sum(1 for r in results if r.passed)
        pass_rate = pass_count / len(results) if results else 0.0

        # Regression: pass_rate dropped >20% vs previous evolve()
        regression = False
        if self._evolve_history:
            prev_rate = self._evolve_history[-1]["pass_rate"]
            regression = pass_rate < prev_rate - 0.20

        summary: Dict[str, Any] = {
            "timestamp": time.time(),
            "pass_rate": round(pass_rate, 3),
            "pass_count": pass_count,
            "total": len(results),
            "regression": regression,
            "results": [r.to_dict() for r in results],
        }

        # Keep last F(8)=21 evolve() snapshots
        self._evolve_history.append(summary)
        if len(self._evolve_history) > 21:
            self._evolve_history.pop(0)

        # Persist probe runs to DB (no-op if benchmark_registry not wired)
        if self.benchmark_registry is not None:
            try:
                await self.benchmark_registry.record_evolve(results)
            except Exception as exc:
                logger.warning("BenchmarkRegistry.record_evolve() failed: %s", exc)

        await get_core_bus().emit(Event(
            type=CoreEvent.META_CYCLE,
            payload={"evolve": summary},
        ))

        log_line = "evolve() %d/%d probes passed (%.0f%%)%s"
        logger.info(
            log_line,
            pass_count, len(results), pass_rate * 100,
            " -- REGRESSION DETECTED" if regression else "",
        )

        return summary

    def stats(self) -> Dict[str, Any]:
        last_evolve = self._evolve_history[-1] if self._evolve_history else None
        return {
            "judgments_total": self._judgment_count,
            "dogs_active": len(self.dogs),
            "consciousness": self._consciousness.to_dict(),
            "evolve_cycles": len(self._evolve_history),
            "last_evolve_pass_rate": last_evolve["pass_rate"] if last_evolve else None,
            "last_evolve_regression": last_evolve["regression"] if last_evolve else False,
            "circuit_breaker": self._circuit_breaker.stats(),
        }
