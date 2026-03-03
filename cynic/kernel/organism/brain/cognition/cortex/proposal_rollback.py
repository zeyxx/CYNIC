"""
ProposalRollback  Tracking and reverting executed proposals.

Maintains a persistent log of executed proposals with enough information
to reverse them if needed. Supports:
  - Recording executions (proposal_id, dimension, old/new values)
  - Rollback last N proposals
  - Rollback all proposals from last X minutes
  - History query (most recent entries)

Persistence:
  - Stored in ~/.cynic/proposal_rollback.json
  - Last 100 entries kept (rolling window)
"""

from __future__ import annotations

import json
import logging
import os
import time
from dataclasses import dataclass, field
from typing import Any

logger = logging.getLogger("cynic.kernel.organism.brain.cognition.cortex.proposal_rollback")

_CYNIC_DIR = os.path.join(os.path.expanduser("~"), ".cynic")
_ROLLBACK_PATH = os.path.join(_CYNIC_DIR, "proposal_rollback.json")

# Rolling cap: keep last 100 entries
_MAX_ROLLBACK_ENTRIES: int = 100


@dataclass
class RollbackEntry:
    """One recorded proposal execution."""

    proposal_id: str
    dimension: str  # QTABLE, METRICS, ESCORE, RESIDUAL, ARCHITECTURE, CONFIG
    target: str  # state:action, metric name, dog name, parameter name, etc.
    old_value: float
    new_value: float
    executed_at: float = field(default_factory=time.time)
    reversible: bool = True

    def to_dict(self) -> dict[str, Any]:
        return {
            "proposal_id": self.proposal_id,
            "dimension": self.dimension,
            "target": self.target,
            "old_value": round(self.old_value, 4),
            "new_value": round(self.new_value, 4),
            "executed_at": self.executed_at,
            "reversible": self.reversible,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> RollbackEntry:
        return cls(
            proposal_id=d["proposal_id"],
            dimension=d["dimension"],
            target=d["target"],
            old_value=float(d["old_value"]),
            new_value=float(d["new_value"]),
            executed_at=float(d["executed_at"]),
            reversible=bool(d.get("reversible", True)),
        )


class ProposalRollback:
    """
    Tracks executed proposals and supports rollback.

    Wire at build_kernel():
      rollback = ProposalRollback()
      # After each successful proposal execution:
      rollback.record(proposal_id, dimension, target, old_value, new_value)

      # To query:
      history = rollback.history(limit=50)

      # To rollback:
      rolled_back = rollback.rollback_last(5)
      rolled_back = rollback.rollback_since(minutes_ago=5.0)
    """

    def __init__(self, rollback_path: str = _ROLLBACK_PATH) -> None:
        self._path = rollback_path
        self._entries: list[RollbackEntry] = []
        self._load()

    #  Public API 

    def record(
        self,
        proposal_id: str,
        dimension: str,
        target: str,
        old_value: float,
        new_value: float,
        reversible: bool = True,
    ) -> None:
        """
        Record a proposal execution.

        Args:
            proposal_id: The proposal ID
            dimension: QTABLE, METRICS, ESCORE, RESIDUAL, ARCHITECTURE, CONFIG
            target: state:action, metric name, dog name, parameter name, etc.
            old_value: Previous value before execution
            new_value: New value after execution
            reversible: Whether this change can be reversed
        """
        entry = RollbackEntry(
            proposal_id=proposal_id,
            dimension=dimension,
            target=target,
            old_value=old_value,
            new_value=new_value,
            reversible=reversible,
        )
        self._entries.append(entry)

        # Rolling cap
        while len(self._entries) > _MAX_ROLLBACK_ENTRIES:
            self._entries.pop(0)

        self._save()
        logger.info(
            "ProposalRollback: Recorded %s [%s:%s] = %.4f  %.4f",
            proposal_id,
            dimension,
            target,
            old_value,
            new_value,
        )

    def rollback_last(self, count: int = 1) -> list[dict[str, Any]]:
        """
        Revert the last N proposals.

        Returns entries in reverse order (most recent first).

        Args:
            count: Number of proposals to rollback

        Returns:
            List of rolled back entries (reversed)
        """
        rolled_back: list[RollbackEntry] = []
        for _ in range(min(count, len(self._entries))):
            entry = self._entries.pop()
            rolled_back.append(entry)
            logger.info(
                "ProposalRollback: Rolled back %s [%s:%s] = %.4f  %.4f",
                entry.proposal_id,
                entry.dimension,
                entry.target,
                entry.new_value,  # Reversed: show new as old
                entry.old_value,  # Reversed: show old as new
            )

        self._save()
        return [e.to_dict() for e in rolled_back]

    def rollback_since(self, minutes_ago: float) -> list[dict[str, Any]]:
        """
        Revert all proposals from the last X minutes.

        Args:
            minutes_ago: Time window in minutes

        Returns:
            List of rolled back entries
        """
        now = time.time()
        cutoff = now - (minutes_ago * 60)

        rolled_back: list[RollbackEntry] = []
        remaining: list[RollbackEntry] = []

        for entry in self._entries:
            if entry.executed_at >= cutoff:
                rolled_back.append(entry)
            else:
                remaining.append(entry)

        self._entries = remaining
        self._save()

        logger.info(
            "ProposalRollback: Rolled back %d entries from last %.2f minutes",
            len(rolled_back),
            minutes_ago,
        )
        return [e.to_dict() for e in rolled_back]

    def history(self, limit: int = 50) -> list[dict[str, Any]]:
        """
        Get recent execution history.

        Returns entries in reverse order (most recent first).

        Args:
            limit: Maximum number of entries to return

        Returns:
            List of recent entries
        """
        # Return in reverse order (most recent first), limited to count
        reversed_entries = list(reversed(self._entries))
        return [e.to_dict() for e in reversed_entries[:limit]]

    #  Persistence 

    def _save(self) -> None:
        try:
            os.makedirs(os.path.dirname(self._path), exist_ok=True)
            with open(self._path, "w", encoding="utf-8") as fh:
                json.dump([e.to_dict() for e in self._entries], fh, indent=2)
        except OSError as exc:
            logger.debug("ProposalRollback._save failed: %s", exc)

    def _load(self) -> None:
        try:
            with open(self._path, encoding="utf-8") as fh:
                data = json.load(fh)
            if isinstance(data, list):
                for d in data:
                    self._entries.append(RollbackEntry.from_dict(d))
        except (json.JSONDecodeError, FileNotFoundError):
            pass
