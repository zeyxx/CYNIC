"""
CYNIC API State — Kernel singleton wired at startup.

One AppState per process. Initialized via FastAPI lifespan.
All routes get this via Depends(get_app_state).
"""
from __future__ import annotations

import json
import os
import time
import logging
from dataclasses import dataclass, field
from typing import Dict, List, Optional

from cynic.core.axioms import AxiomArchitecture
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.consciousness import get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.phi import MAX_CONFIDENCE
from cynic.dogs.base import DogId
from cynic.dogs.cynic_dog import CynicDog
from cynic.dogs.guardian import GuardianDog
from cynic.dogs.analyst import AnalystDog
from cynic.dogs.janitor import JanitorDog
from cynic.dogs.architect import ArchitectDog
from cynic.dogs.oracle import OracleDog
from cynic.dogs.sage import SageDog
from cynic.dogs.scholar import ScholarDog
from cynic.dogs.cartographer import CartographerDog
from cynic.dogs.deployer import DeployerDog
from cynic.dogs.scout import ScoutDog
from cynic.judge.orchestrator import JudgeOrchestrator
from cynic.judge.residual import ResidualDetector
from cynic.judge.decide import DecideAgent
from cynic.judge.account import AccountAgent
from cynic.judge.axiom_monitor import AxiomMonitor
from cynic.judge.action_proposer import ActionProposer
from cynic.judge.self_probe import SelfProber
from cynic.judge.lod import LODController, SurvivalLOD
from cynic.core.escore import EScoreTracker
from cynic.learning.qlearning import QTable, LearningLoop
from cynic.perceive.workers import GitWatcher, HealthWatcher, SelfWatcher, MarketWatcher, SolanaWatcher, SocialWatcher, DiskWatcher, MemoryWatcher
from cynic.core.storage.gc import StorageGarbageCollector
from cynic.perceive import checkpoint as _session_checkpoint
from cynic.perceive.checkpoint import CHECKPOINT_EVERY
from cynic.scheduler import DogScheduler
from cynic.act.telemetry import TelemetryStore
from cynic.perceive.compressor import ContextCompressor

logger = logging.getLogger("cynic.api.state")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


async def _on_judgment_created(event: Event) -> None:
    """
    Write guidance.json from ANY judgment source (hook, scheduler, API).

    This is the key feedback loop: JUDGMENT_CREATED fires after every judgment,
    including MACRO scheduler jobs where SAGE temporal MCTS ran.
    The hook reads guidance.json on next UserPromptSubmit → Claude Code gets wisdom.

    By subscribing here (not just in /judge and /perceive endpoints),
    SAGE's Ollama-powered judgments finally reach the feedback loop.
    """
    try:
        p = event.payload
        os.makedirs(os.path.dirname(_GUIDANCE_PATH), exist_ok=True)
        with open(_GUIDANCE_PATH, "w", encoding="utf-8") as fh:
            json.dump({
                "timestamp": time.time(),
                "state_key": p.get("state_key", ""),
                "verdict": p.get("verdict", "WAG"),
                "q_score": round(float(p.get("q_score", 0.0)), 3),
                "confidence": round(min(float(p.get("confidence", 0.0)), MAX_CONFIDENCE), 4),
                "reality": p.get("reality", "CODE"),
                "dog_votes": {
                    k: round(float(v), 3)
                    for k, v in (p.get("dog_votes") or {}).items()
                },
            }, fh)
    except Exception as exc:
        logger.debug("guidance.json write skipped: %s", exc)


@dataclass
class AppState:
    """
    The kernel wired and ready.

    Built once at startup, lives for the process lifetime.
    """
    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    residual_detector: ResidualDetector
    scheduler: DogScheduler
    started_at: float = field(default_factory=time.time)
    _pool: Optional[object] = None  # asyncpg pool (None if no DB)
    last_judgment: Optional[Dict] = None  # state_key, action, judgment_id — for /feedback
    decide_agent: Optional[object] = None   # DecideAgent — auto-decides on BARK/GROWL
    account_agent: Optional[object] = None  # AccountAgent — step 6 cost ledger
    runner: Optional[object] = None        # ClaudeCodeRunner — spawns claude autonomously
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)  # session data
    context_compressor: ContextCompressor = field(default_factory=ContextCompressor)  # γ2 token budget
    axiom_monitor: AxiomMonitor = field(default_factory=AxiomMonitor)       # δ1 emergent axiom tracker
    lod_controller: LODController = field(default_factory=LODController)   # δ2 graceful degradation
    escore_tracker: EScoreTracker = field(default_factory=EScoreTracker)   # γ4 reputation scoring
    action_proposer: ActionProposer = field(default_factory=ActionProposer) # P5 action queue
    self_prober: SelfProber = field(default_factory=SelfProber)             # L4 self-improvement

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> List[str]:
        return list(self.orchestrator.dogs.keys())


def build_kernel(db_pool=None, registry=None) -> AppState:
    """
    Wire the full kernel: dogs → orchestrator → learning loop.

    db_pool: asyncpg pool (optional — kernel works without DB, just no persistence).
    registry: LLMRegistry (optional — kernel works without LLM, heuristic mode only).
              When provided, LLM dogs get registry injected → temporal MCTS enabled.
    """
    logger.info("Building CYNIC kernel...")

    # Dogs (non-LLM — always available, no API keys needed)
    cynic_dog = CynicDog()
    # QTable must exist before OracleDog (Oracle reads it for predictions)
    qtable = QTable()
    dogs = {
        DogId.CYNIC:        cynic_dog,
        DogId.SAGE:         SageDog(),
        DogId.GUARDIAN:     GuardianDog(),
        DogId.ANALYST:      AnalystDog(),
        DogId.JANITOR:      JanitorDog(),
        DogId.ARCHITECT:    ArchitectDog(),
        DogId.ORACLE:       OracleDog(qtable=qtable),
        DogId.SCHOLAR:      ScholarDog(),
        DogId.CARTOGRAPHER: CartographerDog(),
        DogId.DEPLOYER:     DeployerDog(),
        DogId.SCOUT:        ScoutDog(),
    }

    # ── Inject LLMRegistry into all LLM-capable dogs ──────────────────────
    # Dogs use registry to get the best adapter (Ollama first, then Claude).
    # SageDog → Temporal MCTS (7 parallel Ollama calls, asyncio.gather)
    # Other LLM dogs → Phase 2 when they implement their LLM paths
    if registry is not None:
        llm_dogs_wired = 0
        for dog in dogs.values():
            if hasattr(dog, "set_llm_registry"):
                dog.set_llm_registry(registry)
                llm_dogs_wired += 1
        available_count = len(registry.get_available())
        logger.info(
            "LLMRegistry injected into %d dogs, %d adapters available",
            llm_dogs_wired, available_count,
        )
    else:
        logger.info("No LLMRegistry — all dogs run in heuristic mode")

    # ── Scholar ↔ QTable: recursive meta-learning ──────────────────────────
    # Scholar reads QTable (read-only) to blend historical Q-values with TF-IDF.
    # Effect: "TF-IDF + QTable both say BARK → higher confidence than either alone"
    scholar = dogs.get(DogId.SCHOLAR)
    if scholar is not None and hasattr(scholar, "set_qtable"):
        scholar.set_qtable(qtable)
        logger.info("ScholarDog: QTable injected — recursive meta-learning active")

    axiom_arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
    learning_loop = LearningLoop(qtable=qtable, pool=db_pool)
    learning_loop.start(get_core_bus())

    # ResidualDetector — subscribes to JUDGMENT_CREATED, emits EMERGENCE_DETECTED
    residual_detector = ResidualDetector()
    residual_detector.start(get_core_bus())

    orchestrator = JudgeOrchestrator(
        dogs=dogs,
        axiom_arch=axiom_arch,
        cynic_dog=cynic_dog,
        residual_detector=residual_detector,
    )

    scheduler = DogScheduler(orchestrator=orchestrator)

    # ── DecideAgent — subscribes to JUDGMENT_CREATED, auto-decides on BARK/GROWL
    decide_agent = DecideAgent(qtable=qtable)
    decide_agent.start(get_core_bus())

    # ── ActionProposer — P5: DECISION_MADE → ProposedAction queue ─────────
    # Subscribes to DECISION_MADE (same as _on_decision_made in server.py).
    # Writes ~/.cynic/pending_actions.json — human-visible action queue.
    action_proposer = ActionProposer()
    action_proposer.start(get_core_bus())

    # ── AccountAgent — step 6 (ACCOUNT): cost ledger + budget enforcement ──
    # Subscribes to JUDGMENT_CREATED. Tracks cost per reality/dog.
    # Emits BUDGET_WARNING at PHI_INV_2 remaining, BUDGET_EXHAUSTED at 0.
    # Updates EScore "RUN" dimension: free Ollama + high Q → RUN=100.
    account_agent = AccountAgent()

    # ── EMERGENCE_DETECTED -> META cycle trigger ───────────────────────────
    # When ResidualDetector detects an emergence pattern, submit a META cell
    # to the scheduler so the organism can evolve in response.
    async def _on_emergence(event: Event) -> None:
        from cynic.core.judgment import Cell
        cell = Cell(
            reality="CYNIC", analysis="EMERGE", time_dim="PRESENT",
            content=event.payload,
            context="Emergence detected — META cycle triggered",
            risk=0.5, complexity=0.6, budget_usd=0.1,
            metadata={
                "source": "emergence_trigger",
                "pattern": event.payload.get("pattern_type", "UNKNOWN") if isinstance(event.payload, dict) else "UNKNOWN",
            },
        )
        scheduler.submit(cell, level=ConsciousnessLevel.META, source="emergence_trigger")

    get_core_bus().on(CoreEvent.EMERGENCE_DETECTED, _on_emergence)

    # ── δ1 AxiomMonitor + δ2 LODController + γ4 EScoreTracker ────────────
    # Wired to JUDGMENT_CREATED so every judgment updates the living system.
    axiom_monitor = AxiomMonitor()
    lod_controller = LODController()
    escore_tracker = EScoreTracker()

    # ── LOD↔EScore immune system — wire tracker into orchestrator ──────────
    # Orchestrator uses this to bypass Dogs with E-Score < GROWL_MIN (38.2%)
    # in MACRO cycles. Dogs that consistently score poorly are excluded until
    # their E-Score recovers above the φ-threshold.
    orchestrator.escore_tracker = escore_tracker

    # ── γ3: Axiom→Budget multiplier — wire axiom_monitor into orchestrator ──
    # Orchestrator scales budget_usd by PHI^(active_count-2) before level selection.
    # More active axioms → larger budget → deeper judgment → more axiom signals.
    orchestrator.axiom_monitor = axiom_monitor

    # ── δ2→JUDGE: LOD → consciousness cap ──────────────────────────────────
    # LODController aggregates ALL health signals (disk, memory, error, latency, queue).
    # Injected into orchestrator so _select_level() respects system health.
    # LOD 1 REDUCED → MICRO max; LOD 2+ EMERGENCY/MINIMAL → REFLEX only.
    orchestrator.lod_controller = lod_controller

    # ── AccountAgent EScoreTracker injection + start ────────────────────────
    # RUN dimension: AccountAgent rewards efficient Dogs (high Q / low cost).
    # JUDGE dimension: _on_judgment_for_intelligence (below) updates Dog JUDGE.
    # Together: JUDGE tracks prediction accuracy, RUN tracks cost efficiency.
    account_agent.set_escore_tracker(escore_tracker)
    account_agent.start(get_core_bus())

    # ── Budget→LOD enforcement loop ────────────────────────────────────────
    # AccountAgent emits BUDGET_WARNING (38.2% remaining) and BUDGET_EXHAUSTED (0).
    # Orchestrator reacts: stress → cap at MICRO; exhausted → force REFLEX.
    # Closes the loop: cost pressure → safer, cheaper judgment paths.
    async def _on_budget_warning(event: Event) -> None:
        try:
            orchestrator.on_budget_warning()
        except Exception:
            pass

    async def _on_budget_exhausted(event: Event) -> None:
        try:
            orchestrator.on_budget_exhausted()
        except Exception:
            pass

    get_core_bus().on(CoreEvent.BUDGET_WARNING, _on_budget_warning)
    get_core_bus().on(CoreEvent.BUDGET_EXHAUSTED, _on_budget_exhausted)

    # ── SelfProber — L4 CYNIC→CYNIC self-improvement loop ─────────────────
    # Subscribes to EMERGENCE_DETECTED. Analyzes QTable, EScore, Residual.
    # Generates SelfProposal objects → persists to ~/.cynic/self_proposals.json.
    # Emits SELF_IMPROVEMENT_PROPOSED so downstream can display / act on them.
    self_prober = SelfProber()
    self_prober.set_qtable(qtable)
    self_prober.set_residual_detector(residual_detector)
    self_prober.set_escore_tracker(escore_tracker)
    self_prober.start(get_core_bus())

    _escore_persist_counter = [0]  # mutable cell for closure

    # ── Shared health cache — prevents accumulated-metrics reset bug ───────
    # Each sensor calls assess() with only its own dimension; without caching,
    # a DiskWatcher assess(disk_pct=0.94) would be undone by the next
    # assess(error_rate=0.01) call (which sets disk_pct=0.0 → LOD recovers).
    # _health_cache is the single source of truth for ALL lod_controller.assess() calls.
    _health_cache: Dict = {
        "error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0,
        "memory_pct": 0.0, "disk_pct": 0.0,
    }

    # Rolling outcome window — F(8)=21 recent judgment results (True=ok, False=failed)
    # Shared between JUDGMENT_CREATED (success) and JUDGMENT_FAILED (failure).
    # error_rate = failures / window_size → feeds LOD health check.
    # Self-healing: after 21 successful judgments, error_rate returns to 0.
    _OUTCOME_WINDOW = 21
    _outcome_window: List[bool] = []

    def _update_error_rate() -> None:
        failures = sum(1 for ok in _outcome_window if not ok)
        _health_cache["error_rate"] = failures / len(_outcome_window) if _outcome_window else 0.0

    async def _on_judgment_for_intelligence(event: Event) -> None:
        try:
            p = event.payload
            # Record success in rolling window; compute real error_rate from outcomes
            _outcome_window.append(True)
            if len(_outcome_window) > _OUTCOME_WINDOW:
                _outcome_window.pop(0)
            _update_error_rate()
            _health_cache["latency_ms"] = float(p.get("duration_ms", 0.0))
            # Scheduler queue depth — LOD thresholds: 34 → REDUCED, 89 → EMERGENCY, 144 → MINIMAL
            _health_cache["queue_depth"] = scheduler.total_queue_depth()

            # δ2: Assess LOD from all accumulated health signals
            lod_controller.assess(**_health_cache)

            # γ4: Update E-Score for each Dog that voted
            dog_votes: dict = p.get("dog_votes") or {}
            for dog_id, vote_score in dog_votes.items():
                escore_tracker.update(f"agent:{dog_id}", "JUDGE", float(vote_score))

            # Persist E-Score to DB every 5 judgments (non-blocking best-effort)
            _escore_persist_counter[0] += 1
            if _escore_persist_counter[0] % 5 == 0 and db_pool is not None:
                await escore_tracker.persist(db_pool)

        except Exception:
            pass  # Never block the judgment pipeline

    # δ1: EMERGENCE_DETECTED → signal "EMERGENCE" axiom, emit AXIOM_ACTIVATED if threshold crossed
    async def _on_emergence_signal(event: Event) -> None:
        try:
            new_state = axiom_monitor.signal("EMERGENCE")
            if new_state == "ACTIVE":
                await get_core_bus().emit(Event(
                    type=CoreEvent.AXIOM_ACTIVATED,
                    payload={"axiom": "EMERGENCE", "maturity": axiom_monitor.get_maturity("EMERGENCE")},
                    source="emergence_detector",
                ))
        except Exception:
            pass

    # δ1: DECISION_MADE → signal "AUTONOMY" (Dogs coordinate autonomously)
    async def _on_decision_made_for_axiom(event: Event) -> None:
        try:
            new_state = axiom_monitor.signal("AUTONOMY")
            if new_state == "ACTIVE":
                await get_core_bus().emit(Event(
                    type=CoreEvent.AXIOM_ACTIVATED,
                    payload={"axiom": "AUTONOMY", "maturity": axiom_monitor.get_maturity("AUTONOMY")},
                    source="decide_agent",
                ))
        except Exception:
            pass

    # δ1: AXIOM_ACTIVATED → log milestone; emit TRANSCENDENCE when all 4 active
    async def _on_axiom_activated(event: Event) -> None:
        try:
            axiom_name = event.payload.get("axiom", "?")
            maturity = event.payload.get("maturity", 0.0)
            active = axiom_monitor.active_axioms()
            logger.info(
                "AXIOM_ACTIVATED: %s (maturity=%.1f) — %d/%d axioms active: %s",
                axiom_name, maturity, len(active), 4, active,
            )
            if len(active) == 4:
                await get_core_bus().emit(Event(
                    type=CoreEvent.TRANSCENDENCE,
                    payload={"active_axioms": active, "tier": "TRANSCENDENT"},
                    source="axiom_monitor",
                ))
        except Exception:
            pass

    # ── JUDGMENT_FAILED → LOD error_rate ──────────────────────────────────
    # Circuit breaker open OR pipeline exception → record failure in rolling
    # window → error_rate rises → LOD degrades → cheaper judgment paths.
    # Self-healing: 21 consecutive successful judgments reset error_rate to 0.
    async def _on_judgment_failed(event: Event) -> None:
        try:
            _outcome_window.append(False)
            if len(_outcome_window) > _OUTCOME_WINDOW:
                _outcome_window.pop(0)
            _update_error_rate()
            lod_controller.assess(**_health_cache)
            logger.warning(
                "JUDGMENT_FAILED → error_rate=%.2f, LOD=%s",
                _health_cache["error_rate"], lod_controller.current.name,
            )
        except Exception:
            pass

    # ── L4→P5 bridge: SELF_IMPROVEMENT_PROPOSED → ActionProposer ─────────────
    # SelfProber (L4) analyzes QTable/EScore/Residual → emits SelfProposals.
    # ActionProposer (P5) shows them in pending_actions.json alongside DECISION_MADE.
    # Closes the loop: self-insight → human-visible action queue → accept/reject.
    async def _on_self_improvement_proposed(event: Event) -> None:
        try:
            proposals = (event.payload or {}).get("proposals", [])
            for p in proposals:
                action_proposer.propose_self_improvement(p)
        except Exception:
            pass

    get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _on_judgment_for_intelligence)
    get_core_bus().on(CoreEvent.JUDGMENT_FAILED, _on_judgment_failed)
    get_core_bus().on(CoreEvent.EMERGENCE_DETECTED, _on_emergence_signal)
    get_core_bus().on(CoreEvent.DECISION_MADE, _on_decision_made_for_axiom)
    get_core_bus().on(CoreEvent.AXIOM_ACTIVATED, _on_axiom_activated)
    get_core_bus().on(CoreEvent.SELF_IMPROVEMENT_PROPOSED, _on_self_improvement_proposed)

    # ── Guidance feedback loop — ALL judgment sources ──────────────────────
    # Subscribes to JUDGMENT_CREATED from ANY source: /perceive (REFLEX),
    # /judge (MACRO), or DogScheduler background workers (MACRO with SAGE).
    # This ensures SAGE's Ollama temporal MCTS reaches the guidance.json loop.
    get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _on_judgment_created)

    # ── ContextCompressor (γ2) — accumulate judgment history ──────────────
    # Each judgment produces a compact summary → compressor.add()
    # The /judge endpoint uses compressor.get_compressed_context() to enrich
    # the Cell context with recent session history before LLM calls.
    # This gives SAGE and other LLM dogs temporal continuity across judgments.
    compressor = ContextCompressor()
    # Restore session context from last checkpoint (cross-crash continuity)
    _n_restored = _session_checkpoint.restore(compressor)
    if _n_restored:
        logger.info("build_kernel: session checkpoint restored %d chunks", _n_restored)

    # ── Compressor ↔ SAGE bidirectional attention loop ─────────────────────
    # SAGE reads compressed context (Compressor→SAGE, existing).
    # After each judgment, SAGE signals which content was relevant back
    # to the Compressor (SAGE→Compressor, new — closes the feedback loop).
    sage = dogs.get(DogId.SAGE)
    if sage is not None and hasattr(sage, "set_compressor"):
        sage.set_compressor(compressor)

    _checkpoint_counter = [0]  # mutable cell for closure

    async def _on_judgment_for_compressor(event: Event) -> None:
        try:
            p = event.payload
            verdict = p.get("verdict", "?")
            q = p.get("q_score", 0.0)
            sk = p.get("state_key", "")
            preview = str(p.get("content_preview", ""))[:120].replace("\n", " ")
            summary = f"[{verdict} Q={q:.1f}] {sk}: {preview}"
            compressor.add(summary)

            # γ2: Checkpoint session every CHECKPOINT_EVERY (F(8)=21) judgments
            _checkpoint_counter[0] += 1
            if _checkpoint_counter[0] % CHECKPOINT_EVERY == 0:
                _session_checkpoint.save(compressor)
        except Exception:
            pass  # Never block on compressor errors

    get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _on_judgment_for_compressor)

    # ── PerceiveWorkers (autonomous sensors) ──────────────────────────────
    # Wired here; actually started when scheduler.start() is called (in lifespan).
    # CODE×PERCEIVE — git working tree changes
    scheduler.register_perceive_worker(GitWatcher())
    # CYNIC×PERCEIVE — timer degradation
    scheduler.register_perceive_worker(HealthWatcher())
    # CYNIC×LEARN — Q-Table health self-monitoring
    scheduler.register_perceive_worker(SelfWatcher(qtable_getter=lambda: qtable))
    # MARKET×PERCEIVE — SOL/USD price significant moves
    scheduler.register_perceive_worker(MarketWatcher())
    # SOLANA×PERCEIVE — mainnet slot + TPS anomalies
    scheduler.register_perceive_worker(SolanaWatcher())
    # SOCIAL×PERCEIVE — social signals from ~/.cynic/social.json feed
    scheduler.register_perceive_worker(SocialWatcher())
    # CYNIC×PERCEIVE — disk pressure (φ-thresholds: 61.8% / 76.4% / 90%)
    scheduler.register_perceive_worker(DiskWatcher())
    # CYNIC×PERCEIVE — RAM pressure (same φ-thresholds, no psutil needed)
    scheduler.register_perceive_worker(MemoryWatcher())

    # ── StorageGarbageCollector — triggered by DISK_PRESSURE ───────────────
    # Reacts to DiskWatcher's DISK_PRESSURE event.
    # Pure SQL: prunes BARK judgments, oversized buffers, stale benchmarks.
    storage_gc = StorageGarbageCollector()

    async def _on_disk_pressure(event: Event) -> None:
        pressure = event.payload.get("pressure", "WARN")
        used_pct = event.payload.get("used_pct", 0.0)
        _health_cache["disk_pct"] = event.payload.get("disk_pct", used_pct)
        lod_controller.assess(**_health_cache)

        logger.warning(
            "DISK_PRESSURE: %s (%.1f%% used) → running StorageGC",
            pressure, used_pct * 100,
        )
        result = await storage_gc.collect(db_pool)
        if result.get("total", 0) > 0:
            logger.info("StorageGC freed %d rows (disk was %.1f%% full)", result["total"], used_pct * 100)

    async def _on_memory_pressure(event: Event) -> None:
        pressure = event.payload.get("pressure", "WARN")
        used_pct = event.payload.get("used_pct", 0.0)
        _health_cache["memory_pct"] = event.payload.get("memory_pct", used_pct)
        lod_controller.assess(**_health_cache)
        logger.warning(
            "MEMORY_PRESSURE: %s (%.1f%% RAM used)",
            pressure, used_pct * 100,
        )

    get_core_bus().on(CoreEvent.DISK_PRESSURE, _on_disk_pressure)
    get_core_bus().on(CoreEvent.MEMORY_PRESSURE, _on_memory_pressure)

    logger.info(
        "Kernel ready: %d dogs, scheduler wired, learning loop + residual detector active, pool=%s, llm=%s",
        len(dogs),
        "connected" if db_pool else "none",
        f"{len(registry.get_available())} adapters" if registry else "none",
    )

    return AppState(
        orchestrator=orchestrator,
        qtable=qtable,
        learning_loop=learning_loop,
        residual_detector=residual_detector,
        scheduler=scheduler,
        _pool=db_pool,
        decide_agent=decide_agent,
        account_agent=account_agent,
        context_compressor=compressor,
        axiom_monitor=axiom_monitor,
        lod_controller=lod_controller,
        escore_tracker=escore_tracker,
        action_proposer=action_proposer,
        self_prober=self_prober,
    )


# Process-level singleton — set during lifespan startup
_state: Optional[AppState] = None


def set_state(state: AppState) -> None:
    global _state
    _state = state


def get_state() -> AppState:
    if _state is None:
        raise RuntimeError("AppState not initialized — lifespan not started")
    return _state


async def restore_state(state: AppState) -> None:
    """
    Restore persistent state after kernel startup.

    Call this in the FastAPI lifespan, AFTER build_kernel() and set_state().
    Restores:
      - EScoreTracker entities from e_scores table (γ4)
      - ContextCompressor session from ~/.cynic/session-latest.json (γ2)
    """
    from cynic.perceive import checkpoint as _ckpt

    pool = state._pool

    # γ4: Restore E-Score reputation (DB)
    if pool is not None:
        n = await state.escore_tracker.restore(pool)
        logger.info("restore_state: EScore restored %d entities", n)
    else:
        logger.info("restore_state: no DB pool — EScore not restored")

    # γ2: Restore session context (disk)
    n = _ckpt.restore(state.context_compressor)
    logger.info("restore_state: session checkpoint restored %d chunks", n)
