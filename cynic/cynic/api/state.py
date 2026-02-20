"""
CYNIC API State — Organism instance awakened at startup.

One CynicOrganism per process. Initialized via FastAPI lifespan.
All routes get this via Depends(get_state).
"""
from __future__ import annotations

import json
import os
import time
import logging
from dataclasses import dataclass, field
from typing import Any, TYPE_CHECKING


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
from cynic.nervous import ServiceStateRegistry, ComponentType, EventJournal, DecisionTracer, LoopClosureValidator
from cynic.senses.workers import GitWatcher, HealthWatcher, SelfWatcher, MarketWatcher, SolanaWatcher, SocialWatcher, DiskWatcher, MemoryWatcher
from cynic.core.storage.gc import StorageGarbageCollector
from cynic.senses import checkpoint as _session_checkpoint
from cynic.senses.checkpoint import CHECKPOINT_EVERY
from cynic.scheduler import ConsciousnessRhythm
from cynic.metabolism.telemetry import TelemetryStore
from cynic.metabolism.llm_router import LLMRouter
from cynic.metabolism.runner import ClaudeCodeRunner
from cynic.metabolism.auto_benchmark import AutoBenchmark
from cynic.metabolism.universal import UniversalActuator
from cynic.cognition.cortex.mirror import KernelMirror
from cynic.cognition.cortex.decision_validator import DecisionValidator, BlockedDecision
from cynic.llm.adapter import LLMRegistry
from cynic.senses.compressor import ContextCompressor
from cynic.core.container import DependencyContainer
from cynic.immune.power_limiter import PowerLimiter
from cynic.immune.alignment_checker import AlignmentSafetyChecker
from cynic.immune.human_approval_gate import HumanApprovalGate
from cynic.immune.transparency_audit import TransparencyAuditTrail
from cynic.core.topology import (
    SourceWatcher,
    IncrementalTopologyBuilder,
    HotReloadCoordinator,
    TopologyMirror,
    ChangeTracker,
    ChangeAnalyzer,
)
from cynic.core.convergence import ConvergenceValidator

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


# ═══════════════════════════════════════════════════════════════════════════════
# FAÇADES — 4 biological role groups (KILL GOD OBJECT)
# ═══════════════════════════════════════════════════════════════════════════════

@dataclass
class CognitionCore:
    """
    BRAIN — Judgment, learning, axioms, decisions, safety guardrails.

    φ-Explicit: F(6)=8 required fields form the Fibonacci-derived core:
      1. orchestrator     — Judgment engine
      2. qtable           — Learning memory
      3. learning_loop    — Meta-learning
      4. residual_detector — Anomaly detection
      5. power_limiter    — Resource guardian
      6. alignment_checker — Axiom validator
      7. human_gate       — Approval gating
      8. audit_trail      — Immutable record

    Optional fields extend cognition without changing φ-core:
      - decision_validator — Composite of guardrails (wired after instantiation)
      - decide_agent, account_agent — Specialized agents
      - axiom_monitor, lod_controller, escore_tracker — Meta-cognition (default_factory)
    """
    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    residual_detector: ResidualDetector
    power_limiter: PowerLimiter
    alignment_checker: AlignmentSafetyChecker
    human_gate: HumanApprovalGate
    audit_trail: TransparencyAuditTrail
    # Optional: Composite of the 4 required guardrails above
    decision_validator: DecisionValidator | None = None
    decide_agent: DecideAgent | None = None
    account_agent: AccountAgent | None = None
    axiom_monitor: AxiomMonitor = field(default_factory=AxiomMonitor)
    lod_controller: LODController = field(default_factory=LODController)
    escore_tracker: EScoreTracker = field(default_factory=EScoreTracker)


@dataclass
class MetabolicCore:
    """
    BODY — Execution, scheduling, routing, telemetry, actuators.

    φ-Explicit: F(5)=5 required fields form the Fibonacci-derived metabolism:
      1. scheduler        — Consciousness rhythm controller
      2. runner           — Autonomous executor (Claude Code SDK)
      3. llm_router       — LLM selection + routing
      4. telemetry_store  — Performance metrics
      5. universal_actuator — Action execution engine

    Optional fields extend metabolism without changing φ-core:
      - auto_benchmark    — Self-performance tracking
    """
    scheduler: ConsciousnessRhythm
    runner: ClaudeCodeRunner | None = None
    llm_router: LLMRouter | None = None
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)
    universal_actuator: UniversalActuator = field(default_factory=UniversalActuator)
    auto_benchmark: AutoBenchmark | None = None


@dataclass
class SensoryCore:
    """
    NERVOUS SYSTEM — Compression, registry, world model, topology consciousness.

    φ-Explicit: F(6)=8 required fields form the Fibonacci-derived nervous system:
      1. context_compressor    — Memory compression (TF-IDF, rolling cap)
      2. service_registry      — Runtime health tracking
      3. event_journal         — Event persistence
      4. decision_tracer       — Decision auditing
      5. loop_closure_validator — Loop integrity checks
      6. world_model           — Environmental state tracking
      7. source_watcher        — L0 Layer 1: source observation
      8. topology_builder      — L0 Layer 2: codebase topology mapping

    Optional fields extend perception without changing φ-core:
      - hot_reload_coordinator — L0 Layer 3: dynamic reload
      - topology_mirror        — L0 Layer 4: topology state
      - change_tracker         — L0 Layer 4.5: change detection
      - change_analyzer        — L0 Layer 4.6: change analysis
      - convergence_validator  — Phase 3: announcement vs outcome verification
    """
    context_compressor: ContextCompressor = field(default_factory=ContextCompressor)
    service_registry: ServiceStateRegistry = field(default_factory=ServiceStateRegistry)
    event_journal: EventJournal = field(default_factory=EventJournal)
    decision_tracer: DecisionTracer = field(default_factory=DecisionTracer)
    loop_closure_validator: LoopClosureValidator = field(default_factory=LoopClosureValidator)
    world_model: WorldModelUpdater = field(default_factory=WorldModelUpdater)
    source_watcher: SourceWatcher = field(default_factory=SourceWatcher)
    topology_builder: IncrementalTopologyBuilder = field(default_factory=IncrementalTopologyBuilder)
    hot_reload_coordinator: HotReloadCoordinator | None = None
    topology_mirror: TopologyMirror | None = None
    change_tracker: ChangeTracker | None = None
    change_analyzer: ChangeAnalyzer | None = None
    convergence_validator: ConvergenceValidator = field(default_factory=ConvergenceValidator)


@dataclass
class MemoryCore:
    """
    ARCHIVE — Reflection, proposals, self-improvement.

    φ-Explicit: F(4)=3 required fields form the Fibonacci-derived memory system:
      1. kernel_mirror   — Organism self-observation and consciousness snapshots
      2. action_proposer — Proposed action queue (DECISION_MADE → ProposedAction)
      3. self_prober     — Self-improvement proposals (L4 meta-cognition)
    """
    kernel_mirror: KernelMirror = field(default_factory=KernelMirror)
    action_proposer: ActionProposer = field(default_factory=ActionProposer)
    self_prober: SelfProber = field(default_factory=SelfProber)


@dataclass
class CynicOrganism:
    """
    The organism awakened and ready — thin envelope wrapping 4 façades.

    Built once at startup, lives for the process lifetime.
    """
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: MemoryCore
    started_at: float = field(default_factory=time.time)
    _pool: asyncpg.Pool | None = None
    last_judgment: dict | None = None  # state_key, action, judgment_id — for /feedback
    container: DependencyContainer = field(default_factory=DependencyContainer)
    _handler_registry: object = field(default=None)  # HandlerRegistry — for introspection

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> list[str]:
        return list(self.cognition.orchestrator.dogs.keys())

    # ── Backward-compat properties for old code paths ──
    @property
    def orchestrator(self) -> JudgeOrchestrator:
        return self.cognition.orchestrator

    @property
    def qtable(self) -> QTable:
        return self.cognition.qtable

    @property
    def learning_loop(self) -> LearningLoop:
        return self.cognition.learning_loop

    @property
    def residual_detector(self) -> ResidualDetector:
        return self.cognition.residual_detector

    @property
    def scheduler(self) -> ConsciousnessRhythm:
        return self.metabolism.scheduler

    @property
    def runner(self) -> ClaudeCodeRunner | None:
        return self.metabolism.runner

    @runner.setter
    def runner(self, value: ClaudeCodeRunner | None) -> None:
        self.metabolism.runner = value

    @property
    def llm_router(self) -> LLMRouter | None:
        return self.metabolism.llm_router

    @llm_router.setter
    def llm_router(self, value: LLMRouter | None) -> None:
        self.metabolism.llm_router = value

    @property
    def context_compressor(self) -> ContextCompressor:
        return self.senses.context_compressor

    @property
    def event_journal(self) -> EventJournal:
        return self.senses.event_journal

    @property
    def decision_tracer(self) -> DecisionTracer:
        return self.senses.decision_tracer

    @property
    def loop_closure_validator(self) -> LoopClosureValidator:
        return self.senses.loop_closure_validator

    @property
    def service_registry(self) -> ServiceStateRegistry:
        return self.senses.service_registry

    @property
    def world_model(self) -> WorldModelUpdater:
        return self.senses.world_model

    @property
    def kernel_mirror(self) -> KernelMirror:
        return self.memory.kernel_mirror

    @property
    def action_proposer(self) -> ActionProposer:
        return self.memory.action_proposer

    @property
    def self_prober(self) -> SelfProber:
        return self.memory.self_prober

    @property
    def decide_agent(self) -> DecideAgent | None:
        return self.cognition.decide_agent

    @property
    def account_agent(self) -> AccountAgent | None:
        return self.cognition.account_agent

    @property
    def axiom_monitor(self) -> AxiomMonitor:
        return self.cognition.axiom_monitor

    @property
    def lod_controller(self) -> LODController:
        return self.cognition.lod_controller

    @property
    def escore_tracker(self) -> EScoreTracker:
        return self.cognition.escore_tracker

    @property
    def telemetry_store(self) -> TelemetryStore:
        return self.metabolism.telemetry_store

    @property
    def universal_actuator(self) -> UniversalActuator:
        return self.metabolism.universal_actuator

    @property
    def auto_benchmark(self) -> AutoBenchmark | None:
        return self.metabolism.auto_benchmark

    @auto_benchmark.setter
    def auto_benchmark(self, value: AutoBenchmark | None) -> None:
        self.metabolism.auto_benchmark = value

    # ── Guardrails (backward-compat, accessed via cognition) ──
    @property
    def power_limiter(self) -> PowerLimiter:
        return self.cognition.power_limiter

    @property
    def alignment_checker(self) -> AlignmentSafetyChecker:
        return self.cognition.alignment_checker

    @property
    def human_gate(self) -> HumanApprovalGate:
        return self.cognition.human_gate

    @property
    def audit_trail(self) -> TransparencyAuditTrail:
        return self.cognition.audit_trail

    @property
    def convergence_validator(self) -> ConvergenceValidator:
        return self.senses.convergence_validator

    @property
    def decision_validator(self) -> DecisionValidator | None:
        return self.cognition.decision_validator

    # ── Topology system L0 (backward-compat, accessed via senses) ──
    @property
    def source_watcher(self) -> Any:
        return self.senses.source_watcher

    @property
    def topology_builder(self) -> Any:
        return self.senses.topology_builder

    @property
    def hot_reload_coordinator(self) -> Any:
        return self.senses.hot_reload_coordinator

    @property
    def topology_mirror(self) -> Any:
        return self.senses.topology_mirror

    @property
    def change_tracker(self) -> Any:
        return self.senses.change_tracker

    @property
    def change_analyzer(self) -> Any:
        return self.senses.change_analyzer


# Type alias — old code may reference AppState
AppState = CynicOrganism


class _OrganismAwakener:
    """
    One-time organism awakener.

    Holds all awakening-time state so event handler methods can close over
    ``self`` instead of a tangle of nested closures.  Call ``awaken()``
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
        self.scheduler:        ConsciousnessRhythm = None  # type: ignore[assignment]
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
        self.universal_actuator: UniversalActuator = None  # type: ignore[assignment]

        # ── Guardrails — Safety checks before ACT phase ──────────────────────
        self.power_limiter:       PowerLimiter              = None  # type: ignore[assignment]
        self.alignment_checker:   AlignmentSafetyChecker    = None  # type: ignore[assignment]
        self.human_gate:          HumanApprovalGate         = None  # type: ignore[assignment]
        self.audit_trail:         TransparencyAuditTrail    = None  # type: ignore[assignment]
        self.decision_validator:  DecisionValidator         = None  # type: ignore[assignment]

        # ── Topology System (L0: Real-time architecture awareness) ──────────
        self.source_watcher:   Any = None  # SourceWatcher
        self.topology_builder: Any = None  # IncrementalTopologyBuilder
        self.hot_reload_coordinator: Any = None  # HotReloadCoordinator
        self.topology_mirror:  Any = None  # TopologyMirror
        self.change_tracker:   Any = None  # ChangeTracker
        self.change_analyzer:  Any = None  # ChangeAnalyzer
        self.convergence_validator: Any = None  # ConvergenceValidator — Phase 3 observability

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
        # Dogs — auto-discovered from cynic.cognition.neurons (no manual import list)
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

        self.scheduler = ConsciousnessRhythm(orchestrator=self.orchestrator)

        self.decide_agent = DecideAgent(qtable=self.qtable)
        self.decide_agent.start(get_core_bus())

        self.action_proposer = ActionProposer()
        self.action_proposer.start(get_core_bus())

        self.account_agent = AccountAgent()

        # ── ClaudeCodeRunner (Metabolism × ACT) ───────────────────────────────
        # Spawns `claude --sdk-url` subprocess for autonomous task execution
        # Wired to MCP server for Claude Code integration
        runner_sessions: dict[str, Any] = {}  # session registry for ClaudeCodeRunner
        self.runner = ClaudeCodeRunner(
            bus=get_core_bus(),
            sessions_registry=runner_sessions,
            port=8765,  # Match FastAPI lifespan port config
        )
        logger.info("ClaudeCodeRunner initialized — autonomous Claude Code execution ready")

        self.llm_router = LLMRouter()
        self.universal_actuator = UniversalActuator()

        self.axiom_monitor  = AxiomMonitor()
        self.lod_controller = LODController()
        self.escore_tracker = EScoreTracker()

        # ── Tier 1 Nervous System: Service State Registry ──────────────────
        self.service_registry = ServiceStateRegistry()

        self.orchestrator.escore_tracker = self.escore_tracker
        self.orchestrator.axiom_monitor  = self.axiom_monitor
        self.orchestrator.lod_controller = self.lod_controller
        self.orchestrator.service_registry = self.service_registry

        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start(get_core_bus())

        self.self_prober = SelfProber()
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)
        # Handler registry is set later after creation — see _wire_event_handlers()
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

        # ── Guardrails (Safety integration layer) ──────────────────────────
        # Chain of guardrails that validate decisions before ACT phase:
        # PowerLimiter → AlignmentChecker → AuditTrail → HumanGate
        self.power_limiter = PowerLimiter()
        self.power_limiter.start()

        self.alignment_checker = AlignmentSafetyChecker()
        self.alignment_checker.start()

        self.audit_trail = TransparencyAuditTrail()
        self.audit_trail.start()

        self.human_gate = HumanApprovalGate()
        self.human_gate.start()

        # DecisionValidator is the integration chassis that chains all guardrails
        self.decision_validator = DecisionValidator(
            power_limiter=self.power_limiter,
            alignment_checker=self.alignment_checker,
            human_gate=self.human_gate,
            audit_trail=self.audit_trail,
        )

        # Inject decision_validator into orchestrator so _act_phase() can call it
        self.orchestrator.decision_validator = self.decision_validator
        logger.info("Guardrails initialized: PowerLimiter → AlignmentChecker → AuditTrail → HumanGate → DecisionValidator")

        # ── WorldModelUpdater (T27) — cross-reality state aggregator ──────
        # Subscribes to JUDGMENT_CREATED; computes composite_risk and
        # dominant_reality across all 7 realities. Previously coded but
        # never instantiated (0% LIVE). Now LIVE.
        self.world_model = WorldModelUpdater()
        self.world_model.start()

        # ── Topology System (L0) — Real-time architecture consciousness ────
        # Layer 1: Monitor source files for changes
        self.source_watcher = SourceWatcher()
        # Layer 2: Compute topology delta on SOURCE_CHANGED
        self.topology_builder = IncrementalTopologyBuilder()
        # Layer 3: Apply changes safely with rollback
        self.hot_reload_coordinator = HotReloadCoordinator()
        # Layer 4: Continuous architecture snapshots
        self.topology_mirror = TopologyMirror()
        # Layer 4.5: Real-time change log (visibility into modifications)
        self.change_tracker = ChangeTracker()
        # Layer 4.6: Semantic analysis of changes (impact classification, risk assessment)
        self.change_analyzer = ChangeAnalyzer()
        # Phase 3: Convergence validator — announcement vs reality verification
        self.convergence_validator = ConvergenceValidator()
        logger.info("Topology system initialized (L0: organism real-time consciousness)")

    # ═══════════════════════════════════════════════════════════════════════
    # NEW METHODS — Handler Registry + Services
    # ═══════════════════════════════════════════════════════════════════════

    def _create_services(self) -> object:  # Returns KernelServices
        """Create KernelServices — the organism's bloodstream.

        Now decomposed into three domain-specific service groups to prevent god object growth:
        - CognitionServices: BRAIN (judgment, axioms, LOD, EScore)
        - MetabolicServices: BODY (execution, scheduling, routing)
        - SensoryServices: SENSES (compression, registry, topology)
        """
        from cynic.api.handlers import KernelServices, CognitionServices, MetabolicServices, SensoryServices

        # BRAIN: Judgment, learning, axioms, consciousness levels
        cognition = CognitionServices(
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

        # BODY: Execution, scheduling, routing, telemetry
        metabolic = MetabolicServices(
            scheduler=self.scheduler,
            runner=self.runner,
            llm_router=self.llm_router,
            db_pool=self.db_pool,
        )

        # SENSES: Perception, compression, topology awareness
        senses = SensoryServices(
            compressor=self.compressor,
            service_registry=self.service_registry,
            world_model=self.world_model,
        )

        # Unified bloodstream: coordinates the three domains
        kernel_services = KernelServices(
            cognition=cognition,
            metabolic=metabolic,
            senses=senses,
        )

        logger.info("KernelServices created: 3 domain-specific service groups wired")
        return kernel_services

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
            direct={"universal_actuator": self.universal_actuator, "qtable": self.qtable},
        )
        for group in groups:
            registry.register(group)
        logger.info("HandlerRegistry: %d groups discovered", len(groups))

        # ── Compile-time validation (Opportunity #3) ────────────────────────
        from cynic.api.handlers.validator import HandlerValidator
        validator = HandlerValidator()
        issues = validator.validate(groups)

        # Log validation results
        if issues:
            logger.warning(validator.report())
            if validator.has_errors():
                logger.error("Handler validation FAILED with errors")
        else:
            logger.debug("Handler validation: OK")

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
        container.register(ConsciousnessRhythm, self.scheduler)
        container.register(AxiomMonitor, self.axiom_monitor)
        container.register(LODController, self.lod_controller)
        container.register(EScoreTracker, self.escore_tracker)
        container.register(ContextCompressor, self.compressor)
        container.register(WorldModelUpdater, self.world_model)
        container.register(ClaudeCodeRunner, self.runner)
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

        # Wire handler registry to SelfProber for architectural analysis
        self.self_prober.set_handler_registry(self._handler_registry)

        # Module-level handlers (not part of any group)
        bus.on(CoreEvent.JUDGMENT_CREATED, _on_judgment_created)  # guidance.json

        # ── Phase 3: Convergence Validator (observability) ──────────────────
        # Track organism announcements for end-to-end verification
        async def _on_judgment_announced(evt: Event) -> None:
            """Record announcement when judgment is made."""
            try:
                payload = JudgmentCreatedPayload(**evt.payload)
                # Record: what organism announced it would do
                self.convergence_validator.announce(
                    verdict=payload.verdict,
                    q_score=payload.q_score,
                    cell_id=payload.state_key if payload.state_key else None,
                    action=f"{payload.reality}:{payload.verdict}",
                    confidence=payload.confidence,
                )
                logger.info(
                    f"[CONVERGENCE] Announced: verdict={payload.verdict} Q={payload.q_score:.1f}"
                )
            except Exception as e:
                logger.error(f"[CONVERGENCE] announce() failed: {e}")

        bus.on(CoreEvent.JUDGMENT_CREATED, _on_judgment_announced)

        # ── Topology System Event Wiring (L0: real-time consciousness) ─────
        # Layer 1.5: Log actual file modifications for visibility
        bus.on(CoreEvent.SOURCE_CHANGED, self.change_tracker.on_source_changed)
        # Layer 4.6: Analyze changes semantically (impact, risk, action)
        bus.on(CoreEvent.SOURCE_CHANGED, self.change_analyzer.on_source_changed)
        # Layer 2: Detect what changed when SOURCE_CHANGED fires
        bus.on(CoreEvent.SOURCE_CHANGED, self.topology_builder.on_source_changed)
        # Layer 3: Apply topology changes safely when TOPOLOGY_CHANGED fires
        bus.on(CoreEvent.TOPOLOGY_CHANGED, lambda evt: self.hot_reload_coordinator.on_topology_changed(
            evt, self._handler_registry, bus, None
        ))
        # Layer 4: Snapshot organism topology on all changes
        # (will be started with continuous_snapshot in lifespan)

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

    def _make_app_state(self) -> CynicOrganism:
        """Assemble the final AppState from all components."""
        logger.info(
            "Kernel ready: %d dogs, scheduler wired, learning loop + residual detector active, pool=%s, llm=%s",
            len(self.dogs),
            "connected" if self.db_pool else "none",
            f"{len(self.registry.get_available())} adapters" if self.registry else "none",
        )
        cognition = CognitionCore(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            account_agent=self.account_agent,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
            escore_tracker=self.escore_tracker,
            power_limiter=self.power_limiter,
            alignment_checker=self.alignment_checker,
            human_gate=self.human_gate,
            audit_trail=self.audit_trail,
            decision_validator=self.decision_validator,
        )
        metabolism = MetabolicCore(
            scheduler=self.scheduler,
            runner=self.runner,
            llm_router=self.llm_router,
            telemetry_store=TelemetryStore(),
            universal_actuator=self.universal_actuator,
            auto_benchmark=None,
        )
        senses = SensoryCore(
            context_compressor=self.compressor,
            service_registry=self.service_registry,
            world_model=self.world_model,
            source_watcher=self.source_watcher,
            topology_builder=self.topology_builder,
            hot_reload_coordinator=self.hot_reload_coordinator,
            topology_mirror=self.topology_mirror,
            change_tracker=self.change_tracker,
            change_analyzer=self.change_analyzer,
            convergence_validator=self.convergence_validator,
        )
        memory = MemoryCore(
            kernel_mirror=KernelMirror(),
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
        )
        return CynicOrganism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            _pool=self.db_pool,
            container=self._container,
            _handler_registry=self._handler_registry,
        )

    def build(self) -> CynicOrganism:
        """Build and return the fully-wired AppState."""
        self._create_components()
        svc = self._create_services()
        self._handler_registry = self._create_handler_registry(svc)
        self._container = self._build_container()
        self._wire_event_handlers()
        self._wire_perceive_workers()
        return self._make_app_state()


def awaken(db_pool=None, registry=None) -> CynicOrganism:
    """Awaken the CYNIC organism. Call once from lifespan startup."""
    return _OrganismAwakener(db_pool, registry).build()


# Process-level singleton — set during lifespan startup
_state: CynicOrganism | None = None


def set_state(state: CynicOrganism) -> None:
    global _state
    _state = state


def get_state() -> CynicOrganism:
    if _state is None:
        raise RuntimeError("CynicOrganism not initialized — lifespan not started")
    return _state


async def restore_state(state: CynicOrganism) -> None:
    """
    Restore persistent state after organism awakening.

    Call this in the FastAPI lifespan, AFTER awaken() and set_state().
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
