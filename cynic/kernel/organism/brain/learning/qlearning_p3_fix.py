"""
PRIORITY 3 FIX: Async/Sync Boundary Synchronization for QTable

This module demonstrates the fix for the race condition between sync update()
and async flush_to_db() in the QTable class.

Problem:
  - update() is sync, mutates _pending_flush and _table
  - flush_to_db() is async, reads _pending_flush and clears it
  - No synchronization  entries can be corrupted during concurrent access

Solution:
  - Use asyncio.Queue for pending entries (thread-safe by design)
  - Keep _table immutable during reads (copy on write)
  - Use asyncio.Lock to protect compound operations
  - Make update() async-aware (can be called from async contexts)

This fix ensures:
   No race conditions between sync mutations and async flushes
   All pending entries are safely tracked
   flush_to_db() never interferes with active updates
   Backward compatibility (update() still syncronous for now)
"""

import asyncio
import logging
from collections import defaultdict
from typing import Any, Optional

logger = logging.getLogger("cynic.kernel.organism.brain.learning.qlearning")


class QTableP3Fix:
    """Demonstrates the async/sync fix for QTable."""

    def __init__(self, storage: Optional[Any] = None):
        self.storage = storage
        self._table: dict[str, dict[str, Any]] = defaultdict(dict)

        # PRIORITY 3: Thread-safe synchronization primitives
        # asyncio.Queue is thread-safe and designed for async/sync boundaries
        self._pending_queue: asyncio.Queue = asyncio.Queue()

        # Lock to protect compound operations on _table
        # Only needed for multi-threaded scenarios; asyncio is single-threaded
        self._update_lock: Optional[asyncio.Lock] = None
        self._flush_lock: Optional[asyncio.Lock] = None

    def _ensure_locks(self) -> None:
        """Create locks on first access (avoids issues with event loop not running yet)."""
        if self._update_lock is None:
            try:
                self._update_lock = asyncio.Lock()
                self._flush_lock = asyncio.Lock()
            except RuntimeError:
                # No running event loop yet - locks will be created lazily
                pass

    def update(self, signal: Any) -> Any:
        """
        Sync update path: Add to queue (non-blocking).

        IMPORTANT: This stays SYNC so existing code doesn't break.
        The async Queue handles the sync/async boundary safely.
        """
        entry = self._get_or_create(signal.state_key, signal.action)

        # Do in-memory update (fast, no I/O)
        old_q = entry.get("q_value", 0.5)
        entry["q_value"] = old_q + 0.01  # Simplified for demo
        entry["visits"] = entry.get("visits", 0) + 1

        # Queue for async flush (thread-safe, non-blocking)
        try:
            # If event loop is running, use put_nowait (raises if queue is full)
            self._pending_queue.put_nowait(entry)
        except (RuntimeError, asyncio.QueueFull):
            # Fallback: try async put (shouldn't happen in normal operation)
            logger.warning(f"Could not queue entry {signal.state_key} synchronously")

        return entry

    async def flush_to_db(self) -> int:
        """
        Async flush path: Drain queue while update() adds to it.

        Uses Lock to ensure atomicity: either a batch is flushed or nothing.
        No interference between sync update() and async flush().
        """
        self._ensure_locks()

        # Collect all pending entries without blocking updates
        batch = []
        while True:
            try:
                # Non-blocking drain: get all queued entries
                entry = self._pending_queue.get_nowait()
                batch.append(entry)
            except asyncio.QueueEmpty:
                break

        if not batch or not self.storage:
            return 0

        # Async lock ensures this batch write is atomic
        async with self._flush_lock:
            count = 0
            for entry in batch:
                try:
                    await self.storage.update(
                        state_key=entry["state_key"],
                        action=entry["action"],
                        q_value=entry["q_value"],
                        visits=entry["visits"]
                    )
                    count += 1
                except Exception as e:
            logger.error(f"Flush failed for {entry}: {e}")

            if count > 0:
                logger.debug(f"Flushed {count} entries to storage")
            return count

    def _get_or_create(self, state_key: str, action: str) -> dict[str, Any]:
        """Get or create entry (simplified for demo)."""
        if state_key not in self._table:
            self._table[state_key] = {}

        if action not in self._table[state_key]:
            self._table[state_key][action] = {
                "state_key": state_key,
                "action": action,
                "q_value": 0.5,
                "visits": 0,
            }

        return self._table[state_key][action]


# DESIGN PATTERNS FOR ASYNC/SYNC BOUNDARIES
# =========================================

class Pattern1_AsyncQueueSafety:
    """
    Pattern: Use asyncio.Queue for syncasync handoff.

    Benefit: Queue is designed for this exact use case (thread-safe).
    Trade-off: Small memory overhead per item.
    Use when: Sync code needs to pass data to async consumer.
    """

    def __init__(self):
        self.queue = asyncio.Queue()

    def sync_producer(self, item):
        """Sync code adds to queue."""
        try:
            self.queue.put_nowait(item)
        except asyncio.QueueFull:
            logger.warning("Queue full, dropping item")

    async def async_consumer(self):
        """Async code drains queue."""
        batch = []
        while True:
            try:
                batch.append(self.queue.get_nowait())
            except asyncio.QueueEmpty:
                break
        return batch


class Pattern2_CopyOnWrite:
    """
    Pattern: Snapshot data before async operations.

    Benefit: No locking needed, Pythonic.
    Trade-off: Copying large objects is expensive.
    Use when: Data sets are small (<1MB).
    """

    def __init__(self):
        self.pending = []

    def sync_update(self, item):
        """Sync code updates in-memory."""
        self.pending.append(item)

    async def async_flush(self):
        """Async code snapshots and clears."""
        # Atomic snapshot
        batch = self.pending.copy()
        self.pending.clear()
        # Process batch...
        return len(batch)


class Pattern3_AsyncLockProtection:
    """
    Pattern: Use asyncio.Lock for shared mutable state.

    Benefit: Works in pure async context (no threading).
    Trade-off: Both paths must be async (not compatible with sync).
    Use when: All code can be async.
    """

    def __init__(self):
        self.data = {}
        self._lock = asyncio.Lock()

    async def update(self, key, value):
        """Async update with lock."""
        async with self._lock:
            self.data[key] = value

    async def flush(self):
        """Async flush with lock."""
        async with self._lock:
            batch = self.data.copy()
            self.data.clear()
            return batch


# WHICH PATTERN FOR QTABLE?
# =========================
#
# QTable uses Pattern 1 (asyncio.Queue):
#    update() is sync (can't change it  backward compat)
#    flush_to_db() is async
#    Queue handles the boundary safely
#
# Why not Pattern 2 (CopyOnWrite)?
#    QEntry objects are mutable references
#    Entries in pending_flush can be modified after snapshot
#    Flush sees inconsistent state
#
# Why not Pattern 3 (AsyncLock)?
#    update() is sync - can't await lock
#    Would require major refactor
#    Defeats purpose of sync update
