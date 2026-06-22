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
    capability   TEXT NOT NULL DEFAULT 'unknown',
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

CREATE TABLE IF NOT EXISTS skill_catalog (
    category   TEXT NOT NULL,
    skill      TEXT NOT NULL,
    source     TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen  TEXT NOT NULL,
    PRIMARY KEY (category, skill)
);

CREATE TABLE IF NOT EXISTS skill_assessments (
    category           TEXT NOT NULL,
    skill              TEXT NOT NULL,
    level              INTEGER NOT NULL,
    confidence         REAL NOT NULL,
    status             TEXT NOT NULL,
    evidence_json      TEXT NOT NULL,
    judge_verdict_json TEXT,
    updated_at         TEXT NOT NULL,
    PRIMARY KEY (category, skill),
    FOREIGN KEY(category, skill) REFERENCES skill_catalog(category, skill)
);

CREATE INDEX IF NOT EXISTS idx_skill_assessments_level
    ON skill_assessments(level DESC, confidence DESC);

CREATE TABLE IF NOT EXISTS raw_events (
    event_id      TEXT PRIMARY KEY,
    captured_at   TEXT NOT NULL,
    host          TEXT NOT NULL,
    method        TEXT NOT NULL,
    path          TEXT NOT NULL,
    endpoint_type TEXT NOT NULL,
    policy        TEXT NOT NULL,
    status_code   INTEGER,
    content_type  TEXT,
    request_url   TEXT NOT NULL,
    body_hash     TEXT,
    raw_json      TEXT
);

CREATE INDEX IF NOT EXISTS idx_discord_raw_events_type_time
    ON raw_events(endpoint_type, captured_at DESC);

CREATE INDEX IF NOT EXISTS idx_discord_raw_events_path_time
    ON raw_events(path, captured_at DESC);

CREATE TABLE IF NOT EXISTS discord_identities (
    user_id      INTEGER PRIMARY KEY,
    username     TEXT,
    global_name  TEXT,
    discriminator TEXT,
    bot          INTEGER NOT NULL DEFAULT 0,
    first_seen   TEXT NOT NULL,
    last_seen    TEXT NOT NULL,
    raw_json      TEXT
);

CREATE TABLE IF NOT EXISTS discord_servers (
    guild_id    INTEGER PRIMARY KEY,
    name        TEXT,
    owner_id    INTEGER,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    raw_json     TEXT
);

CREATE TABLE IF NOT EXISTS discord_channels (
    channel_id  INTEGER PRIMARY KEY,
    guild_id    INTEGER,
    parent_id   INTEGER,
    name        TEXT,
    channel_type INTEGER,
    topic       TEXT,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    raw_json     TEXT
);

CREATE INDEX IF NOT EXISTS idx_discord_channels_guild
    ON discord_channels(guild_id);

CREATE TABLE IF NOT EXISTS discord_threads (
    thread_id   INTEGER PRIMARY KEY,
    channel_id  INTEGER,
    guild_id    INTEGER,
    parent_id   INTEGER,
    name        TEXT,
    archived    INTEGER,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    raw_json     TEXT
);

CREATE INDEX IF NOT EXISTS idx_discord_threads_channel
    ON discord_threads(channel_id);

CREATE TABLE IF NOT EXISTS discord_relationships (
    user_id     INTEGER PRIMARY KEY,
    relation_type TEXT NOT NULL,
    username    TEXT,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    raw_json     TEXT
);

CREATE TABLE IF NOT EXISTS discord_message_entities (
    channel_id  INTEGER NOT NULL,
    message_id  INTEGER NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    label       TEXT,
    raw_json    TEXT,
    PRIMARY KEY (channel_id, message_id, entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_discord_message_entities_entity
    ON discord_message_entities(entity_type, entity_id);
"""


def bootstrap_db(db_path: str) -> sqlite3.Connection:
    """Open the SQLite database and ensure schema exists."""
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.executescript(SCHEMA_SQL)
    migrate_db(conn)
    conn.commit()
    logger.info("schema bootstrapped: %s", db_path)
    return conn


def migrate_db(conn: sqlite3.Connection) -> None:
    """Apply idempotent lightweight migrations for existing runtime DBs."""
    cols = {row[1] for row in conn.execute("PRAGMA table_info(opportunities)")}
    additions = {
        "platform_status": "TEXT NOT NULL DEFAULT 'unseen'",
        "platform_last_seen": "TEXT",
        "platform_title": "TEXT",
        "platform_excerpt": "TEXT",
        "capability": "TEXT NOT NULL DEFAULT 'unknown'",
        "notes": "TEXT NOT NULL DEFAULT ''",
    }
    for name, ddl in additions.items():
        if name not in cols:
            conn.execute(f"ALTER TABLE opportunities ADD COLUMN {name} {ddl}")
