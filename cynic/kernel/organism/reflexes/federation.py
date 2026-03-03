"""
FederationHandler " P2P Knowledge Sharing Organ.

Listens to JUDGMENT_CREATED and RESIDUAL_HIGH events to trigger gossip
sharing between federated CYNIC instances.
"""

import logging
from collections.abc import Callable
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import SensoryServices
from cynic.kernel.organism.perception.federation.gossip import GossipManager

logger = logging.getLogger("cynic.kernel.organism.reflexes.federation")


class FederationHandler(HandlerGroup):
    """Handler group that manages P2P knowledge sharing."""

    def __init__(
        self,
        sensory: SensoryServices,
        *,
        gossip_manager: GossipManager,
        bus: Optional[EventBus] = None,
    ):
        """Initialize handler with sensory services and gossip manager."""
        super().__init__(bus=bus)
        self._sensory = sensory
        self.gossip_manager = gossip_manager
        self._judgment_count = 0
        logger.info("FederationHandler active")

    @property
    def name(self) -> str:
        return "federation"

    def subscriptions(self) -> list[tuple[CoreEvent, Callable]]:
        return [
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_created),
            (CoreEvent.RESIDUAL_HIGH, self._on_residual_high),
        ]

    def dependencies(self) -> frozenset[str]:
        return frozenset({"gossip_manager"})

    async def _on_judgment_created(self, event: Event) -> None:
        """JUDGMENT_CREATED ' increment count and check for gossip trigger."""
        self._judgment_count += 1
        self.gossip_manager.on_judgment(self._judgment_count)

    async def _on_residual_high(self, event: Event) -> None:
        """RESIDUAL_HIGH ' record unnameable pattern for sharing."""
        try:
            payload = event.dict_payload or {}
            reality = payload.get("reality", "UNKNOWN")
            self.gossip_manager.record_unnameable(reality)
        except Exception as e:
            logger.warning("Failed to record unnameable in federation: %s", e)
