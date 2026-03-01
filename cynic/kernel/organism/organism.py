"""
CYNIC Organism — The living unified system.
"""

from __future__ import annotations

import logging
from typing import Any
from dataclasses import dataclass

from cynic.kernel.core.container import DependencyContainer
from cynic.kernel.core.event_bus import CoreEvent, Event, current_instance_id
from cynic.kernel.organism.anatomy import ArchiveCore, CognitionCore, MetabolicCore, SensoryCore
from cynic.kernel.organism.state_manager import OrganismState

logger = logging.getLogger("cynic.kernel.organism")

@dataclass
class Organism:
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: ArchiveCore
    state: OrganismState
    instance_id: str
    container: DependencyContainer
    _pool: Any = None

    async def start(self) -> None:
        """Awaken all cores and loops."""
        # Force context identity for all underlying calls
        token = current_instance_id.set(self.instance_id)
        
        try:
            # 1. Start State Respiration
            await self.state.start_processing()

            # 2. Start Learning Loop
            if hasattr(self.cognition, "learning_loop"):
                self.cognition.learning_loop.start()

            # 3. Start Senses
            if hasattr(self.senses, "market_sensor") and self.senses.market_sensor:
                self.senses.market_sensor.start()
            if hasattr(self.senses, "internal_sensor") and self.senses.internal_sensor:
                self.senses.internal_sensor.start()

            # 4. Start World Model
            if hasattr(self.senses, "world_model"):
                self.senses.world_model.start()

            # 5. Start K-NET
            if hasattr(self.senses, "knet_server") and self.senses.knet_server:
                await self.senses.knet_server.start()

            # 6. Start Reflection (SONA)
            if self.memory.sona_emitter:
                await self.memory.sona_emitter.start()

            logger.info(f"Organism [{self.instance_id}]: All systems NOMINAL. Respiration active.")
            
            # Initial Spark
            await self.cognition.orchestrator.bus.emit(Event.typed(
                CoreEvent.AWAKENED,
                {"instance_id": self.instance_id},
                source="organism"
            ))

        except Exception as e:
            logger.critical(f"Organism: FAILED TO START: {e}", exc_info=True)
            await self.stop()
            raise RuntimeError(f"Organism startup failed: {e}") from e
        finally:
            # Context token reset is tricky in async, but here we want it to stay
            # for the duration of the task.
            pass

    async def stop(self) -> None:
        """Gracefully shutdown."""
        await self.state.stop_processing()
        if self.memory.sona_emitter:
            await self.memory.sona_emitter.stop()
        if hasattr(self.senses, "knet_server") and self.senses.knet_server:
            await self.senses.knet_server.stop()
        logger.info(f"Organism [{self.instance_id}]: Dormant.")

async def awaken(db_pool=None, registry=None) -> Organism:
    """Delegates to the factory for awakening."""
    from .factory import _OrganismAwakener
    return await _OrganismAwakener(db_pool, registry).build()
