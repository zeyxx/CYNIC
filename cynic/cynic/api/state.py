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
from typing import Any, TYPE_CHECKING, Dict, List, Optional

if TYPE_CHECKING:
    import asyncpg

from cynic.core.axioms import AxiomArchitecture
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.consciousness import get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.world_model import WorldModelUpdater
from cynic.core.events_schema import (
    ActCompletedPayload,
    JudgmentCreatedPayload,
    SdkToolJudgedPayload,
)
from cynic.core.phi import (
    MAX_CONFIDENCE,
    MAX_Q_SCORE,
    HOWL_MIN,
    WAG_MIN,
    GROWL_MIN,
    PHI_INV,
    PHI_INV_2,
)
from cynic.dogs.base import AbstractDog, DogId
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
from cynic.act.llm_router import LLMRouter
from cynic.act.runner import ClaudeCodeRunner
from cynic.act.auto_benchmark import AutoBenchmark
from cynic.judge.mirror import KernelMirror
from cynic.llm.adapter import LLMRegistry
from cynic.perceive.compressor import ContextCompressor

logger = logging.getLogger("cynic.api.state")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")

# The organism's own identity in the EScore reputation system.
# Used everywhere CYNIC tracks its own performance — 33 call sites, one constant.
_ESCORE_AGENT_ID = "agent:cynic"

# Instance ID for multi-instance guidance isolation (set from lifespan startup via set_instance_id())
_current_instance_id: Optional[str] = None


def set_instance_id(instance_id: str) -> None:
    """Set the instance ID used for guidance-{id}.json isolation (T35)."""
    global _current_instance_id
    _current_instance_id = instance_id


async def _on_judgment_created(event: Event) -> None:
    """
    Write guidance.json from ANY judgment source (hook, scheduler, API).

    This is the key feedback loop: JUDGMENT_CREATED fires after every judgment,
    including MACRO scheduler jobs where SAGE temporal MCTS ran.
    The hook reads guidance.json on next UserPromptSubmit → Claude Code gets wisdom.

    By subscribing here (not just in /judge and /perceive endpoints),
    SAGE's Ollama-powered judgments finally reach the feedback loop.

    T35: also writes guidance-{instance_id}.json for multi-instance isolation.
    kernel-client.js reads all guidance-*.json and picks the most recent.
    """
    try:
        p = event.payload
        payload = {
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
        }
        os.makedirs(os.path.dirname(_GUIDANCE_PATH), exist_ok=True)
        # Always write guidance.json (backward compat for single-instance hooks)
        with open(_GUIDANCE_PATH, "w", encoding="utf-8") as fh:
            json.dump(payload, fh)
        # Also write guidance-{instance_id}.json for multi-instance isolation (T35)
        if _current_instance_id:
            _inst_path = os.path.join(
                os.path.dirname(_GUIDANCE_PATH),
                f"guidance-{_current_instance_id}.json",
            )
            with open(_inst_path, "w", encoding="utf-8") as fh:
                json.dump(payload, fh)
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
    _pool: Optional[asyncpg.Pool] = None
    last_judgment: Optional[Dict] = None  # state_key, action, judgment_id — for /feedback
    decide_agent: Optional[DecideAgent] = None
    account_agent: Optional[AccountAgent] = None
    runner: Optional[ClaudeCodeRunner] = None
    llm_router: Optional[LLMRouter] = None
    kernel_mirror: KernelMirror = field(default_factory=KernelMirror)  # Ring 3 self-reflection
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)  # session data
    context_compressor: ContextCompressor = field(default_factory=ContextCompressor)  # γ2 token budget
    axiom_monitor: AxiomMonitor = field(default_factory=AxiomMonitor)       # δ1 emergent axiom tracker
    lod_controller: LODController = field(default_factory=LODController)   # δ2 graceful degradation
    escore_tracker: EScoreTracker = field(default_factory=EScoreTracker)   # γ4 reputation scoring
    action_proposer: ActionProposer = field(default_factory=ActionProposer) # P5 action queue
    self_prober: SelfProber = field(default_factory=SelfProber)             # L4 self-improvement
    world_model: WorldModelUpdater = field(default_factory=WorldModelUpdater)  # T27 cross-reality aggregator
    auto_benchmark: Optional[AutoBenchmark] = None

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> List[str]:
        return list(self.orchestrator.dogs.keys())


class _KernelBuilder:
    """
    One-time kernel builder.

    Holds all build-time state so event handler methods can close over
    ``self`` instead of a tangle of nested closures.  Call ``build()``
    once; the instance is then discarded.
    """

    # Rolling-window sizes (Fibonacci)
    _OUTCOME_WINDOW     = 21   # F(8) — judgment outcomes
    _SDK_OUTCOME_WINDOW = 13   # F(7) — SDK session outcomes
    _ACT_LOG_CAP        = 89   # F(11) — ACT result JSONL
    _A6_A9 = frozenset({"AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"})
    _ACT_LOG_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "act_results.jsonl")

    def __init__(
        self,
        db_pool: Optional[asyncpg.Pool] = None,
        registry: Optional[LLMRegistry] = None,
    ) -> None:
        self.db_pool: Optional[asyncpg.Pool]   = db_pool
        self.registry: Optional[LLMRegistry]   = registry

        # ── Shared mutable state (used by multiple event handler methods) ──
        # Prevents the [0]-cell hack that was needed with closures.
        self._health_cache: Dict[str, float] = {
            "error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0.0,
            "memory_pct": 0.0, "disk_pct": 0.0,
        }
        self._outcome_window:     List[bool] = []
        self._sdk_outcome_window: List[bool] = []
        self._escore_persist_counter = 0
        self._checkpoint_counter     = 0

        # ── Component placeholders (set by _create_components) ────────────
        self.dogs:             Dict[DogId, AbstractDog] = {}
        self.qtable:           QTable           = None  # type: ignore[assignment]
        self.orchestrator:     JudgeOrchestrator = None  # type: ignore[assignment]
        self.scheduler:        DogScheduler     = None  # type: ignore[assignment]
        self.learning_loop:    LearningLoop     = None  # type: ignore[assignment]
        self.residual_detector: ResidualDetector = None  # type: ignore[assignment]
        self.decide_agent:     DecideAgent      = None  # type: ignore[assignment]
        self.action_proposer:  ActionProposer   = None  # type: ignore[assignment]
        self.account_agent:    AccountAgent     = None  # type: ignore[assignment]
        self.llm_router:       LLMRouter        = None  # type: ignore[assignment]
        self.axiom_monitor:    AxiomMonitor     = None  # type: ignore[assignment]
        self.lod_controller:   LODController    = None  # type: ignore[assignment]
        self.escore_tracker:   EScoreTracker    = None  # type: ignore[assignment]
        self.self_prober:      SelfProber       = None  # type: ignore[assignment]
        self.world_model:      WorldModelUpdater = None  # type: ignore[assignment]
        self.compressor:       ContextCompressor = None  # type: ignore[assignment]
        self.storage_gc:       StorageGarbageCollector = None  # type: ignore[assignment]

    # ═══════════════════════════════════════════════════════════════════════
    # HELPERS
    # ═══════════════════════════════════════════════════════════════════════

    async def _signal_axiom(self, axiom: str, source: str, **extra: Any) -> str:
        """Signal an axiom and emit AXIOM_ACTIVATED if it just became ACTIVE.

        Returns the new axiom state string (e.g. ``"ACTIVE"``, ``"RISING"``).
        Any ``extra`` kwargs are merged into the AXIOM_ACTIVATED payload so
        callers can attach context (trigger name, score, etc.) without
        repeating the boilerplate emit block.

        Usage::

            new_state = await self._signal_axiom(
                "ANTIFRAGILITY", "judgment_intelligence",
                trigger="RECOVERY",
            )
        """
        new_state = self.axiom_monitor.signal(axiom)
        if new_state == "ACTIVE":
            await get_core_bus().emit(Event(
                type=CoreEvent.AXIOM_ACTIVATED,
                payload={
                    "axiom":    axiom,
                    "maturity": self.axiom_monitor.get_maturity(axiom),
                    **extra,
                },
                source=source,
            ))
        return new_state

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 1 — Component creation
    # ═══════════════════════════════════════════════════════════════════════

    def _create_components(self) -> None:
        """Create all kernel components and wire their mutual dependencies."""
        # Dogs (non-LLM — always available, no API keys needed)
        cynic_dog = CynicDog()
        # QTable must exist before OracleDog (Oracle reads it for predictions)
        self.qtable = QTable()
        self.dogs = {
            DogId.CYNIC:        cynic_dog,
            DogId.SAGE:         SageDog(),
            DogId.GUARDIAN:     GuardianDog(),
            DogId.ANALYST:      AnalystDog(),
            DogId.JANITOR:      JanitorDog(),
            DogId.ARCHITECT:    ArchitectDog(),
            DogId.ORACLE:       OracleDog(qtable=self.qtable),
            DogId.SCHOLAR:      ScholarDog(),
            DogId.CARTOGRAPHER: CartographerDog(),
            DogId.DEPLOYER:     DeployerDog(),
            DogId.SCOUT:        ScoutDog(),
        }

        # ── Inject LLMRegistry into all LLM-capable dogs ──────────────────
        if self.registry is not None:
            llm_dogs_wired = 0
            for dog in self.dogs.values():
                if hasattr(dog, "set_llm_registry"):
                    dog.set_llm_registry(self.registry)
                    llm_dogs_wired += 1
            available_count = len(self.registry.get_available())
            logger.info(
                "LLMRegistry injected into %d dogs, %d adapters available",
                llm_dogs_wired, available_count,
            )
        else:
            logger.info("No LLMRegistry — all dogs run in heuristic mode")

        # ── Scholar ↔ QTable: recursive meta-learning ──────────────────────
        scholar = self.dogs.get(DogId.SCHOLAR)
        if scholar is not None and hasattr(scholar, "set_qtable"):
            scholar.set_qtable(self.qtable)
            logger.info("ScholarDog: QTable injected — recursive meta-learning active")

        axiom_arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
        self.learning_loop = LearningLoop(qtable=self.qtable, pool=self.db_pool)
        self.learning_loop.start(get_core_bus())

        self.residual_detector = ResidualDetector()
        self.residual_detector.start(get_core_bus())

        self.orchestrator = JudgeOrchestrator(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
            cynic_dog=cynic_dog,
            residual_detector=self.residual_detector,
        )

        self.scheduler = DogScheduler(orchestrator=self.orchestrator)

        self.decide_agent = DecideAgent(qtable=self.qtable)
        self.decide_agent.start(get_core_bus())

        self.action_proposer = ActionProposer()
        self.action_proposer.start(get_core_bus())

        self.account_agent = AccountAgent()
        self.llm_router = LLMRouter()

        self.axiom_monitor  = AxiomMonitor()
        self.lod_controller = LODController()
        self.escore_tracker = EScoreTracker()

        self.orchestrator.escore_tracker = self.escore_tracker
        self.orchestrator.axiom_monitor  = self.axiom_monitor
        self.orchestrator.lod_controller = self.lod_controller

        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start(get_core_bus())

        self.self_prober = SelfProber()
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)
        self.self_prober.start(get_core_bus())

        # ── ContextCompressor (γ2) ─────────────────────────────────────────
        self.compressor = ContextCompressor()
        _n_restored = _session_checkpoint.restore(self.compressor)
        if _n_restored:
            logger.info("build_kernel: session checkpoint restored %d chunks", _n_restored)

        sage = self.dogs.get(DogId.SAGE)
        if sage is not None and hasattr(sage, "set_compressor"):
            sage.set_compressor(self.compressor)

        self.orchestrator.context_compressor = self.compressor

        self.storage_gc = StorageGarbageCollector()

        # ── WorldModelUpdater (T27) — cross-reality state aggregator ──────
        # Subscribes to JUDGMENT_CREATED; computes composite_risk and
        # dominant_reality across all 7 realities. Previously coded but
        # never instantiated (0% LIVE). Now LIVE.
        self.world_model = WorldModelUpdater()
        self.world_model.start()

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 2 — LOD / health helpers
    # ═══════════════════════════════════════════════════════════════════════

    def _update_error_rate(self) -> None:
        failures = sum(1 for ok in self._outcome_window if not ok)
        self._health_cache["error_rate"] = (
            failures / len(self._outcome_window) if self._outcome_window else 0.0
        )

    async def _assess_lod(self) -> SurvivalLOD:
        """Assess LOD and emit CONSCIOUSNESS_CHANGED on transition."""
        prev   = self.lod_controller.current
        result = self.lod_controller.assess(**self._health_cache)
        if result != prev:
            await get_core_bus().emit(Event(
                type=CoreEvent.CONSCIOUSNESS_CHANGED,
                payload={
                    "from_lod":  prev.value,
                    "to_lod":    result.value,
                    "from_name": prev.name,
                    "to_name":   result.name,
                    "direction": "DOWN" if result > prev else "UP",
                },
                source="lod_controller",
            ))
        return result

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 3 — Intelligence cycle handlers
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_emergence(self, event: Event) -> None:
        """META cycle trigger when ResidualDetector fires."""
        try:
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
            self.scheduler.submit(cell, level=ConsciousnessLevel.META, source="emergence_trigger")
        except Exception:
            pass

    async def _on_budget_warning(self, event: Event) -> None:
        try:
            self.orchestrator.on_budget_warning()
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", GROWL_MIN)
            logger.warning("BUDGET_WARNING → HOLD EScore=%.1f (financial stress)", GROWL_MIN)
        except Exception:
            pass

    async def _on_budget_exhausted(self, event: Event) -> None:
        try:
            self.orchestrator.on_budget_exhausted()
            # HOLD: total financial collapse = complete operational destabilization.
            # 0.0: cannot sustain operations — hardest HOLD signal possible.
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", 0.0)
            logger.warning("BUDGET_EXHAUSTED → HOLD EScore=0.0 (financial collapse)")
        except Exception:
            pass

    async def _on_judgment_for_intelligence(self, event: Event) -> None:
        try:
            p = event.payload
            # Record success in rolling window; compute real error_rate from outcomes
            self._outcome_window.append(True)
            if len(self._outcome_window) > self._OUTCOME_WINDOW:
                self._outcome_window.pop(0)
            self._update_error_rate()
            # Only REFLEX latency drives LOD — MACRO/MICRO are slow by design
            if p.get("level_used", "REFLEX") == "REFLEX":
                self._health_cache["latency_ms"] = float(p.get("duration_ms", 0.0))
            # Scheduler queue depth — LOD thresholds: 34 → REDUCED, 89 → EMERGENCY, 144 → MINIMAL
            self._health_cache["queue_depth"] = self.scheduler.total_queue_depth()

            # δ2: Assess LOD from all accumulated health signals
            await self._assess_lod()

            # γ4: Update E-Score for each Dog that voted
            dog_votes: dict = p.get("dog_votes") or {}
            for dog_id, vote_score in dog_votes.items():
                self.escore_tracker.update(f"agent:{dog_id}", "JUDGE", float(vote_score))

            # Persist E-Score to DB every 5 judgments (non-blocking best-effort)
            self._escore_persist_counter += 1
            if self._escore_persist_counter % 5 == 0 and self.db_pool is not None:
                await self.escore_tracker.persist(self.db_pool)

            # δ1: ANTIFRAGILITY — success after stress = system improves under chaos
            # Signal when we succeed in a context where prior failures existed.
            # "Recovery from error" is the ANTIFRAGILITY axiom docstring source.
            had_stress = len(self._outcome_window) > 1 and any(
                not ok for ok in self._outcome_window[:-1]
            )
            if had_stress:
                new_state = await self._signal_axiom("ANTIFRAGILITY", "judgment_intelligence")

        except Exception:
            pass  # Never block the judgment pipeline

    async def _on_judgment_failed(self, event: Event) -> None:
        try:
            self._outcome_window.append(False)
            if len(self._outcome_window) > self._OUTCOME_WINDOW:
                self._outcome_window.pop(0)
            self._update_error_rate()
            await self._assess_lod()

            # EScore updates — total pipeline failure deserves the harshest signal.
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", 0.0)
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD",  GROWL_MIN)

            logger.warning(
                "JUDGMENT_FAILED → error_rate=%.2f, LOD=%s → JUDGE=0.0 HOLD=%.1f",
                self._health_cache["error_rate"], self.lod_controller.current.name, GROWL_MIN,
            )
        except Exception:
            pass

    async def _on_judgment_requested(self, event: Event) -> None:
        try:
            self._health_cache["queue_depth"] = self.scheduler.total_queue_depth()
            await self._assess_lod()
        except Exception:
            pass

    async def _on_judgment_for_compressor(self, event: Event) -> None:
        try:
            p = event.payload
            verdict = p.get("verdict", "?")
            q = p.get("q_score", 0.0)
            sk = p.get("state_key", "")
            preview = str(p.get("content_preview", ""))[:120].replace("\n", " ")
            summary = f"[{verdict} Q={q:.1f}] {sk}: {preview}"
            self.compressor.add(summary)

            # γ2: Checkpoint session every CHECKPOINT_EVERY (F(8)=21) judgments
            self._checkpoint_counter += 1
            if self._checkpoint_counter % CHECKPOINT_EVERY == 0:
                _session_checkpoint.save(self.compressor)
        except Exception:
            pass  # Never block on compressor errors

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 4 — Axiom chain handlers
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_emergence_signal(self, event: Event) -> None:
        """EMERGENCE_DETECTED → signal "EMERGENCE" axiom."""
        try:
            new_state = await self._signal_axiom("EMERGENCE", "emergence_detector")
        except Exception:
            pass

    async def _on_decision_made_for_axiom(self, event: Event) -> None:
        """DECISION_MADE → signal "AUTONOMY" axiom."""
        try:
            new_state = await self._signal_axiom("AUTONOMY", "decide_agent")
        except Exception:
            pass

    async def _on_decision_made_for_run(self, event: Event) -> None:
        """DECISION_MADE → RUN EScore + EMERGENCE on confident BARK."""
        try:
            p       = event.payload or {}
            q_value = float(p.get("q_value", 0.0))   # [0, 1]
            verdict = p.get("recommended_action", "")


            # RUN EScore — decision quality as execution efficiency
            run_score = q_value * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "RUN", run_score)

            # EMERGENCE: confident BARK = organism sure of a critical problem
            emergence_signalled = False
            if verdict == "BARK" and q_value >= PHI_INV_2:
                new_state = await self._signal_axiom("EMERGENCE", "decision_made", trigger="CONFIDENT_BARK")
                emergence_signalled = True

            logger.debug(
                "DECISION_MADE: action=%s q=%.3f → RUN EScore=%.1f%s",
                verdict, q_value, run_score,
                " EMERGENCE signalled" if emergence_signalled else "",
            )
        except Exception:
            pass

    async def _on_axiom_activated(self, event: Event) -> None:
        """AXIOM_ACTIVATED → log milestone; emit TRANSCENDENCE when all A6-A9 active."""
        try:
            axiom_name = event.payload.get("axiom", "?")
            maturity = event.payload.get("maturity", 0.0)
            active = self.axiom_monitor.active_axioms()
            logger.info(
                "AXIOM_ACTIVATED: %s (maturity=%.1f) — active: %s",
                axiom_name, maturity, active,
            )
            # A11 TRANSCENDENCE: check specifically A6-A9 (not A10/A11 themselves)
            a6_a9_active = [a for a in active if a in self._A6_A9]
            if len(a6_a9_active) == 4:
                await get_core_bus().emit(Event(
                    type=CoreEvent.TRANSCENDENCE,
                    payload={
                        "active_axioms": active,
                        "tier": "TRANSCENDENT",
                        "trigger": f"AXIOM_ACTIVATED:{axiom_name}",
                    },
                    source="axiom_monitor",
                ))
        except Exception:
            pass

    async def _on_self_improvement_proposed(self, event: Event) -> None:
        """SELF_IMPROVEMENT_PROPOSED → ActionProposer + A10 + JUDGE."""
        try:
            p         = event.payload or {}
            proposals = p.get("proposals", [])
            severity  = float(p.get("severity", 0.0))

            # Route proposals to ActionProposer (existing behavior)
            for prop in proposals:
                self.action_proposer.propose_self_improvement(prop)

            if not proposals:
                return  # No proposals generated → not a consciousness event


            # A10 CONSCIOUSNESS: self-analysis happened = organism aware of own state
            new_state = await self._signal_axiom(
                "CONSCIOUSNESS", "self_improvement",
                trigger="SELF_IMPROVEMENT_PROPOSED", count=len(proposals),
            )

            # JUDGE EScore: severity of self-analysis = self-judgment quality
            judge_score = severity * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)

            logger.info(
                "SELF_IMPROVEMENT_PROPOSED: count=%d severity=%.3f → "
                "CONSCIOUSNESS signalled, JUDGE=%.1f",
                len(proposals), severity, judge_score,
            )
        except Exception:
            pass

    async def _on_transcendence(self, event: Event) -> None:
        """TRANSCENDENCE → EScore self-reward + milestone log."""
        try:
            active = (event.payload or {}).get("active_axioms", [])
            logger.warning(
                "TRANSCENDENCE — all 4 emergent axioms active: %s",
                active,
            )
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", MAX_Q_SCORE)
        except Exception:
            pass

    async def _on_residual_high(self, event: Event) -> None:
        """RESIDUAL_HIGH → EMERGENCE signal + EScore JUDGE penalty."""
        try:
            p = event.payload or {}
            residual = float(p.get("residual_variance", 0.0))
            cell_id  = p.get("cell_id", "")

            # 1. THE_UNNAMEABLE = EMERGENCE by definition
            new_state = await self._signal_axiom("EMERGENCE", "residual_high")

            # 2. EScore JUDGE penalty — inversely proportional to residual
            penalty_score = (1.0 - min(residual, 1.0)) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", penalty_score)

            logger.warning(
                "RESIDUAL_HIGH: cell=%s residual=%.3f → EMERGENCE signal, "
                "JUDGE EScore penalty=%.1f",
                cell_id, residual, penalty_score,
            )
        except Exception:
            pass

    async def _on_action_proposed(self, event: Event) -> None:
        """ACTION_PROPOSED → EScore BUILD update."""
        try:
            p = event.payload or {}
            priority    = int(p.get("priority", 3))
            action_type = p.get("action_type", "")

            score = {
                1: MAX_Q_SCORE,
                2: HOWL_MIN,
                3: WAG_MIN,
            }.get(priority, GROWL_MIN)

            self.escore_tracker.update(_ESCORE_AGENT_ID, "BUILD", score)
            logger.info(
                "ACTION_PROPOSED: type=%s priority=%d → BUILD EScore=%.1f",
                action_type, priority, score,
            )
        except Exception:
            pass

    async def _on_meta_cycle(self, event: Event) -> None:
        """META_CYCLE → ANTIFRAGILITY signal + EScore JUDGE update."""
        try:
            p = event.payload or {}
            evolve = p.get("evolve", {})
            pass_rate  = float(evolve.get("pass_rate", 0.0))
            regression = bool(evolve.get("regression", False))


            # 1. ANTIFRAGILITY signal — regression = organism under stress
            if regression:
                new_state = await self._signal_axiom(
                    "ANTIFRAGILITY", "meta_cycle", trigger="META_CYCLE_REGRESSION",
                )

            # 1b. A10 CONSCIOUSNESS — organism accurately perceives its own state
            if pass_rate >= PHI_INV:
                new_state = await self._signal_axiom(
                    "CONSCIOUSNESS", "meta_cycle",
                    trigger="META_CYCLE_HEALTH", pass_rate=round(pass_rate, 3),
                )

            # 2. EScore JUDGE update — self-assessment quality
            if pass_rate >= PHI_INV:
                judge_score = pass_rate * MAX_Q_SCORE
            elif pass_rate >= PHI_INV_2:
                judge_score = WAG_MIN
            else:
                judge_score = GROWL_MIN

            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)
            logger.info(
                "META_CYCLE: pass_rate=%.1f%% regression=%s → "
                "JUDGE EScore=%.1f%s",
                pass_rate * 100, regression, judge_score,
                " ANTIFRAGILITY signalled" if regression else "",
            )
        except Exception:
            pass

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 5 — EScore dimension handlers
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_judgment_for_burn(self, event: Event) -> None:
        """JUDGMENT_CREATED → EScore BURN update."""
        try:
            p          = JudgmentCreatedPayload.model_validate(event.payload or {})
            confidence = p.confidence
            reality    = p.reality
            verdict    = p.verdict


            burn_score = min(confidence / MAX_CONFIDENCE, 1.0) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "BURN", burn_score, reality=reality)

            logger.debug(
                "JUDGMENT_CREATED→BURN: verdict=%s conf=%.3f → BURN EScore=%.1f",
                verdict, confidence, burn_score,
            )
        except Exception:
            pass

    async def _on_learning_event(self, event: Event) -> None:
        """LEARNING_EVENT → AUTONOMY signal + EScore JUDGE (high-frequency)."""
        try:
            p      = event.payload or {}
            reward = float(p.get("reward", 0.0))   # [0, 1]
            action = p.get("action", "")


            # 1. JUDGE EScore — judgment quality per cycle
            judge_score = reward * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)

            # 2. AUTONOMY — organism learning without human intervention
            new_state = await self._signal_axiom("AUTONOMY", "learning_event", trigger="LEARNING_EVENT")

            logger.debug(
                "LEARNING_EVENT: action=%s reward=%.3f → JUDGE EScore=%.1f%s",
                action, reward, judge_score,
                " AUTONOMY signalled" if new_state == "ACTIVE" else "",
            )
        except Exception:
            pass

    async def _on_consciousness_changed(self, event: Event) -> None:
        """CONSCIOUSNESS_CHANGED → ANTIFRAGILITY signal + EScore HOLD update."""
        try:
            p         = event.payload or {}
            direction = p.get("direction", "DOWN")
            to_name   = p.get("to_name", "")


            # HOLD: commitment quality — recovering = holding; degrading = yielding
            hold_score = HOWL_MIN if direction == "UP" else GROWL_MIN
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", hold_score)

            # ANTIFRAGILITY: only on recovery (UP) — survived stress and bounced back
            if direction == "UP":
                new_state = await self._signal_axiom(
                    "ANTIFRAGILITY", "consciousness_changed", trigger="LOD_RECOVERY",
                )

            logger.info(
                "CONSCIOUSNESS_CHANGED: %s → LOD=%s, HOLD=%.1f%s",
                direction, to_name, hold_score,
                " ANTIFRAGILITY signalled" if direction == "UP" else "",
            )
        except Exception:
            pass

    async def _on_user_feedback(self, event: Event) -> None:
        """USER_FEEDBACK → EScore JUDGE update for agent:cynic."""
        try:
            p = event.payload or {}
            rating = float(p.get("rating", 3.0))


            # JUDGE: rating quality — proportional to [1, 5] → [0, MAX_Q_SCORE]
            judge_score = (rating - 1) / 4.0 * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)

            # SOCIAL: human engaged with CYNIC = direct community interaction.
            self.escore_tracker.update(_ESCORE_AGENT_ID, "SOCIAL", WAG_MIN)

            logger.info(
                "USER_FEEDBACK: rating=%d/5 → JUDGE EScore=%.1f SOCIAL=%.1f",
                int(rating), judge_score, WAG_MIN,
            )
        except Exception:
            pass

    async def _on_perception_received(self, event: Event) -> None:
        """PERCEPTION_RECEIVED → EScore SOCIAL + HOLD update."""
        try:
            p       = event.payload or {}
            reality = p.get("reality", "CODE")
            source  = p.get("source", "")


            # SOCIAL: direct community engagement vs background monitoring
            social_score = (
                WAG_MIN
                if reality in ("SOCIAL", "HUMAN", "COSMOS")
                else GROWL_MIN
            )
            self.escore_tracker.update(_ESCORE_AGENT_ID, "SOCIAL", social_score, reality=reality)

            # HOLD: organism present + attentive; self-monitoring = highest commitment
            hold_score = HOWL_MIN if reality == "CYNIC" else WAG_MIN
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", hold_score, reality=reality)

            logger.debug(
                "PERCEPTION_RECEIVED: reality=%s source=%s → SOCIAL=%.1f HOLD=%.1f",
                reality, source, social_score, hold_score,
            )
        except Exception:
            pass

    async def _on_ewc_checkpoint(self, event: Event) -> None:
        """EWC_CHECKPOINT → AUTONOMY signal + EScore JUDGE update."""
        try:
            p         = event.payload or {}
            q_value   = float(p.get("q_value", 0.5))
            state_key = p.get("state_key", "")
            action    = p.get("action", "")


            judge_score = q_value * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)

            new_state = await self._signal_axiom("AUTONOMY", "ewc_checkpoint", trigger="EWC_CHECKPOINT")

            # A10 CONSCIOUSNESS: consolidation = organism KNOWS this (state, action) pair
            new_state_c = await self._signal_axiom(
                "CONSCIOUSNESS", "ewc_checkpoint",
                trigger="EWC_CHECKPOINT", q_value=round(q_value, 3),
            )

            logger.info(
                "EWC_CHECKPOINT: state=%s action=%s q=%.3f → JUDGE EScore=%.1f%s%s",
                state_key, action, q_value, judge_score,
                " AUTONOMY signalled" if new_state == "ACTIVE" else "",
                " CONSCIOUSNESS signalled" if new_state_c == "ACTIVE" else "",
            )
        except Exception:
            pass

    async def _on_q_table_updated(self, event: Event) -> None:
        """Q_TABLE_UPDATED → BUILD + HOLD EScore update."""
        try:
            p       = event.payload or {}
            flushed = int(p.get("flushed", 0))


            # BUILD: persisting knowledge = building durable memory
            self.escore_tracker.update(_ESCORE_AGENT_ID, "BUILD", HOWL_MIN)

            # HOLD: routine persistence = steady long-term commitment
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", WAG_MIN)

            logger.info(
                "Q_TABLE_UPDATED: flushed=%d → BUILD=%.1f HOLD=%.1f",
                flushed, HOWL_MIN, WAG_MIN,
            )
        except Exception:
            pass

    async def _on_consensus_reached(self, event: Event) -> None:
        """CONSENSUS_REACHED → SYMBIOSIS signal + EScore BUILD update."""
        try:
            p       = event.payload or {}
            q_score = float(p.get("q_score", 0.0))
            votes   = int(p.get("votes", 0))
            verdict = p.get("verdict", "")

            # BUILD: collaborative judgment quality
            self.escore_tracker.update(_ESCORE_AGENT_ID, "BUILD", q_score)

            # SYMBIOSIS: dogs working as one
            new_state = await self._signal_axiom("SYMBIOSIS", "consensus_reached", trigger="CONSENSUS_REACHED")

            # A10 CONSCIOUSNESS: pack converged = organism knows what it decided.
            new_state_c = await self._signal_axiom(
                "CONSCIOUSNESS", "consensus_reached",
                trigger="CONSENSUS_REACHED", verdict=verdict, q_score=round(q_score, 1),
            )

            logger.debug(
                "CONSENSUS_REACHED: votes=%d verdict=%s q=%.1f → BUILD=%.1f%s%s",
                votes, verdict, q_score, q_score,
                " SYMBIOSIS signalled" if new_state == "ACTIVE" else "",
                " CONSCIOUSNESS signalled" if new_state_c == "ACTIVE" else "",
            )
        except Exception:
            pass

    async def _on_consensus_failed(self, event: Event) -> None:
        """CONSENSUS_FAILED → EMERGENCE signal + EScore JUDGE penalty."""
        try:
            p      = event.payload or {}
            votes  = int(p.get("votes", 0))
            quorum = int(p.get("quorum", 7))


            # JUDGE penalty: proportional to how close dogs got to consensus
            judge_score = (votes / max(quorum, 1)) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", judge_score)

            # EMERGENCE: dogs can't agree = hidden complexity = emergence point
            new_state = await self._signal_axiom("EMERGENCE", "consensus_failed", trigger="CONSENSUS_FAILED")

            logger.info(
                "CONSENSUS_FAILED: votes=%d/%d → JUDGE EScore=%.1f%s",
                votes, quorum, judge_score,
                " EMERGENCE signalled" if new_state == "ACTIVE" else "",
            )
        except Exception:
            pass

    async def _on_user_correction(self, event: Event) -> None:
        """USER_CORRECTION → ANTIFRAGILITY signal + EScore JUDGE penalty."""
        try:
            p         = event.payload or {}
            action    = p.get("action", "")
            state_key = p.get("state_key", "")

            # JUDGE penalty: explicit correction = judgment quality = 0
            self.escore_tracker.update(_ESCORE_AGENT_ID, "JUDGE", 0.0)

            # ANTIFRAGILITY: correction = adversarial stress → organism grows
            new_state = await self._signal_axiom("ANTIFRAGILITY", "user_correction", trigger="USER_CORRECTION")

            logger.info(
                "USER_CORRECTION: state=%s action=%s → JUDGE=0.0%s",
                state_key, action,
                " ANTIFRAGILITY signalled" if new_state == "ACTIVE" else "",
            )
        except Exception:
            pass

    async def _on_anomaly_detected(self, event: Event) -> None:
        """ANOMALY_DETECTED → EScore HOLD (severity-based)."""
        try:
            p        = event.payload or {}
            severity = float(p.get("severity", 0.5))
            reality  = p.get("reality", "CODE")
            analysis = p.get("analysis", "JUDGE")


            # HOLD: stability inversely proportional to SPIKE severity
            hold_score = (1.0 - min(severity, 1.0)) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", hold_score)

            logger.info(
                "ANOMALY_DETECTED: SPIKE severity=%.3f at %s·%s → HOLD EScore=%.1f",
                severity, reality, analysis, hold_score,
            )
        except Exception:
            pass

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 6 — SDK / ACT handlers
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_act_completed(self, event: Event) -> None:
        """ACT_COMPLETED → QTable + EScore BUILD + mark_completed + JSONL log."""
        try:
            from cynic.learning.qlearning import LearningSignal as _LS
            p         = ActCompletedPayload.model_validate(event.payload or {})
            action_id = p.action_id
            success   = p.success
            cost      = p.cost_usd
            exec_id   = p.exec_id

            # 1. Mark action COMPLETED / FAILED in the queue
            if action_id:
                self.action_proposer.mark_completed(action_id, success)

            # 2. QTable: reward = WAG-level on success, GROWL-level on failure
            reward = (WAG_MIN / 100.0) if success else (GROWL_MIN / 100.0)
            self.qtable.update(_LS(
                state_key="ACT_COMPLETED",
                action="EXECUTE",
                reward=reward,
                judgment_id=exec_id,
                loop_name="ACT_COMPLETED",
            ))

            # 3. EScore BUILD dimension — successful ACT builds the organism
            escore_score = HOWL_MIN if success else GROWL_MIN
            self.escore_tracker.update(_ESCORE_AGENT_ID, "BUILD", escore_score)

            # 4. JSONL log (rolling cap F(11)=89)
            try:
                import json as _json
                record = {
                    "ts": time.time(),
                    "action_id": action_id,
                    "exec_id":   exec_id,
                    "success":   success,
                    "cost_usd":  cost,
                }
                os.makedirs(os.path.dirname(self._ACT_LOG_PATH), exist_ok=True)
                lines: list = []
                if os.path.exists(self._ACT_LOG_PATH):
                    with open(self._ACT_LOG_PATH, "r", encoding="utf-8") as _fh:
                        lines = _fh.readlines()
                lines.append(_json.dumps(record) + "\n")
                if len(lines) > self._ACT_LOG_CAP:
                    lines = lines[-self._ACT_LOG_CAP:]
                with open(self._ACT_LOG_PATH, "w", encoding="utf-8") as _fh:
                    _fh.writelines(lines)
            except Exception:
                pass

            logger.info(
                "*%s* ACT_COMPLETED: action=%s exec=%s success=%s cost=$%.4f → BUILD=%.1f",
                "tail wag" if success else "GROWL",
                action_id or "(auto)", exec_id, success, cost, escore_score,
            )
        except Exception:
            pass

    async def _on_sdk_tool_judged(self, event: Event) -> None:
        """SDK_TOOL_JUDGED → SYMBIOSIS signal + GRAPH EScore update."""
        try:
            p       = SdkToolJudgedPayload.model_validate(event.payload or {})
            verdict = p.verdict
            tool    = p.tool


            graph_score = {
                "HOWL":  HOWL_MIN,
                "WAG":   WAG_MIN,
                "GROWL": GROWL_MIN,
                "BARK":  0.0,
            }.get(verdict, WAG_MIN)

            self.escore_tracker.update(_ESCORE_AGENT_ID, "GRAPH", graph_score)

            # SYMBIOSIS: seamless human+machine tool use
            if verdict == "HOWL":
                new_state = await self._signal_axiom("SYMBIOSIS", "sdk_tool_judged", trigger="SDK_TOOL_HOWL")

            logger.info(
                "SDK_TOOL_JUDGED: tool=%s verdict=%s → GRAPH EScore=%.1f%s",
                tool, verdict, graph_score,
                " SYMBIOSIS signalled" if verdict == "HOWL" else "",
            )
        except Exception:
            pass

    async def _on_sdk_session_started(self, event: Event) -> None:
        """SDK_SESSION_STARTED → GRAPH EScore baseline + SYMBIOSIS signal."""
        try:
            p = event.payload or {}
            session_id = p.get("session_id", "")


            # 1. Neutral trust baseline for new session — GRAPH dimension
            self.escore_tracker.update(_ESCORE_AGENT_ID, "GRAPH", WAG_MIN)

            # 2. SYMBIOSIS: human+machine collaboration is beginning
            new_state = await self._signal_axiom("SYMBIOSIS", "sdk_session_started", trigger="SDK_SESSION_STARTED")

            logger.info(
                "SDK_SESSION_STARTED: session=%s → GRAPH EScore=%.1f SYMBIOSIS signalled",
                session_id, WAG_MIN,
            )
        except Exception:
            pass

    async def _on_sdk_result_received(self, event: Event) -> None:
        """SDK_RESULT_RECEIVED → BUILD + RUN EScore + ANTIFRAGILITY signal."""
        try:
            p = event.payload or {}
            session_id     = p.get("session_id", "")
            is_error       = bool(p.get("is_error", False))
            cost_usd       = float(p.get("cost_usd", 0.0))
            output_q_score = float(p.get("output_q_score", 0.0))


            # 1. BUILD EScore — ACT quality feeds CYNIC's code contribution dimension
            self.escore_tracker.update(_ESCORE_AGENT_ID, "BUILD", output_q_score)

            # 2. RUN EScore — efficiency from cost (φ-bounded thresholds)
            if cost_usd == 0.0:
                run_score = HOWL_MIN        # free Ollama = peak efficiency
            elif cost_usd < PHI_INV / 100:  # < $0.00618
                run_score = WAG_MIN
            else:
                run_score = GROWL_MIN
            self.escore_tracker.update(_ESCORE_AGENT_ID, "RUN", run_score)

            # 3. ANTIFRAGILITY — track outcome in rolling window
            success = not is_error
            self._sdk_outcome_window.append(success)
            if len(self._sdk_outcome_window) > self._SDK_OUTCOME_WINDOW:
                self._sdk_outcome_window.pop(0)

            had_prior_stress = (
                len(self._sdk_outcome_window) > 1
                and any(not ok for ok in self._sdk_outcome_window[:-1])
            )
            if success and had_prior_stress:
                new_state = await self._signal_axiom(
                    "ANTIFRAGILITY", "sdk_result_received", trigger="SDK_RESULT_RECOVERY",
                )

            logger.info(
                "SDK_RESULT_RECEIVED: session=%s is_error=%s cost=$%.4f "
                "→ BUILD=%.1f RUN=%.1f%s",
                session_id, is_error, cost_usd,
                output_q_score, run_score,
                " ANTIFRAGILITY signalled" if (success and had_prior_stress) else "",
            )
        except Exception:
            pass

    async def _on_act_requested_for_organism(self, event: Event) -> None:
        """ACT_REQUESTED → EScore HOLD + SOCIAL + AUTONOMY signal."""
        try:
            p           = event.payload or {}
            action_type = p.get("action_type", "")
            reality     = p.get("reality", "CODE")


            # HOLD: execution = peak long-term commitment
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", HOWL_MIN)
            # SOCIAL: organism engaging with external systems
            self.escore_tracker.update(_ESCORE_AGENT_ID, "SOCIAL", WAG_MIN)

            # AUTONOMY: organism taking autonomous action — most definitive form
            new_state = await self._signal_axiom("AUTONOMY", "act_requested", trigger="ACT_REQUESTED")

            logger.info(
                "ACT_REQUESTED: type=%s reality=%s → HOLD=%.1f SOCIAL=%.1f%s",
                action_type, reality, HOWL_MIN, WAG_MIN,
                " AUTONOMY signalled" if new_state == "ACTIVE" else "",
            )
        except Exception:
            pass

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 7 — Health / disk / memory handlers
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_disk_pressure(self, event: Event) -> None:
        try:
            pressure = event.payload.get("pressure", "WARN")
            used_pct = event.payload.get("used_pct", 0.0)
            self._health_cache["disk_pct"] = event.payload.get("disk_pct", used_pct)
            await self._assess_lod()

            hold_score = (1.0 - min(used_pct, 1.0)) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", hold_score)

            logger.warning(
                "DISK_PRESSURE: %s (%.1f%% used) → HOLD EScore=%.1f, running StorageGC",
                pressure, used_pct * 100, hold_score,
            )
            result = await self.storage_gc.collect(self.db_pool)
            if result.get("total", 0) > 0:
                logger.info("StorageGC freed %d rows (disk was %.1f%% full)", result["total"], used_pct * 100)
        except Exception:
            pass

    async def _on_memory_pressure(self, event: Event) -> None:
        try:
            pressure = event.payload.get("pressure", "WARN")
            used_pct = event.payload.get("used_pct", 0.0)
            self._health_cache["memory_pct"] = event.payload.get("memory_pct", used_pct)
            await self._assess_lod()

            hold_score = (1.0 - min(used_pct, 1.0)) * MAX_Q_SCORE
            self.escore_tracker.update(_ESCORE_AGENT_ID, "HOLD", hold_score)

            logger.warning(
                "MEMORY_PRESSURE: %s (%.1f%% RAM used) → HOLD EScore=%.1f",
                pressure, used_pct * 100, hold_score,
            )
        except Exception:
            pass

    async def _on_disk_cleared(self, event: Event) -> None:
        try:
            actual_pct = event.payload.get("disk_pct", 0.0)
            self._health_cache["disk_pct"] = actual_pct
            await self._assess_lod()
            logger.info("DISK_CLEARED: disk_pct=%.2f%% → LOD=%s",
                        actual_pct * 100, self.lod_controller.current.name)
        except Exception:
            pass

    async def _on_memory_cleared(self, event: Event) -> None:
        try:
            actual_pct = event.payload.get("memory_pct", 0.0)
            self._health_cache["memory_pct"] = actual_pct
            await self._assess_lod()
            logger.info("MEMORY_CLEARED: memory_pct=%.2f%% → LOD=%s",
                        actual_pct * 100, self.lod_controller.current.name)
        except Exception:
            pass

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 8 — Wire event subscriptions
    # ═══════════════════════════════════════════════════════════════════════

    def _wire_event_handlers(self) -> None:
        """Register all event bus subscriptions."""
        bus = get_core_bus()

        # Budget
        bus.on(CoreEvent.BUDGET_WARNING,   self._on_budget_warning)
        bus.on(CoreEvent.BUDGET_EXHAUSTED, self._on_budget_exhausted)

        # Intelligence cycle
        bus.on(CoreEvent.JUDGMENT_CREATED,   self._on_judgment_for_intelligence)
        bus.on(CoreEvent.JUDGMENT_FAILED,    self._on_judgment_failed)
        bus.on(CoreEvent.JUDGMENT_REQUESTED, self._on_judgment_requested)

        # Axiom chain
        bus.on(CoreEvent.EMERGENCE_DETECTED,        self._on_emergence_signal)
        bus.on(CoreEvent.DECISION_MADE,             self._on_decision_made_for_axiom)
        bus.on(CoreEvent.DECISION_MADE,             self._on_decision_made_for_run)
        bus.on(CoreEvent.AXIOM_ACTIVATED,           self._on_axiom_activated)
        bus.on(CoreEvent.SELF_IMPROVEMENT_PROPOSED, self._on_self_improvement_proposed)
        bus.on(CoreEvent.TRANSCENDENCE,             self._on_transcendence)
        bus.on(CoreEvent.RESIDUAL_HIGH,             self._on_residual_high)
        bus.on(CoreEvent.ACTION_PROPOSED,           self._on_action_proposed)
        bus.on(CoreEvent.META_CYCLE,                self._on_meta_cycle)

        # EScore dimensions
        bus.on(CoreEvent.JUDGMENT_CREATED,     self._on_judgment_for_burn)
        bus.on(CoreEvent.LEARNING_EVENT,       self._on_learning_event)
        bus.on(CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_changed)
        bus.on(CoreEvent.USER_FEEDBACK,        self._on_user_feedback)
        bus.on(CoreEvent.PERCEPTION_RECEIVED,  self._on_perception_received)
        bus.on(CoreEvent.EWC_CHECKPOINT,       self._on_ewc_checkpoint)
        bus.on(CoreEvent.Q_TABLE_UPDATED,      self._on_q_table_updated)
        bus.on(CoreEvent.CONSENSUS_REACHED,    self._on_consensus_reached)
        bus.on(CoreEvent.CONSENSUS_FAILED,     self._on_consensus_failed)
        bus.on(CoreEvent.USER_CORRECTION,      self._on_user_correction)
        bus.on(CoreEvent.ANOMALY_DETECTED,     self._on_anomaly_detected)

        # SDK / ACT
        bus.on(CoreEvent.ACT_COMPLETED,        self._on_act_completed)
        bus.on(CoreEvent.SDK_TOOL_JUDGED,      self._on_sdk_tool_judged)
        bus.on(CoreEvent.SDK_SESSION_STARTED,  self._on_sdk_session_started)
        bus.on(CoreEvent.SDK_RESULT_RECEIVED,  self._on_sdk_result_received)
        bus.on(CoreEvent.ACT_REQUESTED,        self._on_act_requested_for_organism)

        # Health watchers
        bus.on(CoreEvent.DISK_PRESSURE,   self._on_disk_pressure)
        bus.on(CoreEvent.DISK_CLEARED,    self._on_disk_cleared)
        bus.on(CoreEvent.MEMORY_PRESSURE, self._on_memory_pressure)
        bus.on(CoreEvent.MEMORY_CLEARED,  self._on_memory_cleared)

        # Guidance feedback loop (global handler — not a method)
        bus.on(CoreEvent.JUDGMENT_CREATED, _on_judgment_created)

        # ContextCompressor accumulation
        bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_compressor)

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 9 — Perceive workers + AppState assembly
    # ═══════════════════════════════════════════════════════════════════════

    def _wire_perceive_workers(self) -> None:
        """Register all autonomous sensor workers with the scheduler."""
        # CODE×PERCEIVE — git working tree changes
        self.scheduler.register_perceive_worker(GitWatcher())
        # CYNIC×PERCEIVE — timer degradation
        self.scheduler.register_perceive_worker(HealthWatcher())
        # CYNIC×LEARN — Q-Table health self-monitoring
        self.scheduler.register_perceive_worker(SelfWatcher(qtable_getter=lambda: self.qtable))
        # MARKET×PERCEIVE — SOL/USD price significant moves
        self.scheduler.register_perceive_worker(MarketWatcher())
        # SOLANA×PERCEIVE — mainnet slot + TPS anomalies
        self.scheduler.register_perceive_worker(SolanaWatcher())
        # SOCIAL×PERCEIVE — social signals from ~/.cynic/social.json feed
        self.scheduler.register_perceive_worker(SocialWatcher())
        # CYNIC×PERCEIVE — disk pressure (φ-thresholds: 61.8% / 76.4% / 90%)
        self.scheduler.register_perceive_worker(DiskWatcher())
        # CYNIC×PERCEIVE — RAM pressure (same φ-thresholds, no psutil needed)
        self.scheduler.register_perceive_worker(MemoryWatcher())

    def _make_app_state(self) -> AppState:
        """Assemble the final AppState from all components."""
        logger.info(
            "Kernel ready: %d dogs, scheduler wired, learning loop + residual detector active, pool=%s, llm=%s",
            len(self.dogs),
            "connected" if self.db_pool else "none",
            f"{len(self.registry.get_available())} adapters" if self.registry else "none",
        )
        return AppState(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            scheduler=self.scheduler,
            _pool=self.db_pool,
            decide_agent=self.decide_agent,
            account_agent=self.account_agent,
            context_compressor=self.compressor,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
            escore_tracker=self.escore_tracker,
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
            world_model=self.world_model,
            llm_router=self.llm_router,
            kernel_mirror=KernelMirror(),
        )

    def build(self) -> AppState:
        """Build and return the fully-wired AppState."""
        self._create_components()
        self._wire_event_handlers()
        self._wire_perceive_workers()
        return self._make_app_state()


def build_kernel(db_pool=None, registry=None) -> AppState:
    """Build the CYNIC kernel. Call once from lifespan startup."""
    return _KernelBuilder(db_pool, registry).build()


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
