"""
CYNIC Organism Anatomy â€” Structural cores.

Defines the 4 biological systems that compose the Organism.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    pass


@dataclass
class CognitionCore:
    """BRAIN â€” Orchestration, consensus, learning, agents."""

    orchestrator: Any  # JudgeOrchestrator
    qtable: Any  # QTable
    learning_loop: Any  # LearningLoop
    residual_detector: Any  # ResidualDetector
    decide_agent: Any | None = None  # DecideAgent
    account_agent: Any | None = None  # AccountAgent
    axiom_monitor: Any = None  # AxiomMonitor
    lod_controller: Any = None  # LODController


@dataclass
class MetabolicCore:
    """BODY â€” Execution, scheduling, routing, telemetry, physical hardware."""

    scheduler: Any  # ConsciousnessRhythm
    body: Any | None = None  # HardwareBody
    runner: Any | None = None  # ClaudeCodeRunner
    llm_router: Any | None = None  # LLMRouter
    telemetry_store: Any = field(default_factory=lambda: None)  # TelemetryStore


@dataclass
class SensoryCore:
    """NERVES â€” Perception, world model, topology, MCP."""

    context_compressor: Any  # ContextCompressor
    world_model: Any  # WorldModelUpdater
    source_watcher: Any  # SourceWatcher
    topology_builder: Any  # IncrementalTopologyBuilder
    mcp_bridge: Any  # MCPBridge
    convergence_validator: Any = None
    internal_sensor: Any | None = None


@dataclass
class ArchiveCore:
    """MEMORY â€” Reflection, proposals, self-improvement, federation."""

    state: Any  # OrganismState
    kernel_mirror: Any = field(default_factory=lambda: None)  # KernelMirror
    action_proposer: Any = field(default_factory=lambda: None)  # ActionProposer
    self_prober: Any = field(default_factory=lambda: None)  # SelfProber
    sona_emitter: Any | None = None  # SonaEmitter
    gossip_manager: Any | None = None
    meta_cognition: Any | None = None
