"""
Handler registry and discovery — the nervous system routing table.

Manages handler lifecycle:
1. register() — add handler groups
2. wire() — subscribe all to event bus
3. introspect() — reveal coupling and topology
"""

from __future__ import annotations

import importlib
import logging
import pkgutil
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cynic.core.event_bus import EventBus
    from cynic.api.handlers.base import HandlerGroup, KernelServices

logger = logging.getLogger("cynic.api.handlers")


class HandlerRegistry:
    """
    Handler lifecycle manager — register, wire, introspect.

    Lifecycle: register() → wire(bus) → introspect()
    Cannot register after wire() (like DogScheduler).
    """

    def __init__(self) -> None:
        self._groups: list[HandlerGroup] = []
        self._wired = False

    def register(self, group: HandlerGroup) -> None:
        """Register a handler group."""
        if self._wired:
            raise RuntimeError(f"Cannot register {group.name} after wire()")
        self._groups.append(group)
        logger.debug(f"Registered handler group: {group.name}")

    def wire(self, bus: EventBus) -> None:
        """Subscribe all handler groups to the event bus."""
        if self._wired:
            logger.warning("HandlerRegistry already wired — skipping")
            return

        total_handlers = 0
        for group in self._groups:
            for event_type, handler_fn in group.subscriptions():
                bus.on(event_type, handler_fn)
                total_handlers += 1
            logger.debug(
                f"Wired handler group '{group.name}': "
                f"{len(group.subscriptions())} handlers"
            )

        self._wired = True
        logger.info(f"HandlerRegistry wired: {len(self._groups)} groups, {total_handlers} handlers")

    def introspect(self) -> dict:
        """Handler topology — used by SelfProber + KernelMirror.

        Returns dict with:
        - groups: list of group metadata
        - total_handlers: sum of all handlers
        - total_deps: unique components across all groups
        """
        all_deps = set()
        groups_meta = []

        for group in self._groups:
            deps = group.dependencies()
            all_deps.update(deps)
            groups_meta.append(
                {
                    "name": group.name,
                    "handler_count": len(group.subscriptions()),
                    "dependencies": sorted(deps),
                    "events": [e.value for e, _ in group.subscriptions()],
                }
            )

        return {
            "groups": groups_meta,
            "total_handlers": sum(len(g.subscriptions()) for g in self._groups),
            "total_deps": len(all_deps),
        }


def discover_handler_groups(
    svc: KernelServices, **kwargs
) -> list[HandlerGroup]:
    """
    Auto-discover handler groups — like discover_dogs() but for handlers.

    Scans cynic.api.handlers package for HandlerGroup subclasses.
    Instantiates each with KernelServices + group-specific kwargs.

    Args:
        svc: KernelServices instance (shared bloodstream)
        **kwargs: group-specific constructor arguments (by module name)
            Example: {"intelligence": {"orchestrator": ..., "scheduler": ...}}

    Returns:
        List of instantiated HandlerGroup subclasses
    """
    from cynic.api.handlers.base import HandlerGroup

    groups: list[HandlerGroup] = []

    # Import all handler modules
    from cynic.api import handlers as _pkg

    for _, module_name, _ in pkgutil.iter_modules(_pkg.__path__):
        if module_name.startswith("_") or module_name == "base":
            continue

        try:
            mod = importlib.import_module(f"cynic.api.handlers.{module_name}")
        except ImportError as e:
            logger.warning(f"Failed to import handler module {module_name}: {e}")
            continue

        # Scan for HandlerGroup subclasses
        for attr_name in dir(mod):
            attr = getattr(mod, attr_name)
            if (
                isinstance(attr, type)
                and issubclass(attr, HandlerGroup)
                and attr is not HandlerGroup
            ):
                try:
                    # Instantiate with KernelServices + module-specific kwargs
                    group_kwargs = kwargs.get(module_name, {})
                    instance = attr(svc=svc, **group_kwargs)
                    groups.append(instance)
                    logger.debug(f"Discovered handler group: {instance.name}")
                except EventBusError as e:
                    logger.error(
                        f"Failed to instantiate {attr_name} from {module_name}: {e}"
                    )

    logger.info(f"Discovered {len(groups)} handler groups")
    return groups


__all__ = [
    "HandlerRegistry",
    "discover_handler_groups",
]
