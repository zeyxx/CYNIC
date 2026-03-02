"""
Meta-Cognition Handler â€” Organism's self-tuning loop.
Listens to SONA_TICK and adjusts internal parameters based on axiom health.
"""

from __future__ import annotations

import logging
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices

logger = logging.getLogger("cynic.kernel.organism.reflexes.meta_cognition")

class MetaCognitionHandler(HandlerGroup):
    """Organism self-tuning and parameter adjustment."""

    def __init__(self, cognition: CognitionServices, bus: Optional[EventBus] = None) -> None:
        super().__init__(bus=bus)
        self._cognition = cognition
        self._ticks_processed = 0

    @property
    def name(self) -> str:
        return "meta_cognition"

    def dependencies(self) -> frozenset[str]:
        return frozenset(
            {
                "axiom_monitor",
                "lod_controller",
            }
        )

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.SONA_TICK, self._on_sona_tick),
            (CoreEvent.CONFIGURATION_MUTATED, self._on_config_mutated),
        ]

    async def _on_config_mutated(self, event: Event) -> None:
        """Apply a configuration mutation to the live organism."""
        try:
            p = event.dict_payload
            target = p.get("target")
            parameter = p.get("parameter")
            new_value = p.get("new_value")

            if target == "DOG_REGISTRY":
                from cynic.kernel.organism.brain.cognition.neurons.registry import SOULS
                from cynic.kernel.organism.brain.cognition.neurons.base import DogId
                
                # Update the global registry
                for dog_id in DogId:
                    soul = SOULS.get(dog_id)
                    if soul and hasattr(soul, parameter):
                        setattr(soul, parameter, new_value)
                        logger.info(f"Meta-Cognition: Mutated {dog_id.name}.{parameter} -> {new_value}")
                
                # Note: MasterDog instances share the soul objects in our implementation
                # so the change is live.

        except Exception as e:
            logger.error(f"MetaCognition: Mutation failed: {e}")

    async def _on_sona_tick(self, event: Event) -> None:
        """
        SONA_TICK (every 34s) â’ Analyze organism state and tune.
        """
        try:
            self._ticks_processed += 1
            p = event.dict_payload or {}
            axiom_health = p.get("axiom_health", 0.0)
            q_stability = p.get("q_stability", 0.0)

            # 1. Suppression du spam au dÃ©marrage (Rigueur Senior)
            if self._ticks_processed < 5:
                logger.debug("Meta-Cognition: Warm-up phase (tick %d)", self._ticks_processed)
                return

            # 2. Logique de rÃ©glage (en DEBUG pour ne pas polluer)
            if q_stability < 0.382:  # Below PHI_INV_2
                logger.debug(
                    "Meta-Cognition: Q-Stability low (%.3f). Tuning exploration.", q_stability
                )

            if axiom_health < 0.618:  # Below PHI_INV
                logger.debug("Meta-Cognition: Axiom health suboptimal (%.3f).", axiom_health)

        except Exception as e:
            logger.debug(f"MetaCognitionHandler error: {e}")
