"""
GuidanceWriter — Writes guidance.json from JUDGMENT_CREATED events.

Subscribes to JUDGMENT_CREATED events and persists guidance state.

NOTE: This handler is currently a stub. The actual guidance.json writing
is handled by the module-level _on_judgment_created() function in state.py,
which is subscribed directly to JUDGMENT_CREATED events.

When _write_guidance_async is implemented in core.py, this handler can be
activated by uncommenting the import and implementing the async write logic.

Design rationale:
- guidance.json is written synchronously in state.py for reliability
- This handler exists for future async/parallel guidance writing
- Keeping it as a stub allows the handler registry to discover it
"""
import logging
from typing import Callable
from cynic.core.event_bus import Event, CoreEvent
from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.services import KernelServices
from cynic.core.events_schema import JudgmentCreatedPayload
# Phase 1 fix: disabled - from cynic.api.routers.core import _write_guidance_async
from cynic.core.judgment import Cell, Judgment

logger = logging.getLogger(__name__)


class GuidanceWriter(HandlerGroup):
    """Handler group that writes guidance.json when judgments are created."""

    def __init__(self, svc: KernelServices):
        """Initialize handler with kernel services."""
        self.svc = svc
        logger.info("GuidanceWriter handler initialized (stub — guidance written by state.py)")

    @property
    def name(self) -> str:
        """Unique identifier for this handler group."""
        return "guidance_writer"

    def subscriptions(self) -> list[tuple[CoreEvent, Callable]]:
        """Return event subscriptions."""
        return [
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_created),
        ]

    def dependencies(self) -> frozenset[str]:
        """Return component dependencies (for introspection)."""
        return frozenset()

    async def _on_judgment_created(self, event: Event) -> None:
        """
        JUDGMENT_CREATED → write guidance.json atomically.

        Phase 1 fix: Disabled until _write_guidance_async is implemented.
        Guidance is still written by module-level _on_judgment_created in state.py.
        """
        # Phase 1 fix: guidance writing is handled by state.py:_on_judgment_created
        # This handler is disabled to avoid import errors
        logger.debug("GuidanceWriter: skipped (handled by state.py)")
