"""
Tier 2 INFRASTRUCTURE: Prediction records and outcome join for mirror organ.

K15 Consumer: mirror scoring loop reads resolved predictions to calibrate
              future confidence weights.
Systemd: wired via mirror organ lifecycle (no standalone service).
Promotion date: 2026-05-18 (from mirror Task 8 spec).
Stability: new.

Failure mode: If the JSONL path is unwritable, record() raises immediately.
              Caller must handle.
"""
from __future__ import annotations

import json
import uuid
from dataclasses import asdict, dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

from organs.mirror.sources.base import Event

__version__ = "0.1.0"

# Positive event_types that confirm a bookmark/engage prediction.
_POSITIVE_EVENT_TYPES: frozenset[str] = frozenset(
    {
        "tweet_bookmarked",
        "tweet_liked",
        "tweet_engaged",
        "tweet_clicked",
    }
)

_EXPIRY_HOURS: int = 24


@dataclass
class Prediction:
    """Single prediction record.

    Input contract:
        - id: non-empty UUID string
        - created_at: ISO-8601 datetime with timezone
        - prediction_type: one of "bookmark", "engage", "ignore"
        - target_id: tweet_id, URL, or content hash — non-empty
        - confidence: float in [0, 1]
        - features_used: list of feature name strings (may be empty)
        - outcome: None (pending), "confirmed", "rejected", or "expired"
        - outcome_at: ISO-8601 datetime or None

    Output guarantees:
        - Serialisable to/from dict via dataclasses.asdict.

    Failure modes:
        - No validation here; callers own invariants.

    Valid domains: mirror organ only.
    """

    id: str
    created_at: str
    prediction_type: str
    target_id: str
    confidence: float
    features_used: list[str]
    outcome: str | None
    outcome_at: str | None


class PredictionStore:
    """Append-only store for Prediction records with outcome resolution.

    Input contract:
        - path: writable filesystem path for a JSONL file.

    Output guarantees:
        - record() always appends a line to the JSONL file before returning.
        - check_outcomes() never mutates the file; persistence of resolved
          predictions is a caller responsibility (extend if needed).

    Failure modes:
        - I/O errors propagate without suppression (fail loud, P9).

    Valid domains: mirror organ lifecycle loop.
    """

    def __init__(self, path: Path) -> None:
        self._path = path
        self._pending: list[Prediction] = []

    def record(
        self,
        prediction_type: str,
        target_id: str,
        confidence: float,
        features_used: list[str],
    ) -> Prediction:
        """Create a Prediction, persist it, and track it as pending.

        Returns the newly created Prediction.
        """
        now = datetime.now(timezone.utc).isoformat()
        prediction = Prediction(
            id=str(uuid.uuid4()),
            created_at=now,
            prediction_type=prediction_type,
            target_id=target_id,
            confidence=confidence,
            features_used=list(features_used),
            outcome=None,
            outcome_at=None,
        )
        self._persist(prediction)
        self._pending.append(prediction)
        return prediction

    def get_pending(self) -> list[Prediction]:
        """Return all pending (unresolved) predictions."""
        return list(self._pending)

    def check_outcomes(self, events: list[Event]) -> list[Prediction]:
        """Resolve pending predictions against a list of events.

        Resolution rules (in order):
          1. If created_at older than _EXPIRY_HOURS → outcome = "expired".
          2. If any event matches target_id and is positive → outcome = "confirmed".

        Resolved predictions are removed from _pending.
        Returns the list of newly resolved predictions.
        """
        now = datetime.now(timezone.utc)
        resolved: list[Prediction] = []
        remaining: list[Prediction] = []

        for pred in self._pending:
            outcome = _resolve(pred, events, now)
            if outcome is not None:
                pred.outcome = outcome
                pred.outcome_at = now.isoformat()
                resolved.append(pred)
            else:
                remaining.append(pred)

        self._pending = remaining
        return resolved

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    def _persist(self, prediction: Prediction) -> None:
        """Append prediction as a JSON line to the JSONL file."""
        self._path.parent.mkdir(parents=True, exist_ok=True)
        with self._path.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(asdict(prediction), ensure_ascii=False) + "\n")


def _resolve(
    pred: Prediction, events: list[Event], now: datetime
) -> str | None:
    """Return outcome string if resolvable, else None.

    Expiry takes priority over confirmation so that a 25h-old bookmarked
    tweet doesn't produce an ambiguous "confirmed+expired" state.
    """
    created = datetime.fromisoformat(pred.created_at)
    if now - created >= timedelta(hours=_EXPIRY_HOURS):
        return "expired"

    for event in events:
        tweet_id = event.data.get("tweet_id")
        if tweet_id != pred.target_id:
            continue
        if event.event_type in _POSITIVE_EVENT_TYPES:
            return "confirmed"

    return None
