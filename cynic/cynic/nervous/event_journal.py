"""
CYNIC Tier 1 Nervous System — Event Journal

Component 2 (Sequencing): Records all events flowing through the system in order.

Every event gets captured:
  - timestamp (ms precision)
  - type (CoreEvent enum string)
  - source (component that fired it)
  - payload (event data)
  - duration (how long it took to propagate)

Pattern: Append-only rolling buffer, φ-indexed by type, queryable by time range.
Rolling cap: F(11) = 89 events (oldest dropped when 90th arrives).

Queryable via:
  - recent(limit=10) — last N events
  - filter_by_type(type_name) — all events of type X
  - time_range(start_ms, end_ms) — events in window
  - get_event(event_id) — single event lookup

This component enables:
  - Component 3 (DecisionTrace): Uses events to build execution DAG
  - Component 4 (LoopClosureValidator): Uses events to detect stalls
  - L2 Feedback loop: Claude Code sees event patterns
"""
from __future__ import annotations

import asyncio
import hashlib
import logging
import time
from dataclasses import asdict, dataclass, field
from typing import Any, Optional
from collections import deque
from enum import StrEnum

logger = logging.getLogger("cynic.nervous.event_journal")

# φ-derived rolling cap: F(11) = 89
JOURNAL_CAP = 89


class EventCategory(StrEnum):
    """Event type categories for grouping."""
    PERCEPTION = "perception"       # PERCEIVE phase
    JUDGMENT = "judgment"            # JUDGE phase
    DECISION = "decision"            # DECIDE phase
    ACTION = "action"                # ACT phase
    LEARNING = "learning"            # LEARN phase
    ACCOUNTING = "accounting"        # ACCOUNT phase
    EMERGENCE = "emergence"          # EMERGE phase
    SYSTEM = "system"                # Internal (startup, shutdown, errors)
    ADMIN = "admin"                  # Manual interventions


@dataclass
class JournalEntry:
    """Single event recorded in the journal."""
    event_id: str                    # Hash of (timestamp, source, type)
    timestamp_ms: float              # Time.time() * 1000
    event_type: str                  # CoreEvent enum value
    category: EventCategory          # Parent category
    source: str                       # Component that emitted it
    payload_keys: list[str]          # Keys from payload (not full payload, for privacy)
    duration_ms: float = 0.0         # How long processing took

    # Causality tracking
    parent_event_id: Optional[str] = None  # Which event triggered this
    child_event_ids: list[str] = field(default_factory=list)  # Events this triggered

    # Error tracking
    is_error: bool = False
    error_message: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["category"] = str(self.category)
        return d

    @staticmethod
    def from_dict(d: dict[str, Any]) -> JournalEntry:
        d_copy = dict(d)
        d_copy["category"] = EventCategory(d_copy["category"])
        return JournalEntry(**{k: v for k, v in d_copy.items() if k in JournalEntry.__dataclass_fields__})


class EventJournal:
    """
    Append-only event journal — records all events in sequence.

    Thread-safe (asyncio.Lock), rolling buffer with cap F(11)=89.
    Enables causality tracing (parent/child relationships).
    """

    def __init__(self):
        self._entries: deque = deque(maxlen=JOURNAL_CAP)  # Auto-drops oldest
        self._lock = asyncio.Lock()
        self._index_by_type: dict[str, list[str]] = {}  # type → [event_ids]
        self._index_by_source: dict[str, list[str]] = {}  # source → [event_ids]
        self._event_map: dict[str, JournalEntry] = {}  # event_id → entry
        self._stats = {
            "total_recorded": 0,
            "by_category": {},
            "last_updated_ms": 0.0,
        }

    async def record(
        self,
        event_type: str,
        category: EventCategory,
        source: str,
        payload: dict[str, Any],
        parent_event_id: Optional[str] = None,
        duration_ms: float = 0.0,
        is_error: bool = False,
        error_message: Optional[str] = None,
    ) -> str:
        """
        Record an event in the journal.

        Returns event_id (can be used as parent for child events).
        """
        async with self._lock:
            timestamp_ms = time.time() * 1000.0
            payload_keys = list(payload.keys()) if payload else []

            # Generate deterministic event_id
            event_id = hashlib.sha256(
                f"{timestamp_ms}:{source}:{event_type}".encode()
            ).hexdigest()[:16]

            # Create entry
            entry = JournalEntry(
                event_id=event_id,
                timestamp_ms=timestamp_ms,
                event_type=event_type,
                category=category,
                source=source,
                payload_keys=payload_keys,
                duration_ms=duration_ms,
                parent_event_id=parent_event_id,
                is_error=is_error,
                error_message=error_message,
            )

            # Store in circular buffer (auto-evicts oldest if at cap)
            old_size = len(self._entries)
            self._entries.append(entry)

            # If evicted, clean up indices
            if old_size == JOURNAL_CAP:
                await self._clean_indices()

            # Index for fast lookup
            self._event_map[event_id] = entry

            if event_type not in self._index_by_type:
                self._index_by_type[event_type] = []
            self._index_by_type[event_type].append(event_id)

            if source not in self._index_by_source:
                self._index_by_source[source] = []
            self._index_by_source[source].append(event_id)

            # Update stats
            self._stats["total_recorded"] += 1
            cat_str = str(category)
            self._stats["by_category"][cat_str] = self._stats["by_category"].get(cat_str, 0) + 1
            self._stats["last_updated_ms"] = timestamp_ms

            logger.debug(
                f"Event recorded: {event_id} | {event_type} | {source} | "
                f"duration={duration_ms:.1f}ms | buffer={len(self._entries)}/{JOURNAL_CAP}"
            )

            return event_id

    async def _clean_indices(self) -> None:
        """Remove indices for events that were evicted from buffer."""
        if not self._entries:
            return

        # Keep only event_ids that are still in buffer
        live_ids = {e.event_id for e in self._entries}

        # Remove dead indices
        for event_id in list(self._event_map.keys()):
            if event_id not in live_ids:
                del self._event_map[event_id]

        # Rebuild type/source indices from live entries
        self._index_by_type.clear()
        self._index_by_source.clear()

        for entry in self._entries:
            if entry.event_type not in self._index_by_type:
                self._index_by_type[entry.event_type] = []
            self._index_by_type[entry.event_type].append(entry.event_id)

            if entry.source not in self._index_by_source:
                self._index_by_source[entry.source] = []
            self._index_by_source[entry.source].append(entry.event_id)

    async def recent(self, limit: int = 10) -> list[JournalEntry]:
        """Get last N events (most recent first)."""
        async with self._lock:
            entries = list(self._entries)
            return entries[-limit:][::-1]  # Reverse to get newest first

    async def filter_by_type(self, event_type: str, limit: int = 50) -> list[JournalEntry]:
        """Get all events of a specific type."""
        async with self._lock:
            event_ids = self._index_by_type.get(event_type, [])
            entries = [self._event_map[eid] for eid in event_ids if eid in self._event_map]
            return entries[-limit:][::-1]  # Newest first

    async def filter_by_source(self, source: str, limit: int = 50) -> list[JournalEntry]:
        """Get all events from a specific component."""
        async with self._lock:
            event_ids = self._index_by_source.get(source, [])
            entries = [self._event_map[eid] for eid in event_ids if eid in self._event_map]
            return entries[-limit:][::-1]

    async def filter_by_category(self, category: EventCategory, limit: int = 50) -> list[JournalEntry]:
        """Get all events in a category."""
        async with self._lock:
            entries = [e for e in self._entries if e.category == category]
            return entries[-limit:][::-1]

    async def time_range(self, start_ms: float, end_ms: float) -> list[JournalEntry]:
        """Get events within time window [start_ms, end_ms]."""
        async with self._lock:
            entries = [
                e for e in self._entries
                if start_ms <= e.timestamp_ms <= end_ms
            ]
            return entries  # Chronological order

    async def get_event(self, event_id: str) -> Optional[JournalEntry]:
        """Look up single event by ID."""
        async with self._lock:
            return self._event_map.get(event_id)

    async def causality_chain(self, event_id: str, direction: str = "down") -> list[JournalEntry]:
        """
        Trace causality: follow parent_event_id (up) or child_event_ids (down).

        direction: "up" (causes) or "down" (effects)
        """
        async with self._lock:
            chain = []
            current = self._event_map.get(event_id)

            if not current:
                return []

            if direction == "up":
                # Walk up parent chain
                while current and current.parent_event_id:
                    chain.append(current)
                    current = self._event_map.get(current.parent_event_id)
                if current:
                    chain.append(current)
            else:  # down
                # BFS through children (not recursive for safety)
                queue = [current]
                visited = set()

                while queue:
                    entry = queue.pop(0)
                    if entry.event_id in visited:
                        continue
                    visited.add(entry.event_id)
                    chain.append(entry)

                    for child_id in entry.child_event_ids:
                        child = self._event_map.get(child_id)
                        if child and child.event_id not in visited:
                            queue.append(child)

            return chain

    async def errors_since(self, timestamp_ms: float) -> list[JournalEntry]:
        """Get all error events since timestamp."""
        async with self._lock:
            errors = [
                e for e in self._entries
                if e.is_error and e.timestamp_ms >= timestamp_ms
            ]
            return errors

    async def stats(self) -> dict[str, Any]:
        """Get journal statistics."""
        async with self._lock:
            return {
                **self._stats,
                "buffer_size": len(self._entries),
                "buffer_cap": JOURNAL_CAP,
                "unique_types": len(self._index_by_type),
                "unique_sources": len(self._index_by_source),
            }

    async def snapshot(self) -> dict[str, Any]:
        """Get complete journal state (for testing/debugging)."""
        async with self._lock:
            return {
                "entries": [e.to_dict() for e in self._entries],
                "stats": await self.stats(),
            }

    async def clear(self) -> None:
        """Clear all entries (for testing)."""
        async with self._lock:
            self._entries.clear()
            self._index_by_type.clear()
            self._index_by_source.clear()
            self._event_map.clear()
            self._stats = {
                "total_recorded": 0,
                "by_category": {},
                "last_updated_ms": 0.0,
            }
