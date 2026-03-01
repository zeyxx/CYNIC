"""
CYNIC Organism Factory — The Awakening Logic.

Handles the complex instantiation and wiring of all organism components.
"""
from __future__ import annotations

import logging
import os
from typing import Any

from cynic.interfaces.mcp.service import MCPBridge
from cynic.kernel.core.axioms import AxiomArchitecture
from cynic.kernel.core.config import CynicConfig
from cynic.kernel.core.container import DependencyContainer
from cynic.kernel.core.convergence import ConvergenceValidator
from cynic.kernel.core.escore import EScoreTracker
from cynic.kernel.core.event_bus import get_core_bus
from cynic.kernel.core.topology import IncrementalTopologyBuilder, SourceWatcher
from cynic.kernel.core.world_model import WorldModelUpdater
from cynic.kernel.organism.anatomy import ArchiveCore, CognitionCore, MetabolicCore, SensoryCore
from cynic.kernel.organism.brain.cognition.cortex.account import AccountAgent
from cynic.kernel.organism.brain.cognition.cortex.action_proposer import ActionProposer
from cynic.kernel.organism.brain.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.kernel.organism.brain.cognition.cortex.decide import DecideAgent
from cynic.kernel.organism.brain.cognition.cortex.decision_validator import DecisionValidator
from cynic.kernel.organism.brain.cognition.cortex.lod import LODController
from cynic.kernel.organism.brain.cognition.cortex.mirror import KernelMirror
from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

# All component imports
from cynic.kernel.organism.brain.cognition.neurons.discovery import discover_dogs
from cynic.kernel.organism.brain.learning.qlearning import LearningLoop, QTable
from cynic.kernel.organism.metabolism.claude_sdk import ClaudeCodeRunner
from cynic.kernel.organism.metabolism.immune.alignment_checker import AlignmentSafetyChecker
from cynic.kernel.organism.metabolism.immune.human_approval_gate import HumanApprovalGate

# Immune System & Guardrails
from cynic.kernel.organism.metabolism.immune.power_limiter import PowerLimiter
from cynic.kernel.organism.metabolism.immune.transparency_audit import TransparencyAuditTrail
from cynic.kernel.organism.metabolism.llm_router import LLMRouter
from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
from cynic.kernel.organism.metabolism.telemetry import TelemetryStore
from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
from cynic.kernel.organism.sona_emitter import SonaEmitter
from cynic.kernel.organism.state_manager import OrganismState

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
        self.dogs = discover_dogs()  # Auto-discover available dogs
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

        axiom_arch = AxiomArchitecture()
        # Provide state to axiom arch for dynamic weights
        axiom_arch.state = self.state

        self.learning_loop = LearningLoop(qtable=self.qtable, pool=self.db_pool)
        self.learning_loop.start(get_core_bus())

        self.residual_detector = ResidualDetector()
        self.residual_detector.start()

        # GASdf Executor (from config)
        gasdf_executor = None
        if self.config.gasdf_enabled:
            from cynic.kernel.organism.perception.integrations.gasdf.client import GASdfClient
            from cynic.kernel.organism.perception.integrations.gasdf.executor import GASdfExecutor
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
        self.decide_agent.start()

        self.action_proposer = ActionProposer()
        self.action_proposer.start()

        self.account_agent = AccountAgent()
        self.llm_router = LLMRouter()
        self.lod_controller = LODController()
        self.escore_tracker = EScoreTracker()
        
        # Immune System Instantiation
        self.axiom_monitor = AxiomMonitor()
        self.power_limiter = PowerLimiter()
        self.alignment_checker = AlignmentSafetyChecker()
        self.human_gate = HumanApprovalGate()
        self.audit_trail = TransparencyAuditTrail()
        
        self.decision_validator = DecisionValidator(
            power_limiter=self.power_limiter,
            alignment_checker=self.alignment_checker,
            human_gate=self.human_gate,
            audit_trail=self.audit_trail
        )

        # Connect Orchestrator to controllers
        self.orchestrator.escore_tracker = self.escore_tracker
        self.orchestrator.axiom_monitor = self.axiom_monitor
        self.orchestrator.lod_controller = self.lod_controller

        self.account_agent.set_escore_tracker(self.escore_tracker)
        self.account_agent.start()

        self.self_prober = SelfProber()
        self.self_prober.set_qtable(self.qtable)
        self.self_prober.set_residual_detector(self.residual_detector)
        self.self_prober.set_escore_tracker(self.escore_tracker)
        self.self_prober.start()

        # 2. BODY & METABOLISM
        from cynic.kernel.organism.layers.embodiment import HardwareBody
        self.body = HardwareBody()
        
        self.scheduler = ConsciousnessRhythm(
            orchestrator=self.orchestrator,
            body=self.body
        )
        self.runner = ClaudeCodeRunner(bus=get_core_bus(), sessions_registry={})
        self.telemetry_store = TelemetryStore()

        # 3. SENSORY & NERVES
        self.compressor = ContextCompressor()
        self.world_model = WorldModelUpdater()
        self.world_model.start()
        
        self.source_watcher = SourceWatcher()
        self.topology_builder = IncrementalTopologyBuilder()
        self.mcp_bridge = MCPBridge(bus_name="CORE")
        self.convergence_validator = ConvergenceValidator()
        
        from cynic.kernel.organism.perception.senses.internal import InternalSensor
        self.internal_sensor = InternalSensor()
        self.internal_sensor.start()

        # 4. MEMORY & FEDERATION
        self.sona_emitter = SonaEmitter(bus=get_core_bus(), db_pool=self.db_pool)

        from cynic.kernel.organism.perception.federation.gossip import GossipManager
        instance_id = os.environ.get("CYNIC_INSTANCE_ID", os.urandom(4).hex())
        self.gossip_manager = GossipManager(instance_id=instance_id, q_table=self.qtable)

        # Agency (Manager Role)
        from cynic.kernel.organism.manager import OrganismManager
        self.organism_manager = OrganismManager(confidence_provider=self.state)

        # Sovereignty (Impact)
        from cynic.kernel.organism.brain.agents.sovereignty import SovereigntyAgent
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
        from cynic.kernel.core.storage.gc import StorageGarbageCollector
        from cynic.kernel.organism.handlers import (
            CognitionServices,
            HandlerRegistry,
            KernelServices,
            MetabolicServices,
            SensoryServices,
        )
        storage_gc = StorageGarbageCollector() if self.db_pool else None
        
        # Internal Service Mapping
        cognition_svc = CognitionServices(
            orchestrator=self.orchestrator,
            qtable=self.qtable,
            learning_loop=self.learning_loop,
            residual_detector=self.residual_detector,
            decide_agent=self.decide_agent,
            lod_controller=self.lod_controller,
            escore_tracker=self.escore_tracker,
            axiom_monitor=self.axiom_monitor,
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
        
        # Explicit Wiring (Rigueur Senior)
        from cynic.kernel.organism.handlers.axiom import AxiomHandlers
        from cynic.kernel.organism.handlers.federation import FederationHandler
        from cynic.kernel.organism.handlers.health import HealthHandlers
        from cynic.kernel.organism.handlers.intelligence import IntelligenceHandlers
        from cynic.kernel.organism.handlers.judgment_executor import JudgmentExecutorHandler
        from cynic.kernel.organism.handlers.knet_handler import KNetHandler
        from cynic.kernel.organism.handlers.meta_cognition import MetaCognitionHandlers
        from cynic.kernel.organism.handlers.sdk import SDKHandlers

        handler_registry.register(IntelligenceHandlers(services, orchestrator=self.orchestrator, scheduler=self.scheduler, db_pool=self.db_pool, compressor=self.compressor))
        handler_registry.register(FederationHandler(services, gossip_manager=self.gossip_manager))
        handler_registry.register(AxiomHandlers(services, action_proposer=self.action_proposer))
        handler_registry.register(HealthHandlers(services, storage_gc=storage_gc, db_pool=self.db_pool))
        handler_registry.register(SDKHandlers(services, action_proposer=self.action_proposer, qtable=self.qtable))
        handler_registry.register(KNetHandler(services))
        handler_registry.register(MetaCognitionHandlers(services))
        handler_registry.register(JudgmentExecutorHandler(services, orchestrator=self.orchestrator))

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

def create_organism(db_pool=None, registry=None):
    """Alias for awaken() used by tests."""
    return awaken(db_pool, registry)
