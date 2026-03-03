"""
CYNIC Organism — The living unified system.
"""

from __future__ import annotations

import logging
from typing import Any
from dataclasses import dataclass

from prometheus_client import Counter, Gauge

from cynic.kernel.core.container import DependencyContainer
from cynic.kernel.core.event_bus import CoreEvent, Event, current_instance_id
from cynic.kernel.organism.anatomy import ArchiveCore, CognitionCore, MetabolicCore, SensoryCore
from cynic.kernel.organism.state_manager import OrganismState

logger = logging.getLogger("cynic.kernel.organism")

# Prometheus metrics for organism observability
organism_consciousness_level = Gauge(
    "cynic_organism_consciousness_level",
    "Current consciousness level of the organism (0-100)"
)

organism_judgments_total = Counter(
    "cynic_organism_judgments_total",
    "Total number of judgments created by the organism"
)

@dataclass
class Organism:
    cognition: CognitionCore
    metabolism: MetabolicCore
    senses: SensoryCore
    memory: ArchiveCore
    state: OrganismState
    instance_id: str
    container: DependencyContainer
    vascular: Any = None
    bridge: Any = None
    automation_bus: Any = None
    agent_bus: Any = None
    _pool: Any = None
    _start_time: float = 0.0 # Initialized in start() or factory

    @property
    def bus(self):
        """Unified access to the core event bus."""
        return self.cognition.orchestrator.bus

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

            # 3. Start Senses (Selective)
            if hasattr(self.senses, "somatic_gateway") and self.senses.somatic_gateway:
                await self.senses.somatic_gateway.start()
                
            if hasattr(self.senses, "internal_sensor") and self.senses.internal_sensor:
                self.senses.internal_sensor.start()
            
            # Note: market_sensor is NOT started by default to avoid noise
            # It must be started explicitly by the orchestrator if needed

            # 4. Start World Model
            if hasattr(self.senses, "world_model"):
                self.senses.world_model.start()

            # 5. Start K-NET
            if hasattr(self.senses, "knet_server") and self.senses.knet_server:
                await self.senses.knet_server.start()

            # 6. Start Reflection (SONA)
            if self.memory.sona_emitter:
                await self.memory.sona_emitter.start()

            # 7. Start EventForwarder (PHASE 2: SIEM Foundation)
            if hasattr(self, "event_forwarder") and self.event_forwarder:
                await self.event_forwarder.start()
                logger.info(f"Organism [{self.instance_id}]: EventForwarder started (SIEM logging active)")

            logger.info(f"Organism [{self.instance_id}]: All systems NOMINAL. Respiration active.")

            # Wire observability metrics handlers
            bus = self.cognition.orchestrator.bus
            bus.on(CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_changed)
            bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_created)

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
        """Gracefully shutdown all components in reverse order of startup."""
        logger.info(f"Organism [{self.instance_id}]: Initiating shutdown sequence...")

        # 1. COGNITION SHUTDOWN
        try:
            # Stop orchestrator first (stops judgment flow)
            if hasattr(self.cognition, "orchestrator") and hasattr(self.cognition.orchestrator, "stop"):
                await self.cognition.orchestrator.stop()
        except Exception as e:
            logger.debug(f"Error stopping orchestrator: {e}")

        try:
            # Stop residual detector
            if hasattr(self.cognition, "residual_detector") and hasattr(self.cognition.residual_detector, "stop"):
                self.cognition.residual_detector.stop()
        except Exception as e:
            logger.debug(f"Error stopping residual_detector: {e}")

        try:
            # Stop learning loop
            if hasattr(self.cognition, "learning_loop") and hasattr(self.cognition.learning_loop, "stop"):
                self.cognition.learning_loop.stop()
        except Exception as e:
            logger.debug(f"Error stopping learning_loop: {e}")

        # 2. MEMORY/ARCHIVE SHUTDOWN
        try:
            # Stop self-prober
            if hasattr(self.memory, "self_prober") and hasattr(self.memory.self_prober, "stop"):
                self.memory.self_prober.stop()
        except Exception as e:
            logger.debug(f"Error stopping self_prober: {e}")

        try:
            # Stop SONA emitter
            if self.memory.sona_emitter and hasattr(self.memory.sona_emitter, "stop"):
                await self.memory.sona_emitter.stop()
        except Exception as e:
            logger.debug(f"Error stopping sona_emitter: {e}")

        try:
            # Stop executor
            if hasattr(self.memory, "executor") and hasattr(self.memory.executor, "stop"):
                self.memory.executor.stop()
        except Exception as e:
            logger.debug(f"Error stopping executor: {e}")

        try:
            # Stop gossip manager
            if hasattr(self.memory, "gossip_manager") and hasattr(self.memory.gossip_manager, "stop"):
                self.memory.gossip_manager.stop()
        except Exception as e:
            logger.debug(f"Error stopping gossip_manager: {e}")

        # 3. SENSORY SHUTDOWN
        try:
            # Stop world model
            if hasattr(self.senses, "world_model") and hasattr(self.senses.world_model, "stop"):
                self.senses.world_model.stop()
        except Exception as e:
            logger.debug(f"Error stopping world_model: {e}")

        try:
            # Stop internal sensor
            if hasattr(self.senses, "internal_sensor") and self.senses.internal_sensor:
                if hasattr(self.senses.internal_sensor, "stop"):
                    self.senses.internal_sensor.stop()
        except Exception as e:
            logger.debug(f"Error stopping internal_sensor: {e}")

        try:
            # Stop market sensor
            if hasattr(self.senses, "market_sensor") and self.senses.market_sensor:
                if hasattr(self.senses.market_sensor, "stop"):
                    self.senses.market_sensor.stop()
        except Exception as e:
            logger.debug(f"Error stopping market_sensor: {e}")

        try:
            # Stop source watcher
            if hasattr(self.senses, "source_watcher") and hasattr(self.senses.source_watcher, "stop"):
                self.senses.source_watcher.stop()
        except Exception as e:
            logger.debug(f"Error stopping source_watcher: {e}")

        try:
            # Stop K-NET server
            if hasattr(self.senses, "knet_server") and self.senses.knet_server:
                if hasattr(self.senses.knet_server, "stop"):
                    await self.senses.knet_server.stop()
        except Exception as e:
            logger.debug(f"Error stopping knet_server: {e}")

        # 4. METABOLISM/BODY SHUTDOWN
        try:
            # Stop scheduler
            if hasattr(self.metabolism, "scheduler") and hasattr(self.metabolism.scheduler, "stop"):
                result = self.metabolism.scheduler.stop()
                if result and hasattr(result, '__await__'):
                    await result
        except Exception as e:
            logger.debug(f"Error stopping scheduler: {e}")

        try:
            # Stop motor
            if hasattr(self.metabolism, "motor") and hasattr(self.metabolism.motor, "stop"):
                self.metabolism.motor.stop()
        except Exception as e:
            logger.debug(f"Error stopping motor: {e}")

        # 5. EVENT FORWARDER SHUTDOWN (PHASE 2: SIEM Foundation)
        try:
            if hasattr(self, "event_forwarder") and self.event_forwarder:
                await self.event_forwarder.stop()
        except Exception as e:
            logger.debug(f"Error stopping event_forwarder: {e}")

        # 6. STATE SHUTDOWN (last)
        try:
            await self.state.stop_processing()
        except Exception as e:
            logger.debug(f"Error stopping state: {e}")

        logger.info(f"Organism [{self.instance_id}]: Dormant. Shutdown complete.")

    async def _on_consciousness_changed(self, event: Event) -> None:
        """Update consciousness level metric when consciousness changes."""
        try:
            payload = event.dict_payload
            level = payload.get("level", 0)
            organism_consciousness_level.set(level)
        except Exception as e:
            logger.debug(f"Error updating consciousness metric: {e}")

    async def _on_judgment_created(self, event: Event) -> None:
        """Increment judgment counter when judgment is created."""
        try:
            organism_judgments_total.inc()
        except Exception as e:
            logger.debug(f"Error updating judgment counter: {e}")

async def awaken(db_pool=None, registry=None) -> Organism:
    """Delegates to the factory for awakening."""
    from .factory import _OrganismAwakener
    return await _OrganismAwakener(db_pool, registry).build()
