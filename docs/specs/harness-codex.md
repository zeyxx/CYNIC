# Harness `codex` — knowledge capsule (retired T6)

Codex CLI rollouts. Python chain retired 2026-10-19 (T6, second harness,
operator HITL) : no Rust vertical (T7). Code history :
`48c5cd1b` (birth, slice 3) -> `2fe85682` -> `2f3d7e19`.
Unlike windows_gemini/omp, this adapter had NO closed-world FIELD_FATE
table. It had 5 contract tests + a full sweep in
`test_normalizers_contract.py` (removed with the chain; the Claude Code
remainder stays).

## Sources

- Live : `/home/user/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<iso>-<uuid>.jsonl`
  (95 files, 2026/06-08 — stale since August 2026).
- Frozen : `/var/lib/cynic/vault/corpus/sources/codex/2026/{06,07,08}/`
  (94 files).
- Provenance gap (recorded, not hidden) : silver holds 193 `codex`
  sessions (+1 `windows_codex`, +1 `windows_codex_rollout`) but only 94
  rollout files are frozen. The extra sessions were ingested from live
  reads that no longer exist on disk. A T7 vertical must re-enumerate
  from live + frozen and reconcile the count.

## Native shape (one JSON object per line, `type` + `payload`)

- `type` is one of `session_meta` | `event_msg` | `response_item` |
  `world_state` | `turn_context`.
- `session_meta.payload` : `session_id`, `cwd`, `cli_version`,
  `model_provider` — session identity + native context.
- `response_item.payload` : `role` | `type`, the dialogue stream —
  `message` (text) | `reasoning` | `function_call` |
  `function_call_output`.
- `response_item.payload.summary` : reasoning chain-of-thought.
- `turn_context.payload` : `model`, `cwd`, `approval_policy`,
  `file_system_sandbox_policy`, `effort` -> a genuine StateSnapshot.
- NO parent pointer -> sequence edges DERIVED from line order.
- Native naming kept : tool results are `function_call_output` (not
  `tool_result`) ; the canonical entity keeps the native name in
  `tool_name`.

## Coverage at retirement

- Silver : 193 sessions (+2 windows_codex* orphans of unknown adapter).
- Guards removed : 5 contract tests + full sweep (claude-only remainder
  stays) ; registry entry removed.
- Port doc updated (`ports.py` : Codex bullet retired).

## T7 rebuild pointers

- Scan : `<root>/<yyyy>/<mm>/<dd>/rollout-*.jsonl`, one session per file.
- Five line types above ; order lines, derive sequence edges.
- NO fate table existed — the vertical must establish one (closed-world
  test like `test_windows_gemini_field_coverage.py`, deleted with T6-1
  but described in `docs/specs/harness-windows-gemini.md`).
- Reconcile 193 silver sessions against 94 frozen + live files.
