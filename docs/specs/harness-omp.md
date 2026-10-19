# Harness `omp` — knowledge capsule (retired T6)

OMP agent sessions. Python chain retired 2026-10-19 (T6, third harness,
operator HITL) : no Rust vertical (T7). The richest retired adapter
(867 lines, closed-world FIELD_FATE, NATIVE-pointer family). Code history :
slice 3 era -> `2fe85682` -> `2f3d7e19`.

## Sources

- Live : `/home/user/.omp/agent/sessions/<slug>/<iso>_<uuid>.jsonl`
  (1342 files — mirror of the frozen set, Aug-Sep 2026).
- Frozen : `/var/lib/cynic/vault/corpus/sources/omp/<slug>/`
  (1341 files : session JSONL + per-session dirs with `*.bash-original.log`).
- Slugs observed : `-Bureau-SOLANA-asdf-forge-zeyxx-CYNIC`,
  `-Bureau-SOLANA-asdf-forge-zeyxx-CYNIC-wt-rag-eval`.

## Native shape (one JSON object per line, chronological, single parentId chain)

- `type` : `title` | `session` | `model_change` | `thinking_level_change` |
  `message` | `custom` | `custom_message` | `compaction` | `credential_pin` |
  `title_change` | `ttsr_injection` | `session_init` | `branch_summary` |
  `model_usage`.
- `parentId` -> explicit chain pointer, so REPLY edges are NATIVE
  (only the leading records have `parentId: null`).
- `message.message.role` : `user` | `assistant` | `toolResult` | `developer`
  (tool results ride as their own role, not as blocks).
- `message.message.content` : LIST of `text` | `thinking` | `toolCall` |
  `image` blocks (rarely a bare string).
- `toolCall` block : `{id, name, arguments, thoughtSignature}`.
- `toolResult` message : `{toolCallId, toolName, content, details, isError}`.
- `custom/tool_execution_start` : `{toolCallId, toolName, startedAt, args}`
  — harness explicitly stating execution start, so TOOL_CALL edges are
  NATIVE and carry `startedAt`.
- `message.usage` (assistant turns) : `{input, output, totalTokens,
  reasoningTokens, cost}` + `message.contextSnapshot` -> StateSnapshot.
- `compaction` : `summary` / `firstKeptEntryId` / `tokensBefore` ->
  StateSnapshot(compacted=True) + COMPACTION edge.
- `model_change` : new `model` -> MODEL_CHANGE edge.
- `thinking` blocks -> one ReasoningTrace each.
- Skipped with intent (no dialogue semantics, still in the chain) :
  `title` / `title_change` (kept in native_meta), `session` (identity),
  `session_init`, `thinking_level_change`.
- Credential hygiene : `credential_pin.hash` presence counted, value
  NEVER persisted ; provider counted.

## FIELD_FATE (closed world — 129 lines, engraved in full)

```
FIELD_FATE: dict[str, str] = {
    # -- envelope (all records) --
    "*.id": "preserved: chain resolution + turn.native.record_id",
    "*.parentId": "preserved: REPLY edges (native parentId chain)",
    "*.timestamp": "preserved: timestamp_iso",
    "*.type": "preserved: dispatch",
    # -- title / session identity --
    "title.title": "preserved: meta.title",
    "title.source": "preserved: meta.last_title_source",
    "title.updatedAt": "declared: envelope duplicate of chain time",
    "title.v": "declared: file format version, no behavior signal",
    "title.pad": "declared: whitespace padding, no signal",
    "session.id": "preserved: session_uid",
    "session.cwd": "preserved: meta.cwd + state.cwd",
    "session.version": "preserved: meta.session_version",
    "session.parentSession": "preserved: meta.session_parent",
    "session.title": "preserved: meta.title",
    "session.titleSource": "preserved: meta.last_title_source",
    "session_init.agent": "preserved: meta.session_init",
    "session_init.modelRole": "preserved: meta.session_init",
    "session_init.outputSchema": "preserved: meta.session_init",
    "session_init.outputSchemaMode": "preserved: meta.session_init",
    "session_init.readOnly": "preserved: meta.session_init",
    "session_init.resolvedModel": "preserved: meta.session_init",
    "session_init.spawns": "preserved: meta.session_init",
    "session_init.systemPrompt": "preserved: meta.session_init.systemPrompt",
    "session_init.task": "preserved: meta.session_init",
    "session_init.tools": "preserved: meta.session_init",
    "title_change.title": "preserved: meta.title_history + last_title",
    "title_change.source": "preserved: meta.title_history",
    "title_change.trigger": "preserved: meta.title_history",
    "title_change.previousTitle": "preserved: meta.title_history",
    # -- model / thinking config --
    "model_change.model": "preserved: turn.native_model + edge.payload + meta.model_history",
    "model_change.resolvedModelIsFallback": "preserved: edge.payload + meta.model_history",
    "model_change.role": "preserved: meta.model_history",
    "thinking_level_change.thinkingLevel": "preserved: turn.native.thinkingLevel",
    "thinking_level_change.configured": "preserved: turn.native.thinkingConfigured",
    # -- message core --
    "message.role": "preserved: turn.role",
    "message.content[]:text": "preserved: turn.content",
    "message.content[]:thinking": "preserved: reasoning entity + REASONING edge",
    "message.content[]:toolCall": "preserved: tool entity + TOOL_CALL edge",
    "message.content[]:image": "preserved: turn.native.image_blocks (full block)",
    "message.content:<str>": "preserved: turn.content",
    "message.content:<none>": "preserved: empty turn.content",
    "message.attribution": "preserved: turn.native.attribution",
    "message.api": "preserved: turn.native.api",
    "message.provider": "preserved: turn.native.provider",
    "message.model": "preserved: turn.native_model",
    "message.usage": "preserved: state (tokens) + state.native (cost)",
    "message.contextSnapshot": "preserved: state.context_*_tokens",
    "message.stopReason": "preserved: turn.native + state.native",
    "message.stopDetails": "preserved: turn.native.stopDetails",
    "message.duration": "preserved: turn.native + state.native",
    "message.ttft": "preserved: turn.native + state.native",
    "message.completedAt": "preserved: turn.native.completedAt",
    "message.timestamp": "preserved: turn.native.timestamp_unix_ms",
    "message.errorId": "preserved: turn.native.errorId",
    "message.errorStatus": "preserved: turn.native.errorStatus",
    "message.errorMessage": "preserved: turn.native.errorMessage",
    "message.retryRecovery": "preserved: turn.native.retryRecovery",
    "message.useless": "preserved: turn.native.useless",
    "message.prunedAt": "preserved: turn.native.prunedAt",
    "message.providerPayload": "preserved: turn.native.providerPayload",
    "message.responseId": "preserved: turn.native.responseId",
    "message.steering": "preserved: turn.native.steering",
    "message.credentialId": "preserved: turn.native.credentialId",
    "message.synthetic": "preserved: turn.native.synthetic",
    "message.userInitiated": "preserved: turn.native.userInitiated",
    "message.toolCallId": "preserved: tool.call_id + TOOL_RESULT edge",
    "message.toolName": "preserved: tool.native.toolName",
    "message.details": "preserved: turn.native.details (full) + result entity",
    "message.isError": "preserved: tool.is_error + exit_code",
    # -- toolCall block fields --
    "toolCall.id": "preserved: tool.call_id",
    "toolCall.name": "preserved: tool.tool_name",
    "toolCall.arguments": "preserved: tool.command_or_args (superset: includes wire args + intent gloss)",
    "toolCall.thoughtSignature": "preserved: tool.native.thoughtSignature",
    "toolCall.intent": "preserved: tool.native.intent + edge.payload.intent",
    "toolCall.partialArgs": "preserved: tool.native.partialArgs",
    "toolCall.streamIndex": "preserved: tool.native.streamIndex",
    "toolCall.customWireName": "preserved: tool.native.customWireName",
    # -- execution start --
    "custom.customType": "preserved: dispatch",
    "custom_message.customType": "preserved: dispatch",
    "custom:tool_execution_start.data.toolCallId": "preserved: TOOL_CALL edge linkage",
    "custom:tool_execution_start.data.toolName": "preserved: edge.payload",
    "custom:tool_execution_start.data.startedAt": "preserved: edge.payload",
    "custom:tool_execution_start.data.intent": "preserved: edge.payload.intent",
    "custom:tool_execution_start.data.args": "declared: proven carried by toolCall.arguments on this corpus (equal, or the harness's 200-char ellipsis display truncation); the entity carries the full value",
    # -- compaction --
    "compaction.summary": "preserved: state.native.summary",
    "compaction.shortSummary": "preserved: state.native.shortSummary",
    "compaction.details": "preserved: state.native.details",
    "compaction.firstKeptEntryId": "preserved: COMPACTION edge + state anchor",
    "compaction.tokensBefore": "preserved: state.native",
    "compaction.tokensAfter": "preserved: state.native",
    "compaction.method": "preserved: state.native",
    "compaction.fromExtension": "preserved: state.native",
    "compaction.preserveData": "preserved: state.native",
    "compaction.providerReplayThroughEntryId": "preserved: state.native",
    # -- telemetry kept compact --
    "model_usage.purpose": "preserved: meta.usage_telemetry",
    "model_usage.role": "preserved: meta.usage_telemetry",
    "model_usage.api": "preserved: meta.usage_telemetry",
    "model_usage.provider": "preserved: meta.usage_telemetry",
    "model_usage.model": "preserved: meta.usage_telemetry",
    "model_usage.usage": "preserved: meta.usage_telemetry (tokens + cost_total)",
    "model_usage.stopReason": "preserved: meta.usage_telemetry",
    "custom:session_exit.data.reason": "preserved: meta.session_exits",
    "custom:session_exit.data.kind": "preserved: meta.session_exits",
    "custom:session_exit.data.pendingToolCalls": "preserved: meta.session_exits",
    "custom:session_exit.data.recordedAt": "declared: envelope duplicate of chain time",
    "branch_summary.fromId": "preserved: meta.branch_summaries",
    "branch_summary.summary": "preserved: meta.branch_summaries",
    "branch_summary.details": "preserved: meta.branch_summaries",
    "ttsr_injection.injectedRules": "preserved: meta.ttsr_rules",
    "credential_pin.provider": "declared: auth-adjacent; provider counted, hash never persisted",
    "credential_pin.hash": "declared: credential material; presence counted, value never persisted",
    # -- declared noise: scheduler/UI notices, no dialogue semantics --
    "custom_message.content": "declared: scheduler nudge text; inventory by customType in skipped counts",
    "custom_message.details": "declared: scheduler notice payload; re-derivable from frozen capture",
    "custom_message.display": "declared: UI flag, no signal",
    "custom_message.attribution": "declared: always agent on notices",
    "custom:todo_hud_state.data.fingerprint": "declared: UI hud state",
    "custom:todo_hud_state.data.sourceEntryId": "declared: UI hud state",
    "custom:todo_hud_state.data.visibility": "declared: UI hud state",
}
```

## Coverage at retirement

- Silver : 88 sessions.
- Guards removed : capture, closed-world field coverage, normalizer tests.
- Registry entry removed : `UNCONTRACTED["omp"]`
  (`test_conservation_contract.py`) -> `~/.omp/agent/sessions`.

## T7 rebuild pointers

- Scan : `<slug>/<iso>_<uuid>.jsonl`, one file = one parentId chain.
- NATIVE edges : REPLY from parentId, TOOL_CALL from tool_execution_start
  (startedAt), COMPACTION from compaction records, MODEL_CHANGE from
  model_change records.
- Replicate the FIELD_FATE table above as the contract ; malformed fates
  fail closed. Credential rule : count presence, never persist values.
