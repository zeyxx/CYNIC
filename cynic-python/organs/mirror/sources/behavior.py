"""BehaviorSource — reads behavior_log.jsonl line by line.

Tier 2 INFRASTRUCTURE: behavior log ingestion for mirror agent.

K15 Consumer: MirrorCoordinator (event loop processes yielded Events)
Systemd: mirror-agent.service (daemon calls read_from on each cycle)
Promotion date: 2026-05-18

Consumable types: click, key, mouse_move, scroll, focus_change, idle_start, idle_end.
Skipped: health_checkpoint, entries without a "type" field.

Input contract: path points to a JSONL file (may grow between calls).
Output guarantee: yields only well-formed Events for consumable types.
Failure modes: malformed JSON lines are skipped, error_count incremented.
Valid domains: behavior_log.jsonl produced by the mirror sensor daemon.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Iterator

from organs.mirror.sources.base import Event

_CONSUMABLE = {"click", "key", "mouse_move", "scroll", "focus_change", "idle_start", "idle_end"}


class BehaviorSource:
    """Reads behavior_log.jsonl, emitting consumable Events from a given offset.

    Offset is measured in line count (number of lines already consumed).
    Each call to read_from() advances current_offset to the total line count
    of the file (including skipped lines), so the caller can persist the
    cursor and resume without re-reading.
    """

    def __init__(self, path: Path) -> None:
        self._path = path
        self._offset: int = 0
        self._errors: int = 0

    @property
    def name(self) -> str:
        return "behavior"

    @property
    def current_offset(self) -> int:
        return self._offset

    @property
    def error_count(self) -> int:
        return self._errors

    def read_from(self, offset: int) -> Iterator[Event]:
        """Yield Events from line `offset` onward.

        Skips the first `offset` lines (already consumed), then parses each
        subsequent line. Malformed JSON and non-consumable types are skipped.
        Updates current_offset to reflect the total lines read after the call.

        Input contract: offset >= 0, offset <= total line count.
        Output guarantee: yields Event objects only for consumable types.
        """
        line_index = 0
        with open(self._path) as fh:
            for line in fh:
                if line_index < offset:
                    line_index += 1
                    continue

                line_index += 1
                stripped = line.rstrip("\n")
                if not stripped:
                    continue

                try:
                    raw: dict = json.loads(stripped)
                except json.JSONDecodeError:
                    self._errors += 1
                    continue

                event_type = raw.get("type")
                if event_type not in _CONSUMABLE:
                    continue

                ts = raw.get("ts", "")
                yield Event(
                    source=self.name,
                    event_type=str(event_type),
                    timestamp=str(ts),
                    data={k: v for k, v in raw.items() if k != "type"},
                )

        self._offset = line_index
