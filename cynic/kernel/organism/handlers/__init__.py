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
from typing import TYPE_CHECKING, Any

from .introspect import (
    ArchitectureSnapshot,
    CouplingGrowth,
    HandlerAnalysis,
    HandlerArchitectureIntrospector,
)
from .services import (
    CognitionServices,
    MetabolicServices,
    SensoryServices,
)
from .validator import (
    HandlerValidator,
    ValidationIssue,
)

if TYPE_CHECKING:
    from cynic.kernel.core.event_bus import EventBus

    from .base import HandlerGroup

logger = logging.getLogger("cynic.kernel.organism.handlers")


class HandlerRegistry:
    """
    Handler lifecycle manager — register, wire, introspect.
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
                f"Wired handler group '{group.name}': " f"{len(group.subscriptions())} handlers"
            )

        self._wired = True
        logger.info(f"HandlerRegistry wired: {len(self._groups)} groups, {total_handlers} handlers")

    def introspect(self) -> dict:
        """Handler topology snapshot."""
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
    cognition: CognitionServices,
    metabolic: MetabolicServices,
    sensory: SensoryServices,
    **kwargs: Any,
) -> list[HandlerGroup]:
    """
    Auto-discover and instantiate handler groups by domain.
    """
    from .base import HandlerGroup

    groups: list[HandlerGroup] = []
    from cynic.kernel.organism import handlers as _pkg

    # Domain mapping logic
    domain_map = {
        "axiom": (cognition, "cognition"),
        "health": (cognition, "cognition"),
        "escore": (cognition, "cognition"),
        "intelligence": (cognition, "cognition"),
        "judgment_executor": (cognition, "cognition"),
        "meta_cognition": (cognition, "cognition"),
        "guidance_writer": (cognition, "cognition"),
        "consciousness_writer": (cognition, "cognition"),
        "direct": (metabolic, "metabolism"),
        "sdk": (metabolic, "metabolism"),
        "knet_handler": (sensory, "sensory"),
        "federation": (sensory, "sensory"),
    }

    for _, module_name, _ in pkgutil.iter_modules(_pkg.__path__):
        if module_name.startswith("_") or module_name == "base":
            continue

        try:
            mod = importlib.import_module(f"cynic.kernel.organism.handlers.{module_name}")
        except ImportError as e:
            logger.warning(f"Failed to import handler module {module_name}: {e}")
            continue

        for attr_name in dir(mod):
            attr = getattr(mod, attr_name)
            if (
                isinstance(attr, type)
                and issubclass(attr, HandlerGroup)
                and attr is not HandlerGroup
            ):
                try:
                    # Select appropriate service facade and parameter name
                    config = domain_map.get(module_name, (cognition, "cognition"))
                    svc_facade, param_name = config

                    # Merge domain service with extra specialized kwargs
                    group_kwargs = kwargs.get(module_name, {})
                    init_args = {param_name: svc_facade}
                    init_args.update(group_kwargs)

                    instance = attr(**init_args)
                    groups.append(instance)
                    logger.debug(f"Discovered handler group: {instance.name}")
                except Exception as e:
                    logger.warning(f"Failed to instantiate {attr_name} from {module_name}: {e}")

    logger.info(f"Discovered {len(groups)} handler groups")
    return groups
