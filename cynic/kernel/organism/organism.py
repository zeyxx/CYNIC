"""
CYNIC Organism — The living unified system.

Coordinates Brain, Body, Nerves and Memory via Event-driven architecture.
One Organism per process. 
"""
from __future__ import annotations
import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

from cynic.kernel.core.event_bus import get_core_bus, CoreEvent, Event
from cynic.kernel.organism.anatomy import CognitionCore, MetabolicCore, SensoryCore, ArchiveCore
from cynic.kernel.organism.state_manager import OrganismState

# Aliases for assembler/builders
MemoryCore = ArchiveCore

logger = logging.getLogger("cynic.kernel.organism")

@dataclass
class Organism:
    """
    The root coordinator for all CYNIC activity.
    Organized into 4 specialized cores.
    """
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: ArchiveCore
    state: OrganismState
    
    _pool: Optional[Any] = None
    container: Any = None
    _handler_registry: Optional[Any] = None
    started_at: float = field(default_factory=time.time)

    def __post_init__(self):
        self._wire_event_handlers()
        logger.info("Organism: Post-init complete. Nerves connected.")

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    # --- Brain Accessors ---
    @property
    def orchestrator(self): return self.cognition.orchestrator
    @property
    def qtable(self): return self.cognition.qtable
    @property
    def learning_loop(self): return self.cognition.learning_loop
    @property
    def dogs(self): return self.cognition.orchestrator.dogs
    @property
    def decide_agent(self): return self.cognition.decide_agent

    # --- Body Accessors ---
    @property
    def scheduler(self): return self.metabolism.scheduler
    @property
    def runner(self): return self.metabolism.runner

    # --- Memory Accessors ---
    @property
    def gossip_manager(self): return self.memory.gossip_manager
    @property
    def manager(self): return self.memory.meta_cognition

    # --- Event Wiring ---
    def _wire_event_handlers(self) -> None:
        """Register all event bus subscriptions."""
        bus = get_core_bus()
        if self._handler_registry:
            self._handler_registry.wire(bus)

        # Core state tracking
        bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_created)
        bus.on(CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_changed)

    async def _on_judgment_created(self, event: Event) -> None:
        await self.state.add_judgment(event.dict_payload)
        
    async def _on_consciousness_changed(self, event: Event) -> None:
        level = event.dict_payload.get("level", "REFLEX")
        await self.state.update_consciousness_level(level)

def awaken(db_pool=None, registry=None) -> Organism:
    """Delegates to the factory for awakening."""
    from .factory import _OrganismAwakener
    return _OrganismAwakener(db_pool, registry).build()
