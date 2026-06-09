"""SQLite schema for Telegram organ — bootstrap and migrations."""
import logging
import sqlite3

logger = logging.getLogger("telegram.schema")

SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS channels (
    channel_id   INTEGER PRIMARY KEY,
    name         TEXT,
    username     TEXT,
    channel_type TEXT,
    added_at     TEXT NOT NULL,
    notes        TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    channel_id   INTEGER NOT NULL,
    message_id   INTEGER NOT NULL,
    author_id    INTEGER,
    author_name  TEXT,
    date         TEXT NOT NULL,
    text         TEXT,
    media_type   TEXT,
    media_path   TEXT,
    reply_to     INTEGER,
    forward_from INTEGER,
    forward_msg_id INTEGER,
    block_id     TEXT,
    raw_json     TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (channel_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_messages_date ON messages(date);
CREATE INDEX IF NOT EXISTS idx_messages_block ON messages(block_id);
CREATE INDEX IF NOT EXISTS idx_messages_channel_date ON messages(channel_id, date);
"""


def bootstrap_db(db_path: str) -> sqlite3.Connection:
    """Create or open the SQLite database and ensure schema exists.

    Returns an open connection. Caller is responsible for closing it.
    """
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.executescript(SCHEMA_SQL)
    conn.commit()
    logger.info("schema bootstrapped: %s", db_path)
    return conn
