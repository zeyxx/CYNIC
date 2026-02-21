"""
CYNIC Organism — Root Coordinator for the Living System

MOVED FROM: cynic/api/state.py (was hidden as CynicOrganism)
RENAMED FROM: CynicOrganism → Organism
REASON: This IS the organism. It should live in organism/, not api/state.py

ARCHITECTURE:
  Organism = Root Coordinator
  ├─ CognitionCore (BRAIN) — Judgment, learning, axioms
  ├─ MetabolicCore (BODY) — Scheduling, execution, routing
  ├─ SensoryCore (SENSES) — Perception, topology, world model
  └─ MemoryCore (ARCHIVE) — Reflection, proposals, self-improvement

This file is ~500 LOC but is THE cornerstone. Everything else depends on it.
"""
from __future__ import annotations

import json
import os
import time
import logging
import threading
from dataclasses import dataclass, field
from typing import Any, TYPE_CHECKING, Optional

if TYPE_CHECKING:
    import asyncpg

from cynic.core.axioms import AxiomArchitecture
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.consciousness import ConsciousnessLevel, get_consciousness
from cynic.core.event_bus import get_core_bus, get_automation_bus, get_agent_bus, Event, CoreEvent
from cynic.core.formulas import ACT_LOG_CAP
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
    MAX_CONFIDENCE, MAX_Q_SCORE, HOWL_MIN, WAG_MIN, GROWL_MIN,
    PHI_INV, PHI_INV_2,
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
from cynic.mcp.service import MCPBridge
from cynic.organism import ConsciousState, get_conscious_state
from cynic.organism.state_manager import OrganismState, StateLayer, StateSnapshot

logger = logging.getLogger("cynic.organism.organism")

# The organism's own identity in the EScore reputation system
_ESCORE_AGENT_ID = "agent:cynic"


# ═════════════════════════════════════════════════════════════════════════════
# FOUR BIOLOGICAL SYSTEMS (Façade Pattern — KILL GOD OBJECT)
# ═════════════════════════════════════════════════════════════════════════════


@dataclass
class CognitionCore:
    """BRAIN — Judgment, learning, axioms, decisions, safety guardrails."""
    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    residual_detector: ResidualDetector
    power_limiter: PowerLimiter
    alignment_checker: AlignmentSafetyChecker
    human_gate: HumanApprovalGate
    audit_trail: TransparencyAuditTrail
    decision_validator: Optional[DecisionValidator] = None
    decide_agent: Optional[DecideAgent] = None
    account_agent: Optional[AccountAgent] = None
    axiom_monitor: AxiomMonitor = field(default_factory=AxiomMonitor)
    lod_controller: LODController = field(default_factory=LODController)
    escore_tracker: EScoreTracker = field(default_factory=EScoreTracker)


@dataclass
class MetabolicCore:
    """BODY — Execution, scheduling, routing, telemetry, actuators."""
    scheduler: ConsciousnessRhythm
    runner: Optional[ClaudeCodeRunner] = None
    llm_router: Optional[LLMRouter] = None
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)
    universal_actuator: UniversalActuator = field(default_factory=UniversalActuator)
    auto_benchmark: Optional[AutoBenchmark] = None


@dataclass
class SensoryCore:
    """NERVOUS SYSTEM — Compression, registry, world model, topology, MCP bridge."""
    context_compressor: ContextCompressor = field(default_factory=ContextCompressor)
    service_registry: ServiceStateRegistry = field(default_factory=ServiceStateRegistry)
    event_journal: EventJournal = field(default_factory=EventJournal)
    decision_tracer: DecisionTracer = field(default_factory=DecisionTracer)
    loop_closure_validator: LoopClosureValidator = field(default_factory=LoopClosureValidator)
    world_model: WorldModelUpdater = field(default_factory=WorldModelUpdater)
    source_watcher: SourceWatcher = field(default_factory=SourceWatcher)
    topology_builder: IncrementalTopologyBuilder = field(default_factory=IncrementalTopologyBuilder)
    hot_reload_coordinator: Optional[HotReloadCoordinator] = None
    topology_mirror: Optional[TopologyMirror] = None
    change_tracker: Optional[ChangeTracker] = None
    change_analyzer: Optional[ChangeAnalyzer] = None
    convergence_validator: ConvergenceValidator = field(default_factory=ConvergenceValidator)
    mcp_bridge: MCPBridge = field(default_factory=lambda: MCPBridge(bus_name="CORE"))


@dataclass
class MemoryCore:
    """ARCHIVE — Reflection, proposals, self-improvement."""
    conscious_state: ConsciousState = field(default_factory=get_conscious_state)
    kernel_mirror: KernelMirror = field(default_factory=KernelMirror)
    action_proposer: ActionProposer = field(default_factory=ActionProposer)
    self_prober: SelfProber = field(default_factory=SelfProber)


# ═════════════════════════════════════════════════════════════════════════════
# THE ORGANISM
# ═════════════════════════════════════════════════════════════════════════════


@dataclass
class Organism:
    """
    The living organism — thin envelope composing 4 biological systems.

    Built once at startup, lives for the process lifetime.
    This is the root coordinator for all CYNIC activity.

    NEW: Now integrates OrganismState for unified state management (Phase 3 Tier 1).
    """
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: MemoryCore
    state: OrganismState = field(default_factory=OrganismState)
    started_at: float = field(default_factory=time.time)
    _pool: Optional[Any] = None
    last_judgment: Optional[dict] = None
    container: DependencyContainer = field(default_factory=DependencyContainer)
    _handler_registry: object = field(default=None)

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> list[str]:
        return list(self.cognition.orchestrator.dogs.keys())

    # ── Backward-compatibility properties ──
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
    def runner(self) -> Optional[ClaudeCodeRunner]:
        return self.metabolism.runner

    @runner.setter
    def runner(self, value: Optional[ClaudeCodeRunner]) -> None:
        self.metabolism.runner = value

    @property
    def llm_router(self) -> Optional[LLMRouter]:
        return self.metabolism.llm_router

    @llm_router.setter
    def llm_router(self, value: Optional[LLMRouter]) -> None:
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
    def conscious_state(self) -> ConsciousState:
        return self.memory.conscious_state

    @property
    def decide_agent(self) -> Optional[DecideAgent]:
        return self.cognition.decide_agent

    @property
    def account_agent(self) -> Optional[AccountAgent]:
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
    def auto_benchmark(self) -> Optional[AutoBenchmark]:
        return self.metabolism.auto_benchmark

    @auto_benchmark.setter
    def auto_benchmark(self, value: Optional[AutoBenchmark]) -> None:
        self.metabolism.auto_benchmark = value

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
    def mcp_bridge(self) -> MCPBridge:
        return self.senses.mcp_bridge

    @property
    def decision_validator(self) -> Optional[DecisionValidator]:
        return self.cognition.decision_validator

    @property
    def source_watcher(self) -> SourceWatcher:
        return self.senses.source_watcher

    @property
    def topology_builder(self) -> IncrementalTopologyBuilder:
        return self.senses.topology_builder

    @property
    def hot_reload_coordinator(self) -> Optional[HotReloadCoordinator]:
        return self.senses.hot_reload_coordinator

    @property
    def topology_mirror(self) -> Optional[TopologyMirror]:
        return self.senses.topology_mirror

    @property
    def change_tracker(self) -> Optional[ChangeTracker]:
        return self.senses.change_tracker

    @property
    def change_analyzer(self) -> Optional[ChangeAnalyzer]:
        return self.senses.change_analyzer

    # ── OrganismState unified access ───────────────────────────────────────
    @property
    def state_snapshot(self) -> StateSnapshot:
        """Get immutable snapshot of all three state layers."""
        return self.state.snapshot()


# Type alias for backward compatibility
AppState = Organism


# ═════════════════════════════════════════════════════════════════════════════
# ORGANISM AWAKENER — Builder Pattern (Extracted from api/state.py)
# ═════════════════════════════════════════════════════════════════════════════


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
    _ACT_LOG_CAP        = ACT_LOG_CAP  # F(11) — ACT result JSONL (imported from formulas.py)
    _A6_A9 = frozenset({"AUTONOMY", "SYMBIOSIS", "EMERGENCE", "ANTIFRAGILITY"})
    _ACT_LOG_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "act_results.jsonl")

    def __init__(
        self,
        db_pool: Optional[Any] = None,
        registry: Optional[LLMRegistry] = None,
    ) -> None:
        self.db_pool: Optional[Any]   = db_pool
        self.registry: Optional[LLMRegistry]   = registry

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
        self.dogs:             dict[str, Any] = {}
        self.qtable:           Optional[QTable] = None
        self.orchestrator:     Optional[JudgeOrchestrator] = None
        self.scheduler:        Optional[ConsciousnessRhythm] = None
        self.learning_loop:    Optional[LearningLoop] = None
        self.residual_detector: Optional[ResidualDetector] = None
        self.decide_agent:     Optional[DecideAgent] = None
        self.action_proposer:  Optional[ActionProposer] = None
        self.account_agent:    Optional[AccountAgent] = None
        self.llm_router:       Optional[LLMRouter] = None
        self.axiom_monitor:    Optional[AxiomMonitor] = None
        self.lod_controller:   Optional[LODController] = None
        self.escore_tracker:   Optional[EScoreTracker] = None
        self.self_prober:      Optional[SelfProber] = None
        self.world_model:      Optional[WorldModelUpdater] = None
        self.compressor:       Optional[ContextCompressor] = None
        self.storage_gc:       Optional[StorageGarbageCollector] = None
        self.universal_actuator: Optional[UniversalActuator] = None

        # ── Guardrails — Safety checks before ACT phase ──────────────────────
        self.power_limiter:       Optional[PowerLimiter] = None
        self.alignment_checker:   Optional[AlignmentSafetyChecker] = None
        self.human_gate:          Optional[HumanApprovalGate] = None
        self.audit_trail:         Optional[TransparencyAuditTrail] = None
        self.decision_validator:  Optional[DecisionValidator] = None

        # ── MCP Bridge (Sensory integration) ────────────────────────────────
        self.mcp_bridge: Optional[MCPBridge] = None

        # ── Topology System (L0: Real-time architecture awareness) ──────────
        self.source_watcher:   Optional[Any] = None  # SourceWatcher
        self.topology_builder: Optional[Any] = None  # IncrementalTopologyBuilder
        self.hot_reload_coordinator: Optional[Any] = None  # HotReloadCoordinator
        self.topology_mirror:  Optional[Any] = None  # TopologyMirror
        self.change_tracker:   Optional[Any] = None  # ChangeTracker
        self.change_analyzer:  Optional[Any] = None  # ChangeAnalyzer
        self.convergence_validator: Optional[Any] = None  # ConvergenceValidator

    # ═══════════════════════════════════════════════════════════════════════
    # PHASE 1 — Component creation (COMPLETE IMPLEMENTATION FROM api/state.py)
    # ═══════════════════════════════════════════════════════════════════════

    def _create_components(self) -> None:
        """Create all kernel components and wire their mutual dependencies."""
        from cynic.api.state import _on_judgment_created

        # Dogs — auto-discovered from cynic.cognition.neurons (no manual import list)
        # QTable must exist before OracleDog (Oracle reads it for predictions)
        self.qtable = QTable()
        self.dogs = discover_dogs(ORACLE=OracleDog(qtable=self.qtable))
        cynic_dog = self.dogs[DogId.CYNIC]
        self.cynic_dog = cynic_dog  # Store for handler injection later

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

        # ── Phase 2B: Handler Composition DAG ──────────────────────────────────
        # Create 7 handlers for explicit, testable orchestration
        from cynic.cognition.cortex.handlers import (
            HandlerRegistry, HandlerComposer, LevelSelector, ReflexCycleHandler,
            MicroCycleHandler, MacroCycleHandler, ActHandler, EvolveHandler,
            BudgetManager,
        )

        level_selector = LevelSelector(
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
        )
        cycle_reflex = ReflexCycleHandler(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
        )
        cycle_micro = MicroCycleHandler(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
            cynic_dog=self.cynic_dog,
            lod_controller=self.lod_controller,
        )
        cycle_macro = MacroCycleHandler(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
            cynic_dog=self.cynic_dog,
            escore_tracker=self.escore_tracker,
            lod_controller=self.lod_controller,
            axiom_monitor=self.axiom_monitor,
            context_compressor=self.compressor,
            act_phase_fn=None,  # ACT handled by composer after cycle, not during
        )
        act_executor = ActHandler(
            decide_agent=self.decide_agent,
            decision_validator=self.decision_validator,
            runner=self.runner,
        )
        evolve_handler = EvolveHandler(
            orchestrator=self.orchestrator,
            benchmark_registry=self.orchestrator.benchmark_registry,
        )
        budget_manager = BudgetManager(
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
        )

        # Register handlers
        handler_registry = HandlerRegistry()
        handler_registry.register("level_selector", level_selector)
        handler_registry.register("cycle_reflex", cycle_reflex)
        handler_registry.register("cycle_micro", cycle_micro)
        handler_registry.register("cycle_macro", cycle_macro)
        handler_registry.register("act_executor", act_executor)
        handler_registry.register("evolve", evolve_handler)
        handler_registry.register("budget_manager", budget_manager)

        # Create composer and wire into orchestrator
        composer = HandlerComposer(handler_registry)
        self.orchestrator._composer = composer
        logger.info("Phase 2B: Handler composition DAG initialized (%d handlers)", handler_registry.count())

        # ── WorldModelUpdater (T27) — cross-reality state aggregator ──────
        # Subscribes to JUDGMENT_CREATED; computes composite_risk and
        # dominant_reality across all 7 realities. Previously coded but
        # never instantiated (0% LIVE). Now LIVE.
        self.world_model = WorldModelUpdater()
        self.world_model.start()

        # ── MCPBridge (Sensory integration) ────────────────────────────────
        self.mcp_bridge = MCPBridge(bus_name="CORE")
        logger.info("MCPBridge initialized (WebSocket /ws/mcp)")

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
    # Service/Handler/Container creation (handlers + DI)
    # ═══════════════════════════════════════════════════════════════════════

    def _create_services(self) -> Any:  # Returns KernelServices
        """Create KernelServices — the organism's bloodstream."""
        from cynic.api.handlers import KernelServices, CognitionServices, MetabolicServices, SensoryServices

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

        metabolic = MetabolicServices(
            scheduler=self.scheduler,
            runner=self.runner,
            llm_router=self.llm_router,
            db_pool=self.db_pool,
        )

        senses = SensoryServices(
            compressor=self.compressor,
            service_registry=self.service_registry,
            world_model=self.world_model,
        )

        kernel_services = KernelServices(
            cognition=cognition,
            metabolic=metabolic,
            senses=senses,
        )

        logger.info("KernelServices created: 3 domain-specific service groups wired")
        return kernel_services

    def _create_handler_registry(self, svc: Any) -> Any:  # Returns HandlerRegistry
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
            judgment_executor={
                "orchestrator": self.orchestrator,
            },
            axiom={"action_proposer": self.action_proposer},
            sdk={"action_proposer": self.action_proposer, "qtable": self.qtable},
            health={"storage_gc": self.storage_gc, "db_pool": self.db_pool},
            direct={"universal_actuator": self.universal_actuator, "qtable": self.qtable},
        )
        for group in groups:
            registry.register(group)
        logger.info("HandlerRegistry: %d groups discovered", len(groups))

        from cynic.api.handlers.validator import HandlerValidator
        validator = HandlerValidator()
        issues = validator.validate(groups)

        if issues:
            logger.warning(validator.report())
            if validator.has_errors():
                logger.error("Handler validation FAILED with errors")
        else:
            logger.debug("Handler validation: OK")

        return registry

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

    def _wire_event_handlers(self) -> None:
        """Register all event bus subscriptions via HandlerRegistry."""
        bus = get_core_bus()

        # Handler groups (auto-discovered, self-registering)
        self._handler_registry.wire(bus)

        # Wire handler registry to SelfProber for architectural analysis
        self.self_prober.set_handler_registry(self._handler_registry)

        # Note: _on_judgment_created (guidance.json) is wired in api/server.py
        # after AppContainer is created, since it needs get_app_container()

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
            except CynicError as e:
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

    def _make_app_state(self) -> Organism:
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
            mcp_bridge=self.mcp_bridge,
        )
        memory = MemoryCore(
            kernel_mirror=KernelMirror(),
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
        )
        # Initialize OrganismState with database pool
        organism_state = OrganismState()
        # Note: Pool injection happens via setter after creation
        # (see awaken() or server.py lifespan for async initialization)

        return Organism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            state=organism_state,
            _pool=self.db_pool,
            container=self._container,
            _handler_registry=self._handler_registry,
        )

    def build(self) -> Organism:
        """Build and return the fully-wired AppState."""
        self._create_components()
        svc = self._create_services()
        self._handler_registry = self._create_handler_registry(svc)
        self._container = self._build_container()
        self._wire_event_handlers()
        self._wire_perceive_workers()
        return self._make_app_state()


def awaken(db_pool=None, registry=None) -> Organism:
    """Awaken the CYNIC organism. Call once from lifespan startup."""
    return _OrganismAwakener(db_pool, registry).build()
