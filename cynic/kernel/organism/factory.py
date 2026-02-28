"""
CYNIC Organism Factory — The Awakening Logic.

Handles the complex instantiation and wiring of all organism components.
"""
from __future__ import annotations
import logging
import os
import time
from typing import Any, Optional

from cynic.kernel.core.event_bus import get_core_bus
from cynic.kernel.organism.state_manager import OrganismState
from cynic.kernel.organism.anatomy import CognitionCore, MetabolicCore, SensoryCore, ArchiveCore

# All component imports
from cynic.brain.cognition.neurons.discovery import discover_dogs
from cynic.brain.cognition.neurons.cynic_dog import CynicDog
from cynic.brain.cognition.neurons.oracle import OracleDog
from cynic.kernel.core.axioms import AxiomArchitecture, HeuristicFacetScorer
from cynic.brain.learning.qlearning import QTable, LearningLoop
from cynic.brain.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.brain.cognition.cortex.residual import ResidualDetector
from cynic.brain.cognition.cortex.decide import DecideAgent
from cynic.brain.cognition.cortex.action_proposer import ActionProposer
from cynic.brain.cognition.cortex.account import AccountAgent
from cynic.brain.cognition.cortex.self_probe import SelfProber
from cynic.brain.cognition.cortex.mirror import KernelMirror
from cynic.metabolism.scheduler import ConsciousnessRhythm
from cynic.metabolism.telemetry import TelemetryStore
from cynic.metabolism.llm_router import LLMRouter
from cynic.metabolism.runner import ClaudeCodeRunner
from cynic.kernel.core.topology import (
    SourceWatcher, IncrementalTopologyBuilder, ConvergenceValidator
)
from cynic.interfaces.mcp.service import MCPBridge
from cynic.kernel.organism.sona_emitter import SonaEmitter
from cynic.kernel.core.container import DependencyContainer
from cynic.kernel.core.config import CynicConfig
from cynic.brain.cognition.cortex.lod import LODController
from cynic.kernel.core.escore import EScoreTracker
from cynic.kernel.core.consciousness import ContextCompressor, get_consciousness
from cynic.brain.dialogue.agent import DialogueAgent

logger = logging.getLogger("cynic.kernel.organism.factory")

class _OrganismAwakener:
    """
    Builds the Organism step by step.
    Ensures all dependencies are correctly injected.
    """
    def __init__(self, db_pool=None, registry=None):
        self.db_pool = db_pool
        self.registry = registry
        self.state = OrganismState()
        self.config = CynicConfig.from_env()
        
    def build(self) -> Any:
        from cynic.kernel.organism.organism import Organism
        
        # 1. BRAIN COMPONENTS
        self.qtable = QTable()
        self.dogs = discover_dogs(ORACLE=OracleDog(qtable=self.qtable))
        cynic_dog = self.dogs.get("CYNIC")
        
        # Inject LLM Registry into dogs
        if self.registry:
            for dog in self.dogs.values():
                if hasattr(dog, "set_llm_registry"):
                    dog.set_llm_registry(self.registry)
        
        # ScholarDog: recursive meta-learning
        scholar = self.dogs.get("SCHOLAR")
        if scholar and hasattr(scholar, "set_qtable"):
            scholar.set_qtable(self.qtable)

        axiom_arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
        # Provide state to axiom arch for dynamic weights
        axiom_arch.state = self.state

        self.learning_loop = LearningLoop(qtable=self.qtable, pool=self.db_pool)
        self.learning_loop.start(get_core_bus())

        self.residual_detector = ResidualDetector()
        self.residual_detector.start(get_core_bus())

        # GASdf Executor (from config)
        gasdf_executor = None
        if self.config.gasdf_enabled:
            from cynic.perception.integrations.gasdf.client import GASdfClient
            from cynic.perception.integrations.gasdf.executor import GASdfExecutor
            client = GASdfClient(base_url=self.config.gasdf_url)
            gasdf_executor = GASdfExecutor(client=client)

        self.orchestrator = JudgeOrchestrator(
            dogs=self.dogs,
            axiom_arch=axiom_arch,
            cynic_dog=cynic_dog,
            residual_detector=self.residual_detector,
            gasdf_executor=gasdf_executor
        )

        self.decide_agent = DecideAgent(qtable=self.qtable)
        self.decide_agent.start(get_core_bus())

        self.action_proposer = ActionProposer()
        self.action_proposer.start(get_core_bus())

        self.account_agent = AccountAgent()
        self.llm_router = LLMRouter()
        self.lod_controller = LODController()
        self.escore_tracker = EScoreTracker()

        # Connect Orchestrator to controllers
        self.orchestrator.escore_tracker = self.escore_tracker
        self.orchestrator.axiom_monitor = None # AxiomMonitor placeholder
        self.orchestrator.lod_controller = self.lod_controller

        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start(get_core_bus())

        self.self_prober = SelfProber()
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)
        self.self_prober.start(get_core_bus())

        # 2. BODY & METABOLISM
        from cynic.kernel.organism.layers.embodiment import HardwareBody
        self.body = HardwareBody()
        
        self.scheduler = ConsciousnessRhythm(
            axiom_monitor=None,
            escore_tracker=self.escore_tracker,
            oracle_dog=self.dogs.get("ORACLE")
        )
        self.runner = ClaudeCodeRunner()
        self.telemetry_store = TelemetryStore()

        # 3. SENSORY & NERVES
        self.compressor = ContextCompressor()
        self.world_model = WorldModelUpdater()
        self.world_model.start()
        
        self.source_watcher = SourceWatcher()
        self.topology_builder = IncrementalTopologyBuilder()
        self.mcp_bridge = MCPBridge(bus_name="CORE")
        self.convergence_validator = ConvergenceValidator()

        # 4. MEMORY & FEDERATION
        self.sona_emitter = SonaEmitter(bus=get_core_bus(), db_pool=self.db_pool)
        self.sona_emitter.start()

        from cynic.perception.federation.gossip import GossipManager
        instance_id = os.environ.get("CYNIC_INSTANCE_ID", os.urandom(4).hex())
        self.gossip_manager = GossipManager(instance_id=instance_id, q_table=self.qtable)

        # Agency (Manager Role)
        from cynic.kernel.organism.manager import OrganismManager
        self.organism_manager = OrganismManager(confidence_provider=self.state)

        # Sovereignty (Impact)
        from cynic.brain.agents.sovereignty import SovereigntyAgent
        self.sovereignty_agent = SovereigntyAgent(state_manager=self.state)
        self.sovereignty_agent.start()

        # 5. ASSEMBLE CORES
        cognition = CognitionCore(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            account_agent=self.account_agent,
            lod_controller=self.lod_controller
        )
        metabolism = MetabolicCore(
            scheduler=self.scheduler,
            body=self.body,
            runner=self.runner,
            llm_router=self.llm_router,
            telemetry_store=self.telemetry_store
        )
        senses = SensoryCore(
            context_compressor=self.compressor,
            world_model=self.world_model,
            source_watcher=self.source_watcher,
            topology_builder=self.topology_builder,
            mcp_bridge=self.mcp_bridge,
            convergence_validator=self.convergence_validator
        )
        memory = ArchiveCore(
            state=self.state,
            kernel_mirror=KernelMirror(),
            action_proposer=self.action_proposer,
            self_prober=self.self_prober,
            sona_emitter=self.sona_emitter,
            gossip_manager=self.gossip_manager
        )

        # 6. HANDLER REGISTRY
        from cynic.kernel.organism.handlers import (
            HandlerRegistry, discover_handler_groups, KernelServices, 
            CognitionServices, MetabolicServices, SensoryServices
        )
        
        # Internal Service Mapping
        cognition_svc = CognitionServices(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            lod_controller=self.lod_controller,
            escore_tracker=self.escore_tracker,
            health_cache={} # Placeholder
        )
        metabolic_svc = MetabolicServices(
            scheduler=self.scheduler,
            runner=self.runner,
            llm_router=self.llm_router,
            db_pool=self.db_pool
        )
        sensory_svc = SensoryServices(
            compressor=self.compressor,
            service_registry=None,
            world_model=self.world_model
        )
        services = KernelServices(cognition=cognition_svc, metabolic=metabolic_svc, senses=sensory_svc)

        handler_registry = HandlerRegistry()
        groups = discover_handler_groups(
            services,
            intelligence={"orchestrator": self.orchestrator, "scheduler": self.scheduler, "db_pool": self.db_pool, "compressor": self.compressor},
            federation={"gossip_manager": self.gossip_manager},
            judgment_executor={"orchestrator": self.orchestrator},
            axiom={"action_proposer": self.action_proposer},
            act_executor={"agency_manager": self.organism_manager, "body": self.body}
        )
        for g in groups: handler_registry.register(g)

        # 7. DEPENDENCY CONTAINER
        container = DependencyContainer(self.config)
        container.register(QTable, self.qtable)
        container.register(JudgeOrchestrator, self.orchestrator)
        container.register(OrganismState, self.state)
        # (Other registrations as needed...)

        return Organism(
            cognition=cognition,
            metabolism=metabolism,
            senses=senses,
            memory=memory,
            state=self.state,
            _pool=self.db_pool,
            container=container,
            _handler_registry=handler_registry
        )

def awaken(db_pool=None, registry=None):
    """Entry point to wake up CYNIC."""
    return _OrganismAwakener(db_pool, registry).build()
