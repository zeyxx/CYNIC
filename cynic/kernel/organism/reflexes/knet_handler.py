"""
Îº-NET Handler â€” Bridges Organism Events to the Îº-NET Protocol.
Listens to internal events and broadcasts them as Îº-PULSE messages.
"""

from __future__ import annotations

import logging
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import SensoryServices
from cynic.kernel.protocol.kpulse import PulseMessage, PulseType

logger = logging.getLogger("cynic.kernel.organism.reflexes.knet")

class KNetHandler(HandlerGroup):
    """Bridges internal consciousness to the distributed Îº-NET nerves."""

    def __init__(self, sensory: SensoryServices, bus: Optional[EventBus] = None) -> None:
        super().__init__(bus=bus)
        self._sensory = sensory
        self._server = None

    @property
    def name(self) -> str:
        return "knet_bridge"

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.SONA_TICK, self._on_pulse),
            (CoreEvent.JUDGMENT_CREATED, self._on_pulse),
            (CoreEvent.ACTION_PROPOSED, self._on_pulse),
        ]

    async def _on_pulse(self, event: Event) -> None:
        """Capture internal event and broadcast as Îº-PULSE."""
        try:
            # 1. Get the knet server (async lazy init)
            if not self._server:
                from cynic.kernel.protocol.knet_server import get_knet_server

                # Note: This might already be started by FastAPI, but we ensure access
                self._server = await get_knet_server()

            # 2. Map event to PulseType
            p_type = PulseType.SOMATIC_SYNC
            if event.type == CoreEvent.JUDGMENT_CREATED:
                p_type = PulseType.NEURAL_PULSE
            elif event.type == CoreEvent.ACTION_PROPOSED:
                p_type = PulseType.INTENT_SIGNAL

            # 3. Build the Pulse
            # We wrap the event payload into a standardized Pulse
            from cynic.interfaces.api.state import get_state

            org = get_state()

            pulse_data = {"event_type": event.type, "payload": event.dict_payload}

            # Enrich with global health if it's a heartbeat
            if p_type == PulseType.SOMATIC_SYNC and org:
                stats = await org.state.get_stats()
                pulse_data["mind"] = {
                    "status": "AWAKE",
                    "thinking": stats.get("current_analysis", ""),
                    "confidence": stats.get("confidence", 0.618),
                    "e_score": stats.get("e_score", 50.0),
                    "axiom_scores": stats.get("axiom_scores", {}),
                }
                # Add hardware if available
                if hasattr(org.metabolism, "body") and org.metabolism.body:
                    body_state = getattr(org.metabolism.body, "_last_state", None)
                    if body_state:
                        pulse_data["hardware"] = body_state.to_dict()

            pulse = PulseMessage(type=p_type, data=pulse_data)

            # 4. Broadcast to all connected nerves
            await self._server.broadcast(pulse)

        except Exception as e:
            logger.debug(f"Îº-NET Handler error: {e}")
