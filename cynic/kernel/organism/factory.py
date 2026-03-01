"""
CYNIC Organism Factory — The Awakening Logic.

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
from cynic.kernel.core.event_bus import get_core_bus
from cynic.kernel.core.storage.surreal import init_storage
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
from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
from cynic.kernel.organism.brain.learning.qlearning import LearningLoop, QTable
from cynic.kernel.organism.handlers import (
    CognitionServices,
    HandlerRegistry,
    MetabolicServices,
    SensoryServices,
    discover_handler_groups,
)
from cynic.kernel.organism.layers.embodiment import HardwareBody
from cynic.kernel.organism.metabolism.llm_router import LLMRouter
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
from cynic.kernel.organism.sona_emitter import SonaEmitter
from cynic.kernel.organism.state_manager import OrganismState

logger = logging.getLogger("cynic.kernel.organism.factory")


class _OrganismAwakener:
    """Internal helper to build the Organism piece by piece."""

    def __init__(self, db_pool=None, registry=None):
        self.db_pool = db_pool
        self.registry = registry
        from cynic.kernel.core.config import CynicConfig

        self.config = get_container().get(CynicConfig)
        self.storage = None

    async def build(self):
        """Assembles the 4 cores into a living Organism."""
        from cynic.kernel.organism.brain.cognition.neurons.base import DogId

        # 0. STORAGE (SurrealDB)
        try:
            self.storage = await init_storage()
            logger.info("Factory: SurrealDB Storage linked.")
        except Exception as e:
            logger.warning(f"Factory: SurrealDB not available, falling back to local memory: {e}")

        # Generate instance identity (SRE standard)
        instance_id = f"CYNIC-{os.environ.get('NODE_NAME', 'LOCAL')}-{uuid.uuid4().hex[:8]}"

        # 1. BASE STATE
        self.state = OrganismState(storage=self.storage, instance_id=instance_id)
        
        # 1b. LLM REGISTRY (Isolated)
        from cynic.kernel.organism.brain.llm.adapter import LLMRegistry
        self.llm_registry = LLMRegistry()

        # 1c. SERVICE REGISTRY (Isolated)
        from cynic.nervous.service_registry import ServiceStateRegistry
        self.service_registry = ServiceStateRegistry()

        # 2. NEURONS (Dogs)
        self.dogs = discover_dogs(instance_id=instance_id, llm_registry=self.llm_registry)
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
        self.learning_loop.start(event_bus=get_core_bus(instance_id))

        self.residual_detector = ResidualDetector(bus=get_core_bus(instance_id))
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
            residual_detector=self.residual_detector,
            gasdf_executor=gasdf_executor,
            state_manager=self.state,
            instance_id=instance_id,
            llm_registry=self.llm_registry,
        )
        self.orchestrator.service_registry = self.service_registry

        self.decide_agent = DecideAgent(qtable=self.qtable, bus=get_core_bus(instance_id))
        self.decide_agent.start()

        action_repo = self.storage.action_proposals if self.storage else None
        self.action_proposer = ActionProposer(repo=action_repo, bus=get_core_bus(instance_id))
        self.action_proposer.start()

        self.account_agent = AccountAgent(bus=get_core_bus(instance_id))
        self.llm_router = LLMRouter()
        self.lod_controller = LODController()

        # E-Score with DB persistence
        self.escore_tracker = EScoreTracker(state_manager=self.state, instance_id=instance_id)

        self.axiom_monitor = AxiomMonitor(bus=get_core_bus(instance_id))

        # 4. METABOLISM (Body & Rhythm)
        self.body = HardwareBody(bus=get_core_bus(instance_id))
        self.scheduler = ConsciousnessRhythm(self.orchestrator, bus=get_core_bus(instance_id))
        self.scheduler.body = self.body

        from cynic.kernel.organism.metabolism.actuators import FileActuator, UniversalActuator

        self.universal_actuator = UniversalActuator()
        self.universal_actuator.register("file_write", FileActuator())

        from cynic.kernel.organism.metabolism.claude_sdk import ClaudeCodeRunner

        self.runner = ClaudeCodeRunner(bus=get_core_bus(instance_id), sessions_registry={})
        self.telemetry_store = None  # Placeholder

        # 5. SENSES (Perception)
        self.context_compressor = ContextCompressor()
        self.world_model = WorldModelUpdater(bus=get_core_bus(instance_id))
        self.source_watcher = SourceWatcher()
        self.topology_builder = IncrementalTopologyBuilder()
        self.mcp_bridge = MCPBridge(bus_name="CORE")
        self.convergence_validator = ConvergenceValidator()

        from cynic.kernel.organism.perception.senses.internal import InternalSensor
        self.internal_sensor = InternalSensor(bus=get_core_bus(instance_id))
        self.internal_sensor.start()

        from cynic.kernel.organism.perception.senses.market import MarketSensor
        self.market_sensor = MarketSensor(interval_s=30.0, bus=get_core_bus(instance_id))
        self.market_sensor.start()


        # 6. REFLECTION (Memory & Self)
        self.sona_emitter = SonaEmitter(bus=get_core_bus(instance_id), db_pool=self.db_pool, instance_id=instance_id)

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
            bus=get_core_bus(instance_id),
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
        self.handler_registry.wire(get_core_bus(instance_id))

        # 8. START AGENTS
        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start()

        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

        self.self_prober = SelfProber(bus=get_core_bus(instance_id))
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)
        self.self_prober.start()

        from cynic.kernel.organism.brain.agents.sovereignty import SovereigntyAgent
        self.sovereignty_agent = SovereigntyAgent(state_manager=self.state, bus=get_core_bus(instance_id))
        self.sovereignty_agent.start()

        # Assembly
        cognition = CognitionCore(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            account_agent=self.account_agent,
            axiom_monitor=self.axiom_monitor,
            lod_controller=self.lod_controller,
        )

        metabolism = MetabolicCore(
            scheduler=self.scheduler,
            body=self.body,
            runner=self.runner,
            llm_router=self.llm_router,
            telemetry_store=self.telemetry_store,
        )

        senses = SensoryCore(
            context_compressor=self.context_compressor,
            world_model=self.world_model,
            source_watcher=self.source_watcher,
            topology_builder=self.topology_builder,
            mcp_bridge=self.mcp_bridge,
            convergence_validator=self.convergence_validator,
            internal_sensor=self.internal_sensor,
        )

        memory = ArchiveCore(
            state=self.state,
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
            sona_emitter=self.sona_emitter,
            gossip_manager=self.gossip_manager,
        )

        from cynic.kernel.organism.organism import Organism

        return Organism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            state=self.state,
            instance_id=instance_id,
            _pool=self.db_pool,
            container=get_container(),
        )


async def awaken(db_pool=None, registry=None):
    """Entry point to wake up CYNIC."""
    return await _OrganismAwakener(db_pool, registry).build()


async def create_organism(db_pool=None, registry=None):
    """Alias for awaken() used by tests."""
    return await awaken(db_pool, registry)
