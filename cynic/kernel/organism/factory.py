"""
CYNIC Organism Factory - The Awakening Logic.

Handles the complex instantiation and wiring of all organism components.
"""

from __future__ import annotations

import logging
import os
import uuid

from cynic.interfaces.mcp.service import MCPBridge
from cynic.kernel.core.container import get_container
from cynic.kernel.core.convergence import ConvergenceValidator
from cynic.kernel.core.escore import EScoreTracker
from cynic.kernel.core.storage.compliance import AuditLogger, AuditLoggingHandler
from cynic.kernel.core.storage.surreal import SurrealStorage
from cynic.kernel.core.topology.file_watcher import SourceWatcher
from cynic.kernel.core.topology.topology_builder import IncrementalTopologyBuilder
from cynic.kernel.core.world_model import WorldModelUpdater
from cynic.kernel.organism.anatomy import ArchiveCore, CognitionCore, MetabolicCore, SensoryCore
from cynic.kernel.organism.brain.cognition.cortex.account import AccountAgent
from cynic.kernel.organism.brain.cognition.cortex.action_proposer import ActionProposer
from cynic.kernel.organism.brain.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.kernel.organism.brain.cognition.cortex.decide import DecideAgent
from cynic.kernel.organism.brain.cognition.cortex.lod import LODController
from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor
from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
from cynic.kernel.organism.brain.learning.qlearning import LearningLoop, QTable
from cynic.kernel.organism.reflexes import (
    CognitionServices,
    HandlerRegistry,
    MetabolicServices,
    SensoryServices,
    discover_handler_groups,
)
from cynic.kernel.organism.metabolism.embodiment import HardwareBody
from cynic.kernel.organism.metabolism.motor import MotorSystem

from cynic.kernel.organism.metabolism.llm_router import LLMRouter
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
from cynic.kernel.organism.sona_emitter import SonaEmitter
from cynic.kernel.organism.state_manager import OrganismState
from cynic.kernel.organism.perception.senses.web_eye import WebEye
from cynic.kernel.organism.metabolism.web_hand import WebHand
from cynic.kernel.protocol.knet_server import KNetServer
from cynic.kernel.core.vascular import VascularSystem
from cynic.kernel.core.config import CynicConfig
from cynic.nervous.event_journal import EventJournal
from cynic.nervous.bus_journal_adapter import BusJournalAdapter
from cynic.nervous.decision_trace import DecisionTracer
from cynic.nervous.loop_closure import LoopClosureValidator
from cynic.nervous.bus_loop_closure_adapter import BusLoopClosureAdapter
from cynic.nervous.state_reconstructor import StateReconstructor
from cynic.nervous.event_metrics import EventMetricsCollector
from cynic.nervous.bus_metrics_adapter import BusMetricsAdapter

logger = logging.getLogger("cynic.kernel.organism.factory")


class _OrganismAwakener:
    """Internal helper to build the Organism piece by piece."""

    def __init__(self, db_pool=None, registry=None):
        self.db_pool = db_pool
        self.registry = registry
        self.config = get_container().get(CynicConfig)
        self.storage: SurrealStorage | None = None

    async def build(self):
        """Assembles the 4 cores into a living Organism."""
        from cynic.kernel.organism.brain.cognition.neurons.base import DogId

        # 0. STORAGE (SurrealDB) - Isolated per instance config
        try:
            self.storage = await SurrealStorage.create(self.config)
            logger.info(f"Factory: SurrealDB Storage linked to {self.config.surreal_db}")
        except Exception as e:
            logger.warning(f"Factory: SurrealDB not available, falling back to local memory: {e}")

        # 0a. IDENTITY & TASK MANAGEMENT
        instance_id = f"CYNIC-{os.environ.get('NODE_NAME', 'LOCAL')}-{uuid.uuid4().hex[:8]}"
        from cynic.kernel.core.task_registry import TaskRegistry
        self.task_registry = TaskRegistry(instance_id=instance_id)
        
        # 0b. VASCULAR SYSTEM (Network IO Pool)
        self.vascular = VascularSystem(
            instance_id=instance_id,
            redis_url=self.config.redis_url
        )

        # 0c. NERVOUS SYSTEM (Event Buses)
        from cynic.kernel.core.event_bus import EventBus
        self.core_bus = EventBus(bus_id="CORE", instance_id=instance_id)
        self.automation_bus = EventBus(bus_id="AUTOMATION", instance_id=instance_id)
        self.agent_bus = EventBus(bus_id="AGENT", instance_id=instance_id)


        # 0d. SOMATIC GATEWAY (Universal Perception Baselayer)
        from cynic.kernel.organism.perception.somatic_gateway import SomaticGateway
        self.somatic_gateway = SomaticGateway(bus=self.core_bus)

        # 0e. DISTRIBUTED BRIDGE (Nervous System Sync)
        from cynic.kernel.core.distributed import RedisEventBridge
        self.bridge = RedisEventBridge(bus=self.core_bus, vascular=self.vascular)
        try:
            await self.bridge.start()
            logger.info(f"Factory: Redis Bridge active for {instance_id}")
        except Exception as e:
            logger.warning(f"Factory: Redis not available, running in Local-Only mode: {e}")

        # 0e. EVENT JOURNAL - records every bus event automatically
        self.journal = EventJournal()
        self._journal_adapter = BusJournalAdapter(self.journal)
        self.core_bus.on("*", self._journal_adapter.on_event)

        # 0c. DECISION TRACER - builds reasoning DAGs per judgment
        self.tracer = DecisionTracer()

        # 0d. LOOP CLOSURE VALIDATOR - detects stalled / orphaned judgment cycles
        self.loop_validator = LoopClosureValidator()
        self._loop_adapter = BusLoopClosureAdapter(self.loop_validator)
        self.core_bus.on("*", self._loop_adapter.on_event)

        # 0e. STATE RECONSTRUCTOR - audit journal + traces + loop state
        self.reconstructor = StateReconstructor(
            journal=self.journal,
            tracer=self.tracer,
            validator=self.loop_validator,
        )

        # 0f. EVENT METRICS COLLECTOR - rolling rates, histograms, anomaly detection
        self.metrics_collector = EventMetricsCollector()
        self._metrics_adapter = BusMetricsAdapter(self.metrics_collector, bus=self.core_bus)
        self.core_bus.on("*", self._metrics_adapter.on_event)

        # 0g. AUDIT LOGGING HANDLER - subscribes to security/auth events for compliance
        audit_logger = AuditLogger() if self.storage is None else AuditLogger(storage=self.storage)
        self._audit_handler = AuditLoggingHandler(audit_logger)
        # Subscribe to security-related events
        self.core_bus.on("security.auth_attempt", self._audit_handler.on_auth_attempt)
        self.core_bus.on("security.authz_decision", self._audit_handler.on_authz_decision)
        self.core_bus.on("security.data_accessed", self._audit_handler.on_data_accessed)
        self.core_bus.on("security.event", self._audit_handler.on_security_event)

        # 0h. ENCRYPTION SERVICE (Vault Integration)
        self.encryption_service = None
        try:
            from cynic.kernel.security.encryption import EncryptionService, EncryptionKeyManager, EncryptionConfig
            encryption_config = EncryptionConfig(
                vault_addr=self.config.vault_addr,
                vault_token=self.config.vault_token,
            )
            key_manager = EncryptionKeyManager(encryption_config)
            self.encryption_service = EncryptionService(key_manager)
            logger.info("Factory: EncryptionService initialized from Vault")
        except Exception as e:
            logger.warning(f"Factory: EncryptionService not available, continuing without encryption: {e}")

        # 0h. EVENT FORWARDER (PHASE 2: SIEM Foundation)
        self.event_forwarder = None
        try:
            from cynic.kernel.core.storage.event_forwarder import EventForwarder
            if self.storage:
                self.event_forwarder = EventForwarder(
                    bus=self.core_bus,
                    storage=self.storage,
                    encryption_service=self.encryption_service,
                    batch_size=100,
                    flush_interval_sec=5.0,
                )
                logger.info("Factory: EventForwarder initialized (PHASE 2)")
        except Exception as e:
            logger.warning(f"Factory: EventForwarder not available, SIEM logging disabled: {e}")

        # 0i. COGNITIVE SCIENTIST (MCTS + Surgery)
        from cynic.kernel.organism.brain.cognition.cortex.mcts_scientist import ScientificMCTS, Hypothesis
        from cynic.kernel.organism.brain.cognition.cortex.surgery import AutoSurgeon
        from cynic.nervous.flight_recorder import FlightRecorder
        
        self.auto_surgeon = AutoSurgeon(root_dir=str(self.root_dir if hasattr(self, 'root_dir') else "."))
        self.flight_recorder = FlightRecorder(storage=self.storage, bus=self.core_bus)
        
        # Initial MCTS state (will be expanded by Gemini 3 at hackathon)
        self.mcts_scientist = ScientificMCTS(Hypothesis(
            id="ROOT", 
            description="Initial architectural state", 
            target_metric="system_integrity", 
            expected_trend="increase"
        ))

        # 1. BASE STATE
        self.state = OrganismState(instance_id=instance_id, storage=self.storage, bus=self.core_bus)
        
        # 1a. DNA & WILL (Axiomatic Foundation)
        import time
        from cynic.kernel.organism.brain.identity import OrganismIdentity
        from cynic.kernel.organism.brain.judgment_engine import JudgmentEngine
        
        self.identity = OrganismIdentity(
            name="CYNIC",
            birth_timestamp=time.time()
        )
        self.judgment_engine = JudgmentEngine(
            identity=self.identity,
            algorithm="pbft"
        )

        # 1b. LLM REGISTRY (Isolated)
        from cynic.kernel.organism.brain.llm.adapter import LLMRegistry
        self.llm_registry = LLMRegistry(vascular=self.vascular)
        await self.llm_registry.discover(
            ollama_url=self.config.ollama_url,
            claude_api_key=self.config.anthropic_api_key,
            google_api_key=self.config.google_api_key,
            models_dir=self.config.models_dir
        )

        # 1c. SERVICE REGISTRY (Isolated)
        from cynic.nervous.service_registry import ServiceStateRegistry
        self.service_registry = ServiceStateRegistry()

        # 1d. CONSCIOUSNESS (Isolated)
        from cynic.kernel.core.consciousness import ConsciousnessState
        self.consciousness = ConsciousnessState()

        # 2. NEURONS (Dogs)
        self.dogs = discover_dogs(bus=self.core_bus, llm_registry=self.llm_registry, vascular=self.vascular)
        cynic_dog = self.dogs.get(DogId.CYNIC.value)

        # 3. COGNITION & STRATEGY
        self.qtable = QTable(storage=self.storage.qtable if self.storage else None)

        scholar = self.dogs.get(DogId.SCHOLAR.value)
        if scholar and hasattr(scholar, "set_qtable"):
            scholar.set_qtable(self.qtable)

        from cynic.kernel.core.axioms import AxiomArchitecture

        axiom_arch = AxiomArchitecture()
        axiom_arch.state = self.state

        self.learning_loop = LearningLoop(qtable=self.qtable, pool=self.db_pool, instance_id=instance_id)
        self.learning_loop.start(event_bus=self.core_bus)

        self.residual_detector = ResidualDetector(bus=self.core_bus)
        self.residual_detector.start()

        # GASdf Executor
        gasdf_executor = None
        if hasattr(self.config, "gasdf_enabled") and self.config.gasdf_enabled:
            from cynic.kernel.organism.perception.integrations.gasdf.client import GASdfClient
            from cynic.kernel.organism.perception.integrations.gasdf.executor import GASdfExecutor

            client = GASdfClient(base_url=self.config.gasdf_url)
            gasdf_executor = GASdfExecutor(client=client)

        self.orchestrator = JudgeOrchestrator(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
            cynic_dog=cynic_dog,
            bus=self.core_bus,
            residual_detector=self.residual_detector,
            gasdf_executor=gasdf_executor,
            state_manager=self.state,
            instance_id=instance_id,
            llm_registry=self.llm_registry,
            consciousness=self.consciousness,
            identity=self.identity,
            judgment_engine=self.judgment_engine,
        )
        self.orchestrator.service_registry = self.service_registry

        self.decide_agent = DecideAgent(qtable=self.qtable, bus=self.core_bus)
        self.decide_agent.start()

        action_repo = self.storage.action_proposals if self.storage else None
        self.action_proposer = ActionProposer(repo=action_repo, bus=self.core_bus)
        self.action_proposer.start()

        self.account_agent = AccountAgent(bus=self.core_bus)
        self.llm_router = LLMRouter()
        self.lod_controller = LODController()

        # E-Score with DB persistence
        self.escore_tracker = EScoreTracker(bus=self.core_bus, state_manager=self.state, instance_id=instance_id)

        self.axiom_monitor = AxiomMonitor(bus=self.core_bus)

        # 4. METABOLISM (Body & Rhythm)
        self.body = HardwareBody(bus=self.core_bus)
        self.motor = MotorSystem(bus=self.core_bus, body=self.body, state_manager=self.state)
        
        # -NET Somatic Server
        self.knet_server = KNetServer(
            bus=self.core_bus,
            host=self.config.knet_host,
            port=self.config.knet_port
        )
        self.scheduler = ConsciousnessRhythm(
            self.orchestrator, 
            bus=self.core_bus,
            consciousness=self.consciousness
        )
        self.scheduler.body = self.body

        from cynic.kernel.organism.metabolism.actuators import FileActuator, UniversalActuator

        self.universal_actuator = UniversalActuator()
        self.universal_actuator.register("file_write", FileActuator())

        from cynic.kernel.organism.metabolism.claude_sdk import ClaudeCodeRunner

        self.runner = ClaudeCodeRunner(bus=self.agent_bus, sessions_registry={})
        from cynic.kernel.organism.metabolism.telemetry import TelemetryStore
        self.telemetry_store = TelemetryStore(maxlen=1000)

        # 5. SENSES (Perception)
        self.context_compressor = ContextCompressor()
        self.world_model = WorldModelUpdater(bus=self.core_bus)
        self.source_watcher = SourceWatcher(bus=self.core_bus)
        self.topology_builder = IncrementalTopologyBuilder(bus=self.core_bus)
        self.mcp_bridge = MCPBridge(bus=self.core_bus)
        self.convergence_validator = ConvergenceValidator()
        
        # Web Incarnation
        self.web_eye = WebEye(bus=self.core_bus)
        self.web_hand = WebHand() # Bound later in organism.start()

        from cynic.kernel.organism.perception.senses.internal import InternalSensor
        self.internal_sensor = InternalSensor(bus=self.core_bus)
        self.internal_sensor.start()

        # market_sensor is disabled by default to avoid noise
        self.market_sensor = None


        # 6. REFLECTION (Memory & Self)
        self.sona_emitter = SonaEmitter(bus=self.core_bus, db_pool=self.db_pool, instance_id=instance_id)

        from cynic.kernel.organism.perception.federation.gossip import GossipManager
        self.gossip_manager = GossipManager(instance_id=instance_id, q_table=self.qtable)

        # 7. WIRING HANDLERS (New Domain-Specific Pattern)
        cog_svc = CognitionServices(
            escore_tracker=self.escore_tracker,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
            qtable=self.qtable,
            health_cache={},  # Shared health cache
        )
        meta_svc = MetabolicServices(scheduler=self.scheduler, body=self.body, runner=self.runner)
        sens_svc = SensoryServices(
            compressor=self.context_compressor,
            service_registry=self.service_registry,
            world_model=self.world_model,
        )

        self.handler_registry = HandlerRegistry()
        handlers = discover_handler_groups(
            cognition=cog_svc,
            metabolic=meta_svc,
            sensory=sens_svc,
            bus=self.core_bus,
            # Extra kwargs for specific handlers (MUST match module names)
            axiom={"action_proposer": self.action_proposer},
            sdk={"action_proposer": self.action_proposer, "qtable": self.qtable},
            direct={"universal_actuator": self.universal_actuator, "qtable": self.qtable},
            federation={"gossip_manager": self.gossip_manager},
            health={"storage_gc": None, "db_pool": self.db_pool},
            intelligence={
                "orchestrator": self.orchestrator,
                "scheduler": self.scheduler,
                "db_pool": self.db_pool,
                "compressor": self.context_compressor,
                "escore_tracker": self.escore_tracker,
                "axiom_monitor": self.axiom_monitor,
            },
            judgment_executor={
                "orchestrator": self.orchestrator,
                "escore_tracker": self.escore_tracker,
                "axiom_monitor": self.axiom_monitor,
            },
        )

        for group in handlers:
            self.handler_registry.register(group)

        # Use the specific instance bus
        self.handler_registry.wire(self.core_bus)

        # 8. START AGENTS
        self.sona_emitter.set_qtable(self.qtable)
        self.sona_emitter.set_orchestrator(self.orchestrator)
        self.sona_emitter.set_escore_tracker(self.escore_tracker)
        self.sona_emitter.set_state(self.state)
        
        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start()

        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

        self.self_prober = SelfProber(bus=self.core_bus)
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)

        # 6b. METRICS - SELFPROBER WIRING
        self.self_prober.set_metrics_collector(self.metrics_collector)

        # 6c. PROPOSAL EXECUTOR - SELFPROBER WIRING
        self.executor = ProposalExecutor()
        self.self_prober.set_executor(self.executor)

        self.self_prober.start()

        from cynic.kernel.organism.brain.agents.sovereignty import SovereigntyAgent
        self.sovereignty_agent = SovereigntyAgent(state_manager=self.state, bus=self.core_bus)
        self.sovereignty_agent.start()

        # Assembly
        cognition = CognitionCore(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            llm_registry=self.llm_registry,
            decide_agent=self.decide_agent,
            account_agent=self.account_agent,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
            identity=self.identity,
            judgment_engine=self.judgment_engine,
        )

        metabolism = MetabolicCore(
            scheduler=self.scheduler,
            body=self.body,
            motor=self.motor,
            web_hand=self.web_hand,
            runner=self.runner,
            llm_router=self.llm_router,
            telemetry_store=self.telemetry_store,
        )

        senses = SensoryCore(
            context_compressor=self.context_compressor,
            world_model=self.world_model,
            source_watcher=self.source_watcher,
            topology_builder=self.topology_builder,
            somatic_gateway=self.somatic_gateway,
            mcp_bridge=self.mcp_bridge,
            market_sensor=self.market_sensor,
            web_eye=self.web_eye,
            internal_sensor=self.internal_sensor,
            convergence_validator=self.convergence_validator,
            knet_server=self.knet_server,
        )

        memory = ArchiveCore(
            state=self.state,
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
            sona_emitter=self.sona_emitter,
            gossip_manager=self.gossip_manager,
            journal=self.journal,
            loop_validator=self.loop_validator,
            reconstructor=self.reconstructor,
            metrics_collector=self.metrics_collector,
            executor=self.executor,
        )

        from cynic.kernel.organism.organism import Organism

        organism = Organism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            state=self.state,
            instance_id=instance_id,
            storage=self.storage,
            vascular=self.vascular,
            bridge=self.bridge,
            automation_bus=self.automation_bus,
            agent_bus=self.agent_bus,
            task_registry=self.task_registry,
            _pool=self.db_pool,
            container=get_container(),
        )

        # Attach EventForwarder for SIEM logging (PHASE 2)
        organism.event_forwarder = self.event_forwarder

        return organism


async def awaken(db_pool=None, registry=None):
    """Entry point to wake up CYNIC."""
    return await _OrganismAwakener(db_pool, registry).build()


async def create_organism(db_pool=None, registry=None):
    """Alias for awaken() used by tests."""
    return await awaken(db_pool, registry)
