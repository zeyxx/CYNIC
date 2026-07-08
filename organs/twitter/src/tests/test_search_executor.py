import json

from organs.twitter.src.adapters import search_executor


def test_talaria_search_task_routes_to_pending(tmp_path, monkeypatch):
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    monkeypatch.setattr(search_executor, "PENDING_DIR", pending)
    monkeypatch.setattr(search_executor, "PROCESSED_DIR", processed)

    search_executor._route_to_pending_if_talaria(
        {"label": "talaria-token", "query": "$TALARIA"},
        "123",
        "alice",
        "$TALARIA launch signal",
        "https://x.com/alice/status/123",
    )

    payload = json.loads((pending / "tweet_123.json").read_text())
    assert payload["tweet_id"] == "123"
    assert payload["source"] == "search_sweep"
    assert payload["keywords_matched"] == ["$TALARIA"]
    assert payload["url"] == "https://x.com/alice/status/123"


def test_non_talaria_search_task_does_not_route_to_pending(tmp_path, monkeypatch):
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    monkeypatch.setattr(search_executor, "PENDING_DIR", pending)
    monkeypatch.setattr(search_executor, "PROCESSED_DIR", processed)

    search_executor._route_to_pending_if_talaria(
        {"label": "general", "query": "solana"},
        "456",
        "bob",
        "general search result",
        "https://x.com/bob/status/456",
    )

    assert not pending.exists()
