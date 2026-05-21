# Telegram Organ Phase 1: Ingestion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Python daemon that captures Telegram channel messages into local SQLite, with kernel heartbeat observations for silence detection.

**Architecture:** Single Python daemon (`telegram-listener`) using Telethon v1 for MTProto streaming. Messages buffered by author+time window, flushed to SQLite. Media downloaded as blobs. Heartbeat POST to kernel `/observe` every 5 minutes.

**Tech Stack:** Python 3.11+, Telethon v1, SQLite3 (stdlib), PyYAML, requests. Virtualenv at `~/.cynic/organs/telegram/.venv/`.

**Spec:** `docs/superpowers/specs/2026-05-20-telegram-organ-phase1-ingestion-design.md`

**Import convention:** Tests and CLI run from `cynic-python/` as working directory. Imports use `organs.telegram.*` (NOT `cynic-python.*` — the hyphen is a Python SyntaxError).

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `cynic-python/organs/telegram/__init__.py` | Package init, version |
| Create | `cynic-python/organs/telegram/schema.py` | SQLite DDL, bootstrap, migrations |
| Create | `cynic-python/organs/telegram/config.py` | Load YAML config + env vars |
| Create | `cynic-python/organs/telegram/buffer.py` | Message aggregation (author+time+reply) |
| Create | `cynic-python/organs/telegram/listener.py` | Main daemon: --listen, --backfill, --auth |
| Create | `cynic-python/organs/telegram/config.example.yaml` | Example config |
| Create | `cynic-python/organs/telegram/MANIFEST.yaml` | Organ metadata |
| Create | `cynic-python/organs/telegram/tests/__init__.py` | Test package |
| Create | `cynic-python/organs/telegram/tests/test_schema.py` | Schema tests |
| Create | `cynic-python/organs/telegram/tests/test_buffer.py` | Buffer aggregation tests |
| Create | `cynic-python/organs/telegram/tests/test_config.py` | Config loading tests |
| Create | `cynic-python/organs/telegram/tests/test_listener.py` | Heartbeat, store_block, upsert tests |
| Create | `infra/systemd/telegram-listener.service` | Systemd unit file |
| Create | `docs/architecture/organ-data-convention.md` | Organism-level data convention (reference doc) |
| Modify | `cynic-python/pyproject.toml` | Add telegram optional deps |

---

### Task 0: Branch + Dependencies

- [ ] **Step 1: Create feature branch**

```bash
cd /home/user/Bureau/CYNIC
git fetch origin
git checkout -b feat/telegram-organ-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)
```

- [ ] **Step 2: Add Telethon deps to pyproject.toml**

Add under `[project.optional-dependencies]`:

```toml
telegram = [
    "telethon>=1.36",
    "pyyaml>=6.0",
    "requests>=2.31",
]
```

- [ ] **Step 3: Create virtualenv and install**

```bash
python3 -m venv ~/.cynic/organs/telegram/.venv
~/.cynic/organs/telegram/.venv/bin/pip install -e "cynic-python[telegram]"
```

- [ ] **Step 4: Write organ data convention doc**

Create `docs/architecture/organ-data-convention.md` — extract the convention from the spec into a standalone reference doc that future organs follow. Content:
- Storage boundary (SQLite for streams, JSONL for datasets, JSON for artifacts)
- Directory convention (`~/.cynic/organs/{name}/`)
- Retention policy (configurable TTL for raw data, permanent for extractions)
- Query convention (SQLite WAL as bus, single writer, N readers)
- What this replaces (JSONL operational streams)

This is a reference doc, not implementation — it codifies the pattern the Telegram organ establishes.

- [ ] **Step 5: Commit**

```bash
git add cynic-python/pyproject.toml docs/architecture/organ-data-convention.md
git commit -m "feat(telegram): deps + organ data convention doc"
```

---

### Task 1: SQLite Schema Module

**Files:**
- Create: `cynic-python/organs/telegram/__init__.py`
- Create: `cynic-python/organs/telegram/schema.py`
- Create: `cynic-python/organs/telegram/tests/__init__.py`
- Create: `cynic-python/organs/telegram/tests/test_schema.py`

- [ ] **Step 1: Create package structure**

```bash
mkdir -p cynic-python/organs/telegram/tests
```

```python
# cynic-python/organs/telegram/__init__.py
"""
Tier 2 INFRASTRUCTURE: Telegram channel listener — raw message ingestion to SQLite.

K15 Consumer Phase 1: human channel selection gate + kernel silence detection (heartbeat).
K15 Consumer Phase 2: LLM extraction pipeline + Dogs judgment + KAIROS IC measurement.
Systemd: telegram-listener.service
Owns: ~/.cynic/organs/telegram/
"""

__version__ = "0.1.0"
```

```python
# cynic-python/organs/telegram/tests/__init__.py
```

- [ ] **Step 2: Write failing test for schema bootstrap**

```python
# cynic-python/organs/telegram/tests/test_schema.py
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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_schema.py -v`
Expected: FAIL — `ModuleNotFoundError: No module named 'organs.telegram.schema'`

- [ ] **Step 4: Implement schema module**

```python
# cynic-python/organs/telegram/schema.py
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_schema.py -v`
Expected: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add cynic-python/organs/telegram/__init__.py cynic-python/organs/telegram/schema.py cynic-python/organs/telegram/tests/__init__.py cynic-python/organs/telegram/tests/test_schema.py
git commit -m "feat(telegram): SQLite schema module with bootstrap and tests"
```

---

### Task 2: Config Module

**Files:**
- Create: `cynic-python/organs/telegram/config.py`
- Create: `cynic-python/organs/telegram/config.example.yaml`
- Create: `cynic-python/organs/telegram/tests/test_config.py`

- [ ] **Step 1: Write failing test for config loading**

```python
# cynic-python/organs/telegram/tests/test_config.py
"""Tests for config loading — YAML + env vars."""
import tempfile
from pathlib import Path
from unittest.mock import patch

from organs.telegram.config import load_config


def test_load_config_from_yaml() -> None:
    """Config loads non-secret values from YAML file."""
    with tempfile.TemporaryDirectory() as tmp:
        cfg_path = Path(tmp) / "config.yaml"
        cfg_path.write_text(
            "session_path: /tmp/test.session\n"
            "db_path: /tmp/test.db\n"
            "media_dir: /tmp/media/\n"
            "buffer_window_seconds: 30\n"
            "heartbeat_interval_seconds: 120\n"
        )
        cfg = load_config(str(cfg_path))
        assert cfg.buffer_window_seconds == 30
        assert cfg.heartbeat_interval_seconds == 120
        assert cfg.db_path == "/tmp/test.db"


def test_load_config_defaults() -> None:
    """Config uses defaults when no YAML file exists."""
    cfg = load_config("/nonexistent/path.yaml")
    assert cfg.buffer_window_seconds == 60
    assert cfg.heartbeat_interval_seconds == 300


def test_env_vars_for_secrets() -> None:
    """Secrets come from env vars, not YAML."""
    env = {
        "CYNIC_REST_ADDR": "http://localhost:3030",
        "CYNIC_API_KEY": "test-key",
        "TELEGRAM_API_ID": "12345",
        "TELEGRAM_API_HASH": "abcdef",
    }
    with patch.dict("os.environ", env, clear=False):
        cfg = load_config("/nonexistent/path.yaml")
        assert cfg.kernel_url == "http://localhost:3030"
        assert cfg.api_key == "test-key"
        assert cfg.telegram_api_id == 12345
        assert cfg.telegram_api_hash == "abcdef"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_config.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement config module**

```python
# cynic-python/organs/telegram/config.py
"""Configuration loader — YAML for tuning, env vars for secrets."""
import logging
import os
from dataclasses import dataclass, field
from pathlib import Path

import yaml

logger = logging.getLogger("telegram.config")

_DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "telegram"


@dataclass
class RetentionConfig:
    """Retention policy — organism-level convention."""

    raw_days: int = 90
    media_days: int = 30


@dataclass
class TelegramConfig:
    """Telegram organ configuration."""

    # From YAML (non-secret, tunable)
    session_path: str = str(_DEFAULT_ORGAN_DIR / "session.session")
    db_path: str = str(_DEFAULT_ORGAN_DIR / "messages.db")
    media_dir: str = str(_DEFAULT_ORGAN_DIR / "media")
    buffer_window_seconds: int = 60
    heartbeat_interval_seconds: int = 300
    retention: RetentionConfig = field(default_factory=RetentionConfig)

    # From env vars (secrets)
    kernel_url: str = ""
    api_key: str = ""
    telegram_api_id: int = 0
    telegram_api_hash: str = ""


def load_config(yaml_path: str) -> TelegramConfig:
    """Load config from YAML file (if exists) + env vars for secrets."""
    cfg = TelegramConfig()

    p = Path(yaml_path)
    if p.exists():
        with p.open() as f:
            data = yaml.safe_load(f) or {}
        for key in ("session_path", "db_path", "media_dir",
                     "buffer_window_seconds", "heartbeat_interval_seconds"):
            if key in data:
                setattr(cfg, key, data[key])
        if "retention" in data and isinstance(data["retention"], dict):
            cfg.retention = RetentionConfig(
                raw_days=data["retention"].get("raw_days", 90),
                media_days=data["retention"].get("media_days", 30),
            )
        logger.info("config loaded from %s", yaml_path)
    else:
        logger.info("no config file at %s, using defaults", yaml_path)

    cfg.kernel_url = os.environ.get("CYNIC_REST_ADDR", "")
    cfg.api_key = os.environ.get("CYNIC_API_KEY", "")
    api_id = os.environ.get("TELEGRAM_API_ID", "0")
    cfg.telegram_api_id = int(api_id) if api_id.isdigit() else 0
    cfg.telegram_api_hash = os.environ.get("TELEGRAM_API_HASH", "")

    return cfg
```

- [ ] **Step 4: Create example config**

```yaml
# cynic-python/organs/telegram/config.example.yaml
# Telegram organ config — non-secret, tunable values only.
# Secrets (API keys, tokens) come from env vars in ~/.config/cynic/env.
#
# Copy to: ~/.cynic/organs/telegram/config.yaml

session_path: ~/.cynic/organs/telegram/session.session
db_path: ~/.cynic/organs/telegram/messages.db
media_dir: ~/.cynic/organs/telegram/media/
buffer_window_seconds: 60
heartbeat_interval_seconds: 300
retention:
  raw_days: 90
  media_days: 30
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_config.py -v`
Expected: 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add cynic-python/organs/telegram/config.py cynic-python/organs/telegram/config.example.yaml cynic-python/organs/telegram/tests/test_config.py
git commit -m "feat(telegram): config module — YAML for tuning, env for secrets"
```

---

### Task 3: Message Buffer / Aggregation

**Files:**
- Create: `cynic-python/organs/telegram/buffer.py`
- Create: `cynic-python/organs/telegram/tests/test_buffer.py`

- [ ] **Step 1: Write failing tests for buffer aggregation**

```python
# cynic-python/organs/telegram/tests/test_buffer.py
"""Tests for message buffer — aggregation by author+time+reply."""
from datetime import datetime, timezone
from typing import Optional

from organs.telegram.buffer import MessageBuffer, RawMessage


def _msg(
    channel_id: int = 1,
    message_id: int = 1,
    author_id: int = 100,
    author_name: str = "trader",
    text: str = "hello",
    date: Optional[datetime] = None,
    reply_to: Optional[int] = None,
) -> RawMessage:
    """Helper to build a RawMessage."""
    return RawMessage(
        channel_id=channel_id,
        message_id=message_id,
        author_id=author_id,
        author_name=author_name,
        text=text,
        date=date or datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc),
        reply_to=reply_to,
        media_type=None,
        media_path=None,
        forward_from=None,
        forward_msg_id=None,
        raw_json="{}",
    )


def test_single_message_flush() -> None:
    """A single message flushes after the window expires."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 3, tzinfo=timezone.utc))
    assert len(blocks) == 0
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 6, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 1


def test_same_author_aggregates() -> None:
    """Messages from same author+channel within window form one block."""
    buf = MessageBuffer(window_seconds=60)
    buf.add(_msg(message_id=1, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, date=datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=3, date=datetime(2026, 5, 20, 12, 0, 20, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 1, 30, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 3


def test_different_authors_separate_blocks() -> None:
    """Messages from different authors in same channel are separate blocks."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(message_id=1, author_id=100, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, author_id=200, date=datetime(2026, 5, 20, 12, 0, 1, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert len(blocks) == 2


def test_reply_merges_into_parent_block() -> None:
    """A reply from author B merges into author A's block."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(message_id=1, author_id=100, date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    buf.add(_msg(message_id=2, author_id=200, reply_to=1, date=datetime(2026, 5, 20, 12, 0, 2, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert len(blocks) == 1
    assert len(blocks[0].messages) == 2


def test_block_id_format() -> None:
    """block_id = {channel_id}_{author_id}_{first_msg_iso8601}."""
    buf = MessageBuffer(window_seconds=5)
    buf.add(_msg(channel_id=42, author_id=100, message_id=1,
                 date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_ready(datetime(2026, 5, 20, 12, 0, 10, tzinfo=timezone.utc))
    assert blocks[0].block_id == "42_100_2026-05-20T12:00:00+00:00"


def test_flush_all_drains_buffer() -> None:
    """flush_all returns all blocks regardless of window expiry."""
    buf = MessageBuffer(window_seconds=60)
    buf.add(_msg(date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc)))
    blocks = buf.flush_all()
    assert len(blocks) == 1
    assert len(buf.flush_all()) == 0
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_buffer.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement buffer module**

```python
# cynic-python/organs/telegram/buffer.py
"""Message buffer — aggregation by author + time window + reply chain."""
import logging
from dataclasses import dataclass, field
from datetime import datetime
from typing import Optional

logger = logging.getLogger("telegram.buffer")


@dataclass
class RawMessage:
    """A single Telegram message, ready for buffering."""

    channel_id: int
    message_id: int
    author_id: Optional[int]
    author_name: Optional[str]
    text: Optional[str]
    date: datetime
    reply_to: Optional[int]
    media_type: Optional[str]
    media_path: Optional[str]
    forward_from: Optional[int]
    forward_msg_id: Optional[int]
    raw_json: str


@dataclass
class Block:
    """A logical group of messages sharing a block_id."""

    channel_id: int
    originator_author_id: Optional[int]
    first_date: datetime
    messages: list[RawMessage] = field(default_factory=list)

    @property
    def block_id(self) -> str:
        return f"{self.channel_id}_{self.originator_author_id}_{self.first_date.isoformat()}"

    @property
    def last_date(self) -> datetime:
        return max(m.date for m in self.messages)


_BufKey = tuple[int, Optional[int]]


class MessageBuffer:
    """Aggregates messages into blocks by author+time window+reply chain.

    Flush condition: no new message from same (channel, author) for
    window_seconds. Reply messages merge into the parent block
    regardless of author or time window.
    """

    def __init__(self, window_seconds: int = 60) -> None:
        self.window_seconds = window_seconds
        self._blocks: dict[_BufKey, Block] = {}
        self._msg_to_key: dict[tuple[int, int], _BufKey] = {}

    def add(self, msg: RawMessage) -> None:
        """Add a message to the buffer."""
        if msg.reply_to is not None:
            parent_lookup = (msg.channel_id, msg.reply_to)
            if parent_lookup in self._msg_to_key:
                key = self._msg_to_key[parent_lookup]
                self._blocks[key].messages.append(msg)
                self._msg_to_key[(msg.channel_id, msg.message_id)] = key
                return

        key: _BufKey = (msg.channel_id, msg.author_id)
        if key not in self._blocks:
            self._blocks[key] = Block(
                channel_id=msg.channel_id,
                originator_author_id=msg.author_id,
                first_date=msg.date,
            )
        self._blocks[key].messages.append(msg)
        self._msg_to_key[(msg.channel_id, msg.message_id)] = key

    def flush_ready(self, now: datetime) -> list[Block]:
        """Return blocks where the last message is older than window_seconds."""
        ready: list[Block] = []
        to_remove: list[_BufKey] = []
        for key, block in self._blocks.items():
            elapsed = (now - block.last_date).total_seconds()
            if elapsed >= self.window_seconds:
                ready.append(block)
                to_remove.append(key)
        for key in to_remove:
            del self._blocks[key]
            self._msg_to_key = {
                k: v for k, v in self._msg_to_key.items() if v != key
            }
        return ready

    def flush_all(self) -> list[Block]:
        """Flush all blocks regardless of window expiry (for shutdown)."""
        blocks = list(self._blocks.values())
        self._blocks.clear()
        self._msg_to_key.clear()
        return blocks
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_buffer.py -v`
Expected: 6 tests PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/telegram/buffer.py cynic-python/organs/telegram/tests/test_buffer.py
git commit -m "feat(telegram): message buffer with author+time+reply aggregation"
```

---

### Task 4: Listener Daemon — Core Loop

**Files:**
- Create: `cynic-python/organs/telegram/listener.py`
- Create: `cynic-python/organs/telegram/tests/test_listener.py`

Telethon requires a live connection, so tests mock the client. Real integration test is manual (Task 6).

- [ ] **Step 1: Write failing tests**

```python
# cynic-python/organs/telegram/tests/test_listener.py
"""Tests for listener daemon — heartbeat, store, upsert."""
import json
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch, MagicMock

from organs.telegram.listener import post_heartbeat, store_block, upsert_channel
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


def test_store_block_inserts_messages() -> None:
    """store_block writes messages to SQLite with correct block_id."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        msg = RawMessage(
            channel_id=1, message_id=10, author_id=100, author_name="trader",
            text="BTC long", date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc),
            reply_to=None, media_type=None, media_path=None,
            forward_from=None, forward_msg_id=None, raw_json='{"id": 10}',
        )
        block = Block(channel_id=1, originator_author_id=100,
                       first_date=msg.date, messages=[msg])
        inserted = store_block(conn, block, tmp)
        assert inserted == 1
        row = conn.execute("SELECT block_id, text FROM messages WHERE message_id = 10").fetchone()
        assert row[0] == block.block_id
        assert row[1] == "BTC long"
        conn.close()


def test_store_block_dedup() -> None:
    """store_block with INSERT OR IGNORE does not duplicate."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        msg = RawMessage(
            channel_id=1, message_id=10, author_id=100, author_name="trader",
            text="BTC long", date=datetime(2026, 5, 20, 12, 0, 0, tzinfo=timezone.utc),
            reply_to=None, media_type=None, media_path=None,
            forward_from=None, forward_msg_id=None, raw_json='{}',
        )
        block = Block(channel_id=1, originator_author_id=100,
                       first_date=msg.date, messages=[msg])
        store_block(conn, block, tmp)
        store_block(conn, block, tmp)
        count = conn.execute("SELECT count(*) FROM messages").fetchone()[0]
        assert count == 1
        conn.close()


def test_upsert_channel() -> None:
    """upsert_channel creates and updates."""
    with tempfile.TemporaryDirectory() as tmp:
        conn = bootstrap_db(str(Path(tmp) / "test.db"))
        upsert_channel(conn, 42, "OG Channel", "ogchannel", "channel")
        row = conn.execute("SELECT name, username FROM channels WHERE channel_id = 42").fetchone()
        assert row == ("OG Channel", "ogchannel")
        upsert_channel(conn, 42, "Renamed Channel", "ogchannel", "channel")
        row = conn.execute("SELECT name FROM channels WHERE channel_id = 42").fetchone()
        assert row[0] == "Renamed Channel"
        conn.close()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_listener.py -v`
Expected: FAIL — `ModuleNotFoundError`

- [ ] **Step 3: Implement listener module**

```python
# cynic-python/organs/telegram/listener.py
"""
Telegram Listener Daemon — captures channel messages to SQLite.

Modes:
    --listen    Real-time daemon (systemd)
    --backfill  One-shot historical export
    --auth      Interactive Telethon authentication

Environment:
    CYNIC_REST_ADDR, CYNIC_API_KEY — kernel connection
    TELEGRAM_API_ID, TELEGRAM_API_HASH — Telegram API credentials
"""
from __future__ import annotations

import argparse
import asyncio
import json
import logging
import signal
import sqlite3
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional, TYPE_CHECKING

import requests

from . import __version__
from .config import load_config, TelegramConfig
from .schema import bootstrap_db
from .buffer import MessageBuffer, RawMessage, Block

if TYPE_CHECKING:
    from telethon import TelegramClient

logger = logging.getLogger("telegram.listener")

_DEFAULT_CONFIG = Path.home() / ".cynic" / "organs" / "telegram" / "config.yaml"

# ── Heartbeat ──


def post_heartbeat(
    kernel_url: str,
    api_key: str,
    channels_active: int,
    messages_count: int,
    blocks_count: int,
    media_count: int,
) -> None:
    """POST heartbeat observation to kernel. Fire-and-forget."""
    if not kernel_url:
        return
    body = {
        "tool": "telegram_listener",
        "target": "heartbeat",
        "domain": "telegram",
        "agent_id": "hermes-telegram",
        "value": {
            "channels_active": channels_active,
            "messages_last_5min": messages_count,
            "blocks_last_5min": blocks_count,
            "media_downloaded": media_count,
        },
        "context": (
            f"{channels_active} channels active, {messages_count} messages, "
            f"{blocks_count} blocks, {media_count} media in last 5min"
        ),
        "tags": ["heartbeat", "telegram"],
    }
    try:
        resp = requests.post(
            f"{kernel_url}/observe",
            headers={"Authorization": f"Bearer {api_key}",
                     "Content-Type": "application/json"},
            data=json.dumps(body),
            timeout=10,
        )
        if resp.status_code != 200:
            logger.warning("heartbeat POST returned %d", resp.status_code)
    except Exception:
        logger.warning("heartbeat POST failed", exc_info=True)


# ── SQLite persistence ──


def store_block(conn: sqlite3.Connection, block: Block, media_dir: str) -> int:
    """Write a block's messages to SQLite. Returns count of rows inserted."""
    inserted = 0
    for msg in block.messages:
        try:
            conn.execute(
                """INSERT OR IGNORE INTO messages
                   (channel_id, message_id, author_id, author_name, date,
                    text, media_type, media_path, reply_to, forward_from,
                    forward_msg_id, block_id, raw_json)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    msg.channel_id, msg.message_id, msg.author_id,
                    msg.author_name, msg.date.isoformat(), msg.text,
                    msg.media_type, msg.media_path,
                    msg.reply_to, msg.forward_from, msg.forward_msg_id,
                    block.block_id, msg.raw_json,
                ),
            )
            inserted += 1
        except Exception:
            logger.error("failed to insert msg %d/%d",
                         msg.channel_id, msg.message_id, exc_info=True)
    conn.commit()
    return inserted


def upsert_channel(
    conn: sqlite3.Connection,
    channel_id: int,
    name: str,
    username: Optional[str],
    channel_type: str,
) -> None:
    """Insert or update a channel in the channels table."""
    conn.execute(
        """INSERT INTO channels (channel_id, name, username, channel_type, added_at)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(channel_id) DO UPDATE SET
             name=excluded.name, username=excluded.username""",
        (channel_id, name, username, channel_type,
         datetime.now(timezone.utc).isoformat()),
    )
    conn.commit()


# ── Retention cleanup ──


def cleanup_expired(
    conn: sqlite3.Connection,
    media_dir: str,
    raw_days: int,
    media_days: int,
) -> tuple[int, int]:
    """Purge expired raw messages and media. Returns (rows_deleted, files_deleted)."""
    cursor = conn.execute(
        "DELETE FROM messages WHERE date < datetime('now', ?)",
        (f"-{raw_days} days",),
    )
    rows_deleted = cursor.rowcount

    files_deleted = 0
    if media_days > 0:
        cutoff = datetime.now(timezone.utc).isoformat()
        old_media = conn.execute(
            "SELECT media_path FROM messages WHERE media_path IS NOT NULL "
            "AND date < datetime('now', ?)",
            (f"-{media_days} days",),
        ).fetchall()
        for (mpath,) in old_media:
            full = Path(media_dir) / mpath
            if full.exists():
                full.unlink()
                files_deleted += 1
        conn.execute(
            "UPDATE messages SET media_path = NULL, media_type = NULL "
            "WHERE date < datetime('now', ?)",
            (f"-{media_days} days",),
        )

    conn.commit()
    if rows_deleted or files_deleted:
        logger.info("retention cleanup: %d rows, %d media files purged",
                     rows_deleted, files_deleted)
    return rows_deleted, files_deleted


# ── Media download ──


async def download_media(
    client: Any,  # TelegramClient — TYPE_CHECKING only
    message: Any,  # telethon.types.Message
    media_dir: str,
) -> Optional[str]:
    """Download message media to local filesystem. Returns relative path or None."""
    if message.media is None:
        return None
    try:
        channel_dir = Path(media_dir) / str(message.chat_id)
        channel_dir.mkdir(parents=True, exist_ok=True)
        path = await client.download_media(
            message, file=str(channel_dir / str(message.id))
        )
        if path:
            return str(Path(path).relative_to(media_dir))
    except Exception:
        logger.warning("media download failed for %d/%d",
                       message.chat_id, message.id, exc_info=True)
    return None


# ── Telethon → RawMessage ──


def telethon_to_raw(message: Any) -> RawMessage:
    """Convert a Telethon Message object to RawMessage."""
    author_id: Optional[int] = None
    author_name: Optional[str] = None
    if message.sender:
        author_id = message.sender_id
        author_name = (getattr(message.sender, "first_name", None)
                       or getattr(message.sender, "title", None))

    forward_from: Optional[int] = None
    forward_msg_id: Optional[int] = None
    if message.forward:
        if message.forward.chat_id:
            forward_from = message.forward.chat_id
        if message.forward.channel_post:
            forward_msg_id = message.forward.channel_post

    media_type: Optional[str] = None
    if message.photo:
        media_type = "photo"
    elif message.document:
        media_type = "document"
    elif message.video:
        media_type = "video"

    return RawMessage(
        channel_id=message.chat_id,
        message_id=message.id,
        author_id=author_id,
        author_name=author_name,
        text=message.text or message.raw_text,
        date=message.date,
        reply_to=message.reply_to_msg_id if message.reply_to else None,
        media_type=media_type,
        media_path=None,  # Set after download_media
        forward_from=forward_from,
        forward_msg_id=forward_msg_id,
        raw_json=message.to_json(),
    )


# ── Main daemon loop ──


async def listen_loop(cfg: TelegramConfig) -> None:
    """Main listen loop — connect, buffer, flush to SQLite."""
    from telethon import TelegramClient, events
    from telethon.errors import FloodWaitError

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()
    logger.info("telegram-listener v%s connected", __version__)

    conn = bootstrap_db(cfg.db_path)
    buffer = MessageBuffer(window_seconds=cfg.buffer_window_seconds)
    Path(cfg.media_dir).mkdir(parents=True, exist_ok=True)

    # Register all joined channels
    async for dialog in client.iter_dialogs():
        if dialog.is_channel or dialog.is_group:
            ch_type = "channel" if dialog.is_channel else "group"
            upsert_channel(
                conn, dialog.id, dialog.name,
                getattr(dialog.entity, "username", None), ch_type,
            )
    logger.info("channels registered")

    stats = {"messages": 0, "blocks": 0, "media": 0}
    last_heartbeat = time.monotonic()

    @client.on(events.NewMessage)
    async def on_new_message(event: events.NewMessage.Event) -> None:
        msg = event.message
        raw = telethon_to_raw(msg)

        if raw.media_type:
            try:
                media_path = await download_media(client, msg, cfg.media_dir)
                raw.media_path = media_path
                if media_path:
                    stats["media"] += 1
            except FloodWaitError as e:
                logger.warning("FloodWait on media download: %ds", e.seconds)
                await asyncio.sleep(e.seconds)

        buffer.add(raw)
        stats["messages"] += 1

        if msg.chat:
            ch_type = "channel" if getattr(msg.chat, "broadcast", False) else "group"
            upsert_channel(
                conn, msg.chat_id,
                getattr(msg.chat, "title", ""),
                getattr(msg.chat, "username", None), ch_type,
            )

    async def periodic_flush() -> None:
        nonlocal last_heartbeat
        while True:
            await asyncio.sleep(5)
            now = datetime.now(timezone.utc)
            blocks = buffer.flush_ready(now)
            for block in blocks:
                store_block(conn, block, cfg.media_dir)
                stats["blocks"] += 1

            elapsed = time.monotonic() - last_heartbeat
            if elapsed >= cfg.heartbeat_interval_seconds:
                dialogs_count = 0
                try:
                    async for d in client.iter_dialogs():
                        if d.is_channel or d.is_group:
                            dialogs_count += 1
                except FloodWaitError as e:
                    logger.warning("FloodWait on iter_dialogs: %ds", e.seconds)
                    await asyncio.sleep(e.seconds)

                post_heartbeat(
                    kernel_url=cfg.kernel_url,
                    api_key=cfg.api_key,
                    channels_active=dialogs_count,
                    messages_count=stats["messages"],
                    blocks_count=stats["blocks"],
                    media_count=stats["media"],
                )
                stats["messages"] = 0
                stats["blocks"] = 0
                stats["media"] = 0
                last_heartbeat = time.monotonic()

                # Retention cleanup (once per heartbeat cycle)
                cleanup_expired(
                    conn, cfg.media_dir,
                    cfg.retention.raw_days, cfg.retention.media_days,
                )

    # Graceful shutdown
    loop = asyncio.get_running_loop()
    shutdown_event = asyncio.Event()

    def handle_signal() -> None:
        logger.info("shutdown signal received, flushing buffers...")
        shutdown_event.set()

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, handle_signal)

    flush_task = asyncio.create_task(periodic_flush())

    await shutdown_event.wait()
    flush_task.cancel()

    for block in buffer.flush_all():
        store_block(conn, block, cfg.media_dir)
    conn.close()
    await client.disconnect()
    logger.info("telegram-listener stopped cleanly")


# ── Backfill ──


async def backfill(cfg: TelegramConfig, channel: str, limit: int) -> None:
    """One-shot historical export from a channel."""
    from telethon import TelegramClient
    from telethon.errors import FloodWaitError

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()

    conn = bootstrap_db(cfg.db_path)
    Path(cfg.media_dir).mkdir(parents=True, exist_ok=True)

    entity = await client.get_entity(channel)
    ch_type = "channel" if getattr(entity, "broadcast", False) else "group"
    upsert_channel(
        conn, entity.id,
        getattr(entity, "title", channel),
        getattr(entity, "username", None), ch_type,
    )

    count = 0
    buf = MessageBuffer(window_seconds=cfg.buffer_window_seconds)
    try:
        async for message in client.iter_messages(entity, limit=limit):
            raw = telethon_to_raw(message)
            buf.add(raw)
            count += 1
            if count % 100 == 0:
                logger.info("backfill: %d messages processed", count)
    except FloodWaitError as e:
        logger.warning("FloodWait during backfill: %ds, flushing %d msgs", e.seconds, count)

    for block in buf.flush_all():
        store_block(conn, block, cfg.media_dir)

    conn.close()
    await client.disconnect()
    logger.info("backfill complete: %d messages from %s", count, channel)


# ── Auth ──


async def auth(cfg: TelegramConfig) -> None:
    """Interactive Telethon authentication."""
    from telethon import TelegramClient

    client = TelegramClient(
        cfg.session_path, cfg.telegram_api_id, cfg.telegram_api_hash
    )
    await client.start()
    me = await client.get_me()
    logger.info("authenticated as %s (id=%d)", me.first_name, me.id)
    await client.disconnect()


# ── CLI ──


def main() -> None:
    """CLI entry point."""
    logging.basicConfig(
        level=logging.INFO,
        format='{"time":"%(asctime)s","name":"%(name)s","level":"%(levelname)s","msg":"%(message)s"}',
    )
    logger.info("telegram-listener v%s starting", __version__)

    parser = argparse.ArgumentParser(description="CYNIC Telegram Listener")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--listen", action="store_true", help="Real-time daemon (default)")
    group.add_argument("--backfill", type=str, metavar="CHANNEL", help="Backfill channel history")
    group.add_argument("--auth", action="store_true", help="Interactive Telethon auth")
    parser.add_argument("--limit", type=int, default=5000, help="Max messages for backfill")
    parser.add_argument("--config", type=str, default=str(_DEFAULT_CONFIG), help="Config file path")
    args = parser.parse_args()

    cfg = load_config(args.config)

    if args.auth:
        asyncio.run(auth(cfg))
    elif args.backfill:
        asyncio.run(backfill(cfg, args.backfill, args.limit))
    else:
        asyncio.run(listen_loop(cfg))


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd cynic-python && python -m pytest organs/telegram/tests/test_listener.py -v`
Expected: 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add cynic-python/organs/telegram/listener.py cynic-python/organs/telegram/tests/test_listener.py
git commit -m "feat(telegram): listener daemon with FloodWait handling and media persistence"
```

---

### Task 5: MANIFEST + Systemd Service

**Files:**
- Create: `cynic-python/organs/telegram/MANIFEST.yaml`
- Create: `infra/systemd/telegram-listener.service`

- [ ] **Step 1: Create MANIFEST.yaml**

```yaml
# cynic-python/organs/telegram/MANIFEST.yaml
id: telegram-organ
tier: 2
domain: social
status: active
description: "Telegram channel listener — raw message ingestion to SQLite"
inputs:
  - source: telegram_api
    type: mtproto_stream
outputs:
  - target: sqlite_local
    path: ~/.cynic/organs/telegram/messages.db
  - target: kernel_observe
    domain: telegram
    agent_id: hermes-telegram
    type: heartbeat
k15_consumers:
  phase1:
    - human_channel_selection_gate
    - kernel_silence_detection
  phase2:
    - llm_extraction_pipeline
    - dogs_judgment
    - kairos_ic_measurement
crons: []
scripts:
  - name: listener.py
    description: "Main daemon — modes: --listen, --backfill <channel>, --auth"
```

- [ ] **Step 2: Create systemd service**

```ini
# infra/systemd/telegram-listener.service
[Unit]
Description=CYNIC Telegram Listener
After=network-online.target

[Service]
Type=simple
WorkingDirectory=%h/Bureau/CYNIC/cynic-python
ExecStart=%h/.cynic/organs/telegram/.venv/bin/python -m organs.telegram.listener
Restart=on-failure
RestartSec=30
StartLimitIntervalSec=600
StartLimitBurst=3
Environment=PYTHONUNBUFFERED=1
EnvironmentFile=%h/.config/cynic/env

[Install]
WantedBy=default.target
```

- [ ] **Step 3: Verify systemd service has no R1/SYS1 violations**

Run: `grep -n 'User=\|Group=\|/home/user' infra/systemd/telegram-listener.service`
Expected: zero matches

- [ ] **Step 4: Commit**

```bash
git add cynic-python/organs/telegram/MANIFEST.yaml infra/systemd/telegram-listener.service
git commit -m "feat(telegram): MANIFEST + systemd service definition"
```

---

### Task 6: First Run Verification

Manual task — requires interactive Telegram auth. Run on cynic-core.

- [ ] **Step 1: Add Telegram credentials to env**

```bash
# Add to ~/.config/cynic/env (on cynic-core):
# TELEGRAM_API_ID=<from https://my.telegram.org>
# TELEGRAM_API_HASH=<from https://my.telegram.org>
```

- [ ] **Step 2: Create organ directory + install**

```bash
mkdir -p ~/.cynic/organs/telegram/media
cp cynic-python/organs/telegram/config.example.yaml ~/.cynic/organs/telegram/config.yaml
python3 -m venv ~/.cynic/organs/telegram/.venv
~/.cynic/organs/telegram/.venv/bin/pip install -e "$(git rev-parse --show-toplevel)/cynic-python[telegram]"
```

- [ ] **Step 3: Authenticate (interactive)**

```bash
cd "$(git rev-parse --show-toplevel)/cynic-python"
~/.cynic/organs/telegram/.venv/bin/python -m organs.telegram.listener --auth
# Enter phone number, Telegram code, optional 2FA
# Should print: "authenticated as <name> (id=<id>)"
```

- [ ] **Step 4: Backfill one target channel**

```bash
~/.cynic/organs/telegram/.venv/bin/python -m organs.telegram.listener --backfill @<target_channel> --limit 500
# Should print: "backfill complete: N messages from @<target_channel>"
```

- [ ] **Step 5: Verify SQLite data**

```bash
sqlite3 ~/.cynic/organs/telegram/messages.db "SELECT count(*) FROM messages"
# Should show > 0

sqlite3 ~/.cynic/organs/telegram/messages.db "SELECT name, username FROM channels"
# Should show the backfilled channel

sqlite3 ~/.cynic/organs/telegram/messages.db \
  "SELECT date, author_name, substr(text,1,60) FROM messages ORDER BY date DESC LIMIT 5"
# Should show real messages
```

- [ ] **Step 6: Start daemon and verify heartbeat**

```bash
cd "$(git rev-parse --show-toplevel)/cynic-python"
~/.cynic/organs/telegram/.venv/bin/python -m organs.telegram.listener --listen
# Wait 5 minutes for first heartbeat, then check:
curl -s ${CYNIC_REST_ADDR}/observations?domain=telegram&limit=1 \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | python3 -m json.tool
# Should show heartbeat observation with agent_id="hermes-telegram"
```

- [ ] **Step 7: Deploy systemd service**

```bash
cp infra/systemd/telegram-listener.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now telegram-listener
systemctl --user status telegram-listener
journalctl --user -xeu telegram-listener | head -20
```

- [ ] **Step 8: Commit any adjustments from first run**

```bash
git status --short
# If changes, stage specific files (NOT git add -A) and commit
git commit -m "fix(telegram): adjustments from first-run verification"
```

---

### Task 7: Run All Tests + Final Verification

- [ ] **Step 1: Run full test suite**

```bash
cd "$(git rev-parse --show-toplevel)/cynic-python"
~/.cynic/organs/telegram/.venv/bin/python -m pytest organs/telegram/tests/ -v --tb=short
```

Expected: all tests PASS (schema: 4, config: 3, buffer: 6, listener: 5 = 18 tests)

- [ ] **Step 2: Verify no import issues**

```bash
cd "$(git rev-parse --show-toplevel)/cynic-python"
~/.cynic/organs/telegram/.venv/bin/python -c "from organs.telegram import __version__; print(__version__)"
# Should print: 0.1.0
```

- [ ] **Step 3: Verify no hardcoded paths**

```bash
grep -rn '/home/user' cynic-python/organs/telegram/
# Should return zero matches
```

- [ ] **Step 4: Final commit if any changes remain**

```bash
git status --short
# If clean, skip. If changes, commit.
```

---

## Falsification Criteria

| Criterion | How to verify | Target |
|-----------|---------------|--------|
| Daemon runs >48h without crash | `systemctl --user status telegram-listener` after 48h | Active (running) |
| Backfill exports >1000 messages | `sqlite3 messages.db "SELECT count(*) FROM messages"` | > 1000 |
| SQLite is queryable | Run the workflow queries from spec | Non-empty results |
| Heartbeat in kernel | `curl /observations?domain=telegram&limit=1` | Non-empty |
| Buffer aggregation correct | Manual inspection of `block_id` groups | Messages that belong together share block_id |
| No hardcoded paths | `grep -rn '/home/user' cynic-python/organs/telegram/` | Zero matches |
| FloodWait handled | Trigger rate limit during backfill; daemon should not crash | Log + resume |
| Media path persisted | `SELECT media_path FROM messages WHERE media_type IS NOT NULL LIMIT 5` | Non-null for downloaded media |
