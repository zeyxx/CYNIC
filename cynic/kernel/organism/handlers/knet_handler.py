"""
κ-NET Handler — Bridges Organism Events to the κ-NET Protocol.
Listens to internal events and broadcasts them as κ-PULSE messages.
"""

from __future__ import annotations

import logging
from typing import Any

from cynic.kernel.core.event_bus import Event, CoreEvent
from cynic.kernel.organism.handlers.base import HandlerGroup, KernelServices
from cynic.kernel.protocol.kpulse import PulseMessage, PulseType

logger = logging.getLogger("cynic.kernel.organism.handlers.knet")

class KNetHandler(HandlerGroup):
    """Bridges internal consciousness to the distributed κ-NET nerves."""

    def __init__(self, svc: KernelServices) -> None:
        self._svc = svc
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
        """Capture internal event and broadcast as κ-PULSE."""
        try:
            # 1. Get the knet server (async lazy init)
            if not self._server:
                from cynic.kernel.protocol.knet_server import get_knet_server
                # Note: This might already be started by FastAPI, but we ensure access
                self._server = await get_knet_server()

            # 2. Map event to PulseType
            p_type = PulseType.SOMATIC_SYNC
            if event.topic == CoreEvent.JUDGMENT_CREATED:
                p_type = PulseType.NEURAL_PULSE
            elif event.topic == CoreEvent.ACTION_PROPOSED:
                p_type = PulseType.INTENT_SIGNAL

            # 3. Build the Pulse
            # We wrap the event payload into a standardized Pulse
            from cynic.interfaces.api.state import get_state
            org = get_state()
            
            pulse_data = {
                "event_topic": event.topic,
                "payload": event.dict_payload
            }
            
            # Enrich with global health if it's a heartbeat
            if p_type == PulseType.SOMATIC_SYNC and org:
                stats = org.state.get_stats()
                pulse_data["mind"] = {
                    "status": "AWAKE",
                    "thinking": stats.get("current_analysis", ""),
                    "confidence": stats.get("confidence", 0.618),
                    "e_score": stats.get("e_score", 50.0),
                    "axiom_scores": stats.get("axiom_scores", {})
                }
                # Add hardware if available
                if hasattr(org.metabolism, "body") and org.metabolism.body:
                    body_state = getattr(org.metabolism.body, "_last_state", None)
                    if body_state:
                        pulse_data["hardware"] = body_state.to_dict()

            pulse = PulseMessage(
                type=p_type,
                data=pulse_data
            )

            # 4. Broadcast to all connected nerves
            await self._server.broadcast(pulse)

        except Exception as e:
            logger.debug(f"κ-NET Handler error: {e}")
