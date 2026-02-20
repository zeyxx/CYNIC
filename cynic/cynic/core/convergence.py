"""
ConvergenceValidator — Observe announced vs actual behavior.

Phase 3: The primordial missing piece.

Organism announces: "I will judge this cell as HOWL"
We record: announcement + outcome + reality check

Simple data structure:
- announced: what organism said it would do
- actual: what actually happened
- match: did reality match announcement?
- latency: time between announcement and verification
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any, Optional

logger = logging.getLogger("cynic.core.convergence")


@dataclass
class Announcement:
    """What the organism announced it would do."""

    announcement_id: str
    timestamp: float
    announced_verdict: str  # HOWL, WAG, GROWL, BARK
    announced_q_score: float
    announced_action: Optional[str] = None
    cell_id: Optional[str] = None
    confidence: float = 0.0

    def __repr__(self) -> str:
        return f"Announce({self.announced_verdict} Q={self.announced_q_score:.1f})"


@dataclass
class Outcome:
    """What actually happened."""

    outcome_id: str
    timestamp: float
    actual_verdict: str
    actual_q_score: float
    actual_action_executed: bool = False
    error: Optional[str] = None

    def __repr__(self) -> str:
        return f"Outcome({self.actual_verdict} Q={self.actual_q_score:.1f})"


@dataclass
class Convergence:
    """Announced vs actual comparison."""

    convergence_id: str
    announcement: Announcement
    outcome: Outcome
    match: bool = False
    latency_ms: float = 0.0
    metadata: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        """Calculate convergence metrics."""
        self.latency_ms = (self.outcome.timestamp - self.announcement.timestamp) * 1000
        self.match = (
            self.announcement.announced_verdict == self.outcome.actual_verdict
        )

    def __repr__(self) -> str:
        status = "✓ MATCH" if self.match else "✗ DIVERGE"
        return (
            f"{status}: {self.announcement.announced_verdict} "
            f"→ {self.outcome.actual_verdict} ({self.latency_ms:.0f}ms)"
        )


class ConvergenceValidator:
    """
    Track organism announcements and verify outcomes.

    Simple rolling log:
    - Record when organism announces a decision
    - Record when that decision is executed/verified
    - Compare: did reality match announcement?
    """

    def __init__(self, capacity: int = 89):  # F(11)
        self.capacity = capacity
        self._announcements: dict[str, Announcement] = {}  # id → announcement
        self._convergences: list[Convergence] = []  # rolling log
        self._total_announcements = 0
        self._total_matches = 0

    def announce(
        self,
        verdict: str,
        q_score: float,
        cell_id: Optional[str] = None,
        action: Optional[str] = None,
        confidence: float = 0.0,
    ) -> str:
        """
        Record an announcement.

        Args:
            verdict: What organism announced (HOWL, WAG, GROWL, BARK)
            q_score: Q-score announced
            cell_id: Cell being judged (optional)
            action: Action announced (optional)
            confidence: φ-bounded confidence

        Returns:
            announcement_id (for later matching)
        """
        announcement_id = f"ann_{self._total_announcements:06d}"
        self._total_announcements += 1

        ann = Announcement(
            announcement_id=announcement_id,
            timestamp=time.time(),
            announced_verdict=verdict,
            announced_q_score=q_score,
            announced_action=action,
            cell_id=cell_id,
            confidence=confidence,
        )

        self._announcements[announcement_id] = ann
        logger.info(f"*sniff* Announced: {ann}")
        return announcement_id

    def record_outcome(
        self,
        announcement_id: str,
        actual_verdict: str,
        actual_q_score: float,
        action_executed: bool = False,
        error: Optional[str] = None,
    ) -> Convergence:
        """
        Record outcome for an announcement.

        Args:
            announcement_id: ID from announce()
            actual_verdict: What actually happened
            actual_q_score: Actual Q-score
            action_executed: Was action executed?
            error: Error if any

        Returns:
            Convergence record
        """
        ann = self._announcements.get(announcement_id)
        if not ann:
            logger.warning(f"Outcome for unknown announcement: {announcement_id}")
            ann = Announcement(
                announcement_id=announcement_id,
                timestamp=time.time() - 1.0,
                announced_verdict="UNKNOWN",
                announced_q_score=0.0,
            )

        outcome = Outcome(
            outcome_id=f"out_{len(self._convergences):06d}",
            timestamp=time.time(),
            actual_verdict=actual_verdict,
            actual_q_score=actual_q_score,
            actual_action_executed=action_executed,
            error=error,
        )

        convergence = Convergence(
            convergence_id=f"conv_{len(self._convergences):06d}",
            announcement=ann,
            outcome=outcome,
        )

        self._convergences.append(convergence)

        # Keep rolling log capped
        if len(self._convergences) > self.capacity:
            self._convergences.pop(0)

        if convergence.match:
            self._total_matches += 1
            logger.info(f"✓ Convergence MATCH: {convergence}")
        else:
            logger.warning(f"✗ Convergence DIVERGE: {convergence}")

        return convergence

    def recent(self, limit: int = 21) -> list[Convergence]:
        """Get recent convergences."""
        return self._convergences[-limit:]

    def stats(self) -> dict[str, Any]:
        """Get convergence statistics."""
        total = len(self._convergences)
        matches = sum(1 for c in self._convergences if c.match)
        convergence_rate = (matches / max(total, 1)) * 100

        return {
            "total_announced": self._total_announcements,
            "total_outcomes": total,
            "total_matches": self._total_matches,
            "convergence_rate": round(convergence_rate, 1),
            "recent": [str(c) for c in self.recent(5)],
        }
