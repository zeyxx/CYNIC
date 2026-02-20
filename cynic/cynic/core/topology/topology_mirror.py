"""TopologyMirror — Real-time kernel architecture snapshot."""

from __future__ import annotations

import asyncio
import json
import logging
import time
from pathlib import Path
from typing import TYPE_CHECKING, Any

from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.phi import fibonacci
from .payloads import TopologySnapshotPayload

if TYPE_CHECKING:
    from cynic.judge.mirror import KernelMirror
    from cynic.api.state import AppState
    from cynic.core.event_bus import EventBus

logger = logging.getLogger("cynic.core.topology.topology_mirror")


class TopologyMirror:
    """
    Real-time kernel architecture snapshot.

    Responsibility: Keep organism's self-model current.
    Updates: Every F(7)=13 seconds OR on TOPOLOGY_APPLIED event.

    Writes to: ~/.cynic/topology.json (live, human-readable)
    Writes to: ~/.cynic/topology_history.jsonl (append-only log)
    """

    def __init__(self):
        self._snapshot_count = 0
        self._last_snapshot_time = 0.0

    async def continuous_snapshot(
        self,
        bus: EventBus,
        kernel_mirror: KernelMirror,
        state: AppState,
    ) -> None:
        """
        Continuously snapshot organism topology.

        1. Subscribe to TOPOLOGY_APPLIED (immediate snapshots)
        2. Periodic snapshots every 13s
        3. Write to ~/.cynic/topology.json
        4. Emit TOPOLOGY_SNAPSHOT events
        """
        # Subscribe to topology events for immediate updates
        bus.on(CoreEvent.TOPOLOGY_APPLIED, self._on_topology_applied)
        bus.on(CoreEvent.TOPOLOGY_ROLLBACK, self._on_topology_rollback)

        logger.info("TopologyMirror started: continuous snapshots enabled")

        # Periodic snapshot loop
        while True:
            await asyncio.sleep(fibonacci(7))  # 13 seconds

            await self._take_snapshot(bus, kernel_mirror, state)

    async def _on_topology_applied(self, event: Event) -> None:
        """Force immediate snapshot on topology change."""
        logger.debug("TOPOLOGY_APPLIED detected — immediate snapshot")
        self._last_snapshot_time = 0.0  # Force next snapshot immediately

    async def _on_topology_rollback(self, event: Event) -> None:
        """Force immediate snapshot on rollback."""
        logger.debug("TOPOLOGY_ROLLBACK detected — immediate snapshot")
        self._last_snapshot_time = 0.0

    async def _take_snapshot(
        self,
        bus: EventBus,
        kernel_mirror: KernelMirror,
        state: AppState,
    ) -> None:
        """
        Capture current kernel state and write to files.

        1. Get snapshot from KernelMirror
        2. Write to ~/.cynic/topology.json (current state)
        3. Append to ~/.cynic/topology_history.jsonl (history)
        4. Emit TOPOLOGY_SNAPSHOT event
        """
        try:
            snap = kernel_mirror.snapshot(state)
            self._snapshot_count += 1

            # 1. Write current state
            self._write_topology_file(snap)

            # 2. Append to history
            self._append_topology_history(snap)

            # 3. Emit event
            await bus.emit(Event.typed(
                CoreEvent.TOPOLOGY_SNAPSHOT,
                TopologySnapshotPayload(snapshot=snap),
                source="mirror:topology"
            ))

            self._last_snapshot_time = time.time()

        except Exception as e:
            logger.warning("Failed to snapshot topology: %s", e)

    def _write_topology_file(self, snap: dict[str, Any]) -> None:
        """
        Write current topology to ~/.cynic/topology.json.

        This is the "live" view — humans can inspect organism's current structure.
        """
        try:
            path = Path.home() / ".cynic" / "topology.json"
            path.parent.mkdir(parents=True, exist_ok=True)

            # Pretty-print for human readability
            path.write_text(json.dumps(snap, indent=2))
            logger.debug("Wrote topology snapshot to %s", path)

        except Exception as e:
            logger.warning("Failed to write topology.json: %s", e)

    def _append_topology_history(self, snap: dict[str, Any]) -> None:
        """
        Append snapshot to ~/.cynic/topology_history.jsonl.

        JSONL format allows append-only writes without parsing the whole file.
        Each line is a complete JSON object (snapshot at a point in time).
        """
        try:
            path = Path.home() / ".cynic" / "topology_history.jsonl"
            path.parent.mkdir(parents=True, exist_ok=True)

            # Append this snapshot as a single line
            with open(path, "a") as f:
                json.dump(snap, f)
                f.write("\n")

            logger.debug("Appended to topology_history.jsonl")

        except Exception as e:
            logger.warning("Failed to append topology history: %s", e)
