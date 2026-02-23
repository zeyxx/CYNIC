"""
CYNIC Dependency Container — Lightweight typed registry.

NOT a full IoC framework. A simple registry that replaces manual wiring.
Components register by their interface type, retrieved by type.

Usage:
    container = DependencyContainer(config)
    container.register(QTable, qtable_instance)
    qtable = container.get(QTable)

Factory support:
    container.register_factory(QTable, lambda c: QTable())
    qtable = container.get(QTable)  # factory called lazily

φ-Law: BURN — simplest possible DI, not Spring.
"""
from __future__ import annotations

import logging
from collections.abc import Callable
from typing import Any, TypeVar, overload

from cynic.core.config import CynicConfig

logger = logging.getLogger("cynic.container")

T = TypeVar("T")


class DependencyContainer:
    """
    Lightweight DI container for CYNIC kernel components.

    Type-safe get() returns the exact registered type.
    Circular dependency detection prevents infinite loops.
    """

    def __init__(self, config: Optional[CynicConfig] = None) -> None:
        self.config = config or CynicConfig()
        self._instances: dict[type, Any] = {}
        self._factories: dict[type, Callable[[DependencyContainer], Any]] = {}
        self._resolving: set[type] = set()  # circular dep detection

    def register(self, interface: type[T], instance: T) -> None:
        """Register a concrete instance for an interface type."""
        self._instances[interface] = instance

    def register_factory(
        self,
        interface: type[T],
        factory: Callable[[DependencyContainer], T],
    ) -> None:
        """Register a lazy factory for an interface type."""
        self._factories[interface] = factory

    def get(self, interface: type[T]) -> T:
        """
        Get a component by its interface type.

        Resolution order:
          1. Already-created instance
          2. Factory (lazy — created on first get())
          3. KeyError

        Raises KeyError if not registered.
        Raises RuntimeError on circular dependency.
        """
        # 1. Already created
        if interface in self._instances:
            return self._instances[interface]

        # 2. Factory (lazy creation)
        if interface in self._factories:
            if interface in self._resolving:
                chain = " → ".join(t.__name__ for t in self._resolving)
                raise RuntimeError(
                    f"Circular dependency detected: {chain} → {interface.__name__}"
                )
            self._resolving.add(interface)
            try:
                instance = self._factories[interface](self)
                self._instances[interface] = instance
                return instance
            finally:
                self._resolving.discard(interface)

        raise KeyError(
            f"No component registered for {interface.__name__}. "
            f"Registered: {[t.__name__ for t in self._instances]}"
        )

    def has(self, interface: type) -> bool:
        """Check if a component is registered (instance or factory)."""
        return interface in self._instances or interface in self._factories

    @property
    def registered_types(self) -> list[str]:
        """List all registered type names (for debugging)."""
        all_types = set(self._instances.keys()) | set(self._factories.keys())
        return sorted(t.__name__ for t in all_types)

    def stats(self) -> dict[str, Any]:
        """Container health snapshot."""
        return {
            "instances": len(self._instances),
            "factories": len(self._factories),
            "types": self.registered_types,
        }
