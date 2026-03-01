"""
Meta-Cognition Handler — Organism's self-tuning loop.
Listens to SONA_TICK and adjusts internal parameters based on axiom health.
"""

from __future__ import annotations

import logging

from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.organism.handlers.base import HandlerGroup, KernelServices

logger = logging.getLogger("cynic.kernel.organism.handlers.meta_cognition")
class MetaCognitionHandlers(HandlerGroup):
    """Organism self-tuning and parameter adjustment."""

    def __init__(self, svc: KernelServices) -> None:
        self._svc = svc
        self._ticks_processed = 0

    @property
    def name(self) -> str:
        return "meta_cognition"

    def dependencies(self) -> frozenset[str]:
        return frozenset({
            "axiom_monitor",
            "lod_controller",
        })

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.SONA_TICK, self._on_sona_tick),
        ]

    async def _on_sona_tick(self, event: Event) -> None:
        """
        SONA_TICK (every 34s) → Analyze organism state and tune.
        """
        try:
            self._ticks_processed += 1
            p = event.dict_payload or {}
            axiom_health = p.get("axiom_health", 0.0)
            q_stability = p.get("q_stability", 0.0)

            # 1. Suppression du spam au démarrage (Rigueur Senior)
            if self._ticks_processed < 5:
                logger.debug("Meta-Cognition: Warm-up phase (tick %d)", self._ticks_processed)
                return

            # 2. Logique de réglage (en DEBUG pour ne pas polluer)
            if q_stability < 0.382: # Below PHI_INV_2
                logger.debug("Meta-Cognition: Q-Stability low (%.3f). Tuning exploration.", q_stability)

            if axiom_health < 0.618: # Below PHI_INV
                logger.debug("Meta-Cognition: Axiom health suboptimal (%.3f).", axiom_health)

        except Exception as e:
            logger.debug(f"MetaCognitionHandler error: {e}")

