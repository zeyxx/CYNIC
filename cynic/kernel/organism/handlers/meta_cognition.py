"""
Meta-Cognition Handler — Organism's self-tuning loop.
Listens to SONA_TICK and adjusts internal parameters based on axiom health.
"""

from __future__ import annotations

import logging
from typing import Any

from cynic.kernel.core.event_bus import Event, CoreEvent
from cynic.kernel.organism.handlers.base import HandlerGroup, KernelServices

logger = logging.getLogger("cynic.kernel.organism.handlers.meta_cognition")

class MetaCognitionHandlers(HandlerGroup):
    """Organism self-tuning and parameter adjustment."""

    def __init__(self, svc: KernelServices) -> None:
        self._svc = svc

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
            p = event.dict_payload or {}
            axiom_health = p.get("axiom_health", 0.0)
            q_stability = p.get("q_stability", 0.0)
            
            # Logic for dynamic tuning:
            # 1. If stability is low, increase exploration or reduce learning rate
            # 2. If axiom health is low, signal for LOD reduction to conserve energy
            
            if q_stability < 0.382: # Below PHI_INV_2
                logger.warning(f"Meta-Cognition: Q-Stability low ({q_stability:.3f}). Organism is in flux.")
                # Future: Adjust Q-Learning epsilon here
            
            if axiom_health < 0.618: # Below PHI_INV
                logger.info(f"Meta-Cognition: Axiom health suboptimal ({axiom_health:.3f}). Tuning focus.")
            
            logger.debug(
                "SONA_TICK: Axiom Health=%.3f, Q-Stability=%.3f. Meta-cognition pulse processed.",
                axiom_health, q_stability
            )
            
        except Exception as e:
            logger.debug(f"MetaCognitionHandler error: {e}", exc_info=True)
