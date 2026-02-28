"""
FederationHandler — P2P Knowledge Sharing Organ.

Listens to JUDGMENT_CREATED and RESIDUAL_HIGH events to trigger gossip
sharing between federated CYNIC instances.
"""
import logging
from typing import Callable, Any
from cynic.kernel.core.event_bus import Event, CoreEvent
from cynic.kernel.organism.handlers.base import HandlerGroup
from cynic.kernel.organism.handlers.services import KernelServices
from cynic.perception.federation.gossip import GossipManager

logger = logging.getLogger("cynic.kernel.organism.handlers.federation")


class FederationHandler(HandlerGroup):
    """Handler group that manages P2P knowledge sharing."""

    def __init__(self, svc: KernelServices, *, gossip_manager: GossipManager):
        """Initialize handler with kernel services and gossip manager."""
        self.svc = svc
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
        """JUDGMENT_CREATED → increment count and check for gossip trigger."""
        self._judgment_count += 1
        self.gossip_manager.on_judgment(self._judgment_count)

    async def _on_residual_high(self, event: Event) -> None:
        """RESIDUAL_HIGH → record unnameable pattern for sharing."""
        try:
            payload = event.dict_payload or {}
            reality = payload.get("reality", "UNKNOWN")
            self.gossip_manager.record_unnameable(reality)
        except Exception as e:
            logger.warning("Failed to record unnameable in federation: %s", e)
