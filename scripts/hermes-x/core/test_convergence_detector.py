import pytest
from datetime import datetime, timezone, timedelta
from convergence_detector import detect_convergence, ConvergenceSignal


def _obs(author: str, cashtag: str, minutes_ago: int = 0) -> dict:
    ts = datetime.now(timezone.utc) - timedelta(minutes=minutes_ago)
    return {
        "agent_id": "hermes-x-cynic",
        "tags": [cashtag],
        "context": f"@{author} [5]: something about ${cashtag}",
        "created_at": ts.isoformat(),
        "target": f"tweet_{author}_{cashtag}",
    }


def test_convergence_3_authors():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 1
    assert signals[0].cashtag == "SOL"
    assert signals[0].author_count == 3


def test_no_convergence_below_threshold():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0


def test_same_author_doesnt_count_twice():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("alice", "SOL", 20),  # same author
        _obs("bob", "SOL", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0


def test_outside_window_excluded():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 400),  # >6h ago
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 0


def test_multiple_cashtags():
    obs = [
        _obs("alice", "SOL", 10),
        _obs("bob", "SOL", 20),
        _obs("carol", "SOL", 30),
        _obs("dave", "BONK", 10),
        _obs("eve", "BONK", 20),
        _obs("frank", "BONK", 30),
    ]
    signals = detect_convergence(obs, min_authors=3, window_hours=6)
    assert len(signals) == 2
    tags = {s.cashtag for s in signals}
    assert tags == {"SOL", "BONK"}
