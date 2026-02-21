"""
Specialized service groups — reducing KernelServices god object.

Instead of one god KernelServices with all components, decompose into
three domain-specific service facades:
- CognitionServices (BRAIN: judgment, learning, axioms, LOD)
- MetabolicServices (BODY: execution, scheduling, routing, telemetry)
- SensoryServices (SENSES: compression, registry, topology awareness)

Each can grow independently. Each has focused, domain-specific methods.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.escore import EScoreTracker
from cynic.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.cognition.cortex.lod import LODController
from cynic.core.events_schema import AxiomActivatedPayload, ConsciousnessChangedPayload

if TYPE_CHECKING:
    from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.cognition.cortex.decide import DecideAgent
    from cynic.cognition.cortex.residual import ResidualDetector
    from cynic.learning.qlearning import QTable, LearningLoop
    from cynic.scheduler import ConsciousnessRhythm
    from cynic.metabolism.runner import ClaudeCodeRunner
    from cynic.metabolism.llm_router import LLMRouter
    from cynic.senses.compressor import ContextCompressor
    from cynic.nervous import ServiceStateRegistry
    from asyncpg import Pool


@dataclass
class CognitionServices:
    """
    BRAIN operations — Judgment, learning, axioms, consciousness levels.

    Handles:
    - Axiom signaling and activation tracking
    - Level of Detail (LOD) assessment and transitions
    - EScore reputation tracking
    - Health metric management
    """

    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    residual_detector: ResidualDetector
    decide_agent: Optional[DecideAgent]
    axiom_monitor: AxiomMonitor
    lod_controller: LODController
    escore_tracker: EScoreTracker
    health_cache: dict[str, float]  # error_rate, latency_ms, queue_depth, memory_pct, disk_pct

    async def signal_axiom(self, axiom: str, source: str, **extra: Any) -> str:
        """
        Signal an axiom — emits AXIOM_ACTIVATED if it just became ACTIVE.

        Returns the new axiom state string (e.g. ``"ACTIVE"``, ``"RISING"``).
        Any ``extra`` kwargs are merged into the payload.

        Usage::

            new_state = await cognition.signal_axiom(
                "ANTIFRAGILITY", "judgment_intelligence",
                trigger="RECOVERY",
            )
        """
        new_state = self.axiom_monitor.signal(axiom)
        if new_state == "ACTIVE":
            await get_core_bus().emit(
                Event.typed(
                    CoreEvent.AXIOM_ACTIVATED,
                    AxiomActivatedPayload(
                        axiom=axiom,
                        maturity=self.axiom_monitor.get_maturity(axiom),
                        **extra,
                    ),
                    source=source,
                )
            )
        return new_state

    async def assess_lod(self) -> object:  # Returns SurvivalLOD
        """
        Assess LOD from health_cache — emits CONSCIOUSNESS_CHANGED on transition.
        """
        prev = self.lod_controller.current
        result = self.lod_controller.assess(**self.health_cache)
        if result != prev:
            await get_core_bus().emit(
                Event.typed(
                    CoreEvent.CONSCIOUSNESS_CHANGED,
                    ConsciousnessChangedPayload(
                        from_lod=prev.value,
                        to_lod=result.value,
                        from_name=prev.name,
                        to_name=result.name,
                        direction="DOWN" if result > prev else "UP",
                    ),
                    source="lod_controller",
                )
            )
        return result

    def update_health_cache(self, **metrics: float) -> None:
        """Update one or more health metrics."""
        self.health_cache.update(metrics)


@dataclass
class MetabolicServices:
    """
    BODY operations — Execution, scheduling, routing, benchmarking, telemetry.

    Handles:
    - Task execution via ClaudeCodeRunner
    - LLM routing and model selection
    - Consciousness rhythm and scheduler lifecycle
    - Performance telemetry and benchmarking
    """

    scheduler: ConsciousnessRhythm
    runner: Optional[ClaudeCodeRunner] = None
    llm_router: Optional[LLMRouter] = None
    db_pool: Optional[Pool] = None

    def is_runner_available(self) -> bool:
        """Check if autonomous task execution is available."""
        return self.runner is not None

    def is_llm_available(self) -> bool:
        """Check if LLM routing is available."""
        return self.llm_router is not None

    def is_db_available(self) -> bool:
        """Check if database is available."""
        return self.db_pool is not None


@dataclass
class SensoryServices:
    """
    SENSES operations — Perception, topology awareness, compression, registry.

    Handles:
    - Context compression and token budgeting
    - Service state monitoring and health registry
    - Source code topology tracking and change awareness
    - World model updates
    """

    compressor: ContextCompressor
    service_registry: ServiceStateRegistry
    world_model: Any  # WorldModelUpdater — avoid circular import

    def compress_context(self, limit: int = 200) -> str:
        """Get compressed context within token budget."""
        return self.compressor.get_compressed_context(limit)


@dataclass
class KernelServices:
    """
    Unified kernel services — aggregator of three domain-specific service groups.

    This thin wrapper coordinates CognitionServices, MetabolicServices, and
    SensoryServices. It's the "bloodstream" that routes requests to appropriate domains.

    NEW ARCHITECTURE prevents god object growth by making domain boundaries explicit.
    """

    cognition: CognitionServices
    metabolic: MetabolicServices
    senses: SensoryServices

    # Backward compatibility: forward axiom operations to cognition
    async def signal_axiom(self, axiom: str, source: str, **extra: Any) -> str:
        """Delegate to CognitionServices."""
        return await self.cognition.signal_axiom(axiom, source, **extra)

    async def assess_lod(self) -> object:
        """Delegate to CognitionServices."""
        return await self.cognition.assess_lod()

    # Batch health update (used by all handlers collecting metrics)
    def update_health_cache(self, **metrics: float) -> None:
        """Update health metrics in cognition."""
        self.cognition.update_health_cache(**metrics)
