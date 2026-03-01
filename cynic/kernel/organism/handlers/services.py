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
from typing import TYPE_CHECKING, Any, Optional

from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.kernel.core.escore import EScoreTracker
from cynic.kernel.organism.brain.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.kernel.organism.brain.cognition.cortex.lod import LODController
from cynic.kernel.core.events_schema import AxiomActivatedPayload, ConsciousnessChangedPayload

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.kernel.organism.brain.cognition.cortex.decide import DecideAgent
    from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector
    from cynic.kernel.organism.brain.learning.qlearning import QTable, LearningLoop
    from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
    from cynic.kernel.organism.metabolism.claude_sdk import ClaudeCodeRunner
    from cynic.kernel.organism.metabolism.llm_router import LLMRouter
    from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
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

    # Backward compatibility: expose escore_tracker at top level for handlers
    @property
    def escore_tracker(self) -> EScoreTracker:
        """Backward compatibility: escore_tracker nested in cognition."""
        return self.cognition.escore_tracker

    # Backward compatibility: expose axiom_monitor at top level for handlers
    @property
    def axiom_monitor(self) -> AxiomMonitor:
        """Backward compatibility: axiom_monitor nested in cognition."""
        return self.cognition.axiom_monitor

    # Backward compatibility: expose lod_controller at top level for handlers
    @property
    def lod_controller(self) -> LODController:
        """Backward compatibility: lod_controller nested in cognition."""
        return self.cognition.lod_controller

    # Backward compatibility: expose health_cache at top level for handlers
    @property
    def health_cache(self) -> dict[str, float]:
        """Backward compatibility: health_cache nested in cognition."""
        return self.cognition.health_cache

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
