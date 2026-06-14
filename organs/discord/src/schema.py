"""SQLite schema for organ-discord."""
from __future__ import annotations

import logging
import sqlite3

logger = logging.getLogger("discord.schema")

SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS guilds (
    guild_id   INTEGER PRIMARY KEY,
    name       TEXT,
    added_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS channels (
    channel_id   INTEGER PRIMARY KEY,
    guild_id     INTEGER,
    name         TEXT,
    channel_type TEXT,
    added_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
    guild_id     INTEGER,
    channel_id   INTEGER NOT NULL,
    message_id   INTEGER NOT NULL,
    author_id    INTEGER,
    author_name  TEXT,
    date         TEXT NOT NULL,
    text         TEXT,
    reply_to     INTEGER,
    raw_json     TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (channel_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_discord_messages_date ON messages(date);
CREATE INDEX IF NOT EXISTS idx_discord_messages_channel_date
    ON messages(channel_id, date);
CREATE INDEX IF NOT EXISTS idx_discord_messages_guild_date
    ON messages(guild_id, date);

CREATE TABLE IF NOT EXISTS pipeline_processed (
    message_key  TEXT PRIMARY KEY,
    processed_at TEXT NOT NULL,
    signal_type  TEXT NOT NULL,
    domain       TEXT NOT NULL,
    sent         INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS opportunities (
    url          TEXT PRIMARY KEY,
    title        TEXT NOT NULL,
    category     TEXT,
    reward       TEXT,
    reward_usd   INTEGER,
    source       TEXT NOT NULL,
    fit_score    INTEGER NOT NULL,
    status       TEXT NOT NULL DEFAULT 'new',
    platform_status TEXT NOT NULL DEFAULT 'unseen',
    platform_last_seen TEXT,
    platform_title TEXT,
    platform_excerpt TEXT,
    first_seen   TEXT NOT NULL,
    last_seen    TEXT NOT NULL,
    raw_text     TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_opportunities_status_fit
    ON opportunities(status, fit_score DESC);

CREATE TABLE IF NOT EXISTS opportunity_snapshots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    url         TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    status_code INTEGER,
    title       TEXT,
    excerpt     TEXT,
    raw_path    TEXT,
    FOREIGN KEY(url) REFERENCES opportunities(url)
);

CREATE INDEX IF NOT EXISTS idx_opportunity_snapshots_url_time
    ON opportunity_snapshots(url, captured_at DESC);
"""


def bootstrap_db(db_path: str) -> sqlite3.Connection:
    """Open the SQLite database and ensure schema exists."""
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.executescript(SCHEMA_SQL)
    conn.commit()
    logger.info("schema bootstrapped: %s", db_path)
    return conn
