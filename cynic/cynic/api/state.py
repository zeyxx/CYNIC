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
import threading
from dataclasses import dataclass, field
from typing import Any, TYPE_CHECKING


if TYPE_CHECKING:
    import asyncpg

from cynic.core.axioms import AxiomArchitecture
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.consciousness import get_consciousness
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
from cynic.organism import (
    Organism, CognitionCore, MetabolicCore, SensoryCore, MemoryCore,
    ConsciousState, get_conscious_state, awaken as organism_awaken
)

logger = logging.getLogger("cynic.api.state")

# Backward-compatibility aliases (god object extraction)
# These classes are now defined in cynic.organism.organism
# The aliases here allow old code to continue working
CynicOrganism = Organism
AppState = Organism

# The organism's own identity in the EScore reputation system.
# Used everywhere CYNIC tracks its own performance — 33 call sites, one constant.
_ESCORE_AGENT_ID = "agent:cynic"


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
        # Get container for guidance path
        container = get_app_container()
        guidance_dir = os.path.dirname(container.guidance_path)
        os.makedirs(guidance_dir, exist_ok=True)

        # Write instance-specific guidance file
        with open(container.guidance_path, "w", encoding="utf-8") as fh:
            json.dump(payload, fh)

        # Also write guidance.json for backward compatibility (single-instance hooks)
        default_guidance = os.path.join(guidance_dir, "guidance.json")
        with open(default_guidance, "w", encoding="utf-8") as fh:
            json.dump(payload, fh)
    except Exception as exc:
        logger.debug("guidance.json write skipped: %s", exc)


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
    organism: "Organism"  # Type imported at bottom of file
    instance_id: str  # Unique per process (uuid.uuid4().hex[:8])
    guidance_path: str  # ~/.cynic/guidance-{instance_id}.json
    started_at: float = field(default_factory=time.time)

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at


# Process-level singleton — set during lifespan startup (thread-safe)
# FastAPI's lifespan runs once at startup, so Lock overhead is minimal.
# Protects against multi-threaded scenarios (ASGI servers, concurrent requests).
_app_container: Optional[AppContainer] = None
_app_container_lock = threading.RLock()  # Reentrant for nested access


def set_app_container(container: AppContainer) -> None:
    """Set the app container during lifespan startup (thread-safe)."""
    global _app_container
    with _app_container_lock:
        _app_container = container


def get_app_container() -> AppContainer:
    """
    Get the app container (FastAPI dependency — thread-safe).

    Used as: def route(container: AppContainer = Depends(get_app_container))
    Raises RuntimeError if lifespan hasn't started yet.
    """
    with _app_container_lock:
        if _app_container is None:
            raise RuntimeError("AppContainer not initialized — lifespan not started")
        return _app_container



def awaken(db_pool=None, registry=None) -> Organism:
    """Awaken the CYNIC organism. Call once from lifespan startup.

    This function delegates to cynic.organism.awaken() after the god object extraction.
    """
    return organism_awaken(db_pool=db_pool, registry=registry)


async def restore_state(container: AppContainer) -> None:
    """
    Restore persistent state after organism awakening.

    Call this in the FastAPI lifespan, AFTER awaken() and creating AppContainer.
    Initializes:
      - ConsciousState subscriptions to 3 event buses (Phase 1 wiring)
      - EScoreTracker entities from e_scores table (γ4)
      - ContextCompressor session from ~/.cynic/session-latest.json (γ2)
    """
    from cynic.senses import checkpoint as _ckpt

    state = container.organism

    # Phase 1: Wire ConsciousState to event buses (organism observation ports)
    core_bus = get_core_bus()
    automation_bus = get_automation_bus()
    agent_bus = get_agent_bus()
    await state.conscious_state.initialize_from_buses(core_bus, automation_bus, agent_bus)
    logger.info("restore_state: ConsciousState initialized and subscribed to 3 event buses")

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
