"""Tests for dataset.jsonl source — tweet parsing."""
import json
from pathlib import Path
from organs.mirror.sources.x_signals import XSignalsSource

MINIMAL_TWEET = {
    "tweet_id": "123", "text": "test tweet", "author_screen_name": "alice",
    "author_followers_count": 5000, "capture_ts": "2026-05-18T21:00:00Z",
    "signal_score": 0.7, "narratives": ["agent"], "likes": 100, "retweets": 20,
    "views": 5000, "engagement_rate": 0.024, "viewer_bookmarked": False,
    "has_media": False, "is_self_thread": False,
}


def _write_tweets(path: Path, tweets: list[dict]) -> None:
    with open(path, "w") as f:
        for t in tweets:
            f.write(json.dumps(t) + "\n")


def test_parse_tweet(tmp_path: Path) -> None:
    path = tmp_path / "dataset.jsonl"
    _write_tweets(path, [MINIMAL_TWEET])
    source = XSignalsSource(path)
    events = list(source.read_from(0))
    assert len(events) == 1
    assert events[0].source == "x_signals"
    assert events[0].event_type == "tweet_seen"
    assert events[0].data["author_screen_name"] == "alice"


def test_bookmarked_tweet_event_type(tmp_path: Path) -> None:
    path = tmp_path / "dataset.jsonl"
    tweet = {**MINIMAL_TWEET, "viewer_bookmarked": True}
    _write_tweets(path, [tweet])
    source = XSignalsSource(path)
    events = list(source.read_from(0))
    assert events[0].event_type == "tweet_bookmarked"


def test_skip_malformed_tweet(tmp_path: Path) -> None:
    path = tmp_path / "dataset.jsonl"
    with open(path, "w") as f:
        f.write("broken\n")
        f.write(json.dumps(MINIMAL_TWEET) + "\n")
    source = XSignalsSource(path)
    events = list(source.read_from(0))
    assert len(events) == 1
    assert source.error_count == 1
