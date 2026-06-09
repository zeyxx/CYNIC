"""
Tier 2 INFRASTRUCTURE: X.com tweet signal source from dataset.jsonl.

Reads JSONL tweet captures and emits Events for the mirror organ.
K15 Consumer: BehavioralProfile learner (via coordinator event loop)
Systemd: mirror-coordinator.service
Promotion date: 2026-05-18
Stability: initial

Event type classification:
- viewer_bookmarked == True  → "tweet_bookmarked"
- else                       → "tweet_seen"

Rows missing tweet_id are skipped (malformed) and counted in error_count.
"""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, Iterator

from organs.mirror.sources.base import Event

logger = logging.getLogger(__name__)

__version__ = "1.0.0"


class XSignalsSource:
    """Reads dataset.jsonl and yields tweet Events."""

    def __init__(self, path: Path) -> None:
        self._path = path
        self._current_offset: int = 0
        self._error_count: int = 0

    @property
    def name(self) -> str:
        return "x_signals"

    @property
    def current_offset(self) -> int:
        return self._current_offset

    @property
    def error_count(self) -> int:
        return self._error_count

    def read_from(self, offset: int) -> Iterator[Event]:
        """Yield Events starting from byte offset, skipping malformed rows."""
        self._current_offset = offset
        if not self._path.exists():
            logger.warning("dataset.jsonl not found: %s", self._path)
            return

        with self._path.open("r", encoding="utf-8") as fh:
            fh.seek(offset)
            while True:
                raw_line = fh.readline()
                if not raw_line:
                    break
                self._current_offset = fh.tell()
                line = raw_line.strip()
                if not line:
                    continue

                row: dict[str, Any] | None = None
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    self._error_count += 1
                    logger.warning("Skipping malformed JSON line")
                    continue

                if not isinstance(row, dict) or not row.get("tweet_id"):
                    self._error_count += 1
                    logger.warning("Skipping row missing tweet_id")
                    continue

                event_type = (
                    "tweet_bookmarked"
                    if row.get("viewer_bookmarked") is True
                    else "tweet_seen"
                )
                timestamp = str(row.get("capture_ts", ""))

                yield Event(
                    source=self.name,
                    event_type=event_type,
                    timestamp=timestamp,
                    data=row,
                )
