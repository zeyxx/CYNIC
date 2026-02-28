"""
GuidanceWriter — Writes guidance.json from JUDGMENT_CREATED events.

This handler ensures the feedback loop is maintained by persisting the
latest judgment to a local JSON file. This file is read by external hooks
and the TUI to provide context for future judgments.
"""
import json
import logging
import os
import time
from typing import Callable, Any
from cynic.core.event_bus import Event, CoreEvent
from cynic.organism.handlers.base import HandlerGroup
from cynic.organism.handlers.services import KernelServices
from cynic.core.phi import MAX_CONFIDENCE

logger = logging.getLogger("cynic.organism.handlers.guidance_writer")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


class GuidanceWriter(HandlerGroup):
    """Handler group that writes guidance.json when judgments are created."""

    def __init__(self, svc: KernelServices):
        """Initialize handler with kernel services."""
        self.svc = svc
        logger.info("GuidanceWriter handler active")

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
        """Return component dependencies."""
        return frozenset()

    async def _on_judgment_created(self, event: Event) -> None:
        """
        JUDGMENT_CREATED → write guidance.json atomically.
        """
        try:
            p = event.payload or {}
            payload = {
                "timestamp": time.time(),
                "state_key": p.get("state_key", ""),
                "verdict": p.get("verdict", "WAG"),
                "q_score": round(float(p.get("q_score", 0.0)), 3),
                "confidence": round(min(float(p.get("confidence", 0.0)), MAX_CONFIDENCE), 4),
                "reality": p.get("reality", "CODE"),
                "dog_votes": {
                    k: round(float(v), 3)
                    for k, v in (p.get("dog_votes") or {}).items()
                },
            }
            
            os.makedirs(os.path.dirname(_GUIDANCE_PATH), exist_ok=True)
            
            # Atomic write via temporary file
            temp_path = _GUIDANCE_PATH + ".tmp"
            with open(temp_path, "w", encoding="utf-8") as fh:
                json.dump(payload, fh, indent=2)
            os.replace(temp_path, _GUIDANCE_PATH)
            
            logger.debug("GuidanceWriter: guidance.json updated")
            
        except Exception as exc:
            logger.warning("GuidanceWriter: failed to write guidance.json: %s", exc)
