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
from typing import Any, TYPE_CHECKING, Optional


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
    AxiomActivatedPayload,
    ConsciousnessChangedPayload,
    JudgmentCreatedPayload,
    SdkToolJudgedPayload,
    TranscendencePayload,
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
from cynic.cognition.neurons.base import AbstractDog, DogId
from cynic.cognition.neurons.discovery import discover_dogs
from cynic.cognition.neurons.oracle import OracleDog
from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.cognition.cortex.residual import ResidualDetector
from cynic.cognition.cortex.decide import DecideAgent
from cynic.cognition.cortex.account import AccountAgent
from cynic.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.cognition.cortex.action_proposer import ActionProposer
from cynic.cognition.cortex.self_probe import SelfProber
from cynic.cognition.cortex.lod import LODController, SurvivalLOD
from cynic.core.escore import EScoreTracker
from cynic.learning.qlearning import QTable, LearningLoop
from cynic.senses.workers import GitWatcher, HealthWatcher, SelfWatcher, MarketWatcher, SolanaWatcher, SocialWatcher, DiskWatcher, MemoryWatcher
from cynic.core.storage.gc import StorageGarbageCollector
from cynic.senses import checkpoint as _session_checkpoint
from cynic.senses.checkpoint import CHECKPOINT_EVERY
from cynic.cognition.cortex.orchestrator import ConsciousnessScheduler
from cynic.metabolism.telemetry import TelemetryStore
from cynic.metabolism.llm_router import LLMRouter
from cynic.metabolism.runner import ClaudeCodeRunner
from cynic.metabolism.auto_benchmark import AutoBenchmark
from cynic.metabolism.universal import UniversalActuator
from cynic.cognition.cortex.mirror import KernelMirror
from cynic.llm.adapter import LLMRegistry
from cynic.senses.compressor import ContextCompressor
from cynic.core.container import DependencyContainer
from cynic.organism.organism import Organism

logger = logging.getLogger("cynic.api.state")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")

# The organism's own identity in the EScore reputation system.
# Used everywhere CYNIC tracks its own performance — 33 call sites, one constant.
_ESCORE_AGENT_ID = "agent:cynic"

# Instance ID for multi-instance guidance isolation (set from lifespan startup via set_instance_id())
_current_instance_id: str | None = None


def set_instance_id(instance_id: str) -> None:
    """Set the instance ID used for guidance-{id}.json isolation (T35)."""
    global _current_instance_id
    _current_instance_id = instance_id


# ════════════════════════════════════════════════════════════════════════════
# APP CONTAINER — Instance-scoped state (no global singletons)
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class AppContainer:
    """
    Instance-scoped application state.

    Replaces global singletons (_state, _current_instance_id, _GUIDANCE_PATH).
    One container per FastAPI app instance.

    Passed via FastAPI dependency injection: routes call get_app_container().
    """
    organism: "CynicOrganism"
    instance_id: str  # Unique per process (uuid.uuid4().hex[:8])
    guidance_path: str  # ~/.cynic/guidance-{instance_id}.json
    started_at: float = field(default_factory=time.time)

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at


# Process-level singleton — set during lifespan startup
_app_container: Optional[AppContainer] = None


def set_app_container(container: AppContainer) -> None:
    """Set the app container during lifespan startup."""
    global _app_container
    _app_container = container


def get_app_container() -> AppContainer:
    """Get the app container (FastAPI dependency)."""
    if _app_container is None:
        raise RuntimeError("AppContainer not initialized — lifespan not started")
    return _app_container


# Backward compatibility: CynicOrganism is the Organism class
CynicOrganism = Organism


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
    except OSError as exc:
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
    scheduler: ConsciousnessScheduler
    started_at: float = field(default_factory=time.time)
    _pool: asyncpg.Pool | None = None
    last_judgment: dict | None = None  # state_key, action, judgment_id — for /feedback
    decide_agent: DecideAgent | None = None
    account_agent: AccountAgent | None = None
    runner: ClaudeCodeRunner | None = None
    llm_router: LLMRouter | None = None
    kernel_mirror: KernelMirror = field(default_factory=KernelMirror)  # Ring 3 self-reflection
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)  # session data
    context_compressor: ContextCompressor = field(default_factory=ContextCompressor)  # γ2 token budget
    axiom_monitor: AxiomMonitor = field(default_factory=AxiomMonitor)       # δ1 emergent axiom tracker
    lod_controller: LODController = field(default_factory=LODController)   # δ2 graceful degradation
    escore_tracker: EScoreTracker = field(default_factory=EScoreTracker)   # γ4 reputation scoring
    action_proposer: ActionProposer = field(default_factory=ActionProposer) # P5 action queue
    self_prober: SelfProber = field(default_factory=SelfProber)             # L4 self-improvement
    world_model: WorldModelUpdater = field(default_factory=WorldModelUpdater)  # T27 cross-reality aggregator
    auto_benchmark: AutoBenchmark | None = None
    universal_actuator: UniversalActuator = field(default_factory=UniversalActuator)
    container: DependencyContainer = field(default_factory=DependencyContainer)
    _handler_registry: object = field(default=None)  # HandlerRegistry — for introspection

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> list[str]:
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
        db_pool: asyncpg.Pool | None = None,
        registry: LLMRegistry | None = None,
    ) -> None:
        self.db_pool: asyncpg.Pool | None   = db_pool
        self.registry: LLMRegistry | None   = registry

        # ── Shared mutable state (used by multiple event handler methods) ──
        # Prevents the [0]-cell hack that was needed with closures.
        self._health_cache: dict[str, float] = {
            "error_rate": 0.0, "latency_ms": 0.0, "queue_depth": 0.0,
            "memory_pct": 0.0, "disk_pct": 0.0,
        }
        self._outcome_window:     list[bool] = []
        self._sdk_outcome_window: list[bool] = []
        self._escore_persist_counter = 0
        self._checkpoint_counter     = 0

        # ── Component placeholders (set by _create_components) ────────────
        self.dogs:             dict[DogId, AbstractDog] = {}
        self.qtable:           QTable           = None  # type: ignore[assignment]
        self.orchestrator:     JudgeOrchestrator = None  # type: ignore[assignment]
        self.scheduler:        ConsciousnessScheduler     = None  # type: ignore[assignment]
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
            await get_core_bus().emit(Event.typed(
                CoreEvent.AXIOM_ACTIVATED,
                AxiomActivatedPayload(
                    axiom=axiom,
                    maturity=self.axiom_monitor.get_maturity(axiom),
                    **extra,
                ),
                source=source,
            ))
        return new_state

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 1 — Component creation
    # ═══════════════════════════════════════════════════════════════════════

    def _create_components(self) -> None:
        """Create all kernel components and wire their mutual dependencies."""
        # Dogs — auto-discovered from cynic.dogs (no manual import list)
        # QTable must exist before OracleDog (Oracle reads it for predictions)
        self.qtable = QTable()
        self.dogs = discover_dogs(ORACLE=OracleDog(qtable=self.qtable))
        cynic_dog = self.dogs[DogId.CYNIC]

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

        # Scheduler needs oracle dog from orchestrator
        oracle_dog = self.dogs.get(DogId.ORACLE)
        self.scheduler = ConsciousnessScheduler(
            axiom_monitor=self.axiom_monitor,
            escore_tracker=self.escore_tracker,
            oracle_dog=oracle_dog
        )

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
    # NEW METHODS — Handler Registry + Services
    # ═══════════════════════════════════════════════════════════════════════

    def _create_services(self) -> object:  # Returns KernelServices
        """Create KernelServices — the organism's bloodstream."""
        from cynic.api.handlers.services import (
            KernelServices, CognitionServices, MetabolicServices, SensoryServices
        )

        # Create domain-specific service groups
        cognition_svc = CognitionServices(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
            escore_tracker=self.escore_tracker,
            health_cache=self._health_cache,
        )

        metabolic_svc = MetabolicServices(
            scheduler=self.scheduler,
            runner=getattr(self, 'runner', None),
            llm_router=self.llm_router,
            db_pool=self.db_pool,
        )

        # Placeholder service registry (TODO: find actual implementation or define interface)
        service_registry = {}
        senses_svc = SensoryServices(
            compressor=self.compressor,
            service_registry=service_registry,
            world_model=self.world_model,
        )

        # Combine into unified kernel services
        return KernelServices(
            cognition=cognition_svc,
            metabolic=metabolic_svc,
            senses=senses_svc,
        )

    def _create_handler_registry(self, svc: object) -> object:  # Returns HandlerRegistry
        """Discover + register handler groups."""
        from cynic.api.handlers import HandlerRegistry, discover_handler_groups
        registry = HandlerRegistry()
        groups = discover_handler_groups(
            svc,
            intelligence={
                "orchestrator": self.orchestrator,
                "scheduler": self.scheduler,
                "db_pool": self.db_pool,
                "compressor": self.compressor,
            },
            axiom={"action_proposer": self.action_proposer},
            sdk={"action_proposer": self.action_proposer, "qtable": self.qtable},
            health={"storage_gc": self.storage_gc, "db_pool": self.db_pool},
        )
        for group in groups:
            registry.register(group)
        logger.info("HandlerRegistry: %d groups discovered", len(groups))
        return registry

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 7b — Build DependencyContainer (parallel access path to components)
    # ═══════════════════════════════════════════════════════════════════════

    def _build_container(self) -> DependencyContainer:
        """Register all kernel components into the DI container."""
        from cynic.core.config import CynicConfig
        container = DependencyContainer(CynicConfig.from_env())
        container.register(QTable, self.qtable)
        container.register(JudgeOrchestrator, self.orchestrator)
        container.register(ResidualDetector, self.residual_detector)
        container.register(DecideAgent, self.decide_agent)
        container.register(AccountAgent, self.account_agent)
        container.register(ActionProposer, self.action_proposer)
        container.register(SelfProber, self.self_prober)
        container.register(ConsciousnessScheduler, self.scheduler)
        container.register(AxiomMonitor, self.axiom_monitor)
        container.register(LODController, self.lod_controller)
        container.register(EScoreTracker, self.escore_tracker)
        container.register(ContextCompressor, self.compressor)
        container.register(WorldModelUpdater, self.world_model)
        if self.registry is not None:
            container.register(LLMRegistry, self.registry)
        logger.info(
            "DependencyContainer: %d types registered — %s",
            len(container.registered_types), container.registered_types,
        )
        return container

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 8 — Wire event subscriptions
    # ═══════════════════════════════════════════════════════════════════════

    def _wire_event_handlers(self) -> None:
        """Register all event bus subscriptions via HandlerRegistry."""
        bus = get_core_bus()

        # Handler groups (auto-discovered, self-registering)
        self._handler_registry.wire(bus)

        # Module-level handlers (not part of any group)
        bus.on(CoreEvent.JUDGMENT_CREATED, _on_judgment_created)  # guidance.json

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
            container=self._container,
            _handler_registry=self._handler_registry,
        )

    def build(self) -> AppState:
        """Build and return the fully-wired AppState."""
        self._create_components()
        svc = self._create_services()
        self._handler_registry = self._create_handler_registry(svc)
        self._container = self._build_container()
        self._wire_event_handlers()
        self._wire_perceive_workers()
        return self._make_app_state()


def build_kernel(db_pool=None, registry=None) -> AppState:
    """Build the CYNIC kernel. Call once from lifespan startup."""
    return _KernelBuilder(db_pool, registry).build()


# Process-level singleton — set during lifespan startup
_state: AppState | None = None


def set_state(state: AppState) -> None:
    global _state
    _state = state


def get_state() -> AppState:
    if _state is None:
        raise RuntimeError("AppState not initialized — lifespan not started")
    return _state


def awaken(db_pool=None, registry=None) -> AppState:
    """Awaken the CYNIC organism. Call once from lifespan startup."""
    return build_kernel(db_pool, registry)


async def restore_state(state: AppState) -> None:
    """
    Restore persistent state after kernel startup.

    Call this in the FastAPI lifespan, AFTER build_kernel() and set_state().
    Restores:
      - EScoreTracker entities from e_scores table (γ4)
      - ContextCompressor session from ~/.cynic/session-latest.json (γ2)
    """
    from cynic.senses import checkpoint as _ckpt

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
