# scripts/hermes-x/core/test_notification_poller.py
"""Unit tests for notification_poller — all CDP calls mocked."""
import json
import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from pathlib import Path
from unittest.mock import MagicMock, patch, AsyncMock
import pytest
import tempfile


# ── parse_notification_text ──────────────────────────────────────────────────

def test_parse_notification_mention():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@alice mentioned you in a Tweet\nHey $TALARIA is launching!")
    assert result["type"] == "mention"
    assert "TALARIA" in result["text"]


def test_parse_notification_reply():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@bob replied to your Tweet\nWhen is the ICO?")
    assert result["type"] == "reply"


def test_parse_notification_retweet():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@carol retweeted your Tweet")
    assert result["type"] == "retweet"


def test_parse_notification_like():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@dave liked your Tweet")
    assert result["type"] == "like"


def test_parse_notification_unknown_defaults_to_notification():
    from notification_poller import parse_notification_text
    result = parse_notification_text("@eve followed you")
    assert result["type"] == "notification"


# ── build_interaction_entry ─────────────────────────────────────────────────

def test_build_entry_has_required_fields():
    from notification_poller import build_interaction_entry
    entry = build_interaction_entry(
        tweet_id="999",
        notif_type="mention",
        author="alice",
        text="Hello Talaria",
        source="notification_poller",
    )
    assert entry["schema_version"] == 1
    assert entry["tweet_id"] == "999"
    assert entry["type"] == "mention"
    assert entry["author"] == "@alice"
    assert entry["source"] == "notification_poller"
    assert "detected_at" in entry
    assert entry["url"] == "https://x.com/alice/status/999"


def test_build_entry_normalises_author_handle():
    from notification_poller import build_interaction_entry
    entry = build_interaction_entry("1", "mention", "@alice", "text", "notification_poller")
    assert entry["author"] == "@alice"


# ── dedup logic ──────────────────────────────────────────────────────────────

def test_is_already_seen_pending(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (pending / "tweet_123.json").write_text("{}")
    assert is_already_seen("123", pending, processed) is True


def test_is_already_seen_processed(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    (processed / "tweet_456.json").write_text("{}")
    assert is_already_seen("456", pending, processed) is True


def test_is_not_seen(tmp_path):
    from notification_poller import is_already_seen
    pending = tmp_path / "pending"
    processed = tmp_path / "processed"
    pending.mkdir()
    processed.mkdir()
    assert is_already_seen("789", pending, processed) is False


# ── write_pending ────────────────────────────────────────────────────────────

def test_write_pending_creates_file(tmp_path):
    from notification_poller import write_pending
    pending = tmp_path / "pending"
    pending.mkdir()
    entry = {"schema_version": 1, "tweet_id": "abc", "type": "mention"}
    write_pending(entry, pending)
    out = pending / "tweet_abc.json"
    assert out.exists()
    data = json.loads(out.read_text())
    assert data["tweet_id"] == "abc"


# ── monitor_blind logic ───────────────────────────────────────────────────────

def test_monitor_blind_resets_on_nonzero(tmp_path):
    from notification_poller import update_poller_state, load_poller_state
    state_file = tmp_path / "poller_state.json"
    update_poller_state(state_file, found_count=3)
    state = load_poller_state(state_file)
    assert state["consecutive_empty_cycles"] == 0


def test_monitor_blind_increments_on_zero(tmp_path):
    from notification_poller import update_poller_state, load_poller_state
    state_file = tmp_path / "poller_state.json"
    update_poller_state(state_file, found_count=0)
    update_poller_state(state_file, found_count=0)
    state = load_poller_state(state_file)
    assert state["consecutive_empty_cycles"] == 2


def test_monitor_blind_triggers_at_3(tmp_path):
    from notification_poller import update_poller_state, should_emit_blind_alert
    state_file = tmp_path / "poller_state.json"
    for _ in range(3):
        update_poller_state(state_file, found_count=0)
    assert should_emit_blind_alert(state_file) is True


def test_monitor_blind_does_not_trigger_at_2(tmp_path):
    from notification_poller import update_poller_state, should_emit_blind_alert
    state_file = tmp_path / "poller_state.json"
    for _ in range(2):
        update_poller_state(state_file, found_count=0)
    assert should_emit_blind_alert(state_file) is False
