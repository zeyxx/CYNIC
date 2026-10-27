# Raw → Silver → Gold: Essence Loss Audit (findings)

Status: observed (read-only, 2026-10-03). The pipeline produces plausible-looking
data that misses the essential information. Evidence below.

## Verdict

The Gold suite looks good by volume (63 575 samples, 628 MB) but fails three ways:
it carries **byte soup instead of language**, it keeps **no graph or state**, and it
**discards the human signal** — the richest thing the corpus holds.

## Evidence

### 1. Byte soup masquerading as content

Antigravity content is protobuf extracted by `extract_strings_from_bytes()`
(normalizers_v5.py:39) — a regex over raw bytes. Result:

```
[user]      'L\n"'
[assistant] 'N\n\n"\n*2(b51ba703d2a8e0764bc16b22c8d505ef257bf007'
[assistant] 'P\n\n"\n*2(f9f9bab9cf89b79b3d81a70be6a264966b4c4ffe\n\n<tool_response>...'
```

The prompt is 3 bytes. The assistant turns open with `P\n\n"\n*2(hash)` — wire-format
fragments, not reasoning. `sft/rust_expert_train.jsonl` is 100 % antigravity, so the
flagship SFT file is dominated by this.

Garbage rate by row (strict byte-soup test): ~1.8 % overall, but **swarm 12–21 %**
and **dpo/human_alignment_val 22 %**. The `is_garbage` heuristic under-counts because
wire fragments mixed with real text still contain words.

### 2. No graph, no state

Gold top-level fields across all 39 files:
`chosen, compressed_summary, content, context_intent, delegated_subagents, doc_type,
executed_tool, file_path, id, initial_code, messages, messages_chosen, messages_rejected,
metadata, orchestration_type, parent_intent, patch, payload, prompt, rejected,
relative_path, session_uid, steps, target_code, title, total_steps, trigger_reason,
turn_uid, uncompressed_context, verifiable_outcome`

Graph/state fields present: **only `session_uid` and `turn_uid`** — no `parent_turn_uid`,
no `edge_kind`, no state snapshot, no causal link. The hypergraph discovered in Silver
(`docs/design/raw-to-silver-fidelity.md`) is flattened away in Gold.

### 3. The human signal is discarded

- Silver holds **70 550 human (user) turns**.
- Gold carries **no correction classification** — searched all 39 files for
  `is_correction / correction / steer / human_feedback / pushback / intent_delta /
  misalignment`: **NONE**.
- `dpo/human_alignment_train.jsonl` (574 rows) is the only place human steering
  survives, and its `prompt` is a raw JSON blob (`{"role": "user", "content": "..."}`)
  while `rejected` is often the agent's own answer, not a graded correction.
- External state of the art says the human correction is the richest signal
  (*How Coding Agents Fail Their Users*, 20 574 sessions: 91.49 % of resolutions
  require explicit developer pushback). We drop it.

### 4. The funnel

| Stage | Count |
|---|---|
| Silver dialogue_turns | 435 946 (user+assistant) |
| Silver human turns | 70 550 |
| Gold samples | 63 575 |
| Gold SFT samples | **1 897** |

SFT = 1 897 rows from 435 946 turns (**0.44 %**). The "SFT Domain Experts" pillar is
tiny and its content is byte soup. Meanwhile the largest pillar (compactions, 21 167,
33 % of Gold) is the only clean one — real prose both sides.

### 5. Pillars are misnamed

| Pillar | Reality |
|---|---|
| swarm_routing (7 615) | `queue-operation` orchestration JSON, not routing decisions |
| prm_reasoning (15 488) | `context_intent` is byte soup for antigravity |
| sft domains (1 897) | antigravity wire fragments; no real human prompt |
| compactions (21 167) | clean, real — the one solid pillar |
| diffs (15 743) | real patches from claude (initial_code + patch) |
| dpo causal_pairs (281) | "prompt: Execute system operation safely" + garbage rejected |

## What the essence actually is (and where it was lost)

| Essence | Present in raw? | Survives to Silver? | Survives to Gold? |
|---|---|---|---|
| Human intent (real prompt) | yes | yes (user turns) | **no** (3-byte prompts) |
| Trajectory graph (causal edges) | yes (parentUuid/parentId/parent_references) | **mostly no** (flat) | **no** |
| State evolution (tokens, sandbox, cwd) | yes (turn_context/contextSnapshot) | **no** | **no** |
| Reasoning → action chain | yes (CoT + tool calls) | partial (turn_uid link) | **no** |
| Human correction (labeled) | yes (steering turns) | yes (as plain user turn) | **no** (untagged) |
| Outcome / verifiability | yes (commits, tests, exit codes) | partial (exit_code) | partial |
| Environment observation | yes (tool results) | yes | partial (inline) |

## Root cause

The pipeline is **entity-extraction, not trajectory-preservation**. Each normalizer
pulls strings out of raw bytes and emits flat rows. Nothing carries the causal edge,
the state, or the human's intent-as-signal. The Gold generator then reassembles
plausible chat shapes from those fragments — which is why it *looks* good.

Fixing the storage model (schema V5: edges + state) is necessary but not sufficient:
the **Gold generators themselves** must be rewritten to project the trajectory
(human intent → reasoning → action → observation → correction), not to concatenate
extracted strings.

## Next decision

Do not regenerate Gold until Silver carries the graph + state + labeled human signal.
Order: (1) fix Silver ingestion (edges, state, correction tagging), (2) rebuild the
Gold generators on the trajectory model, (3) re-export and re-audit with these probes.
