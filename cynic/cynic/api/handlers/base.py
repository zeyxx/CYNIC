"""
Handler foundation — Living organs with explicit dependency declaration.

Every handler group declares:
- name: unique identifier
- subscriptions(): list of (event, callable) pairs
- dependencies(): frozenset of component names needed

This replaces hidden self.* access with explicit coupling contracts.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Callable

from cynic.core.event_bus import Event, CoreEvent
from cynic.core.escore import EScoreTracker
from cynic.cognition.cortex.axiom_monitor import AxiomMonitor
from cynic.cognition.cortex.lod import LODController


@dataclass
class KernelServices:
    """
    Shared kernel services — the organism's bloodstream.

    Every HandlerGroup receives this. If a handler needs a component
    not in KernelServices, it must be passed via its __init__()
    (visible coupling) or added here (PR-visible change).

    This enforces: all dependencies are EXPLICIT.
    """

    escore_tracker: EScoreTracker
    axiom_monitor: AxiomMonitor
    lod_controller: LODController
    health_cache: dict[str, float]

    async def signal_axiom(self, axiom: str, source: str, **extra) -> str:
        """
        Signal an axiom — emits AXIOM_ACTIVATED if it just became ACTIVE.

        Returns the new axiom state string (e.g. ``"ACTIVE"``, ``"RISING"``).
        Any ``extra`` kwargs are merged into the payload.

        Usage::

            new_state = await svc.signal_axiom(
                "ANTIFRAGILITY", "judgment_intelligence",
                trigger="RECOVERY",
            )
        """
        from cynic.core.event_bus import get_core_bus
        from cynic.core.events_schema import AxiomActivatedPayload

        new_state = self.axiom_monitor.signal(axiom)
        if new_state == "ACTIVE":
            await get_core_bus().emit(
                Event.typed(
                    CoreEvent.AXIOM_ACTIVATED,
                    AxiomActivatedPayload(
                        axiom=axiom,
                        maturity=self.axiom_monitor.get_maturity(axiom),
                        **extra,
                    ),
                    source=source,
                )
            )
        return new_state

    async def assess_lod(self) -> object:  # Returns SurvivalLOD
        """
        Assess LOD from health_cache — emits CONSCIOUSNESS_CHANGED on transition.
        """
        from cynic.core.event_bus import get_core_bus
        from cynic.core.events_schema import ConsciousnessChangedPayload

        prev = self.lod_controller.current
        result = self.lod_controller.assess(**self.health_cache)
        if result != prev:
            await get_core_bus().emit(
                Event.typed(
                    CoreEvent.CONSCIOUSNESS_CHANGED,
                    ConsciousnessChangedPayload(
                        from_lod=prev.value,
                        to_lod=result.value,
                        from_name=prev.name,
                        to_name=result.name,
                        direction="DOWN" if result > prev else "UP",
                    ),
                    source="lod_controller",
                )
            )
        return result


class HandlerGroup(ABC):
    """
    Base class for handler groups — like AbstractDog for event handlers.

    Every group declares:
    - name: unique identifier
    - subscriptions: which (event, callable) pairs it handles
    - dependencies: which components it needs (for introspection)

    This is the "driver interface" — enforcement point for architecture.
    """

    @property
    @abstractmethod
    def name(self) -> str:
        """Unique identifier for this handler group."""
        ...

    @abstractmethod
    def subscriptions(self) -> list[tuple[CoreEvent, Callable]]:
        """Return (event_type, handler_fn) pairs for bus registration.

        Example:
            return [
                (CoreEvent.JUDGMENT_CREATED, self._on_judgment_created),
                (CoreEvent.EMERGENCE_DETECTED, self._on_emergence),
            ]
        """
        ...

    def dependencies(self) -> frozenset[str]:
        """Return component names this group depends on (for introspection).

        Example:
            return frozenset({"escore_tracker", "axiom_monitor", "orchestrator"})

        Used by SelfProber to detect coupling growth.
        """
        return frozenset()
