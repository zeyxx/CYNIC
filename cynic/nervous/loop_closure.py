"""
CYNIC Tier 1 Nervous System — Loop Closure Validator

Component 4 (Feedback Completeness): Detects incomplete cycles and stalled loops.

The 7-step cycle MUST complete: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE

If any step stalls (event doesn't propagate), CYNIC detects it and raises alert.
If any step is missing (judgment made but never acted on), CYNIC detects orphan.

Pattern: State machine tracking phase transitions, detecting stalls & orphans.
Rolling cap: F(9) = 34 closure events (oldest dropped when 35th arrives).

Queryable via:
  - get_open_cycles(max_age_ms) — cycles waiting to close
  - get_stalled_phases(threshold_ms) — phases that stopped moving
  - get_orphan_judgments() — judgments with no action proposed
  - closure_stats() — health metrics

Enables:
  - Component 5 (Meta-cognition): CYNIC proposes auto-fixes for broken loops
  - L4 Self-improvement: SelfProber reacts to stalled loops
  - Circuit breaker: Prevents infinite waits
"""
from __future__ import annotations

import asyncio
import hashlib
import logging
import sys
import time
from collections import deque
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any, Optional

# Python 3.9 compatibility: StrEnum added in Python 3.11
if sys.version_info >= (3, 11):
    from enum import StrEnum
else:
    class StrEnum(str, Enum):
        """Polyfill for Python <3.11."""
        pass

from cynic.core.formulas import LOOP_CLOSURE_CAP

logger = logging.getLogger("cynic.nervous.loop_closure")

# φ-derived rolling cap: F(9) = 34 (imported from formulas.py)
CLOSURE_CAP = LOOP_CLOSURE_CAP

# Timeout thresholds (milliseconds)
STALL_THRESHOLD_MS = 5000  # 5 seconds = stalled
ORPHAN_THRESHOLD_MS = 10000  # 10 seconds = orphan (judgment without action)


class CyclePhase(StrEnum):
    """Phases in the 7-step cycle."""
    PERCEIVE = "perceive"
    JUDGE = "judge"
    DECIDE = "decide"
    ACT = "act"
    LEARN = "learn"
    ACCOUNT = "account"
    EMERGE = "emerge"


# Ordered phases for validation
CYCLE_PHASES = [
    CyclePhase.PERCEIVE,
    CyclePhase.JUDGE,
    CyclePhase.DECIDE,
    CyclePhase.ACT,
    CyclePhase.LEARN,
    CyclePhase.ACCOUNT,
    CyclePhase.EMERGE,
]


@dataclass
class PhaseEntry:
    """Entry for a phase in a cycle."""
    phase: CyclePhase
    event_id: str
    timestamp_ms: float
    component: str
    duration_ms: float = 0.0
    is_error: bool = False


@dataclass
class LoopClosureEvent:
    """Record of a cycle closure or failure."""
    event_id: str
    judgment_id: str
    created_at_ms: float

    # Phase tracking
    phases: list[PhaseEntry] = field(default_factory=list)

    # Status
    is_complete: bool = False  # Did all 7 phases complete?
    is_stalled: bool = False  # Did any phase timeout?
    is_orphan: bool = False  # Judgment without action?

    # Failure details
    last_phase: Optional[CyclePhase] = None
    stalled_at_phase: Optional[CyclePhase] = None
    stall_duration_ms: float = 0.0

    # Metrics
    total_cycle_ms: float = 0.0
    phase_count: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "event_id": self.event_id,
            "judgment_id": self.judgment_id,
            "created_at_ms": self.created_at_ms,
            "phases": [
                {
                    "phase": str(p.phase),
                    "event_id": p.event_id,
                    "timestamp_ms": p.timestamp_ms,
                    "component": p.component,
                    "duration_ms": p.duration_ms,
                    "is_error": p.is_error,
                }
                for p in self.phases
            ],
            "is_complete": self.is_complete,
            "is_stalled": self.is_stalled,
            "is_orphan": self.is_orphan,
            "last_phase": str(self.last_phase) if self.last_phase else None,
            "stalled_at_phase": str(self.stalled_at_phase) if self.stalled_at_phase else None,
            "stall_duration_ms": self.stall_duration_ms,
            "total_cycle_ms": self.total_cycle_ms,
            "phase_count": self.phase_count,
        }


class LoopClosureValidator:
    """
    Detects incomplete/stalled feedback cycles.

    Tracks 7-step cycle: PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE.
    Detects stalls, orphans, and missing phases.
    Thread-safe (asyncio.Lock), rolling buffer with cap F(9)=34.
    """

    def __init__(self):
        self._closures: deque = deque(maxlen=CLOSURE_CAP)
        self._open_cycles: dict[str, LoopClosureEvent] = {}  # judgment_id → event
        self._lock = asyncio.Lock()
        self._stats = {
            "total_cycles": 0,
            "complete_cycles": 0,
            "stalled_cycles": 0,
            "orphan_cycles": 0,
            "last_updated_ms": 0.0,
        }

    async def start_cycle(
        self,
        judgment_id: str,
        initial_event_id: str,
        component: str,
    ) -> str:
        """
        Start tracking a new cycle.

        Returns closure_event_id.
        """
        async with self._lock:
            timestamp_ms = time.time() * 1000.0

            # Generate event_id
            event_id = hashlib.sha256(
                f"{timestamp_ms}:{judgment_id}:cycle".encode()
            ).hexdigest()[:16]

            # Create root phase entry
            root_entry = PhaseEntry(
                phase=CyclePhase.PERCEIVE,
                event_id=initial_event_id,
                timestamp_ms=timestamp_ms,
                component=component,
            )

            closure = LoopClosureEvent(
                event_id=event_id,
                judgment_id=judgment_id,
                created_at_ms=timestamp_ms,
                phases=[root_entry],
                last_phase=CyclePhase.PERCEIVE,
                phase_count=1,
            )

            self._open_cycles[judgment_id] = closure
            logger.debug(f"Cycle started: {event_id} for judgment {judgment_id}")

            return event_id

    async def record_phase(
        self,
        judgment_id: str,
        phase: CyclePhase,
        event_id: str,
        component: str,
        duration_ms: float = 0.0,
        is_error: bool = False,
    ) -> bool:
        """
        Record a phase transition in a cycle.

        Returns True if phase was valid, False if out-of-order or cycle not found.
        """
        async with self._lock:
            closure = self._open_cycles.get(judgment_id)
            if not closure:
                logger.warning(f"Cycle not found for judgment {judgment_id}")
                return False

            timestamp_ms = time.time() * 1000.0

            # Validate phase order
            expected_phase_index = len(closure.phases)
            if expected_phase_index < len(CYCLE_PHASES):
                expected_phase = CYCLE_PHASES[expected_phase_index]
                if phase != expected_phase:
                    logger.warning(
                        f"Out-of-order phase for {judgment_id}: "
                        f"expected {expected_phase}, got {phase}"
                    )
                    # Record as error but continue
                    is_error = True

            # Check for stall (time gap)
            if closure.phases:
                time_since_last = timestamp_ms - closure.phases[-1].timestamp_ms
                if time_since_last > STALL_THRESHOLD_MS:
                    closure.is_stalled = True
                    closure.stalled_at_phase = closure.last_phase
                    closure.stall_duration_ms = time_since_last
                    logger.warning(
                        f"Stall detected in cycle {judgment_id}: "
                        f"{closure.stalled_at_phase} stalled for {time_since_last}ms"
                    )

            # Add phase entry
            entry = PhaseEntry(
                phase=phase,
                event_id=event_id,
                timestamp_ms=timestamp_ms,
                component=component,
                duration_ms=duration_ms,
                is_error=is_error,
            )

            closure.phases.append(entry)
            closure.last_phase = phase
            closure.phase_count = len(closure.phases)

            logger.debug(f"Phase recorded: {judgment_id} → {phase}")

            return True

    async def close_cycle(
        self,
        judgment_id: str,
    ) -> Optional[LoopClosureEvent]:
        """
        Close a cycle (mark complete or as orphan/stalled).

        Returns the closure event.
        """
        async with self._lock:
            closure = self._open_cycles.pop(judgment_id, None)
            if not closure:
                logger.warning(f"Cycle not found to close: {judgment_id}")
                return None

            timestamp_ms = time.time() * 1000.0

            # Check completion
            closure.is_complete = closure.phase_count >= len(CYCLE_PHASES)

            # Check if orphaned (judgment with no ACT phase)
            has_act = any(p.phase == CyclePhase.ACT for p in closure.phases)
            if not has_act and (timestamp_ms - closure.created_at_ms) > ORPHAN_THRESHOLD_MS:
                closure.is_orphan = True
                logger.warning(f"Orphan judgment detected: {judgment_id} (no ACT phase)")

            # Compute total duration
            if closure.phases:
                closure.total_cycle_ms = (
                    closure.phases[-1].timestamp_ms - closure.phases[0].timestamp_ms
                )

            # Store in closed list
            self._closures.append(closure)

            # Update stats
            self._stats["total_cycles"] += 1
            if closure.is_complete:
                self._stats["complete_cycles"] += 1
            if closure.is_stalled:
                self._stats["stalled_cycles"] += 1
            if closure.is_orphan:
                self._stats["orphan_cycles"] += 1
            self._stats["last_updated_ms"] = timestamp_ms

            logger.debug(
                f"Cycle closed: {judgment_id} | "
                f"complete={closure.is_complete} | "
                f"stalled={closure.is_stalled} | "
                f"orphan={closure.is_orphan}"
            )

            return closure

    async def get_open_cycles(self, max_age_ms: Optional[float] = None) -> list[LoopClosureEvent]:
        """Get cycles still in progress (not yet closed)."""
        async with self._lock:
            if max_age_ms is None:
                return list(self._open_cycles.values())

            timestamp_ms = time.time() * 1000.0
            threshold_ms = timestamp_ms - max_age_ms

            return [
                c for c in self._open_cycles.values()
                if c.created_at_ms >= threshold_ms
            ]

    async def get_stalled_phases(
        self,
        threshold_ms: float = STALL_THRESHOLD_MS,
    ) -> list[LoopClosureEvent]:
        """Get cycles with stalled phases."""
        async with self._lock:
            timestamp_ms = time.time() * 1000.0

            stalled = []
            for closure in self._open_cycles.values():
                if closure.phases:
                    time_since_last = timestamp_ms - closure.phases[-1].timestamp_ms
                    if time_since_last > threshold_ms:
                        stalled.append(closure)

            return stalled

    async def get_orphan_judgments(self) -> list[LoopClosureEvent]:
        """Get judgments that were never acted upon."""
        async with self._lock:
            timestamp_ms = time.time() * 1000.0
            orphans = []

            for closure in self._open_cycles.values():
                has_act = any(p.phase == CyclePhase.ACT for p in closure.phases)
                age_ms = timestamp_ms - closure.created_at_ms

                if not has_act and age_ms > ORPHAN_THRESHOLD_MS:
                    orphans.append(closure)

            return orphans

    async def recent_closures(
        self,
        limit: int = 10,
        include_complete_only: bool = False,
    ) -> list[LoopClosureEvent]:
        """Get recent cycle closures."""
        async with self._lock:
            closures = list(self._closures)

            if include_complete_only:
                closures = [c for c in closures if c.is_complete]

            return closures[-limit:][::-1]

    async def stats(self) -> dict[str, Any]:
        """Get closure validator statistics."""
        async with self._lock:
            total = self._stats["total_cycles"]
            complete = self._stats["complete_cycles"]
            stalled = self._stats["stalled_cycles"]
            orphan = self._stats["orphan_cycles"]

            completion_rate = (complete / total * 100) if total > 0 else 0.0
            stall_rate = (stalled / total * 100) if total > 0 else 0.0

            return {
                **self._stats,
                "buffer_size": len(self._closures),
                "buffer_cap": CLOSURE_CAP,
                "open_cycles": len(self._open_cycles),
                "completion_rate_percent": completion_rate,
                "stall_rate_percent": stall_rate,
                "orphan_rate_percent": (orphan / total * 100) if total > 0 else 0.0,
            }

    async def snapshot(self) -> dict[str, Any]:
        """Get complete validator state."""
        async with self._lock:
            return {
                "closed_cycles": [c.to_dict() for c in self._closures],
                "open_cycles": [c.to_dict() for c in self._open_cycles.values()],
                "stats": {
                    **self._stats,
                    "buffer_size": len(self._closures),
                    "buffer_cap": CLOSURE_CAP,
                    "open_cycles": len(self._open_cycles),
                },
            }

    async def clear(self) -> None:
        """Clear all cycles (testing)."""
        async with self._lock:
            self._closures.clear()
            self._open_cycles.clear()
            self._stats = {
                "total_cycles": 0,
                "complete_cycles": 0,
                "stalled_cycles": 0,
                "orphan_cycles": 0,
                "last_updated_ms": 0.0,
            }
