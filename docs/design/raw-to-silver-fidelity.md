# Raw → Silver Fidelity Map (Graph + State Evolution)

Status: observed (read-only probes, 2026-10-03). No mutation yet.

## Purpose

A session is not a flat sequence of turns. It is a multi-scale hypergraph:
macro (session) → meso (phases, compactions, sub-agents) → micro (turn → tool → reasoning),
with a causal edge between every consecutive event and a state that evolves at each step.

The Silver lakehouse (`corpus_master.sqlite`) currently flattens most of this. This
document records exactly which native structure exists in each raw source, and where
the ingestion scripts drop it.

## Observed gap (live read-back, corpus_master.sqlite)

| harness            | turns   | with parent | pct   | sessions |
|--------------------|--------:|------------:|------:|---------:|
| hermes             | 280 275 | 0           | 0.0%  | 2 258    |
| windows_claude     | 252 540 | 250 826     | 99.3% | 3 421    |
| codex              |  38 674 | 564         | 1.5%  | 194      |
| claude_code        |  26 511 | 26 448      | 99.8% | 101      |
| omp                |  25 808 | 0           | 0.0%  | 88       |
| antigravity_gemini |  23 007 | 0           | 0.0%  | 495      |
| windows_gemini_chat|     461 | 0           | 0.0%  | 20       |
| windows_gemini_tools|     13 | 0           | 0.0%  | 13       |
| windows_codex      |       1 | 0           | 0.0%  | 1        |

Only `claude_code` and `windows_claude` reconstruct the DAG. All other harnesses are
stored flat — the causal structure is lost at ingestion, not in the source.

## Native structures available in raw (per harness)

### codex — rollout JSONL + thread_history_1.sqlite
- `turn_id` groups events per turn (7 distinct turn_id in a sample rollout).
- `turn_context` (per turn): `model`, `cwd`, `sandbox_policy` (workspace-write, network_access),
  `approval_policy` (on-request), `collaboration_mode`, `workspace_roots`, `permission_profile`.
- `event_msg` lifecycle: `task_started`, `user_message`, `agent_message`, `token_count`
  (56 events/rollout), `task_complete`, `patch_apply_end`, `web_search_end`.
- `thread_turns`: `rollout_ordinal`, `first_user_item_id`, `final_agent_item_id`,
  `started_at`, `completed_at`, `duration_ms` — explicit turn chaining + timing.
- `thread_items`: `item_type` ∈ {reasoning 1625, commandExecution 1138, agentMessage 564,
  fileChange 226, userMessage 202, contextCompaction 13, webSearch 12, mcpToolCall 6}.
- **No explicit parent UUID** — parent must be reconstructed from `turn_id` +
  `rollout_ordinal` + item ordering. This is deterministic, not inference.

### hermes — state.db (messages table)
- `tool_call_id`, `tool_call_uid`, `tool_call_uids`, `message_uid`, `absorbed_message_uids`
  → explicit links between tool results and tool calls, and message absorption.
- `compacted` (160 323/285 711), `active`, `_compressed_summary` (285 711 rows),
  `api_content`, `display_order` → state evolution.
- sessions table: `message_count`, `tool_call_count`, token counters, `reasoning_tokens`,
  `rewind_count`, `handoff_state`, `compression_ineffective_count`, `end_reason`.
- **No parent message pointer** — chain from `id` ordering + role + tool_call linkage.

### antigravity — per-conversation .db
- `steps`: `idx`, `step_type` (14=system, 15=assistant+CoT, 21=tool call, 23=tool result,
  90/98=other), `status`, `has_subtrajectory`, `permissions`, `task_details`.
- `parent_references` (idx, data) — explicit parent links (protobuf-encoded).
- `battle_mode_infos` (idx, data) — battle/arena structure between agents.
- `trajectory_meta`: `trajectory_id`, `cascade_id`, `trajectory_type`, `source`.
- `conversation_summaries`: `parent_conversation_id`, `nesting_depth`, `battle_id`,
  `winning_conversation_id`, `group_id` → multi-agent hierarchy (observed: 472
  conversations in the summary DB, nesting not yet populated).
- **parent_references is the native edge set** — currently ignored entirely.

### omp — session JSONL
- Every line has `id` and **`parentId`** — an explicit message DAG.
  Observed: 722 distinct parents / 722 messages (a true chain, not a star).
- `model_change` lines carry `parentId` too (model switch is a node in the graph).
- `message.contextSnapshot`: `{promptTokens, nonMessageTokens}` per turn → context growth.
- `message.usage`: `input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens`,
  `reasoningTokens`, `cost`, plus `ttft` and `duration` → full per-turn telemetry.
- **parentId is the native edge set** — currently ignored entirely.

### windows_gemini_chat
- `messages[]` with `type`, `content`, `timestamp`, `thoughts`, `toolCalls`, token telemetry.
- Flat array; parent from ordering.

## Loss points in ingestion code (verified in-tree)

| script                     | line | what is dropped                                   |
|----------------------------|------|---------------------------------------------------|
| normalizers_v5.py          | 261  | codex: `parent_turn_uid = None` (turn_id ignored) |
| normalizers_v5.py          | 529  | hermes: `parent_turn_uid = None` (tool_call ignored) |
| normalizers_v5.py          | 575/583 | antigravity: `parent_turn_uid = None` (parent_references ignored) |
| normalizers_v5.py          | 831  | omp: `parent_turn_uid = None` (parentId ignored)  |
| normalizers_v5.py          | 767  | windows_gemini_chat: `parent_turn_uid = None`     |
| codex_deep_extractor.py    | 261  | container turn `parent_turn_uid = None` (correct as root; children chained at :294) |

Note: `codex_deep_extractor.py:294` DOES set `parent_turn_uid = turn_uid` for assistant
turns — that is the correct pattern the normalizers should follow. The two sub-agent
reports disagreed on this line; the code at :294 confirms the child link is present for
agentMessage only, not for the full item sequence.

`parse_claude_v5` (normalizers_v5.py:82-103) is the reference implementation: 2-pass,
builds `uuid_to_turn_uid`, maps `parentUuid → parent_turn_uid`, preserves `isSidechain`.

## Missing from the schema (state evolution)

`schema_v4.py` stores turn content and ordering but not the evolving state:
- no per-turn context snapshot (promptTokens, files touched, sandbox/approval policy)
- no link reasoning → action that followed
- no link compaction → summarized turn range
- orchestration_events.payload_json is unstructured

## Mathematical domains the corpus touches

Graph theory (directed, DAG, trees) · hypergraph theory (n-ary relations) ·
topological data analysis (persistent homology across scales) · stochastic processes
(Markov chains for turn evolution) · information theory (structural entropy, mutual
information between turns) · GNN / knowledge graphs (representation).

## Principle

Fix the owning layer: the ingestion script, not a post-hoc inference pass.
Every harness must map its native edge set to `parent_turn_uid` deterministically:
- claude/windows_claude → `parentUuid`
- omp → `parentId`
- antigravity → `parent_references`
- hermes → `tool_call_uid` / `id` ordering
- codex → `turn_id` + `rollout_ordinal`
- windows_gemini_chat → message ordering
