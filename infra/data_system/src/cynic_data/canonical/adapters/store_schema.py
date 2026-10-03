"""Canonical store schema (SQLite adapter).

Mirrors the canonical entities exactly. No denormalization, no invention:
unobserved fields are NULL. Edges carry ``source_uids`` as a JSON array so n-ary
relations survive storage.
"""

from __future__ import annotations

SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS sessions (
    session_uid   TEXT PRIMARY KEY,
    harness       TEXT NOT NULL,
    source_ref    TEXT NOT NULL,
    native_meta   TEXT
);

CREATE TABLE IF NOT EXISTS turns (
    turn_uid      TEXT PRIMARY KEY,
    session_uid   TEXT NOT NULL,
    idx           INTEGER NOT NULL,
    role          TEXT NOT NULL,
    content       TEXT,
    timestamp_iso TEXT,
    native_model  TEXT,
    native_fields TEXT
);
CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_uid);

CREATE TABLE IF NOT EXISTS tools (
    exec_uid        TEXT PRIMARY KEY,
    session_uid     TEXT NOT NULL,
    turn_uid        TEXT,
    idx             INTEGER NOT NULL,
    tool_name       TEXT NOT NULL,
    call_id         TEXT,
    command_or_args TEXT,
    raw_output      TEXT,
    exit_code       INTEGER,
    is_error        INTEGER,
    parent_exec_uid TEXT,
    timestamp_iso   TEXT,
    native_fields   TEXT
);
CREATE INDEX IF NOT EXISTS idx_tools_session ON tools(session_uid);

CREATE TABLE IF NOT EXISTS reasoning (
    trace_uid     TEXT PRIMARY KEY,
    session_uid   TEXT NOT NULL,
    turn_uid      TEXT,
    content       TEXT,
    subject       TEXT,
    model         TEXT,
    timestamp_iso TEXT
);
CREATE INDEX IF NOT EXISTS idx_reasoning_session ON reasoning(session_uid);

CREATE TABLE IF NOT EXISTS edges (
    edge_uid      TEXT PRIMARY KEY,
    session_uid   TEXT NOT NULL,
    kind          TEXT NOT NULL,
    source_uids   TEXT NOT NULL,
    target_uid    TEXT,
    provenance    TEXT NOT NULL,
    payload       TEXT,
    timestamp_iso TEXT
);
CREATE INDEX IF NOT EXISTS idx_edges_session ON edges(session_uid);
CREATE INDEX IF NOT EXISTS idx_edges_kind ON edges(kind);

CREATE TABLE IF NOT EXISTS states (
    snapshot_uid              TEXT PRIMARY KEY,
    session_uid               TEXT NOT NULL,
    turn_uid                  TEXT,
    idx                       INTEGER NOT NULL,
    model                     TEXT,
    cwd                       TEXT,
    sandbox_policy            TEXT,
    approval_policy           TEXT,
    context_prompt_tokens     INTEGER,
    context_nonmessage_tokens INTEGER,
    cumulative_tokens         INTEGER,
    compacted                 INTEGER,
    native_fields             TEXT,
    timestamp_iso             TEXT
);
CREATE INDEX IF NOT EXISTS idx_states_session ON states(session_uid);

CREATE TABLE IF NOT EXISTS human_signals (
    human_uid     TEXT PRIMARY KEY,
    session_uid   TEXT NOT NULL,
    turn_uid      TEXT NOT NULL,
    text          TEXT,
    kind          TEXT NOT NULL,
    markers       TEXT,
    timestamp_iso TEXT
);
CREATE INDEX IF NOT EXISTS idx_human_session ON human_signals(session_uid);
CREATE INDEX IF NOT EXISTS idx_human_kind ON human_signals(kind);

-- Large payloads (>1 GiB) live here, content-addressed. The base stores only
-- the hash; the blob store holds the bytes. Nothing is lost.
CREATE TABLE IF NOT EXISTS blobs (
    content_hash  TEXT PRIMARY KEY,
    size_bytes    INTEGER NOT NULL,
    created_at_iso TEXT
);
"""
