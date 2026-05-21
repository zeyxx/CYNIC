# Organ Data Convention

> Reference implementation: Telegram organ (`cynic-python/organs/telegram/`).

Every CYNIC organ that captures external data follows these conventions. Established 2026-05-21 with the Telegram organ as the first implementation.

## Storage Boundary: SQLite vs JSON

| Purpose | Format | Why | Where |
|---------|--------|-----|-------|
| **Operational stream** (daemon append, concurrent reads) | SQLite + WAL | Dedup, indexes, retention, concurrent query | `~/.cynic/organs/{name}/data.db` |
| **Curated datasets** (labeled, portable, training/validation) | JSONL (git-tracked) | Diffable, notebook-friendly, versionable | `cynic-python/datasets/{domain}/` |
| **Kernel artifacts** (consumed by Rust via `include_str!`) | JSON (git-tracked) | Immutable between deploys | `cynic-python/artifacts/` |

SQLite is for high-volume operational streams where the daemon appends and multiple consumers query concurrently. JSONL is for curated datasets — the human explores raw data in SQLite, selects interesting examples, and exports labeled JSONL for training/validation/calibration.

**The flow:** SQLite (mine) -> human exploration -> selection + labeling -> JSONL (refined ore) -> LLM training / Dog calibration.

## Directory Convention

```
~/.cynic/organs/{organ_name}/
├── data.db          # SQLite WAL — single writer (daemon), N readers
├── media/           # blobs if applicable (images, documents)
├── config.yaml      # non-secret tunable values only
└── .venv/           # isolated Python dependencies
```

- `data.db` is the canonical name for the operational SQLite database
- Secrets (`API_KEY`, `API_HASH`) come from env vars via `~/.config/cynic/env`, never from YAML
- The `.venv/` isolates organ dependencies from system Python and other organs
- `media/` stores binary blobs organized by source ID subdirectories

## Retention Policy

Raw operational data (L0) is transient. Extracted/curated data is permanent.

```yaml
# in organ config.yaml
retention:
  raw_days: 90       # L0 raw data purged after N days
  media_days: 30     # blobs purged after N days (text is the priority)
```

The daemon runs periodic cleanup (once per heartbeat cycle):
```sql
DELETE FROM {table} WHERE date < datetime('now', '-{raw_days} days');
```

Extracted data (Phase 2 `extractions` table or equivalent) has no TTL — it is the permanent artifact.

**Rationale (mapped from Solana):** Solana validators purge ledger history after ~2 epochs (~4 days). Full history requires specialized infrastructure (Warehouse nodes). CYNIC follows the same principle: raw L0 data is a consumable stream, not a permanent archive. The permanent artifacts are the extractions and curated datasets.

## Query Convention (Minimal Geyser)

No pub-sub infrastructure. SQLite WAL is the bus.

```
                    SQLite WAL (data.db)
                         |
          +--------------+--------------+
          |              |              |
     Daemon (W)    Kernel (R)     Consumer (R)
     single        OrganPort      direct SQL
     writer        health/fresh   (KAIROS, scripts)
```

- **Writer**: the organ daemon (single process, always). No concurrent writers.
- **Readers**: kernel OrganPort (read-only), analysis scripts, notebooks, other agents (KAIROS)
- **Near-real-time tailing**: consumers poll with `WHERE rowid > ?` (cheap, no infrastructure)
- **Heartbeat**: POST `/observe` for organism-level telemetry (silence detection, volume stats). This is state metadata, not data.

## Heartbeat Convention

Every organ daemon emits a heartbeat observation to the kernel at a configurable interval:

```json
{
  "tool": "{organ}_listener",
  "target": "heartbeat",
  "domain": "{domain}",
  "agent_id": "hermes-{organ}",
  "value": {"sources_active": N, "events_last_period": N},
  "context": "human-readable summary",
  "tags": ["heartbeat", "{domain}"]
}
```

`agent_id` prefix `hermes-` classifies as permanent tier in `classify_source_tier()`, enabling automatic silence detection.

## What This Replaces

Hermes-X currently uses JSONL (`dataset.jsonl`) for its operational stream. This convention establishes SQLite as the standard for operational data. Hermes-X migration to SQLite is a future task — the Telegram organ proves the pattern first.

| Before | After |
|--------|-------|
| JSONL for operational streams (hermes-x) | SQLite for operational streams |
| No retention policy | Configurable TTL per organ |
| `grep`/`tail` for queries | SQL queries with indexes |
| No concurrent read access | WAL mode enables N readers |
| No dedup | PRIMARY KEY constraints |

## Adopting This Convention

When creating a new organ:
1. Follow the directory convention (`~/.cynic/organs/{name}/`)
2. Use SQLite with WAL for the operational stream
3. Add `retention:` to `config.yaml`
4. Implement periodic cleanup in the daemon
5. Emit heartbeats with `agent_id: "hermes-{name}"`
6. Create a `.venv/` for isolated dependencies

When migrating an existing organ (e.g., hermes-x):
1. Create a SQLite schema matching the JSONL fields
2. Write a one-shot migration script (JSONL -> SQLite)
3. Update the daemon to write SQLite instead of JSONL
4. Update consumers to read SQLite instead of parsing JSONL
5. Keep the old JSONL as a backup until migration is validated
