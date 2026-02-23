"""ChangeTracker — Real-time visibility into code modifications.

Captures and logs what changes in the organism's source code.
Writes to ~/.cynic/changes.jsonl for visibility.
"""

from __future__ import annotations

import asyncio
import json
import logging
import time
from pathlib import Path
from typing import Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.topology.payloads import SourceChangedPayload

logger = logging.getLogger("cynic.core.topology.change_tracker")


class ChangeTracker:
    """
    Track and log all code changes in real-time.

    Writes to ~/.cynic/changes.jsonl with:
      - File path
      - Category (handlers, dogs, judge, cli)
      - Timestamp
      - Previous mtime
      - Current mtime
      - Lines changed (estimate)
    """

    _CHANGES_PATH = Path.home() / ".cynic" / "changes.jsonl"
    _CHANGE_HISTORY_CAP = 233  # F(13) - rolling window

    def __init__(self):
        self._change_count = 0
        self._last_file_states: dict[str, float] = {}  # filepath → mtime

    async def on_source_changed(self, event: Event) -> None:
        """
        Log source change with details.

        Fired when SOURCE_CHANGED event detected.
        """
        try:
            payload = event.as_typed(SourceChangedPayload)
        except CynicError as e:
            logger.warning("Invalid SOURCE_CHANGED payload: %s", e)
            return

        # Log each changed file
        for filepath in payload.files:
            try:
                file_path = Path(filepath)
                mtime = file_path.stat().st_mtime if file_path.exists() else 0.0
                prev_mtime = self._last_file_states.get(filepath, 0.0)

                # Determine change type
                if prev_mtime == 0.0:
                    change_type = "ADDED"
                elif mtime == 0.0:
                    change_type = "DELETED"
                elif mtime > prev_mtime:
                    change_type = "MODIFIED"
                else:
                    change_type = "UNKNOWN"

                # Get file line count (rough estimate of scope)
                try:
                    if file_path.exists():
                        lines = len(file_path.read_text(encoding="utf-8", errors="ignore").splitlines())
                    else:
                        lines = 0
                except httpx.RequestError:
                    lines = 0

                # Build change record
                change_record = {
                    "timestamp": payload.timestamp,
                    "filepath": filepath,
                    "category": payload.category,
                    "change_type": change_type,
                    "previous_mtime": prev_mtime,
                    "current_mtime": mtime,
                    "file_lines": lines,
                }

                # Write to JSONL
                await self._append_change(change_record)

                # Update state
                self._last_file_states[filepath] = mtime
                self._change_count += 1

                logger.info(
                    "CHANGE TRACKED: %s %s (%d lines, cat=%s)",
                    change_type, filepath, lines, payload.category,
                )

            except asyncpg.Error as e:
                logger.warning("Failed to track change for %s: %s", filepath, e)

    async def _append_change(self, record: dict[str, Any]) -> None:
        """Append change record to changes.jsonl."""
        try:
            self._CHANGES_PATH.parent.mkdir(parents=True, exist_ok=True)

            # Append this record as a single line
            with open(self._CHANGES_PATH, "a", encoding="utf-8") as f:
                json.dump(record, f)
                f.write("\n")

            # Enforce rolling cap
            await self._enforce_rolling_cap()

        except OSError as e:
            logger.warning("Failed to append change: %s", e)

    async def _enforce_rolling_cap(self) -> None:
        """Keep changes.jsonl under rolling cap (F(13)=233)."""
        try:
            if not self._CHANGES_PATH.exists():
                return

            lines = self._CHANGES_PATH.read_text(encoding="utf-8", errors="ignore").splitlines()
            if len(lines) > self._CHANGE_HISTORY_CAP:
                # Keep only the most recent entries
                kept_lines = lines[-self._CHANGE_HISTORY_CAP :]
                self._CHANGES_PATH.write_text("\n".join(kept_lines) + "\n")
        except OSError as e:
            logger.warning("Failed to enforce rolling cap: %s", e)

    async def get_recent_changes(self, limit: int = 10) -> list[dict]:
        """Get recent changes from the log."""
        try:
            if not self._CHANGES_PATH.exists():
                return []

            lines = self._CHANGES_PATH.read_text(encoding="utf-8", errors="ignore").splitlines()
            changes = []
            for line in lines[-limit :]:
                if line.strip():
                    changes.append(json.loads(line))
            return changes
        except json.JSONDecodeError as e:
            logger.warning("Failed to read recent changes: %s", e)
            return []
