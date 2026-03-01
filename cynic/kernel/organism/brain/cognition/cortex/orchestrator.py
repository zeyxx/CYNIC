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

import httpx
import logging
import time
import uuid
from dataclasses import dataclass, field
from typing import Any, Optional


from cynic.kernel.core.phi import (
    MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV, PHI_INV_2, PHI,
    phi_bound_score, LEARNING_RATE, fibonacci,
)
from cynic.kernel.core.formulas import CONSCIOUSNESS_BLENDING_WEIGHTS
from cynic.kernel.core.consciousness import (
    ConsciousnessLevel, ConsciousnessState, get_consciousness, dogs_for_level,
)
from cynic.kernel.core.judgment import Cell, Judgment, ConsensusResult
from cynic.kernel.core.axioms import AxiomArchitecture, Verdict, verdict_from_q_score
from cynic.kernel.core.event_bus import (
    get_core_bus, Event, CoreEvent,
)
from cynic.kernel.core.event_bus import EventBusError
from cynic.kernel.core.events_schema import (
    ConsensusReachedPayload,
    ConsensusFailedPayload,
    JudgmentCreatedPayload,
    JudgmentFailedPayload,
    JudgmentRequestedPayload,
    LearningEventPayload,
    MetaCyclePayload,
    PerceptionReceivedPayload,
    ResidualHighPayload,
    DecisionMadePayload,
)
from cynic.kernel.organism.brain.cognition.neurons.base import AbstractDog, DogJudgment, DogId
from cynic.kernel.organism.brain.cognition.neurons.cynic_dog import CynicDog
from cynic.kernel.organism.brain.cognition.cortex.circuit_breaker import CircuitBreaker, CircuitState
from cynic.kernel.organism.brain.cognition.cortex.decision_validator import DecisionValidator, BlockedDecision
from cynic.kernel.organism.brain.cognition.cortex.handlers import HandlerComposer
from cynic.kernel.organism.brain.cognition.cortex.pipeline import JudgmentPipeline
from cynic.kernel.organism.brain.cognition.cortex.judgment_stages import execute_judgment_pipeline

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex")

# Maximum Dogs to run in parallel at L1 MACRO (F(7)=13 > 11, so all 11)
MAX_PARALLEL_DOGS = 11


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS SCHEDULER — Blended Escalation Policy (Task #8)
# Note: Now using HandlerComposer for explicit DAG composition
# ════════════════════════════════════════════════════════════════════════════


class JudgeOrchestrator:
    """
    The Judge Orchestrator — runs the 7-step CYNIC cycle.

    Usage:
      orchestrator = JudgeOrchestrator(dogs, axiom_arch, cynic_dog)
      judgment = await orchestrator.run(cell, level=ConsciousnessLevel.MACRO)
    """

    def __init__(
        self,
        dogs: dict[str, AbstractDog],
        axiom_arch: AxiomArchitecture,
        cynic_dog: CynicDog,
        residual_detector=None,
        gasdf_executor=None,
    ) -> None:
        self.dogs = dogs        # {dog_id: AbstractDog}
        self.axiom_arch = axiom_arch
        self.cynic_dog = cynic_dog
        self.residual_detector = residual_detector  # Optional[ResidualDetector]
        self.gasdf_executor = gasdf_executor        # Optional[GASdfExecutor]
        self.benchmark_registry = None  # Optional[BenchmarkRegistry] — set via state.py
        self.escore_tracker = None  # Optional[EScoreTracker] — injected via state.py
        
        # Inject dynamic facet dreaming callback
        self.axiom_arch.on_facets_missing = self._on_facets_missing

        # State and services
        self.axiom_monitor = None  # Optional[AxiomMonitor] — γ3: axiom health → budget multiplier
        self.lod_controller = None  # Optional[LODController] — δ2: system health → level cap
        self.context_compressor = None  # Optional[ContextCompressor] — γ5: memory injection into SAGE
        self.service_registry = None  # Optional[ServiceStateRegistry] — Tier 1 nervous system
        self.consciousness_scheduler = None  # Optional[ConsciousnessScheduler] — Task #8: blended escalation
        self._composer = None  # Optional[HandlerComposer] — Phase 2B: explicit DAG composition
        self.gossip_manager = None  # Optional[GossipManager] — federation P2P gossip
        self._judgment_count = 0
        self._consciousness = get_consciousness()
        # evolve() history — last F(8)=21 META cycles
        self._evolve_history: list[dict[str, Any]] = []
        # Circuit breaker — prevents cascade failures (topology M1)
        self._circuit_breaker = CircuitBreaker()
        # Budget stress flags — set by BUDGET_WARNING/EXHAUSTED events
        self._budget_stress: bool = False      # cap at MICRO when True
        self._budget_exhausted: bool = False   # cap at REFLEX when True
        
        # --- Voice of the Organism (Dialogue) ---
        from cynic.kernel.organism.brain.dialogue.agent import DialogueAgent
        self._dialogue_agent = DialogueAgent()

    async def _on_facets_missing(self, axiom: str, reality: str) -> None:
        """
        Callback triggered when AxiomArchitecture needs facets for a new reality.
        SAGE (Chokmah) is the Faiseur de Mondes.
        """
        from cynic.kernel.organism.brain.cognition.neurons.base import DogId
        sage = self.dogs.get(DogId.SAGE)
        if sage and hasattr(sage, "dream_facets"):
            logger.info(f"Orchestrator: Triggering SAGE to dream facets for {axiom} in {reality}")
            await sage.dream_facets(axiom, reality, self.axiom_arch.registry)
        else:
            logger.warning(f"Orchestrator: Facets missing for {axiom}/{reality}, but SAGE unavailable to dream.")

    def _ensure_composer(self) -> None:
        """
        Ensure HandlerComposer is initialized.

        Called automatically by run() if composer is None. This supports:
        - Tests that manually create orchestrator without build_kernel()
        - Minimal execution without full kernel initialization

        Creates default handlers for REFLEX/MICRO/MACRO cycles.
        """
        if self._composer is not None:
            return  # Already initialized

        from cynic.kernel.organism.brain.cognition.cortex.handlers import (
            HandlerRegistry, HandlerComposer, LevelSelector, ReflexCycleHandler,
            MicroCycleHandler, MacroCycleHandler, ActHandler, EvolveHandler,
            BudgetManager,
        )

        registry = HandlerRegistry()
        registry.register("level_selector", LevelSelector(axiom_monitor=self.axiom_monitor, lod_controller=self.lod_controller))
        registry.register("cycle_reflex", ReflexCycleHandler(dogs=self.dogs, axiom_arch=self.axiom_arch))
        registry.register("cycle_micro", MicroCycleHandler(dogs=self.dogs, axiom_arch=self.axiom_arch, cynic_dog=self.cynic_dog, lod_controller=self.lod_controller))
        registry.register("cycle_macro", MacroCycleHandler(dogs=self.dogs, axiom_arch=self.axiom_arch, cynic_dog=self.cynic_dog, lod_controller=self.lod_controller, axiom_monitor=self.axiom_monitor))
        registry.register("act_executor", ActHandler(runner=None, gasdf_executor=self.gasdf_executor))
        registry.register("evolve", EvolveHandler(orchestrator=self))
        registry.register("budget_manager", BudgetManager())

        self._composer = HandlerComposer(registry)
        logger.debug("HandlerComposer auto-initialized (test/manual setup mode)")

    # ── STEP 0: Entry Point ────────────────────────────────────────────────

    async def run(
        self,
        cell: Cell,
        level: Optional[ConsciousnessLevel] = None,
        budget_usd: Optional[float] = None,
        fractal_depth: int = 1,
    ) -> Judgment:
        """
        Run the complete judgment cycle for a Cell via handler composition DAG.

        Level auto-selected if None (based on budget and timer health).
        Delegates all cycle logic to HandlerComposer for explicit, testable, error-checked execution.
        """
        # Ensure composer is initialized (supports manual orchestrator creation in tests)
        self._ensure_composer()

        pipeline = JudgmentPipeline(cell=cell, fractal_depth=fractal_depth)
        effective_budget = (budget_usd or cell.budget_usd)

        # Circuit breaker — fast-fail when cascade failure detected (topology M1)
        if not self._circuit_breaker.allow():
            cb = self._circuit_breaker
            logger.warning(
                "CircuitBreaker %s — fast-failing judgment %s (failures=%d)",
                cb.state.value, cell.cell_id, cb.failure_count,
            )
            await get_core_bus().emit(Event.typed(
                CoreEvent.JUDGMENT_FAILED,
                JudgmentFailedPayload(
                    cell_id=cell.cell_id,
                    error="circuit_open",
                    circuit_state=cb.state.value,
                    failure_count=cb.failure_count,
                ),
            ))
            raise RuntimeError(
                f"CircuitBreaker OPEN — pipeline suspended "
                f"({cb.failure_count} consecutive failures)"
            )

        try:
            # Compose handlers and execute judgment cycle (Phase 2B DAG)
            # HandlerComposer handles: level selection + LOD cap + cycle dispatch + act + evolve + budget
            t_compose_start = time.perf_counter()
            compose_result = await self._composer.compose(pipeline, level, effective_budget)
            if not compose_result.success:
                raise RuntimeError(f"Handler composition failed: {compose_result.error}")

            judgment = compose_result.output
            selected_level = pipeline.level or level or ConsciousnessLevel.MACRO
            elapsed_compose_ms = (time.perf_counter() - t_compose_start) * 1000

            # Emit JUDGMENT_CREATED (enriched with cell context for DecideAgent)
            self._judgment_count += 1
            self._consciousness.increment(selected_level)
            jc_payload = judgment.to_dict()
            jc_payload["state_key"] = cell.state_key()
            jc_payload["reality"] = cell.reality   # needed by guidance.json writer
            jc_payload["level_used"] = selected_level.name  # needed by LOD latency filter
            jc_payload["content_preview"] = str(cell.content or "")[:200]
            jc_payload["context"] = cell.context or ""
            await get_core_bus().emit(Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                JudgmentCreatedPayload.model_validate(jc_payload),
            ))

            # Tier 1 Nervous System: Record judgment in Service State Registry
            if self.service_registry is not None:
                await self.service_registry.record_judgment(
                    component_name="orchestrator",
                    judgment_id=judgment.judgment_id,
                    verdict=judgment.verdict,
                    q_score=judgment.q_score,
                    metadata={
                        "level": selected_level.name,
                        "state_key": cell.state_key(),
                        "reality": cell.reality,
                        "consensus_reached": judgment.consensus_reached,
                        "confidence": judgment.confidence,
                    },
                )

            # Emit CONSENSUS_REACHED or CONSENSUS_FAILED based on final judgment.
            # Dogs cooperating (PBFT quorum achieved) = CONSENSUS_REACHED.
            # Dogs failing to agree = CONSENSUS_FAILED.
            if judgment.consensus_reached:
                await get_core_bus().emit(Event.typed(
                    CoreEvent.CONSENSUS_REACHED,
                    ConsensusReachedPayload(
                        q_score=judgment.q_score,
                        votes=judgment.consensus_votes,
                        quorum=judgment.consensus_quorum,
                        verdict=judgment.verdict,
                        judgment_id=judgment.judgment_id,
                        cell_id=cell.cell_id,
                        reality=cell.reality,
                    ),
                ))
            else:
                await get_core_bus().emit(Event.typed(
                    CoreEvent.CONSENSUS_FAILED,
                    ConsensusFailedPayload(
                        votes=judgment.consensus_votes,
                        quorum=judgment.consensus_quorum,
                        judgment_id=judgment.judgment_id,
                        residual_variance=judgment.residual_variance,
                        cell_id=cell.cell_id,
                        reality=cell.reality,
                    ),
                ))

                # Record emergence pattern (THE_UNNAMEABLE) for federation sharing
                if self.gossip_manager is not None:
                    self.gossip_manager.record_unnameable(cell.reality)

            # Emit LEARNING_EVENT for ALL cycles (REFLEX/MICRO/MACRO).
            # Was incorrectly placed inside _cycle_macro only — Q-Learning never fired.
            await get_core_bus().emit(Event.typed(
                CoreEvent.LEARNING_EVENT,
                LearningEventPayload(
                    reward=judgment.q_score / MAX_Q_SCORE,
                    action=judgment.verdict,
                    state_key=cell.state_key(),
                    judgment_id=judgment.judgment_id,
                    loop_name="JUDGE_ORCHESTRATOR",
                ),
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

            # γ2: SAGE→Compressor attention feedback loop
            # boost() tells the compressor which content was relevant for this judgment.
            if self.context_compressor is not None:
                try:
                    _content_preview = str(getattr(cell, "content", "") or "")[:200]
                    self.context_compressor.boost(_content_preview, judgment.q_score / 100.0)
                except EventBusError:
                    logger.debug("Compressor boost failed (non-critical)", exc_info=True)

            # Circuit breaker: successful judgment — reset failure counter
            self._circuit_breaker.record_success()

            # --- Generate Explanation (The Voice) ---
            if judgment and not getattr(judgment, "reasoning", ""):
                try:
                    explanation = await self._dialogue_agent.explain_judgment(
                        judgment, 
                        question=str(cell.content)
                    )
                    if hasattr(judgment, "model_copy"):
                        judgment = judgment.model_copy(update={"reasoning": explanation})
                    else:
                        setattr(judgment, "reasoning", explanation)
                except Exception as e:
                    logger.warning("Orchestrator: Failed to generate explanation: %s", e)

            # Trigger gossip batch checking if federation is enabled
            if self.gossip_manager is not None:
                self.gossip_manager.on_judgment(self._judgment_count)

            return judgment

        except httpx.RequestError as e:
            logger.error("Judgment pipeline failed: %s", e, exc_info=True)
            # Circuit breaker: record failure — may open circuit after threshold
            self._circuit_breaker.record_failure()
            await get_core_bus().emit(Event.typed(
                CoreEvent.JUDGMENT_FAILED,
                JudgmentFailedPayload(
                    cell_id=cell.cell_id,
                    error=str(e),
                ),
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

    # ── Budget Enforcement (ACCOUNT → JUDGE feedback) ──────────────────────

    def on_budget_warning(self) -> None:
        """
        React to BUDGET_WARNING — cap future judgments at MICRO level.

        Called by state.py when AccountAgent emits BUDGET_WARNING (38.2% left).
        Prevents Ollama MACRO calls while budget is low.
        """
        if not self._budget_stress:
            self._budget_stress = True
            logger.warning(
                "*GROWL* Budget stress: capping judgment level at MICRO "
                "(no MACRO/Ollama until budget resets)"
            )

    def on_budget_exhausted(self) -> None:
        """
        React to BUDGET_EXHAUSTED — cap future judgments at REFLEX level.

        Called by state.py when AccountAgent emits BUDGET_EXHAUSTED (budget=0).
        Zero LLM calls — pure heuristic until session resets.
        """
        if not self._budget_exhausted:
            self._budget_exhausted = True
            logger.error(
                "*GROWL* Budget exhausted: forcing REFLEX-only mode "
                "(zero LLM calls)"
            )

    # ── LOD CAP (B2 fix) ───────────────────────────────────────────────────

    def _apply_lod_cap(self, level: ConsciousnessLevel) -> ConsciousnessLevel:
        """
        Enforce LOD cap on any level — explicit or auto-selected (B2 fix).

        _select_level() already enforces this for the auto-select path, but:
          1. run(cell, level=MACRO) bypasses _select_level entirely.
          2. _cycle_micro escalation calls _cycle_macro directly (no level check).
        This method is the single enforcement point for both cases.
        """
        if self.lod_controller is None:
            return level
        from cynic.kernel.organism.brain.cognition.cortex.lod import SurvivalLOD
        lod = self.lod_controller.current
        if lod >= SurvivalLOD.EMERGENCY:
            if level != ConsciousnessLevel.REFLEX:
                logger.warning(
                    "LOD cap: %s → REFLEX (LOD=%s, system under stress)",
                    level.name, lod.name,
                )
            return ConsciousnessLevel.REFLEX
        if lod == SurvivalLOD.REDUCED and level == ConsciousnessLevel.MACRO:
            logger.info("LOD cap: MACRO → MICRO (LOD=REDUCED)")
            return ConsciousnessLevel.MICRO
        return level

    # ── LEVEL SELECTION ────────────────────────────────────────────────────

    async def _select_level(self, cell: Cell, budget_usd: float) -> ConsciousnessLevel:
        """Auto-select consciousness level based on budget and cell metadata.

        Priority order:
        1. LOD enforcement (system health caps depth)
        2. Budget enforcement (stress/exhausted caps depth)
        3. ConsciousnessScheduler (blended axiom + e_score + oracle) if available
        4. Cell's own consciousness gradient (fallback)
        """
        # LOD enforcement (health→JUDGE loop): system health caps depth first
        # LOD aggregates all signals: disk, memory, error rate, latency, queue.
        # Takes priority — a crashed system can't afford Ollama regardless of budget.
        if self.lod_controller is not None:
            from cynic.kernel.organism.brain.cognition.cortex.lod import SurvivalLOD
            lod = self.lod_controller.current
            if lod >= SurvivalLOD.EMERGENCY:
                return ConsciousnessLevel.REFLEX
            if lod == SurvivalLOD.REDUCED:
                # Cap at MICRO — same as budget stress
                suggested = self._consciousness.should_downgrade(budget_usd)
                if suggested == ConsciousnessLevel.REFLEX:
                    return ConsciousnessLevel.REFLEX
                return ConsciousnessLevel.MICRO

        # Budget enforcement (ACCOUNT→JUDGE loop): stressed budget caps depth
        if self._budget_exhausted:
            return ConsciousnessLevel.REFLEX
        if self._budget_stress:
            # Allow at most MICRO — no Ollama calls
            suggested = self._consciousness.should_downgrade(budget_usd)
            if suggested == ConsciousnessLevel.REFLEX:
                return ConsciousnessLevel.REFLEX
            return ConsciousnessLevel.MICRO

        # Task #8: Use ConsciousnessScheduler if available (blended escalation policy)
        if self.consciousness_scheduler is not None:
            try:
                level = await self.consciousness_scheduler.select_level(cell)
                logger.debug(f"ConsciousnessScheduler selected {level.name}")
                return level
            except CynicError as e:
                logger.warning(f"ConsciousnessScheduler failed, falling back: {e}")
                # Fall through to legacy logic

        # Legacy fallback: budget-based downgrade
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
        dog_judgments: list[DogJudgment] = await asyncio.gather(*tasks, return_exceptions=False)
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
        # B2 fix: apply LOD cap before escalating — EMERGENCY LOD blocks MACRO.
        if not consensus.consensus:
            remaining_budget = cell.budget_usd * (1.0 - PHI_INV_2)  # ~61.8% left
            if remaining_budget > 0.0001:  # $0.1 milli minimum
                capped = self._apply_lod_cap(ConsciousnessLevel.MACRO)
                if capped != ConsciousnessLevel.MACRO:
                    logger.info(
                        "L2→L1 escalation suppressed: LOD cap=%s for cell %s",
                        capped.name, cell.cell_id,
                    )
                else:
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

    # ── STEP 3+4: DECIDE + ACT ────────────────────────────────────────────────

    async def _act_phase(self, judgment: Judgment, pipeline: JudgmentPipeline) -> Optional[dict]:
        """
        STEP 3 (DECIDE) + STEP 4 (ACT) — unified action execution with guardrails.

        1. Call DecideAgent.decide_for_judgment() to get action recommendation (DECIDE)
        2. Call DecisionValidator to pass all guardrails (PowerLimiter, Alignment, HumanGate, Audit)
        3. If validation passes and reality warrants action, call runner.execute() (ACT)
        4. Return action result or None if no action taken/blocked

        Args:
            judgment: Final judgment from JUDGE phase
            pipeline: JudgmentPipeline (for context/logging)

        Returns:
            {
                "action_id": str,
                "success": bool,
                "output": str,
                "duration_ms": float,
                "error": str or None,
            }
            or None if no action warranted/blocked
        """
        decide_agent = getattr(self, 'decide_agent', None)
        if not decide_agent:
            return None

        # STEP 3: DECIDE — run DecideAgent synchronously
        decision = decide_agent.decide_for_judgment(judgment)
        if not decision:
            return None  # No action needed

        # NEW: GUARDRAIL VALIDATION — DecisionValidator chains all safety checks
        decision_validator = getattr(self, 'decision_validator', None)
        if decision_validator:
            try:
                validated_decision = await decision_validator.validate_decision(
                    decision=decision,
                    judgment=judgment,
                    recent_judgments=self._recent_judgments[-5:] if hasattr(self, '_recent_judgments') else [],
                    scheduler=pipeline.scheduler if hasattr(pipeline, 'scheduler') else None,
                )
                # Decision passed all guardrails
                logger.info(f"Decision validated: {decision['verdict']} → proceeding to ACT")
            except BlockedDecision as e:
                # Decision blocked by guardrail
                logger.warning(
                    f"Decision BLOCKED [{e.guardrail}]: {e.reason} "
                    f"→ {e.recommendation}"
                )
                # Return block result without executing
                return {
                    "action_id": decision.get("judgment_id", "")[:8],
                    "success": False,
                    "output": "",
                    "duration_ms": 0.0,
                    "error": f"[{e.guardrail}] {e.reason}",
                }
        else:
            logger.debug("No DecisionValidator available — skipping guardrail checks")

        # Filter: only execute for actionable realities
        from cynic.kernel.organism.brain.cognition.cortex.decide import _ACT_REALITIES
        if decision["reality"] not in _ACT_REALITIES:
            # Still emit DECISION_MADE for human review, but don't auto-execute
            await get_core_bus().emit(Event.typed(
                CoreEvent.DECISION_MADE,
                DecisionMadePayload(
                    verdict=decision["verdict"],
                    reality=decision["reality"],
                    state_key=decision["state_key"],
                    q_value=decision["q_value"],
                    confidence=decision["confidence"],
                    recommended_action=decision["recommended_action"],
                    action_prompt=decision["action_prompt"],
                    trigger="decide_phase",
                    mcts=True,
                    judgment_id=decision["judgment_id"],
                ),
                source="orchestrator_act_phase",
            ))
            return None

        # STEP 4: ACT — execute the action
        runner = getattr(self, 'runner', None)
        if not runner:
            logger.warning("No runner available — cannot execute action")
            return None

        import time
        t0 = time.perf_counter()
        try:
            action_result = await runner.execute(
                prompt=decision["action_prompt"],
                timeout=30,
            )
            duration_ms = (time.perf_counter() - t0) * 1000

            result = {
                "action_id": decision.get("judgment_id", "")[:8],
                "success": action_result.get("success", False),
                "output": action_result.get("output", ""),
                "duration_ms": duration_ms,
                "error": action_result.get("error"),
            }

            # Emit ACT_COMPLETED event (for feedback loops L3, L4)
            from cynic.kernel.core.events_schema import ActCompletedPayload
            await get_core_bus().emit(Event.typed(
                CoreEvent.ACT_COMPLETED,
                ActCompletedPayload(
                    success=result["success"],
                    action_id=result["action_id"],
                    duration_ms=result["duration_ms"],
                    error=result["error"],
                ),
            ))

            logger.info(
                "ACT: executed %s (success=%s, %.0fms)",
                result["action_id"], result["success"], duration_ms,
            )
            return result

        except CynicError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            logger.error("ACT: execution failed: %s (%.0fms)", e, duration_ms)
            return {
                "action_id": decision.get("judgment_id", "")[:8],
                "success": False,
                "output": "",
                "duration_ms": duration_ms,
                "error": str(e),
            }

    # ── L1 MACRO CYCLE (~2.85s, full 7-step) ──────────────────────────────

    async def _cycle_macro(self, pipeline: JudgmentPipeline) -> Judgment:
        """
        L1 MACRO cycle: full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE.

        This is the canonical CYNIC judgment cycle.
        Now uses pluggable stages from judgment_stages.py for testability.
        """
        # Execute the 7-step pipeline using modular stages
        pipeline = await execute_judgment_pipeline(self, pipeline)

        # Return final judgment
        judgment = pipeline.final_judgment
        if judgment is None:
            raise RuntimeError("No judgment produced by pipeline")

        return judgment

    # ── L4 META EVOLUTION ─────────────────────────────────────────────────────

    async def evolve(self) -> dict[str, Any]:
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
        from cynic.kernel.organism.brain.cognition.cortex.probes import PROBE_CELLS, ProbeResult

        results: list[ProbeResult] = []

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
            except CynicError as exc:
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

        summary: dict[str, Any] = {
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
            except CynicError as exc:
                logger.warning("BenchmarkRegistry.record_evolve() failed: %s", exc)

        await get_core_bus().emit(Event.typed(
            CoreEvent.META_CYCLE,
            MetaCyclePayload(evolve=summary),
        ))

        log_line = "evolve() %d/%d probes passed (%.0f%%)%s"
        logger.info(
            log_line,
            pass_count, len(results), pass_rate * 100,
            " -- REGRESSION DETECTED" if regression else "",
        )

        return summary

    def stats(self) -> dict[str, Any]:
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
