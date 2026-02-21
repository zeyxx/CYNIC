"""SourceWatcher — Real-time file system monitoring for topology changes."""

from __future__ import annotations

import asyncio
import logging
import time
from pathlib import Path
from typing import Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.phi import fibonacci
from cynic.core.topology.payloads import SourceChangedPayload

logger = logging.getLogger("cynic.core.topology.file_watcher")

# Compute package root from this file's location
_CYNIC_ROOT = Path(__file__).resolve().parent.parent.parent  # → /app/cynic


class SourceWatcher:
    """
    Monitor source files for topology changes.

    Responsibility: Detect what changed, not how to fix it.
    Emits: SOURCE_CHANGED events with file path + timestamp.

    Watched directories:
      - cynic/api/handlers/ → category="handlers"
      - cynic/dogs/ → category="dogs"
      - cynic/judge/ → category="judge"
      - cynic/cli/ → category="cli"

    Polling: Every F(7)=13 seconds (efficient, not continuous)
    """

    # Directories to watch → category name (absolute paths from package root)
    _WATCHED_DIRS = {
        "handlers": _CYNIC_ROOT / "api" / "handlers",
        "dogs": _CYNIC_ROOT / "dogs",
        "judge": _CYNIC_ROOT / "judge",
        "cli": _CYNIC_ROOT / "cli",
    }

    def __init__(self):
        self._previous_state: dict[str, dict[str, float]] = {}
        self._snapshot_failures: dict[str, int] = {}  # category → failure count
        self._last_successful_snapshot: dict[str, float] = {}  # category → timestamp

    async def watch(self) -> None:
        """
        Continuously monitor source tree for changes.

        Polls every 13s, emits SOURCE_CHANGED events on delta.
        """
        bus = get_core_bus()

        # Initial snapshot
        self._previous_state = self._snapshot_tree()
        logger.info("SourceWatcher initialized: monitoring %s directories", len(self._WATCHED_DIRS))

        while True:
            await asyncio.sleep(fibonacci(7))  # 13 seconds

            current_state = self._snapshot_tree()
            changes = self._diff_state(self._previous_state, current_state)

            if changes:
                # Emit: SOURCE_CHANGED event
                for category, files in changes.items():
                    payload = SourceChangedPayload(
                        category=category,
                        files=files,
                        timestamp=time.time(),
                    )
                    await bus.emit(Event.typed(
                        CoreEvent.SOURCE_CHANGED,
                        payload,
                        source="watcher:filesystem"
                    ))
                    logger.info(
                        "SOURCE_CHANGED: %s (%d files)",
                        category, len(files)
                    )

            self._previous_state = current_state

    def _snapshot_tree(self) -> dict[str, dict[str, float]]:
        """
        Return {category: {filepath: mtime}}.

        Captures modification times of all .py files in watched dirs.
        Returns empty dict for category if dir doesn't exist.
        """
        result = {}
        for category, dir_path in self._WATCHED_DIRS.items():
            result[category] = {}

            if not dir_path.exists():
                continue

            try:
                for py_file in dir_path.glob("*.py"):
                    if py_file.is_file():
                        # Store relative path + mtime (relative to CYNIC root, not cwd())
                        # This is portable: works both locally and in Docker
                        try:
                            rel_path = str(py_file.relative_to(_CYNIC_ROOT))
                        except ValueError:
                            # If file is outside CYNIC_ROOT, use absolute path
                            rel_path = str(py_file.absolute())
                        result[category][rel_path] = py_file.stat().st_mtime
                # Reset failure counter on success
                if category in self._snapshot_failures:
                    if self._snapshot_failures[category] > 0:
                        logger.info("*tail wag* Snapshot %s recovered after %d failures",
                                  category, self._snapshot_failures[category])
                        self._snapshot_failures[category] = 0
                self._last_successful_snapshot[category] = time.time()
            except Exception as e:
                # Track failure + emit alert
                self._snapshot_failures[category] = self._snapshot_failures.get(category, 0) + 1
                logger.error(
                    "*GROWL* Snapshot FAILED %s (attempt %d): %s",
                    category, self._snapshot_failures[category], e
                )
                # Emit critical alert if too many failures
                if self._snapshot_failures[category] >= 3:
                    logger.critical(
                        "CRITICAL: Snapshot %s failed %d times — topology may be stale!",
                        category, self._snapshot_failures[category]
                    )

        return result

    def _diff_state(
        self,
        prev: dict[str, dict[str, float]],
        curr: dict[str, dict[str, float]],
    ) -> dict[str, list[str]]:
        """
        Compute what changed between two snapshots.

        Returns: {category: [changed_files]}

        Changed = added, removed, or modified (mtime different)
        """
        changes: dict[str, list[str]] = {}

        # Check all categories
        all_categories = set(prev.keys()) | set(curr.keys())

        for category in all_categories:
            prev_files = prev.get(category, {})
            curr_files = curr.get(category, {})

            changed_files = []

            # Added or modified files
            for filepath, mtime in curr_files.items():
                if filepath not in prev_files or prev_files[filepath] != mtime:
                    changed_files.append(filepath)

            # Deleted files
            for filepath in prev_files:
                if filepath not in curr_files:
                    changed_files.append(filepath)

            if changed_files:
                changes[category] = changed_files

        return changes

    def get_health(self) -> dict[str, Any]:
        """
        Return health status of the watcher.

        Used for diagnostics and monitoring integration.
        """
        return {
            "status": "healthy" if all(f == 0 for f in self._snapshot_failures.values()) else "degraded",
            "snapshot_failures": dict(self._snapshot_failures),
            "last_successful_snapshot": dict(self._last_successful_snapshot),
            "categories_monitored": list(self._WATCHED_DIRS.keys()),
        }
