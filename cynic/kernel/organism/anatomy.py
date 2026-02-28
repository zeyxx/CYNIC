"""
CYNIC Organism Anatomy — Structural cores.

Defines the 4 biological systems that compose the Organism.
"""
from __future__ import annotations
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.brain.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.brain.cognition.cortex.residual import ResidualDetector
    from cynic.brain.cognition.cortex.decide import DecideAgent
    from cynic.brain.cognition.cortex.account import AccountAgent
    from cynic.brain.cognition.cortex.self_probe import SelfProber
    from cynic.brain.cognition.cortex.mirror import KernelMirror
    from cynic.brain.cognition.cortex.action_proposer import ActionProposer
    from cynic.kernel.core.escore import EScoreTracker
    from cynic.brain.learning.qlearning import QTable, LearningLoop
    from cynic.kernel.core.config import CynicConfig
    from cynic.kernel.core.event_bus import EventBus
    from cynic.kernel.core.consciousness import ConsciousnessRhythm
    from cynic.metabolism.telemetry import TelemetryStore
    from cynic.metabolism.llm_router import LLMRouter
    from cynic.metabolism.runner import ClaudeCodeRunner
    from cynic.kernel.core.topology import SourceWatcher, IncrementalTopologyBuilder, HotReloadCoordinator, TopologyMirror, ChangeTracker, ChangeAnalyzer
    from cynic.kernel.core.convergence import ConvergenceValidator
    from cynic.interfaces.mcp.service import MCPBridge
    from cynic.kernel.kernel.core.consciousness import ContextCompressor # Fix for the double kernel if present
    from cynic.kernel.core.consciousness import ContextCompressor
    from cynic.kernel.organism.state_manager import OrganismState
    from cynic.kernel.organism.sona_emitter import SonaEmitter

@dataclass
class CognitionCore:
    """BRAIN — Orchestration, consensus, learning, agents."""
    orchestrator: Any # JudgeOrchestrator
    qtable: Any # QTable
    learning_loop: Any # LearningLoop
    residual_detector: Any # ResidualDetector
    decide_agent: Optional[Any] = None # DecideAgent
    account_agent: Optional[Any] = None # AccountAgent
    axiom_monitor: Any = None # AxiomMonitor
    lod_controller: Any = None # LODController

@dataclass
class MetabolicCore:
    """BODY — Execution, scheduling, routing, telemetry, physical hardware."""
    scheduler: Any # ConsciousnessRhythm
    body: Optional[Any] = None # HardwareBody
    runner: Optional[Any] = None # ClaudeCodeRunner
    llm_router: Optional[Any] = None # LLMRouter
    telemetry_store: Any = field(default_factory=lambda: None) # TelemetryStore

@dataclass
class SensoryCore:
    """NERVES — Perception, world model, topology, MCP."""
    context_compressor: Any # ContextCompressor
    world_model: Any # WorldModelUpdater
    source_watcher: Any # SourceWatcher
    topology_builder: Any # IncrementalTopologyBuilder
    mcp_bridge: Any # MCPBridge
    convergence_validator: Any = None

@dataclass
class ArchiveCore:
    """MEMORY — Reflection, proposals, self-improvement, federation."""
    state: Any # OrganismState
    kernel_mirror: Any = field(default_factory=lambda: None) # KernelMirror
    action_proposer: Any = field(default_factory=lambda: None) # ActionProposer
    self_prober: Any = field(default_factory=lambda: None) # SelfProber
    sona_emitter: Optional[Any] = None # SonaEmitter
    gossip_manager: Optional[Any] = None
    meta_cognition: Optional[Any] = None
