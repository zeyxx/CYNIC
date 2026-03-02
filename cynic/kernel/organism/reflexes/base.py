"""
Handler foundation â€” Living organs with explicit dependency declaration.

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
from collections.abc import Callable

from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import CoreEvent, EventBus

# Import the new service architecture


class HandlerGroup(ABC):
    """
    Base class for handler groups â€” like AbstractDog for event handlers.
    """

    def __init__(self, bus: Optional[EventBus] = None):
        """Initialize handler with instance-specific bus."""
        # Note: bus must be provided by factory.py
        if bus is None:
             raise RuntimeError(f"HandlerGroup {self.name} initialized without a bus. This violates isolation.")
        self.bus = bus

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
