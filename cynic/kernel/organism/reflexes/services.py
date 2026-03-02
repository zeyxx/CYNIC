"""
Specialized service groups â€” domain-specific facades for handlers.

Decomposes the organism into three functional areas:
- CognitionServices (BRAIN: judgment, learning, axioms, LOD)
- MetabolicServices (BODY: hardware, scheduler, sdk, actuators)
- SensoryServices   (NERVES: perception, world model, topology, MCP)

Ï-Law: VERIFY â€” explicit dependency injection via domain facades.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cynic.kernel.core.escore import EScoreTracker
    from cynic.kernel.organism.brain.cognition.cortex.axiom_monitor import AxiomMonitor
    from cynic.kernel.organism.brain.cognition.cortex.lod import LODController
    from cynic.kernel.organism.brain.learning.qlearning import QTable
    from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
    from cynic.kernel.organism.perception.senses.compressor import ContextCompressor
    from cynic.nervous.service_registry import ServiceStateRegistry

logger = logging.getLogger("cynic.kernel.handlers.services")


@dataclass
class CognitionServices:
    """
    BRAIN Domain Services.
    Handles:
    - Reputation (E-Score)
    - Axiom health and activation
    - Level of Detail (LOD) scaling
    - Learning (Q-Table)
    """

    escore_tracker: EScoreTracker
    axiom_monitor: AxiomMonitor
    lod_controller: LODController
    qtable: QTable
    health_cache: dict[str, float]

    async def signal_axiom(self, axiom: str, source: str, **kwargs) -> None:
        """Signal an axiom event via the monitor."""
        await self.axiom_monitor.signal(axiom, source, **kwargs)

    async def assess_lod(self) -> None:
        """Trigger a LOD recalculation based on health cache."""
        await self.lod_controller.assess_from_cache(self.health_cache)

    def update_health_cache(self, **metrics: float) -> None:
        """Update health metrics."""
        self.health_cache.update(metrics)


@dataclass
class MetabolicServices:
    """
    BODY Domain Services.
    Handles:
    - Resource scheduling
    - Hardware body state
    - Task execution (SDK/Claude)
    """

    scheduler: ConsciousnessRhythm
    body: Any  # HardwareBody
    runner: Any  # ClaudeCodeRunner


@dataclass
class SensoryCoreServices:
    """
    NERVES Domain Services.
    Handles:
    - Context compression and token budgeting
    - Service state monitoring and health registry
    - Source code topology tracking and change awareness
    - World model updates
    """

    compressor: ContextCompressor
    service_registry: ServiceStateRegistry
    world_model: Any  # WorldModelUpdater â€” avoid circular import

    def compress_context(self, limit: int = 200) -> str:
        """Get compressed context within token budget."""
        return self.compressor.get_compressed_context(limit)


# SensoryCoreServices is also known as SensoryServices in legacy references
SensoryServices = SensoryCoreServices
