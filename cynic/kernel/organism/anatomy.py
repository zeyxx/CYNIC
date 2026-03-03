"""
CYNIC Organism Anatomy â€" Structural cores.

Defines the 4 biological systems that compose the Organism.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Optional

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.kernel.organism.brain.learning.qlearning import QTable, LearningLoop
    from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
    from cynic.kernel.organism.brain.cognition.cortex.decide import DecideAgent
    from cynic.kernel.organism.brain.cognition.cortex.account import AccountAgent
    from cynic.kernel.organism.brain.cognition.cortex.axiom_monitor import AxiomMonitor
    from cynic.kernel.organism.brain.cognition.cortex.lod import LODController

    from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
    from cynic.kernel.organism.layers.embodiment import HardwareBody
    from cynic.kernel.organism.layers.motor import MotorSystem
    from cynic.kernel.organism.metabolism.claude_sdk import ClaudeCodeRunner
    from cynic.kernel.organism.metabolism.llm_router import LLMRouter

    from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
    from cynic.kernel.core.world_model import WorldModelUpdater
    from cynic.kernel.core.topology.file_watcher import SourceWatcher
    from cynic.kernel.core.topology.topology_builder import IncrementalTopologyBuilder
    from cynic.kernel.organism.perception.somatic_gateway import SomaticGateway
    from cynic.interfaces.mcp.service import MCPBridge
    from cynic.kernel.core.convergence import ConvergenceValidator
    from cynic.kernel.protocol.knet_server import KNetServer

    from cynic.kernel.organism.state_manager import OrganismState
    from cynic.kernel.organism.brain.cognition.cortex.action_proposer import ActionProposer
    from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber
    from cynic.kernel.organism.brain.cognition.cortex.proposal_executor import ProposalExecutor
    from cynic.kernel.organism.sona_emitter import SonaEmitter
    from cynic.nervous.event_journal import EventJournal
    from cynic.nervous.loop_closure import LoopClosureValidator
    from cynic.nervous.state_reconstructor import StateReconstructor
    from cynic.nervous.event_metrics import EventMetricsCollector


@dataclass(frozen=True)
class CognitionCore:
    """BRAIN - Orchestration, consensus, learning, agents."""

    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    residual_detector: ResidualDetector
    llm_registry: Any # LLMRegistry
    decide_agent: Optional[DecideAgent] = None
    account_agent: Optional[AccountAgent] = None
    axiom_monitor: Optional[AxiomMonitor] = None
    lod_controller: Optional[LODController] = None


@dataclass(frozen=True)
class MetabolicCore:
    """BODY - Execution, scheduling, routing, telemetry, physical hardware."""

    scheduler: ConsciousnessRhythm
    body: Optional[HardwareBody] = None
    motor: Optional[MotorSystem] = None
    web_hand: Optional[Any] = None
    runner: Optional[ClaudeCodeRunner] = None
    llm_router: Optional[LLMRouter] = None
    telemetry_store: Any = None


@dataclass(frozen=True)
class SensoryCore:
    """NERVES - Perception, world model, topology, MCP."""

    context_compressor: ContextCompressor
    world_model: WorldModelUpdater
    source_watcher: SourceWatcher
    topology_builder: IncrementalTopologyBuilder
    somatic_gateway: SomaticGateway
    mcp_bridge: MCPBridge
    market_sensor: Optional[Any] = None
    web_eye: Optional[Any] = None
    internal_sensor: Optional[Any] = None
    convergence_validator: Optional[ConvergenceValidator] = None
    knet_server: Optional[KNetServer] = None


@dataclass(frozen=True)
class ArchiveCore:
    """MEMORY - Reflection, proposals, self-improvement, federation."""

    state: OrganismState
    action_proposer: Optional[ActionProposer] = None
    self_prober: Optional[SelfProber] = None
    sona_emitter: Optional[SonaEmitter] = None
    gossip_manager: Optional[Any] = None # GossipManager can have complex cycles
    meta_cognition: Optional[Any] = None
    journal: Optional[EventJournal] = None
    loop_validator: Optional[LoopClosureValidator] = None
    reconstructor: Optional[StateReconstructor] = None
    metrics_collector: Optional[EventMetricsCollector] = None
    executor: Optional[ProposalExecutor] = None
