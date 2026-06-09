"""Tests for listener daemon — heartbeat, store, upsert, cleanup."""
import json
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch, MagicMock

from organs.telegram.listener import (
    post_heartbeat, store_block, upsert_channel, cleanup_expired,
)
from organs.telegram.schema import bootstrap_db
from organs.telegram.buffer import RawMessage, Block


def test_heartbeat_post_format() -> None:
    """Heartbeat sends correct observation format to kernel."""
    mock_resp = MagicMock()
    mock_resp.status_code = 200
    with patch("organs.telegram.listener.requests.post", return_value=mock_resp) as mock_post:
        post_heartbeat(
            kernel_url="http://localhost:3030",
            api_key="test-key",
            channels_active=8,
            messages_count=47,
            blocks_count=12,
            media_count=3,
        )
        mock_post.assert_called_once()
        call_args = mock_post.call_args
        body = json.loads(call_args.kwargs.get("data", "{}"))
        assert body["tool"] == "telegram_listener"
        assert body["domain"] == "telegram"
        assert body["agent_id"] == "hermes-telegram"
        assert body["target"] == "heartbeat"
        assert body["value"]["channels_active"] == 8
        assert body["value"]["messages_last_5min"] == 47


def test_heartbeat_failure_does_not_raise() -> None:
    """Heartbeat failure is logged, not raised."""
    with patch("organs.telegram.listener.requests.post", side_effect=ConnectionError("down")):
        post_heartbeat(
            kernel_url="http://localhost:3030",
            api_key="test-key",
            channels_active=0, messages_count=0,
            blocks_count=0, media_count=0,
        )


def test_heartbeat_skips_when_no_url() -> None:
    """Heartbeat does nothing when kernel_url is empty."""
    with patch("organs.telegram.listener.requests.post") as mock_post:
        post_heartbeat(
            kernel_url="",
            api_key="",
            channels_active=0, messages_count=0,
            blocks_count=0, media_count=0,
        )
        mock_post.assert_not_called()


def _make_block(
    channel_id: int = 1,
    author_id: int = 100,
    text: str = "BTC long",
    msg_id: int = 10,
) -> Block:
    msg = RawMessage(
        channel_id=channel_id, message_id=msg_id, author_id=author_id,
        author_name="trader", text=text,
        date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc),
        reply_to=None, media_type=None, media_path=None,
        forward_from=None, forward_msg_id=None, raw_json='{"id": 10}',
    )
    return Block(channel_id=channel_id, originator_author_id=author_id,
                 first_date=msg.date, messages=[msg])


def test_store_block_inserts_messages() -> None:
    """store_block writes messages to SQLite with correct block_id."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        block = _make_block()
        inserted = store_block(conn, block, tmp)
        assert inserted == 1
        row = conn.execute(
            "SELECT block_id, text FROM messages WHERE message_id = 10"
        ).fetchone()
        assert row[0] == block.block_id
        assert row[1] == "BTC long"
        conn.close()


def test_store_block_dedup() -> None:
    """store_block with INSERT OR IGNORE does not duplicate."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        block = _make_block()
        store_block(conn, block, tmp)
        store_block(conn, block, tmp)
        count = conn.execute("SELECT count(*) FROM messages").fetchone()[0]
        assert count == 1
        conn.close()


def test_upsert_channel_create_and_update() -> None:
    """upsert_channel creates and updates."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        upsert_channel(conn, 42, "OG Channel", "ogchannel", "channel")
        row = conn.execute(
            "SELECT name, username FROM channels WHERE channel_id = 42"
        ).fetchone()
        assert row == ("OG Channel", "ogchannel")
        upsert_channel(conn, 42, "Renamed Channel", "ogchannel", "channel")
        row = conn.execute(
            "SELECT name FROM channels WHERE channel_id = 42"
        ).fetchone()
        assert row[0] == "Renamed Channel"
        conn.close()


def test_cleanup_expired_deletes_old_messages() -> None:
    """cleanup_expired removes messages older than raw_days."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        # Insert an old message (200 days ago) and a recent one
        conn.execute(
            "INSERT INTO messages (channel_id, message_id, date, raw_json) "
            "VALUES (1, 1, datetime('now', '-200 days'), '{}')"
        )
        conn.execute(
            "INSERT INTO messages (channel_id, message_id, date, raw_json) "
            "VALUES (1, 2, datetime('now', '-1 day'), '{}')"
        )
        conn.commit()
        rows_del, _ = cleanup_expired(conn, tmp, raw_days=90, media_days=30)
        assert rows_del == 1
        remaining = conn.execute("SELECT count(*) FROM messages").fetchone()[0]
        assert remaining == 1
        conn.close()
