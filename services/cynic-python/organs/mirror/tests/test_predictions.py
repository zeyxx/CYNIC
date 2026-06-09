"""Tests for prediction record storage and outcome join."""
from datetime import datetime, timezone, timedelta
from pathlib import Path

import pytest

from organs.mirror.predictions import Prediction, PredictionStore
from organs.mirror.sources.base import Event


def test_record_and_retrieve(tmp_path: Path) -> None:
    store = PredictionStore(tmp_path / "predictions.jsonl")
    pred = store.record(
        prediction_type="bookmark",
        target_id="tweet_123",
        confidence=0.45,
        features_used=["narrative_affinity", "author_affinity"],
    )
    assert pred.outcome is None
    pending = store.get_pending()
    assert len(pending) == 1
    assert pending[0].target_id == "tweet_123"


def test_confirm_prediction(tmp_path: Path) -> None:
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("bookmark", "tweet_123", 0.45, ["narrative_affinity"])
    event = Event(
        source="x_signals",
        event_type="tweet_bookmarked",
        timestamp="2026-05-18T22:00:00+00:00",
        data={"tweet_id": "tweet_123", "viewer_bookmarked": True},
    )
    resolved = store.check_outcomes([event])
    assert len(resolved) == 1
    assert resolved[0].outcome == "confirmed"


def test_expire_old_predictions(tmp_path: Path) -> None:
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("bookmark", "tweet_old", 0.45, ["narrative_affinity"])
    # Age the prediction past 24h
    store._pending[0].created_at = (
        datetime.now(timezone.utc) - timedelta(hours=25)
    ).isoformat()
    resolved = store.check_outcomes([])
    assert len(resolved) == 1
    assert resolved[0].outcome == "expired"


def test_pending_cleared_after_resolution(tmp_path: Path) -> None:
    """Resolved predictions must be removed from _pending."""
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("engage", "tweet_abc", 0.6, ["author_affinity"])
    event = Event(
        source="x_signals",
        event_type="tweet_liked",
        timestamp="2026-05-18T22:00:00+00:00",
        data={"tweet_id": "tweet_abc"},
    )
    store.check_outcomes([event])
    assert store.get_pending() == []


def test_unmatched_event_leaves_pending(tmp_path: Path) -> None:
    """An event that does not match target_id must not resolve the prediction."""
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("bookmark", "tweet_xyz", 0.5, [])
    event = Event(
        source="x_signals",
        event_type="tweet_bookmarked",
        timestamp="2026-05-18T22:00:00+00:00",
        data={"tweet_id": "tweet_different"},
    )
    resolved = store.check_outcomes([event])
    assert resolved == []
    assert len(store.get_pending()) == 1


def test_record_persists_to_jsonl(tmp_path: Path) -> None:
    """Each recorded prediction must append a JSON line to disk."""
    import json

    path = tmp_path / "predictions.jsonl"
    store = PredictionStore(path)
    store.record("ignore", "tweet_z", 0.1, ["recency"])
    store.record("engage", "tweet_w", 0.9, ["author_affinity"])

    lines = path.read_text(encoding="utf-8").strip().splitlines()
    assert len(lines) == 2
    row = json.loads(lines[0])
    assert row["target_id"] == "tweet_z"
    assert row["outcome"] is None


def test_multiple_pending_partial_resolution(tmp_path: Path) -> None:
    """Only matching predictions resolve; others stay pending."""
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("bookmark", "tweet_A", 0.7, [])
    store.record("bookmark", "tweet_B", 0.3, [])

    event = Event(
        source="x_signals",
        event_type="tweet_bookmarked",
        timestamp="2026-05-18T22:00:00+00:00",
        data={"tweet_id": "tweet_A"},
    )
    resolved = store.check_outcomes([event])
    assert len(resolved) == 1
    assert resolved[0].target_id == "tweet_A"
    pending = store.get_pending()
    assert len(pending) == 1
    assert pending[0].target_id == "tweet_B"


def test_outcome_at_set_on_resolution(tmp_path: Path) -> None:
    """Resolved predictions must carry an outcome_at timestamp."""
    store = PredictionStore(tmp_path / "predictions.jsonl")
    store.record("engage", "tweet_ts", 0.55, [])
    event = Event(
        source="x_signals",
        event_type="tweet_clicked",
        timestamp="2026-05-18T23:00:00+00:00",
        data={"tweet_id": "tweet_ts"},
    )
    resolved = store.check_outcomes([event])
    assert resolved[0].outcome_at is not None
    # Must be parseable as a datetime
    datetime.fromisoformat(resolved[0].outcome_at)
