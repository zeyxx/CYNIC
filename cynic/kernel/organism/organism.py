"""
CYNIC Organism — The living unified system.

Coordinates Brain, Body, Nerves and Memory via Event-driven architecture.
One Organism per process.
"""

from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any

from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus
from cynic.kernel.organism.anatomy import ArchiveCore, CognitionCore, MetabolicCore, SensoryCore
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
    instance_id: str = "DEFAULT"

    _pool: Any | None = None
    container: Any = None
    _handler_registry: Any | None = None
    started_at: float = field(default_factory=time.time)

    def __post_init__(self):
        # Initialize isolated buses for this instance
        from cynic.kernel.core.event_bus import get_core_bus, get_agent_bus, get_automation_bus
        self.bus = get_core_bus(self.instance_id)
        self.agent_bus = get_agent_bus(self.instance_id)
        self.automation_bus = get_automation_bus(self.instance_id)

        self._wire_event_handlers()
        from cynic.interfaces.bots.telegram_bridge import TelegramBridge

        self._telegram = TelegramBridge()
        logger.info(f"Organism [{self.instance_id}]: Nerves connected.")

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    # --- Brain Accessors ---
    @property
    def orchestrator(self):
        return self.cognition.orchestrator

    @property
    def qtable(self):
        return self.cognition.qtable

    @property
    def learning_loop(self):
        return self.cognition.learning_loop

    @property
    def dogs(self):
        return self.cognition.orchestrator.dogs

    @property
    def decide_agent(self):
        return self.cognition.decide_agent

    # --- Body Accessors ---
    @property
    def scheduler(self):
        return self.metabolism.scheduler

    @property
    def runner(self):
        return self.metabolism.runner

    # --- Memory Accessors ---
    @property
    def gossip_manager(self):
        return self.memory.gossip_manager

    @property
    def manager(self):
        return self.memory.meta_cognition

    # --- Senses Accessors ---
    @property
    def source_watcher(self):
        return self.senses.source_watcher

    @property
    def convergence_validator(self):
        return self.senses.convergence_validator

    @property
    def world_model(self):
        return self.senses.world_model

    @property
    def topology_builder(self):
        return self.senses.topology_builder

    @property
    def mcp_bridge(self):
        return self.senses.mcp_bridge

    # --- Memory Accessors (continued) ---
    @property
    def kernel_mirror(self):
        return self.memory.kernel_mirror

    @property
    def self_prober(self):
        return self.memory.self_prober

    @property
    def action_proposer(self):
        return self.memory.action_proposer

    # --- Other Accessors ---
    @property
    def residual_detector(self):
        return self.cognition.residual_detector

    @property
    def account_agent(self):
        return self.cognition.account_agent

    @property
    def llm_router(self):
        return self.metabolism.llm_router

    # --- Event Wiring ---
    def _wire_event_handlers(self) -> None:
        """Register all event bus subscriptions."""
        # Note: self.bus is initialized in __post_init__ from instance_id
        if self._handler_registry:
            self._handler_registry.wire(self.bus)

        # Core state tracking
        self.bus.on(CoreEvent.JUDGMENT_CREATED, self._on_judgment_created)
        self.bus.on(CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_changed)
        self.bus.on(CoreEvent.ANOMALY_DETECTED, self._on_anomaly_detected)
        self.bus.on(CoreEvent.LEARNING_EVENT, self._on_learning_event)

    async def _on_judgment_created(self, event: Event) -> None:
        await self.state.add_judgment(event.dict_payload)

    async def _on_consciousness_changed(self, event: Event) -> None:
        level = event.dict_payload.get("level", "REFLEX")
        await self.state.update_consciousness_level(level)

    async def _on_anomaly_detected(self, event: Event) -> None:
        """Reflex: notify Telegram on system anomalies."""
        if hasattr(self, "_telegram") and self._telegram.active:
            data = event.dict_payload
            await self._telegram.notify_anomaly(
                source=data.get("source", event.source),
                error=data.get("type", "Unknown Anomaly"),
                details=str(data.get("value", "")),
            )

    async def _on_learning_event(self, event: Event) -> None:
        """Notify Telegram when CYNIC updates its Q-Table (Learning)."""
        if hasattr(self, "_telegram") and self._telegram.active:
            p = event.dict_payload
            reward = p.get("reward", 0.0)
            # Only notify major learning (high reward or major penalty)
            if abs(reward) > 0.5:
                msg = (
                    f"🧠 <b>CYNIC LEARNED SOMETHING</b>\n\n"
                    f"<b>State:</b> <code>{p.get('state_key')}</code>\n"
                    f"<b>Action:</b> <code>{p.get('action')}</code>\n"
                    f"<b>Reward:</b> {'🟢' if reward > 0 else '🔴'} {reward:+.3f}\n"
                    f"<b>New Q-Value:</b> {p.get('q_value_new', 0.0):.3f}"
                )
                await self._telegram.notify(msg)

    async def start(self, db=None) -> None:
        """
        Start the organism's background processing loops.
        Must be called within an active event loop.

        Strict: Raises RuntimeError if any vital component fails.
        """
        logger.info(f"Organism [{self.instance_id}]: Starting vital signs...")

        try:
            # 1. Start state processing (metrics, history)
            await self.state.start_processing(db=db)

            # 2. Start SONA heartbeat (self-assessment)
            if hasattr(self.memory, "sona_emitter"):
                self.memory.sona_emitter.start()
            else:
                raise RuntimeError("Critical: SonaEmitter missing from ArchiveCore")

            # 3. Start Consciousness Scheduler (Metabolism)
            if hasattr(self.metabolism, "scheduler") and self.metabolism.scheduler:
                self.metabolism.scheduler.start()
                logger.info(f"Organism [{self.instance_id}]: Metabolism (Scheduler) activated.")

            # 4. Trigger Initial Perception (The Awakening)
            # Force a somatic sensation and a SONA_TICK to populate TUI immediately
            if hasattr(self.metabolism, "body") and self.metabolism.body:
                await self.metabolism.body.pulse()
            
            from cynic.kernel.core.events_schema import PerceptionReceivedPayload
            await self.bus.emit(Event.typed(
                CoreEvent.PERCEPTION_RECEIVED,
                PerceptionReceivedPayload(
                    reality="CYNIC",
                    source="awakening",
                    data="Organism is now awake. Initialize all systems.",
                    run_judgment=True
                ),
                source="organism"
            ))

            await self.bus.emit(Event.typed(
                CoreEvent.SONA_TICK,
                {"timestamp": time.time(), "source": "awakening"},
                source="organism"
            ))

            # 5. Start World Model
            if hasattr(self.senses, "world_model"):
                # WorldModelUpdater.start is sync event registration
                self.senses.world_model.start()

            # 4. Start Gossip (Federation) if applicable
            if hasattr(self.memory, "gossip_manager"):
                if hasattr(self.memory.gossip_manager, "start"):
                    await self.memory.gossip_manager.start()

            # 5. INTEGRITY CHECK
            self._validate_integrity()

            logger.info("Organism: All systems NOMINAL. Respiration active.")

        except Exception as e:
            logger.critical(f"Organism: FAILED TO START: {e}", exc_info=True)
            await self.stop()
            raise RuntimeError(f"Organism startup failed: {e}") from e

    def _validate_integrity(self) -> None:
        """Verify all attributes expected by UI/API are present."""
        expected_state_attrs = ["total_judgments", "consciousness", "reflex_cycles"]
        for attr in expected_state_attrs:
            if not hasattr(self.state, attr):
                raise AttributeError(f"Integrity Error: OrganismState missing '{attr}'")

        if not self.cognition.orchestrator:
            raise RuntimeError("Integrity Error: CognitionCore missing orchestrator")

    async def stop(self) -> None:
        """Graceful shutdown of all loops."""
        await self.state.stop_processing()
        if hasattr(self.memory, "sona_emitter"):
            await self.memory.sona_emitter.stop()
        if hasattr(self.memory, "gossip_manager") and hasattr(self.memory.gossip_manager, "stop"):
            await self.memory.gossip_manager.stop()
        logger.info("Organism: Dormant.")


async def awaken(db_pool=None, registry=None) -> Organism:
    """Delegates to the factory for awakening."""
    from .factory import _OrganismAwakener

    return await _OrganismAwakener(db_pool, registry).build()
