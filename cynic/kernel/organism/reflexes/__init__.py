"""
Reflex registry and discovery — the nervous system routing table.

Manages reflex lifecycle:
1. register() — add reflex groups
2. wire() — subscribe all to event bus
3. introspect() — reveal coupling and topology
"""

from __future__ import annotations

import importlib
import logging
import pkgutil
from typing import TYPE_CHECKING, Any, Optional

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

logger = logging.getLogger("cynic.kernel.organism.reflexes")


class HandlerRegistry:
    """
    Reflex lifecycle manager — register, wire, introspect.
    """

    def __init__(self) -> None:
        self._groups: list[HandlerGroup] = []
        self._wired = False

    def register(self, group: HandlerGroup) -> None:
        """Register a reflex group."""
        if self._wired:
            raise RuntimeError(f"Cannot register {group.name} after wire()")
        self._groups.append(group)
        logger.debug(f"Registered reflex group: {group.name}")

    def wire(self, bus: EventBus) -> None:
        """Subscribe all reflex groups to the event bus."""
        if self._wired:
            logger.warning("HandlerRegistry already wired — skipping")
            return

        total_handlers = 0
        for group in self._groups:
            for event_type, handler_fn in group.subscriptions():
                bus.on(event_type, handler_fn)
                total_handlers += 1
            logger.debug(
                f"Wired reflex group '{group.name}': " f"{len(group.subscriptions())} handlers"
            )

        self._wired = True
        logger.info(f"HandlerRegistry wired: {len(self._groups)} groups, {total_handlers} handlers")

    def introspect(self) -> dict:
        """Reflex topology snapshot."""
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
                    "events": [e.value if hasattr(e, 'value') else str(e) for e, _ in group.subscriptions()],
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
    bus: Optional[EventBus] = None,
    **kwargs: Any,
) -> list[HandlerGroup]:
    """
    Auto-discover and instantiate reflex groups by domain.
    """
    from .base import HandlerGroup

    groups: list[HandlerGroup] = []
    from cynic.kernel.organism import reflexes as _pkg

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
            mod = importlib.import_module(f"cynic.kernel.organism.reflexes.{module_name}")
        except ImportError as e:
            logger.warning(f"Failed to import reflex module {module_name}: {e}")
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
                    init_args = {param_name: svc_facade, "bus": bus}
                    init_args.update(group_kwargs)

                    instance = attr(**init_args)
                    groups.append(instance)
                    logger.debug(f"Discovered reflex group: {instance.name}")
                except Exception as e:
                    logger.warning(f"Failed to instantiate {attr_name} from {module_name}: {e}")

    logger.info(f"Discovered {len(groups)} reflex groups")
    return groups
