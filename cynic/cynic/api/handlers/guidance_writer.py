"""
Phase 0: Guidance Writer Handler
When JUDGMENT_CREATED is emitted, writes guidance.json atomically.
Subscribes to JUDGMENT_CREATED events and persists guidance state.
"""
import logging
from typing import Callable
from cynic.core.event_bus import Event, CoreEvent
from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.services import KernelServices
from cynic.core.events import JudgmentCreatedPayload
from cynic.api.routers.core import _write_guidance_async
from cynic.core.judgment import Cell, Judgment

logger = logging.getLogger(__name__)


class GuidanceWriter(HandlerGroup):
    """Handler group that writes guidance.json when judgments are created."""

    def __init__(self, svc: KernelServices):
        """Initialize handler with kernel services."""
        self.svc = svc
        logger.info("GuidanceWriter handler initialized")

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

        Receives enriched judgment payload with cell context and writes to disk.
        This is the persistence layer for Phase 0: ensures guidance state survives crashes.
        """
        try:
            payload = event.payload
            if isinstance(payload, JudgmentCreatedPayload):
                # Reconstruct cell from payload
                cell = Cell(
                    reality=payload.get("reality", "CODE"),
                    analysis=payload.get("analysis", "JUDGE"),
                    time_dim=payload.get("time_dim", "PRESENT"),
                    content=payload.get("content_preview", ""),
                    context=payload.get("context", ""),
                    lod=payload.get("lod", 0),
                    budget_usd=payload.get("budget_usd", 0.01),
                )

                # Reconstruct judgment object for guidance write
                judgment = Judgment(
                    cell=cell,
                    verdict=payload.get("verdict", "PENDING"),
                    q_score=payload.get("q_score", 0.0),
                    confidence=payload.get("confidence", 0.0),
                    dog_votes=payload.get("dog_votes", {}),
                    cost_usd=payload.get("cost_usd", 0.0),
                )
                judgment.judgment_id = payload.get("judgment_id")

                # Write guidance.json atomically
                await _write_guidance_async(cell, judgment)
                logger.debug(
                    "Guidance written for judgment %s", judgment.judgment_id
                )

        except Exception as e:
            logger.warning("Failed to write guidance for judgment: %s", e)
            # Don't fail the system — guidance write is best-effort durability
