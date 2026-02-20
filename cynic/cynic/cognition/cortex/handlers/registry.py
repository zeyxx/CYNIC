"""
HandlerRegistry — Discover and manage handler groups.

Follows the same pattern as LLMRegistry:
- Introspect current package to find all Handler subclasses
- Cache instances (singleton per handler class)
- Enable selection by handler_id
- Provide metadata for orchestration
"""
from __future__ import annotations

import importlib
import inspect
import logging
from typing import Any, Dict, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler

logger = logging.getLogger("cynic.cognition.cortex.handlers.registry")


class HandlerRegistry:
    """
    Central registry for all available handlers.

    Usage:
        registry = HandlerRegistry()
        handlers = registry.discover()  # Find all handlers
        executor = registry.get("act_handler")  # Get by ID
    """

    def __init__(self) -> None:
        self._handlers: Dict[str, BaseHandler] = {}  # handler_id → instance
        self._metadata: Dict[str, dict] = {}  # handler_id → metadata
        logger.info("HandlerRegistry initialized")

    def register(self, handler: BaseHandler) -> None:
        """Register a handler instance."""
        handler_id = handler.handler_id
        self._handlers[handler_id] = handler
        self._metadata[handler_id] = handler.metadata()
        logger.info(f"Handler registered: {handler_id} v{handler.version}")

    def get(self, handler_id: str) -> Optional[BaseHandler]:
        """Retrieve a handler by ID."""
        return self._handlers.get(handler_id)

    def all(self) -> Dict[str, BaseHandler]:
        """Return all registered handlers."""
        return dict(self._handlers)

    def metadata(self) -> Dict[str, dict]:
        """Return metadata for all handlers."""
        return dict(self._metadata)

    def count(self) -> int:
        """Return number of registered handlers."""
        return len(self._handlers)

    def stats(self) -> dict[str, Any]:
        """Return registry statistics."""
        return {
            "total_handlers": len(self._handlers),
            "handler_ids": list(self._handlers.keys()),
            "metadata": self._metadata,
        }


def discover_handlers() -> Dict[str, type[BaseHandler]]:
    """
    Auto-discover all Handler subclasses in this package.

    Returns:
        Dict mapping handler_id → Handler class
    """
    handlers: Dict[str, type[BaseHandler]] = {}

    # Get all modules in handlers package
    handler_modules = [
        "cynic.cognition.cortex.handlers.level_selector",
        "cynic.cognition.cortex.handlers.cycle_reflex",
        "cynic.cognition.cortex.handlers.cycle_micro",
        "cynic.cognition.cortex.handlers.cycle_macro",
        "cynic.cognition.cortex.handlers.act_executor",
        "cynic.cognition.cortex.handlers.evolve",
        "cynic.cognition.cortex.handlers.budget_manager",
    ]

    for module_name in handler_modules:
        try:
            module = importlib.import_module(module_name)
            # Find all BaseHandler subclasses in this module
            for name, obj in inspect.getmembers(module, inspect.isclass):
                if issubclass(obj, BaseHandler) and obj is not BaseHandler:
                    handlers[obj.handler_id] = obj
                    logger.debug(f"Discovered handler: {obj.handler_id} ({name})")
        except ImportError as e:
            logger.warning(f"Failed to import {module_name}: {e}")
        except Exception as e:
            logger.error(f"Error discovering handlers in {module_name}: {e}")

    logger.info(f"*sniff* Handler discovery: {len(handlers)} handlers found")
    return handlers
