"""
ConsciousnessWriter " Periodic persistence of OrganismState to consciousness.json.

This handler ensures the TUI and other external tools have a fresh view of 
the organism's internal metrics (cycles, health, axioms, reputation).

Listens to:
  - CoreEvent.JUDGMENT_CREATED (immediate update)
  - CoreEvent.SONA_TICK (periodic update every 34s)
"""

import json
import logging
import os
import time
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices

logger = logging.getLogger("cynic.kernel.organism.reflexes.consciousness_writer")

_CONSCIOUSNESS_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "consciousness.json")


class ConsciousnessWriter(HandlerGroup):
    """Handler group that writes consciousness.json periodically."""

    def __init__(self, cognition: CognitionServices, bus: Optional[EventBus] = None):
        """Initialize handler with cognition services."""
        super().__init__(bus=bus)
        self._cognition = cognition
        self._last_write = 0.0
        self._write_interval = 13.0  # F(7) seconds throttle
        logger.info("ConsciousnessWriter handler active")

    @property
    def name(self) -> str:
        """Unique identifier for this handler group."""
        return "consciousness_writer"

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        """Return event subscriptions."""
        return [
            (CoreEvent.JUDGMENT_CREATED, self._on_update_signal),
            (CoreEvent.SONA_TICK, self._on_update_signal),
        ]

    def dependencies(self) -> frozenset[str]:
        """Return component dependencies."""
        return frozenset({"escore_tracker", "axiom_monitor", "lod_controller"})

    async def _on_update_signal(self, event: Event) -> None:
        """Throttle updates to avoid disk thrashing."""
        now = time.time()
        if now - self._last_write < self._write_interval:
            return

        await self._write_consciousness()
        self._last_write = now

    async def _write_consciousness(self) -> None:
        """Capture live state and write to consciousness.json atomically."""
        try:
            # We need access to the OrganismState. 
            # In our handler architecture, CognitionServices usually has what we need.
            
            # Since we don't have a direct reference to OrganismState here 
            # (unless we add it to CognitionServices), we'll use a live snapshot 
            # from the services we DO have.
            
            # Note: A better way would be to have OrganismState write its own 
            # consciousness.json, but handlers are our primary way to react to events.
            
            payload = {
                "timestamp": time.time(),
                "axioms": {
                    "active": self._cognition.axiom_monitor.active_axioms(),
                    "dashboard": self._cognition.axiom_monitor.dashboard(),
                },
                "escore": self._cognition.escore_tracker.stats(),
                "lod": self._cognition.lod_controller.status() if hasattr(self._cognition.lod_controller, "status") else {},
                "health": self._cognition.health_cache,
            }

            # If we can find the state manager (it's often injected into trackers)
            if hasattr(self._cognition.escore_tracker, "state_manager") and self._cognition.escore_tracker.state_manager:
                state = self._cognition.escore_tracker.state_manager
                payload["cycles"] = {
                    "reflex": state.reflex_cycles,
                    "micro": state.micro_cycles,
                    "macro": state.macro_cycles,
                    "meta": state.meta_cycles,
                    "total": state.total_cycles,
                }
                payload["total_judgments"] = state.total_judgments

            os.makedirs(os.path.dirname(_CONSCIOUSNESS_PATH), exist_ok=True)

            # Atomic write via temporary file
            temp_path = _CONSCIOUSNESS_PATH + ".tmp"
            with open(temp_path, "w", encoding="utf-8") as fh:
                json.dump(payload, fh, indent=2)
            os.replace(temp_path, _CONSCIOUSNESS_PATH)

            logger.debug("ConsciousnessWriter: consciousness.json updated")

        except Exception as exc:
            logger.warning("ConsciousnessWriter: failed to write consciousness.json: %s", exc)
