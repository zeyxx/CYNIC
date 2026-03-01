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
from collections.abc import Callable

from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.core.phi import MAX_CONFIDENCE
from cynic.kernel.organism.handlers.base import HandlerGroup
from cynic.kernel.organism.handlers.services import CognitionServices

logger = logging.getLogger("cynic.kernel.organism.handlers.guidance_writer")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


class GuidanceWriter(HandlerGroup):
    """Handler group that writes guidance.json when judgments are created."""

    def __init__(self, cognition: CognitionServices):
        """Initialize handler with cognition services."""
        self._cognition = cognition
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
            p = event.dict_payload or {}
            payload = {
                "timestamp": time.time(),
                "state_key": p.get("state_key", ""),
                "verdict": p.get("verdict", "WAG"),
                "q_score": round(float(p.get("q_score", 0.0)), 3),
                "confidence": round(min(float(p.get("confidence", 0.0)), MAX_CONFIDENCE), 4),
                "reality": p.get("reality", "CODE"),
                "dog_votes": {k: round(float(v), 3) for k, v in (p.get("dog_votes") or {}).items()},
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
