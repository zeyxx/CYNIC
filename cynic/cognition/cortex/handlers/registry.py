"""
HandlerRegistry — Centralized consciousness handler instance storage.

Part of Phase 2B: Orchestration Refactoring — explicit handler composition DAG.

Responsibility:
- Register handler instances by ID
- Retrieve handlers by ID
- Enumerate all handlers (for discovery, introspection)
- Provide summary metadata (versions, descriptions) for all handlers
"""
from __future__ import annotations

import logging
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler

logger = logging.getLogger("cynic.cognition.cortex.handlers.registry")


class HandlerRegistry:
    """
    Centralized registry of consciousness handlers.

    Used by HandlerComposer to dispatch handlers in execution DAG.
    Handlers registered here are injectable (no god functions, no hidden state lookups).

    Thread-safe: All operations atomic (dict ops in CPython GIL-protected).
    """

    def __init__(self) -> None:
        """Initialize empty registry."""
        self.handlers: dict[str, BaseHandler] = {}

    def register(self, handler_id: str, handler: BaseHandler) -> None:
        """
        Register a handler instance by ID.

        Args:
            handler_id: Unique handler identifier (e.g., "cycle_reflex", "level_selector")
            handler: BaseHandler instance

        Raises:
            ValueError: If handler_id already registered
        """
        if handler_id in self.handlers:
            raise ValueError(
                f"Handler '{handler_id}' already registered. "
                "Use unregister() first if replacement intended."
            )
        self.handlers[handler_id] = handler
        logger.debug(f"Registered handler '{handler_id}' ({handler.handler_id})")

    def get(self, handler_id: str) -> BaseHandler:
        """
        Retrieve handler by ID.

        Args:
            handler_id: Unique handler identifier

        Returns:
            BaseHandler instance

        Raises:
            KeyError: If handler_id not found
        """
        if handler_id not in self.handlers:
            available = ", ".join(self.list_ids())
            raise KeyError(
                f"Handler '{handler_id}' not found. Available: {available}"
            )
        return self.handlers[handler_id]

    def all(self) -> dict[str, BaseHandler]:
        """
        Get copy of all registered handlers.

        Returns:
            dict mapping handler_id → BaseHandler instance
        """
        return self.handlers.copy()

    def list_ids(self) -> list[str]:
        """
        Get list of all registered handler IDs in registration order.

        Returns:
            list of handler IDs
        """
        return list(self.handlers.keys())

    def has(self, handler_id: str) -> bool:
        """
        Check if handler is registered.

        Args:
            handler_id: Unique handler identifier

        Returns:
            True if registered, False otherwise
        """
        return handler_id in self.handlers

    def count(self) -> int:
        """
        Get number of registered handlers.

        Returns:
            Integer count
        """
        return len(self.handlers)

    def unregister(self, handler_id: str) -> None:
        """
        Unregister a handler (for testing, reconfiguration).

        Args:
            handler_id: Unique handler identifier

        Raises:
            KeyError: If handler_id not found
        """
        if handler_id not in self.handlers:
            raise KeyError(f"Handler '{handler_id}' not found")
        del self.handlers[handler_id]
        logger.debug(f"Unregistered handler '{handler_id}'")

    def summary(self) -> dict[str, dict[str, Any]]:
        """
        Get summary metadata for all registered handlers.

        Returns:
            dict mapping handler_id → {version, description, handler_class}
        """
        result = {}
        for handler_id, handler in self.handlers.items():
            result[handler_id] = {
                "version": handler.version,
                "description": handler.description,
                "handler_class": handler.__class__.__name__,
                "handler_id": handler.handler_id,
            }
        return result
