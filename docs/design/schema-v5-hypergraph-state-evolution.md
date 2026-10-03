# Schema V5 Design — Hypergraph + State Evolution (proposal)

Status: proposed (not applied). Awaiting validation before mutating schema_v4/normalizers_v5.

## Principle

The session is a multi-scale hypergraph whose state evolves at every event.
Raw sources already carry the edges and the state; ingestion drops them.
Schema V5 transcribes both into the processed data — deterministically from the
native field, never by post-hoc inference.

## Part A — Reconstruct the causal edge (binary DAG)

`dialogue_turns.parent_turn_uid` stays the canonical binary causal edge.
Add `edge_kind` so the provenance of each edge is machine-readable.

```sql
ALTER TABLE dialogue_turns ADD COLUMN edge_kind TEXT;  -- native source of the edge
```

| harness            | native edge source            | edge_kind value      |
|--------------------|-------------------------------|----------------------|
| claude_code        | `parentUuid` (2-pass map)     | `parent_uuid`        |
| windows_claude     | `parentUuid` (2-pass map)     | `parent_uuid`        |
| omp                | `parentId` (message DAG)      | `parent_id`          |
| antigravity        | `parent_references` (protobuf)| `step_parent`        |
| hermes             | `tool_call_uid` / `id` order  | `tool_call` / `sequence` |
| codex              | `turn_id` + `rollout_ordinal` | `turn_order`         |
| windows_gemini_chat| message order                 | `sequence`           |

## Part B — The hypergraph (n-ary edges)

A binary parent pointer cannot express an n-ary relation (one compaction summarizing
N turns; one battle between M agents; one tool call producing K results).
New table:

```sql
CREATE TABLE IF NOT EXISTS turn_edges (
    edge_uid     TEXT PRIMARY KEY,
    session_uid  TEXT NOT NULL,
    edge_kind    TEXT NOT NULL,   -- reply | tool_call | tool_result |
                                  -- compaction_summary | sidechain_branch |
                                  -- subagent_dispatch | battle | model_change
    source_uids  TEXT NOT NULL,   -- JSON array of turn_uid (n-ary)
    target_uid   TEXT,            -- turn_uid the edge points to (nullable for pure n-ary)
    payload_json TEXT,            -- edge-specific data (battle winner, dispatch id...)
    timestamp_iso TEXT
);
CREATE INDEX IF NOT EXISTS idx_edges_session ON turn_edges(session_uid);
CREATE INDEX IF NOT EXISTS idx_edges_kind ON turn_edges(edge_kind);
CREATE INDEX IF NOT EXISTS idx_edges_target ON turn_edges(target_uid);
```

Populated from:
- antigravity `battle_mode_infos` → `battle`; `parent_references` → `step_parent`
- antigravity `has_subtrajectory` + stype 132 → `subagent_dispatch`
- hermes `absorbed_message_uids` → `compaction_summary` (which turns were absorbed)
- codex `thread_turns.first_user_item_id`/`final_agent_item_id` → `reply` bounds
- omp `model_change.parentId` → `model_change`
- claude `isSidechain` transitions → `sidechain_branch`

## Part C — State evolution (per-turn snapshot)

The state that changes at each event. New table:

```sql
CREATE TABLE IF NOT EXISTS turn_state_snapshots (
    snapshot_uid          TEXT PRIMARY KEY,
    session_uid           TEXT NOT NULL,
    turn_uid              TEXT,
    turn_index            INTEGER,
    -- context growth (omp contextSnapshot; codex token_count)
    context_prompt_tokens     INTEGER,
    context_nonmessage_tokens INTEGER,
    cumulative_tokens         INTEGER,
    -- execution environment (codex turn_context)
    model                 TEXT,
    cwd                   TEXT,
    sandbox_policy        TEXT,
    approval_policy       TEXT,
    permission_profile    TEXT,
    workspace_roots_json  TEXT,
    -- context window state (hermes)
    context_window_tokens INTEGER,
    compacted             INTEGER DEFAULT 0,
    timestamp_iso         TEXT
);
CREATE INDEX IF NOT EXISTS idx_state_session ON turn_state_snapshots(session_uid);
CREATE INDEX IF NOT EXISTS idx_state_turn ON turn_state_snapshots(turn_uid);
```

Populated from:
- omp `message.contextSnapshot` + `usage` → tokens
- codex `turn_context` → model/cwd/sandbox/approval/workspace_roots; `event_msg.token_count` → tokens
- hermes `messages.compacted` + session counters → compacted/context_window

## Part D — Link reasoning → action, and compaction → range

```sql
-- reasoning that produced an action
ALTER TABLE reasoning_traces ADD COLUMN led_to_exec_uid TEXT;
-- tool result linked to the call that produced it
ALTER TABLE tool_executions ADD COLUMN parent_exec_uid TEXT;
-- compaction summarizing a range of turns
ALTER TABLE context_compactions ADD COLUMN summarized_turn_uids TEXT;  -- JSON array
ALTER TABLE context_compactions ADD COLUMN summarized_token_count INTEGER;
```

Populated from:
- hermes `tool_call_id` → `tool_executions.parent_exec_uid` (result ↔ call)
- omp `parentId` chain → `reasoning_traces.led_to_exec_uid`
- codex `contextCompaction` items → `summarized_turn_uids`

## Part E — Parser changes (R)

Every `turns.append(...)` currently passes `None` at index 2. Replace with the
native edge. Each parser gets a small state machine tracking the previous relevant
turn_uid (mirroring `parse_claude_v5` at normalizers_v5.py:82-103, the reference impl).

| parser                     | line   | change                                             |
|----------------------------|--------|----------------------------------------------------|
| parse_codex_v5             | 309    | parent = last turn of same `turn_id` (turn_order)  |
| parse_hermes_v5            | 528    | parent = last turn; tool results chain to call     |
| parse_antigravity_v5       | 574/582| parent from `parent_references`; system=root       |
| parse_omp_v5               | 830    | parent = `parentId` mapped to turn_uid             |
| parse_windows_gemini_chats_v5 | 766 | parent = previous message                          |

## Validation plan

After implementation, re-run ingestion on a single harness (OMP first — explicit
`parentId`), then read back the coverage:

```sql
SELECT agent_family, count(*), sum(parent_turn_uid IS NOT NULL)
FROM dialogue_turns GROUP BY agent_family;
```

Target: `omp` 0% → >99%, then antigravity, codex, hermes. No regression on
claude_code / windows_claude (must stay ~99%).

## Open questions (need decision)

1. Schema migration: rebuild `corpus_master.sqlite` from scratch (V5 DDL) vs
   `ALTER TABLE` in place. Recommend rebuild — the lakehouse is derived, and a
   clean rebuild proves conservation.
2. `turn_edges` scope: populate only explicit native edges, or also derive the
   implicit reply edges (user→assistant)? Recommend explicit-first, derive later.
