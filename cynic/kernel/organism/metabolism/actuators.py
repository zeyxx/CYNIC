"""
CYNIC Motor System â€” Actuators for physical impact.

Defines the interface for all components that can change the
external or internal world (Files, API calls, CLI commands).

Ï†-Law: BURN â€” Every action has a cost and must be justified by judgment.
"""

from __future__ import annotations

import logging
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

logger = logging.getLogger("cynic.metabolism.actuators")


@dataclass
class ActResult:
    """The raw truth of an action outcome."""

    success: bool
    output: Any
    duration_ms: float
    error: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)


class AbstractActuator(ABC):
    """Base class for all physical effectors."""

    @abstractmethod
    async def execute(self, payload: dict[str, Any]) -> ActResult:
        pass


class UniversalActuator:
    """
    The 'Hands' of the organism.
    Dispatches action requests to specialized actuators.
    """

    def __init__(self):
        self._actuators: dict[str, AbstractActuator] = {}

    def register(self, action_type: str, actuator: AbstractActuator):
        self._actuators[action_type] = actuator
        logger.info(f"Actuator: Registered handler for '{action_type}'")

    async def dispatch(self, action_type: str, payload: dict[str, Any]) -> ActResult:
        """Execute an action and return the reality check."""
        start = time.perf_counter()
        actuator = self._actuators.get(action_type)

        if not actuator:
            return ActResult(
                success=False,
                output=None,
                duration_ms=(time.perf_counter() - start) * 1000,
                error=f"No actuator registered for type: {action_type}",
            )

        try:
            result = await actuator.execute(payload)
            return result
        except Exception as e:
            return ActResult(
                success=False,
                output=None,
                duration_ms=(time.perf_counter() - start) * 1000,
                error=str(e),
            )


# --- CONCRETE ACTUATORS ---


class FileActuator(AbstractActuator):
    """Allows CYNIC to write to its own file system."""

    async def execute(self, payload: dict[str, Any]) -> ActResult:
        start = time.perf_counter()
        path = payload.get("path")
        content = payload.get("content")

        if not path or content is None:
            return ActResult(False, None, 0.0, "Missing path or content")

        try:
            from pathlib import Path

            file_path = Path(path)
            file_path.parent.mkdir(parents=True, exist_ok=True)
            file_path.write_text(content, encoding="utf-8")

            return ActResult(
                success=True,
                output=f"Written {len(content)} chars to {path}",
                duration_ms=(time.perf_counter() - start) * 1000,
            )
        except Exception as e:
            return ActResult(False, None, 0.0, str(e))
