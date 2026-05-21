"""Tests for SQLite schema bootstrap and integrity."""
import sqlite3
import tempfile
from pathlib import Path

from organs.telegram.schema import bootstrap_db


def test_bootstrap_creates_tables() -> None:
    """Bootstrap on empty DB creates channels and messages tables."""
    with tempfile.TemporaryDirectory() as tmp:
        db_path = Path(tmp) / "test.db"
        conn = bootstrap_db(str(db_path))
        cursor = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )
        tables = [row[0] for row in cursor.fetchall()]
        assert "channels" in tables
        assert "messages" in tables
        conn.close()


def test_bootstrap_idempotent() -> None:
    """Calling bootstrap twice does not error."""
    with tempfile.TemporaryDirectory() as tmp:
        db_path = Path(tmp) / "test.db"
        conn1 = bootstrap_db(str(db_path))
        conn1.close()
        conn2 = bootstrap_db(str(db_path))
        conn2.close()


def test_messages_primary_key_dedup() -> None:
    """INSERT OR IGNORE on duplicate (channel_id, message_id) does not error."""
    with tempfile.TemporaryDirectory() as tmp:
        db_path = Path(tmp) / "test.db"
        conn = bootstrap_db(str(db_path))
        conn.execute(
            "INSERT INTO messages (channel_id, message_id, date, raw_json) "
            "VALUES (1, 1, '2026-05-20T12:00:00Z', '{}')"
        )
        conn.execute(
            "INSERT OR IGNORE INTO messages (channel_id, message_id, date, raw_json) "
            "VALUES (1, 1, '2026-05-20T12:00:00Z', '{}')"
        )
        cursor = conn.execute("SELECT count(*) FROM messages")
        assert cursor.fetchone()[0] == 1
        conn.close()


def test_indexes_exist() -> None:
    """Schema creates required indexes."""
    with tempfile.TemporaryDirectory() as tmp:
        db_path = Path(tmp) / "test.db"
        conn = bootstrap_db(str(db_path))
        cursor = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'"
        )
        indexes = {row[0] for row in cursor.fetchall()}
        assert "idx_messages_date" in indexes
        assert "idx_messages_block" in indexes
        assert "idx_messages_channel_date" in indexes
        conn.close()
