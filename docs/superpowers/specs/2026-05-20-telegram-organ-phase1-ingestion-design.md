# Telegram Organ — Phase 1: Ingestion

> Capture raw Telegram messages from crypto trader channels into a local SQLite store. No extraction, no judgment — observe first.

**Date:** 2026-05-20
**Status:** DESIGN
**Tier:** 2 INFRASTRUCTURE
**Scope:** R8 Social Signal Ingestion (CYNIC side)
**Consumer Phase 1:** Human (channel selection gate — decides which channels feed Phase 2) + kernel (silence detection via heartbeat)
**Consumer Phase 2:** LLM extraction pipeline + Dogs judgment + KAIROS IC measurement

---

## Context

KAIROS (the quant desk) needs social signal data to measure Information Coefficient (IC) per source x coin x horizon. CYNIC is the producer — it captures Telegram channels, extracts structured signals, and emits K15-compliant observations.

Phase 1 builds the ingestion layer only. The extraction/analysis pipeline (6-layer framework: facts, analysis, calls, sentiment, track record, mental models) comes after observing the real signal shape.

### Decisions locked

| Decision | Choice | Why |
|----------|--------|-----|
| Text or multimodal | Text first, charts stored as blobs | No public benchmark for vision LLM on ICT/SMC charts. Research needed. |
| Extraction architecture | Hybrid (Python extracts, Dogs judge) | Extraction != judgment. Phase 2. |
| Telegram client | Telethon v1 (Codeberg) | MTProto native, sovereign, sessions local. Maintenance mode acceptable. |
| Account | Personal (existing) | Dedicated account if ban risk materializes. |
| Message aggregation | Time window (60s) + reply chain | Same author + channel + <60s = one logical block. Reply chains aggregate beyond window. |
| Raw storage | SQLite local | Queryable, dedup native, pattern exists (RtkReader). |
| Kernel integration | Heartbeat /observe only | Raw messages too granular for observation table. Metadata only. |
| TelegramReader Rust | Phase 2 | No kernel consumer yet (K15). Will come when extraction pipeline needs it. |
| Data convention | SQLite for operational streams, JSONL for curated datasets | Telegram organ establishes the organism-level convention. |

---

## Organ Data Convention (organism-level, established by this organ)

The Telegram organ is the reference implementation for organism-level data management. Future organs follow these conventions.

### Storage boundary: SQLite vs JSON

| Purpose | Format | Why | Where |
|---------|--------|-----|-------|
| Operational stream (daemon append, concurrent reads) | SQLite + WAL | Dedup, indexes, retention, query | `~/.cynic/organs/{name}/data.db` |
| Curated datasets (labeled, portable, training) | JSONL (git-tracked) | Diffable, notebook-friendly, versionable | `cynic-python/datasets/{domain}/` |
| Kernel artifacts (consumed by Rust) | JSON (git-tracked) | `include_str!()`, immutable between deploys | `cynic-python/artifacts/` |

SQLite replaces JSONL for high-volume operational streams. JSONL remains the format for curated datasets — the human explores raw data in SQLite, selects interesting examples, exports labeled JSONL for training/validation.

### Directory convention

```
~/.cynic/organs/{organ_name}/
├── data.db          # SQLite WAL — single writer (daemon), N readers (kernel, scripts, KAIROS)
├── media/           # blobs if applicable
├── config.yaml      # non-secret tunable values
└── .venv/           # isolated Python dependencies
```

### Retention policy

Raw operational data (L0) is transient — kept for analysis, then prunable after extraction. Curated datasets (JSONL) are permanent.

```yaml
# in organ config.yaml
retention:
  raw_days: 90       # L0 messages purged after 90 days
  media_days: 30     # blobs purged after 30 days (text is the priority)
```

The daemon runs periodic cleanup: `DELETE FROM messages WHERE date < datetime('now', '-{raw_days} days')`. The extracted data (Phase 2 `extractions` table) has no TTL — it is the permanent artifact.

### Query convention (minimal Geyser)

No pub-sub infrastructure. SQLite WAL is the bus.

- **Writer**: the organ daemon (single process, always)
- **Readers**: kernel OrganPort (read-only), KAIROS (direct SQL), analysis scripts, notebooks
- **Streaming**: consumers poll with `WHERE rowid > ?` for near-real-time tailing
- **Heartbeat**: POST `/observe` for organism-level telemetry (silence detection, volume stats) — metadata only, not data

```
                    SQLite WAL (data.db)
                         |
          +--------------+--------------+
          |              |              |
     Daemon (W)    Kernel (R)     KAIROS (R)
     writes        OrganPort      direct SQL
                   health/fresh   analysis
```

### What this replaces

Hermes-X currently uses JSONL (`dataset.jsonl`) for its operational stream. This convention establishes SQLite as the standard. Hermes-X migration to SQLite is a future task — the Telegram organ proves the pattern first.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      cynic-core                          │
│                                                          │
│   telegram-listener (Python, systemd)                    │
│   ├── Telethon v1 (MTProto, session locale)              │
│   ├── @client.on(NewMessage) → buffer                    │
│   ├── buffer flush (auteur+60s+reply) → SQLite + media/  │
│   └── heartbeat POST /observe (every 5min)               │
│                                                          │
│   ~/.cynic/organs/telegram/                              │
│   ├── messages.db          (SQLite, single writer)       │
│   ├── media/                                             │
│   │   └── {channel_id}/{message_id}.{ext}                │
│   ├── session.session      (Telethon auth, SENSITIVE)    │
│   └── config.yaml                                        │
│                                                          │
│   cynic-kernel                                           │
│   └── receives heartbeat via POST /observe               │
│       agent_id: "hermes-telegram" (permanent tier)       │
│       → silence detection if heartbeat stops             │
└─────────────────────────────────────────────────────────┘
```

Single component in Phase 1: the Python daemon. The kernel is a passive consumer of heartbeat metadata only.

---

## Component: telegram-listener

### Responsibilities

1. Connect to Telegram via Telethon, listen to all joined channels
2. Aggregate messages into logical blocks (author + channel + 60s window + reply chains)
3. Write raw messages to SQLite with full Telethon JSON preserved
4. Download media to local filesystem (photos, documents — stored as blobs)
5. Emit heartbeat observations to kernel every 5 minutes
6. Auto-register new channels in the `channels` table

### Execution modes

- `--listen` (default) — real-time daemon, managed by systemd
- `--backfill <channel> [--limit N]` — one-shot historical export, `INSERT OR IGNORE` for dedup
- `--auth` — interactive one-time Telethon authentication (phone number + code + optional 2FA), creates session file

### Configuration

File: `~/.cynic/organs/telegram/config.yaml`

```yaml
session_path: ~/.cynic/organs/telegram/session.session
db_path: ~/.cynic/organs/telegram/messages.db
media_dir: ~/.cynic/organs/telegram/media/
buffer_window_seconds: 60
heartbeat_interval_seconds: 300
retention:
  raw_days: 90
  media_days: 30
```

`kernel_url` and `api_key` are read from environment variables (`CYNIC_REST_ADDR`, `CYNIC_API_KEY`) via `os.environ`, not from YAML. Telegram credentials (`TELEGRAM_API_ID`, `TELEGRAM_API_HASH`) are also read from environment. Config YAML is for non-secret, non-path settings only.

No channel list in config. The daemon listens to ALL channels joined by the Telegram account. Add/remove channels by joining/leaving in Telegram. The daemon auto-detects membership changes.

### Lifecycle

```
startup
  → Telethon connect (local session file)
  → bootstrap SQLite schema if absent
  → register all joined channels in channels table
  → start NewMessage event handler → buffer
  → start heartbeat timer (every 5min)

running
  → new message → add to buffer[channel_id][author_id]
  → if reply_to: merge into parent block regardless of time window
  → if buffer[channel][author] idle > 60s: flush block
      → INSERT messages (one row per message in block)
      → download media if present
      → assign block_id = "{channel_id}_{author_id}_{first_msg_timestamp}"
  → every 5min: POST /observe heartbeat with volume stats

shutdown (SIGTERM)
  → flush all remaining buffers
  → close SQLite
  → disconnect Telethon gracefully
```

### Buffer aggregation logic

```python
# Pseudocode — not implementation
class MessageBuffer:
    # Key: (channel_id, author_id) → list of messages
    # Flush condition: no new message from same author+channel for 60s
    # Reply override: if msg.reply_to_msg_id matches a buffered message,
    #                 merge into that block even if from different author
    #                 or beyond time window
```

Flush produces a block: ordered list of messages sharing one `block_id`. Each message is a separate SQLite row (for granular queries), linked by `block_id` (for logical grouping).

**Cross-author replies:** When a reply from author B merges into author A's block, the `block_id` retains author A's ID (the block originator). The block may contain messages from multiple authors. Phase 2 consumers must use `SELECT * FROM messages WHERE block_id = ?` to get all messages in a block — never parse `author_id` from the `block_id` string.

### Heartbeat observation

```json
{
  "tool": "telegram_listener",
  "target": "heartbeat",
  "domain": "telegram",
  "agent_id": "hermes-telegram",
  "value": {"channels_active": 8, "messages_last_5min": 47, "blocks_last_5min": 12, "media_downloaded": 3},
  "context": "8 channels active, 47 messages, 12 blocks, 3 media in last 5min",
  "tags": ["heartbeat", "telegram"]
}
```

Structured data goes in `value` (parsed as `serde_json::Value` by kernel). `context` stays human-readable prose. Kernel truncates `context` at 2000 chars (source: `ccm/intake.rs:418` — API.md's 200-char claim is stale).

`agent_id: "hermes-telegram"` is classified as `permanent` tier by `classify_source_tier()` via the `starts_with("hermes")` rule. No kernel code change needed — the prefix match is the mechanism, not a hardcoded agent list. Silence detection activates automatically if heartbeats stop.

### Error handling

- `FloodWaitError`: sleep for `error.seconds`, then resume. Log the wait duration. Do not crash.
- `ConnectionError` / `ServerError`: retry with exponential backoff (5s, 10s, 30s, 60s, 300s max). Log each retry.
- SQLite write failure: log error, skip message, continue. Do not crash the listener for a single write failure.
- Media download failure: log error, set `media_path = NULL`, continue. The text is more important.
- Kernel heartbeat failure: log warning, continue. The daemon does not depend on the kernel being alive.

### Transient vs permanent errors (P18)

- Telegram FloodWait (420), ServerError (5xx): transient — retry with backoff
- Telegram AuthKeyError, SessionRevokedError: permanent — log CRITICAL, exit. Human intervention needed.
- SQLite SQLITE_FULL: permanent — log CRITICAL, exit. Disk space issue.

---

## SQLite Schema

File: `~/.cynic/organs/telegram/messages.db`

```sql
CREATE TABLE IF NOT EXISTS channels (
    channel_id   INTEGER PRIMARY KEY,
    name         TEXT,
    username     TEXT,
    channel_type TEXT,    -- "channel", "group", "supergroup"
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
```

### Design choices

- `raw_json`: complete Telethon message serialized. Nothing is lost. Fields we didn't think to extract are recoverable.
- `block_id`: computed at buffer flush time. Format: `{channel_id}_{author_id}_{ISO8601_first_msg}`. Links messages in a logical block.
- `media_path`: relative to `media_dir` config. Format: `{channel_id}/{message_id}.{ext}`.
- No extraction columns — Phase 2 adds a separate `extractions` table referencing `block_id`.
- `PRIMARY KEY (channel_id, message_id)`: natural dedup. Backfill uses `INSERT OR IGNORE`.
- `forward_from` + `forward_msg_id`: enables cross-channel dedup at query time (find all forwards of the same original message).

### Phase 2 extension point

Phase 2 will add an `extractions` table referencing `block_id`. This table is NOT created in Phase 1 — `schema.py` must not include it. The schema extension will be designed after observing real data.

---

## Deployment

### Systemd service

File: `infra/systemd/telegram-listener.service`
Deploy to: `~/.config/systemd/user/telegram-listener.service`

```ini
[Unit]
Description=CYNIC Telegram Listener
After=network-online.target

[Service]
Type=simple
WorkingDirectory=%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
ExecStart=/usr/bin/python3 -m cynic-python.organs.telegram.listener --listen
Restart=on-failure
RestartSec=30
StartLimitIntervalSec=600
StartLimitBurst=3
Environment=PYTHONUNBUFFERED=1
Environment="PYTHONPATH=%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"
EnvironmentFile=%h/.config/cynic/env

[Install]
WantedBy=default.target
```

No `User=`/`Group=` directives (SYS1). `%h` for all paths (R1). `PYTHONPATH` set explicitly for module imports (SYS2). `RestartSec=30` gives time for Telegram flood waits to expire. `StartLimitBurst=3` per 10min prevents infinite restart loop on permanent errors (account ban).

### First run

```bash
# 1. Create organ directory
mkdir -p ~/.cynic/organs/telegram/media

# 2. Create config
cp cynic-python/organs/telegram/config.example.yaml ~/.cynic/organs/telegram/config.yaml
# Edit with actual values

# 3. Authenticate Telethon (interactive, one-time)
python3 cynic-python/organs/telegram/listener.py --auth
# Enter phone number, code, 2FA if needed
# Creates session.session file

# 4. Backfill target channels
python3 cynic-python/organs/telegram/listener.py --backfill @TargetChannel --limit 5000

# 5. Deploy daemon
cp infra/systemd/telegram-listener.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now telegram-listener
journalctl --user -fu telegram-listener
```

### Telethon API credentials

Telethon requires a Telegram API ID and hash (from https://my.telegram.org). These go in `~/.config/cynic/env` (same file as other CYNIC secrets, loaded by systemd via `EnvironmentFile`):

```bash
TELEGRAM_API_ID=<id>
TELEGRAM_API_HASH=<hash>
```

The session file (`session.session`) contains the auth key — treat as a secret. Do not commit to git. The session lives outside the repo (`~/.cynic/organs/telegram/`), so `.gitignore` is not required, but the path must never be referenced in committed code.

---

## File structure

```
cynic-python/organs/telegram/
├── MANIFEST.yaml
├── __init__.py
├── listener.py       -- main daemon (--listen / --backfill / --auth)
├── schema.py         -- SQLite CREATE TABLE, migrations
├── buffer.py         -- message aggregation (author+time+reply)
├── config.py         -- load config.yaml + env vars
├── config.example.yaml
└── tests/
    └── test_buffer.py
```

### MANIFEST.yaml

```yaml
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
    - human_channel_selection_gate  # human reads data → decides which channels feed Phase 2
    - kernel_silence_detection      # heartbeat absence triggers alert
  phase2:
    - llm_extraction_pipeline
    - dogs_judgment
    - kairos_ic_measurement
crons: []
scripts:
  - name: listener.py
    description: "Main daemon — modes: --listen (daemon), --backfill <channel> (export), --auth (setup)"
```

---

## User workflow (Phase 1 deliverable)

Once deployed, the human can:

```bash
# Backfill a channel
python3 cynic-python/organs/telegram/listener.py --backfill @CryptoTraderX --limit 1000

# Explore raw data
sqlite3 ~/.cynic/organs/telegram/messages.db \
  "SELECT date, author_name, substr(text,1,80) FROM messages
   WHERE channel_id = 123 ORDER BY date DESC LIMIT 20"

# Volume by channel
sqlite3 ~/.cynic/organs/telegram/messages.db \
  "SELECT c.name, count(*) as msgs FROM messages m
   JOIN channels c ON m.channel_id = c.channel_id
   GROUP BY m.channel_id ORDER BY msgs DESC"

# Blocks (logical message groups)
sqlite3 ~/.cynic/organs/telegram/messages.db \
  "SELECT block_id, count(*) as msg_count, group_concat(substr(text,1,40), ' | ')
   FROM messages WHERE block_id IS NOT NULL
   GROUP BY block_id ORDER BY date DESC LIMIT 10"

# Check daemon health
systemctl --user status telegram-listener
curl -s ${CYNIC_REST_ADDR}/observations?domain=telegram&limit=5 \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | jq '.[0].context'
```

This is the deliverable: raw data, queryable, waiting for the human to decide what to extract.

---

## What this does NOT include (explicit scope boundaries)

- **LLM extraction** — no structured signal parsing. Phase 2.
- **Dog judgment** — no `build_telegram_stimulus()`, no deterministic dog. Phase 2.
- **TelegramReader Rust** — no OrganPort impl. Phase 2, when a kernel consumer exists.
- **KAIROS query API** — KAIROS reads SQLite directly or waits for Phase 2 kernel integration.
- **Vision/chart analysis** — charts stored as blobs, not analyzed. Separate research track.
- **Multi-account** — single Telegram account. Scale later if needed.
- **Channel recommendation** — which channels to monitor is a human decision.

---

## Phase 2 preview (not in scope, for context)

When Phase 1 data reveals the signal shape:

1. **Extraction pipeline**: Python reads SQLite blocks → calls sovereign LLM (Qwen on core/gpu) → produces free-form JSON → writes to `extractions` table
2. **Judgment pipeline**: enriched blocks → `build_telegram_stimulus()` → POST `/judge` → Dogs score reliability
3. **TelegramReader Rust**: OrganPort reading SQLite, exposing health/freshness/snapshot to kernel
4. **KAIROS integration**: kernel exposes `/observations?domain=telegram` with extraction data, KAIROS measures IC per source x coin x horizon
5. **Vision track**: research Qwen2-VL-7B on chart extraction, benchmark on real chart images from Phase 1 media/

---

## Falsification

- **Phase 1 success**: daemon runs >48h without crash, backfill exports >1000 messages from a target channel, SQLite is queryable, heartbeat appears in kernel observations.
- **Phase 1 failure**: Telegram bans the account, Telethon can't maintain connection, SQLite write throughput insufficient for message volume.
- **Design failure**: if the buffer aggregation logic produces nonsensical blocks (messages that shouldn't be grouped together, or blocks that should be one but are split), the 60s window or reply-chain heuristic needs tuning. Measure on real data before adjusting.
