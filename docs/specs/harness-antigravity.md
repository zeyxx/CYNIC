# Harness `antigravity` — knowledge capsule (retired T6)

Antigravity (Gemini) agent trajectories. Python chain retired 2026-10-19
(T6, fifth and last harness, operator HITL) : no Rust vertical (T7).
The PROTOBUST harness — schema-less protobuf, wire format walked by hand.
Code history : slice 3 era -> `2fe85682` -> `2f3d7e19`.

## Sources

- Frozen (ONLY source — no live dir) :
  `/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations/`
  (502 files : 500 session DBs + 2 sidecars ; 499 readable, 1 dead).
- Silver family name : `antigravity_gemini` (500 sessions).

## Native shape (one SQLite DB per session, `steps` table)

- `step_payload` (and `error_details`) : protobuf WITHOUT discoverable
  schema — the adapter walks the wire format directly. Field numbers and
  structure preserved exactly, names NEVER invented (`f13`, `f5.f4`).
- Envelope : `f1` = step type (mirrors `step_type` column), `f4` = status
  (mirrors `status`, proven equal on all 56 155 steps), `f5` = record
  envelope (timestamps, role, tool detail, trace checkpoints).
- `f5.f1` = `(epoch_sec, nanos)` -> step timestamp.
- `f5.f3` = native side marker : 2 = tool-side, 4 = config, 5 = agent-side.
- `f5.f4` = tool call detail (`f1` call id, `f2` tool name, `f3` JSON args)
  on call types {5, 8, 9, 132, 138}.
- Top-level field `N` (one per type) = the type-specific payload.
- NO call<->result id linkage anywhere (628 call ids, zero appear in any
  result step) -> tool edges DERIVED from order, never NATIVE.
- `parent_references` / `battle_mode_infos` empty in all 499 readable DBs
  (declared, not linked).
- One DB (`7d1d0212-...`) dead on disk (`unable to open`) -> `session_refs`
  skips it, `broken` records it in meta.

## FIELD_FATE (closed world — 76 lines, engraved in full)

```
FIELD_FATE: dict[str, str] = {
    # -- steps row --
    "steps.idx": "preserved: ordering (timestamp, idx tiebreak)",
    "steps.step_type": "preserved: payload f1 (mirrored, cross-checked)",
    "steps.status": "preserved: payload f4 (proven equal on all steps)",
    "steps.has_subtrajectory": "declared: always 0 in the corpus, no sub-trajectories exist",
    "steps.metadata": "preserved: turn.native.metadata (walked protobuf)",
    "steps.error_details": "preserved: tool/turn native.error (walked protobuf); presence sets is_error",
    "steps.permissions": "preserved: turn.native.permissions (walked protobuf)",
    "steps.task_details": "preserved: turn.native.task_details (walked protobuf)",
    "steps.render_info": "preserved: turn.native.render_info (walked protobuf)",
    "steps.step_payload": "preserved: dispatch + sha256 in native.payload_hash",
    "steps.step_format": "declared: always 0 in the corpus",
    # -- envelope f5 --
    "f5.f1": "preserved: timestamp_iso (epoch sec, nanos)",
    "f5.f3": "preserved: turn.native.side (2 tool / 4 config / 5 agent)",
    "f5.f4": "preserved: tool call (f1 id, f2 name, f3 args JSON)",
    "f5.f4.f7": "preserved: tool.native.binary_args (non-text arg bytes)",
    "f5.f4.f9": "preserved: tool.native.call_status (n_command counter, 17815 occurrences)",
    "f5.f5": "preserved: turn.native.arg_keys (arg schema key names)",
    "f5.f6/f7/f8": "preserved: turn.native.envelope_times (secondary timestamps)",
    "f5.f9": "preserved: turn.native.f9 (agent-side envelope detail)",
    "f5.f11": "preserved: turn.native.model_code",
    "f5.f12": "preserved: turn.native.trace_id",
    "f5.f13": "preserved: turn.native.envelope_13",
    "f5.f20": "preserved: turn.native.step_refs (trajectory/step uuids)",
    "f5.f21/f22/f23": "preserved: turn.native.envelope_21_22_23",
    "f5.f25": "preserved: turn.native.envelope_25 (id refs)",
    "f5.f26": "preserved: turn.native.checkpoints (ordered seq+timestamp trace)",
    "f5.f33": "preserved: turn.native.envelope_33 (agent-message marker ints)",
    "f5.f28": "preserved: turn.native.envelope_28",
    "f5.f30/f31/f32": "preserved: turn.native.envelope_30_31_32 (text + timestamps)",
    # -- type payloads --
    "f7.*": "preserved: turn.content (search/grep matches) + native.command",
    "f13.*": "preserved: turn.content (search matches) + native.query/command",
    "f14.*": "preserved: turn.content (fetched live content JSON)",
    "f15.*": "preserved: turn.content (directory listing entries)",
    "f19.*": "preserved: turn.content (system prompt) + native.config",
    "f20.*": "preserved: turn.content (model response) + native.response envelope",
    "f21.*": "preserved: turn.native.shell_output (directory listing text)",
    "f24.*": "preserved: turn.content (API error: message, HTTP trace, retry info)",
    "f28.*": "preserved: turn.content (command output / cascade error)",
    "f30.*": "preserved: turn.content (result text + transcript/system refs)",
    "f31.*": "preserved: turn.content (failure detail) + native.tool_name/target",
    "f33.*": "preserved: turn.native.agent_message envelope",
    "f37.*": "preserved: turn.content (cascade error refs)",
    "f40.*": "preserved: turn.content (fetched URL + live JSON)",
    "f42.*": "preserved: turn.content (query string)",
    "f47.*": "preserved: tool.call (f2 name, f3 args)",
    "f62.*": "preserved: turn.content (MCP server name)",
    "f90.*": "preserved: turn.native.ephemeral_marker",
    "f91.*": "preserved: turn.native.calltype_91",
    "f103.*": "preserved: turn.content (ephemeral system reminder)",
    "f104.*": "preserved: turn.content (image prompt/model/path)",
    "f10.*": "preserved: turn.content (edit description + file context)",
    "f111.*": "preserved: turn.native.heartbeat (empty marker message)",
    "f114.*": "preserved: turn.content (agent message: sender/priority/text)",
    "f127.*": "preserved: turn.native.delegation (MCP caller instruction)",
    "f132.*": "preserved: turn.native.calltype_132",
    "f133.*": "preserved: tool.command_or_args (shell plan commands) + native",
    "f138.*": "preserved: turn.native.calltype_138",
    "f140.*": "preserved: turn.native.calltype_140",
    "f143.*": "preserved: turn.content (delegation instruction + transcript refs)",
    "f147.*": "preserved: tool.command_or_args (shell plan commands) + native",
    "f148.*": "preserved: turn.content (subagent task id + log + timer prompt)",
    "f154.*": "preserved: turn.content (user question + options)",
    "f156.*": "preserved: turn.content (file path + text)",
    "f56.*": "preserved: turn.content (user question + options)",
    # -- trajectory tables --
    "trajectory_meta.*": "preserved: session meta (trajectory/cascade id, type, source)",
    "trajectory_metadata_blob": "preserved: session meta.blob (walked protobuf)",
    "gen_metadata": "preserved: session meta.gen (walked protobuf)",
    "executor_metadata": "preserved: session meta.executor (walked protobuf)",
    "parent_references": "declared: empty in all DBs — no native parent pointers exist",
    "battle_mode_infos": "declared: empty in all DBs",
}
```

## Coverage at retirement

- Silver : 500 sessions (`antigravity_gemini`).
- Guards removed : capture, field coverage, normalizer tests.
- Registry : UNCONTRACTED emptied (was the last entry).
- Rust check : `cynic-traces` has zero antigravity refs ; `cynic-eval`'s
  `AntigravityCli` is the agent executor runtime, unrelated to this
  normalizer — nothing to migrate.

## T7 rebuild pointers

- Scan : `conversations/<uuid>.db`, `steps` table ordered by
  (timestamp, idx).
- Walk the wire format field-by-field ; never invent names.
- Edges DERIVED from order (no id linkage exists) — say so in the proof.
- Replicate the FIELD_FATE table above as the contract ; fail closed.
- Expect exactly one dead DB ; record it, don't crash.
