"""
Handler foundation — Living organs with explicit dependency declaration.

Every handler group declares:
- name: unique identifier
- subscriptions(): list of (event, callable) pairs
- dependencies(): frozenset of component names needed

This replaces hidden self.* access with explicit coupling contracts.

NOTE: KernelServices is now imported from services.py and should be used
as the primary entry point. For backward compatibility, it's also available here.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Callable

from cynic.core.event_bus import Event, CoreEvent

# Import the new service architecture
from cynic.api.handlers.services import (
    KernelServices,
    CognitionServices,
    MetabolicServices,
    SensoryServices,
)


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
