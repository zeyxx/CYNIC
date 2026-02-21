"""
BaseHandler — Interface for all orchestrator handlers.

Defines the contract that all handlers must fulfill:
- Async execution with result type
- Observable (logs, emits events)
- Dependency injection (no god constructor)
- Standardized metadata (handler_id, version, description)
"""
from __future__ import annotations

import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Optional

logger = logging.getLogger("cynic.cognition.cortex.handlers")


@dataclass
class HandlerResult:
    """Standardized result from any handler execution."""

    success: bool
    handler_id: str
    output: Any = None
    error: Optional[str] = None
    duration_ms: float = 0.0
    metadata: dict[str, Any] = field(default_factory=dict)

    def __repr__(self) -> str:
        status = "✓" if self.success else "✗"
        return f"{status} {self.handler_id}: {self.output or self.error} ({self.duration_ms:.0f}ms)"


class BaseHandler(ABC):
    """
    Abstract base class for all CYNIC orchestrator handlers.

    Handlers decompose the JudgeOrchestrator god class into single-responsibility units.
    Each handler owns one phase or decision point in the judgment cycle.

    Subclasses must:
    1. Set handler_id (unique string identifier)
    2. Implement execute() method
    3. Accept all dependencies as constructor parameters (no self.* lookup)
    """

    handler_id: str = "base"
    version: str = "1.0"
    description: str = "Base handler (abstract)"

    @abstractmethod
    async def execute(self, **kwargs: Any) -> HandlerResult:
        """
        Execute the handler's primary logic.

        Args:
            **kwargs: Handler-specific parameters

        Returns:
            HandlerResult with success/error/output
        """
        raise NotImplementedError

    def _log_execution(self, action: str, details: str = "") -> None:
        """Log handler execution for observability."""
        msg = f"[{self.handler_id}] {action}"
        if details:
            msg += f" — {details}"
        logger.info(msg)

    def _log_error(self, action: str, error: Exception) -> None:
        """Log handler error for observability."""
        logger.error(f"[{self.handler_id}] {action} failed: {error}")

    def metadata(self) -> dict[str, Any]:
        """Return handler metadata for introspection."""
        return {
            "handler_id": self.handler_id,
            "version": self.version,
            "description": self.description,
        }
