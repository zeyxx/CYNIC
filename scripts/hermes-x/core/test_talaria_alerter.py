# scripts/hermes-x/core/test_talaria_alerter.py
"""Unit tests for talaria_alerter — Telegram calls mocked."""
import json
import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from pathlib import Path
from unittest.mock import patch, MagicMock
import pytest


def test_format_normal_interaction():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "mention",
        "author": "@alice",
        "author_followers": 12400,
        "text": "MetaDAO ICO is live!",
        "url": "https://x.com/alice/status/123",
        "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z",
    }
    msg = format_telegram_message(entry)
    assert "🔔" in msg
    assert "@TalariaBuild" in msg
    assert "mention" in msg
    assert "@alice" in msg
    assert "12.4K" in msg
    assert "MetaDAO ICO is live!" in msg
    assert "https://x.com/alice/status/123" in msg


def test_format_unknown_followers():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "reply",
        "author": "@bob",
        "author_followers": None,
        "text": "When launch?",
        "url": "https://x.com/bob/status/456",
        "source": "search_sweep",
        "detected_at": "2026-06-02T18:01:00Z",
    }
    msg = format_telegram_message(entry)
    assert "@bob" in msg
    assert "None" not in msg


def test_format_monitor_blind():
    from talaria_alerter import format_telegram_message
    entry = {
        "type": "monitor_blind",
        "author": "@system",
        "author_followers": None,
        "text": "3 consecutive empty cycles.",
        "url": "",
        "source": "notification_poller",
        "detected_at": "2026-06-02T18:05:00Z",
    }
    msg = format_telegram_message(entry)
    assert "⚠️" in msg
    assert "MONITOR BLIND" in msg


def test_send_telegram_returns_true_on_200():
    from talaria_alerter import send_telegram
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is True


def test_send_telegram_returns_false_on_429():
    from talaria_alerter import send_telegram
    mock_resp = MagicMock()
    mock_resp.status_code = 429
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is False


def test_send_telegram_returns_false_on_exception():
    from talaria_alerter import send_telegram
    with patch("talaria_alerter.requests.post", side_effect=Exception("network down")):
        result = send_telegram("fake_token", "-123456", "hello")
    assert result is False


def test_process_pending_moves_on_success(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    entry = {
        "schema_version": 1, "tweet_id": "abc", "type": "mention",
        "author": "@alice", "author_followers": None, "text": "test",
        "url": "https://x.com/alice/status/abc", "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z", "keywords_matched": [],
    }
    (pending / "tweet_abc.json").write_text(json.dumps(entry))
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")
    assert sent == 1
    assert kept == 0
    assert not (pending / "tweet_abc.json").exists()
    assert (processed / "tweet_abc.json").exists()


def test_process_pending_keeps_on_429(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    entry = {
        "schema_version": 1, "tweet_id": "xyz", "type": "mention",
        "author": "@bob", "author_followers": None, "text": "test",
        "url": "https://x.com/bob/status/xyz", "source": "notification_poller",
        "detected_at": "2026-06-02T18:00:00Z", "keywords_matched": [],
    }
    (pending / "tweet_xyz.json").write_text(json.dumps(entry))
    mock_resp = MagicMock()
    mock_resp.status_code = 429
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")
    assert sent == 0
    assert kept == 1
    assert (pending / "tweet_xyz.json").exists()


def test_process_pending_respects_batch_cap(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    for i in range(25):
        entry = {
            "schema_version": 1, "tweet_id": str(i), "type": "mention",
            "author": "@user", "author_followers": None, "text": "t",
            "url": f"https://x.com/u/status/{i}", "source": "notification_poller",
            "detected_at": "2026-06-02T18:00:00Z", "keywords_matched": [],
        }
        (pending / f"tweet_{i}.json").write_text(json.dumps(entry))
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")
    assert sent == 20
    assert kept == 5


def test_process_pending_skips_invalid_json(tmp_path):
    from talaria_alerter import process_pending
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (pending / "tweet_bad.json").write_text("not json {{")
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("talaria_alerter.requests.post", return_value=mock_resp):
        sent, kept = process_pending(pending, processed, "fake_token", "-123")
    assert sent == 0
    assert kept == 0
