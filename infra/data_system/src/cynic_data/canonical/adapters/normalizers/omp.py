"""OMP normalizer.

Source: ``/home/user/.omp/agent/sessions/<slug>/<iso>_<uuid>.jsonl`` — one JSON
object per line, chronological, forming a single ``parentId`` chain (only the
leading records have ``parentId: null``).

Native shape (the NATIVE-pointer family, like Claude Code):
  * ``type`` is ``title`` | ``session`` | ``model_change`` |
    ``thinking_level_change`` | ``message`` | ``custom`` | ``custom_message`` |
    ``compaction`` | ``credential_pin`` | ``title_change`` | ``ttsr_injection``
    | ``session_init`` | ``branch_summary`` | ``model_usage``
  * ``parentId`` -> explicit chain pointer, so REPLY edges are NATIVE
  * ``message.message.role`` is ``user`` | ``assistant`` | ``toolResult`` |
    ``developer`` (tool results ride as their own role, not as blocks)
  * ``message.message.content`` is a LIST of ``text`` | ``thinking`` |
    ``toolCall`` | ``image`` blocks (rarely a bare string)
  * ``toolCall`` block: ``{id, name, arguments, thoughtSignature}``
  * ``toolResult`` message: ``{toolCallId, toolName, content, details, isError}``
  * ``custom/tool_execution_start``: ``{toolCallId, toolName, startedAt, args}``
    — the harness explicitly stating execution start, so TOOL_CALL edges are
    NATIVE and carry ``startedAt``
  * ``message.usage`` (assistant turns): ``{input, output, totalTokens,
    reasoningTokens, cost}`` + ``message.contextSnapshot`` ->
    a genuine StateSnapshot
  * ``compaction`` carries ``summary`` / ``firstKeptEntryId`` / ``tokensBefore``
    -> StateSnapshot(compacted=True) + COMPACTION edge
  * ``model_change`` carries the new ``model`` -> MODEL_CHANGE edge
  * ``thinking`` blocks -> ReasoningTrace each

Skipped with intent (no dialogue semantics; still mapped in the parent chain so
reply resolution never breaks): ``title`` / ``title_change`` (kept in
native_meta), ``session`` (identity), ``session_init``, ``thinking_level_change``
(kept as current level on subsequent turns), ``credential_pin``,
``ttsr_injection``, ``branch_summary``, ``model_usage`` (telemetry),
``custom_message`` and non-execution ``custom`` records (agent notices).

Per the operator invariant: OMP carries its own model. ``message.model`` (or the
last ``model_change``) is kept on every Turn; unobserved fields stay None.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ....kernel.ids import (
    SessionUid,
    edge_uid,
    human_uid,
    reasoning_uid,
    state_uid,
    tool_uid,
    turn_uid,
)
from ...domain.entities import (
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    ReasoningTrace,
    StateSnapshot,
    ToolCall,
    Turn,
)
from ...domain.session import NormalizedSession
from ._human import classify_human_turn

HARNESS = "omp"

# Record types that carry no dialogue semantics. They are mapped in the parent
# chain but never become entities; the list is declared so the review can audit
# it against the raw inventory.
SKIPPED_TYPES = frozenset(
    {
        "title",
        "title_change",
        "session_init",
        "thinking_level_change",
        "credential_pin",
        "ttsr_injection",
        "branch_summary",
        "model_usage",
    }
)

# Every raw field path observed in the corpus maps to exactly one fate.
# Preserved fates name the canonical home; declared fates name the reason the
# bytes stay out of the canonical layer (re-derivable from the frozen capture).
# The closed-world test fails on any observed path missing here, so a new
# harness field can never slip in silently.
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


class OmpNormalizer:
    harness = HARNESS

    def __init__(self, sessions_root: str = "/home/user/.omp/agent/sessions") -> None:
        self.sessions_root = Path(sessions_root)

    def can_read(self, source_ref: str) -> bool:
        # Bound to the adapter's root, not to a hardcoded live path: a frozen
        # capture preserves the relative tree, so the same adapter reads both.
        try:
            inside = Path(source_ref).resolve().is_relative_to(self.sessions_root.resolve())
        except OSError:
            inside = False
        return source_ref.endswith(".jsonl") and inside

    def session_refs(self) -> list[str]:
        """An OMP session is one session JSONL file under the sessions tree."""
        if not self.sessions_root.is_dir():
            return []
        return sorted(str(p) for p in self.sessions_root.rglob("*.jsonl"))

    # -- helpers -----------------------------------------------------------

    @staticmethod
    def _text_blocks(content: Any) -> tuple[str, list[dict[str, Any]]]:
        """Split message content into (joined text, structured blocks)."""
        if isinstance(content, str):
            return content, []
        if not isinstance(content, list):
            return "", []
        texts: list[str] = []
        structured: list[dict[str, Any]] = []
        for block in content:
            if not isinstance(block, dict):
                continue
            btype = block.get("type")
            if btype == "text":
                texts.append(block.get("text", ""))
            elif btype in ("thinking", "toolCall", "image"):
                structured.append(block)
        return "".join(texts), structured

    def normalize(self, session_ref: str) -> NormalizedSession:
        path = Path(session_ref)

        records: list[dict[str, Any]] = []
        with path.open("r", encoding="utf-8", errors="replace") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    records.append(json.loads(line))
                except json.JSONDecodeError:
                    continue

        # -- session identity (first pass over header records) --
        raw_id = path.stem
        cwd: str | None = None
        title: str | None = None
        last_title: str | None = None
        last_title_source: str | None = None
        session_version: Any = None
        session_parent: str | None = None
        for rec in records:
            if rec.get("type") == "session" and rec.get("id"):
                raw_id = rec["id"]
                cwd = rec.get("cwd")
                session_version = rec.get("version")
                session_parent = rec.get("parentSession")
                if rec.get("title"):
                    title = rec["title"]
                    last_title_source = rec.get("titleSource")
            elif rec.get("type") == "title" and rec.get("title"):
                title = rec["title"]
                if rec.get("source"):
                    last_title_source = rec["source"]
        session_uid = str(SessionUid(HARNESS, raw_id))

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=str(path),
            native_meta={
                "ordering": "file line order",
                "parent_pointer": "native parentId chain",
                "cwd": cwd,
                "title": title,
            },
        )

        # Every record id maps to the nearest turn seen so far, so REPLY edges
        # resolve through interleaved config records (model_change, custom, ...)
        # instead of breaking on them.
        chain_to_turn: dict[str, str | None] = {}
        call_id_to_exec: dict[str, str] = {}
        # toolCall blocks awaiting their native tool_execution_start record
        pending_tool_edge: dict[str, tuple[str, str]] = {}

        current_model: str | None = None
        current_thinking: str | None = None
        current_thinking_configured: Any = None
        title_history: list[dict[str, Any]] = []
        session_init_record: dict[str, Any] = {}
        model_history: list[dict[str, Any]] = []
        usage_telemetry: list[dict[str, Any]] = []
        session_exits: list[dict[str, Any]] = []
        branch_summaries: list[dict[str, Any]] = []
        ttsr_rules: list[str] = []
        seen_models: set[str] = set()
        skipped: dict[str, int] = {}
        turn_idx = tool_idx = state_idx = edge_idx = reasoning_idx = 0
        prev_turn: str | None = None

        def reply_edge(t_uid: str, rec: dict[str, Any]) -> None:
            nonlocal edge_idx
            parent_id = rec.get("parentId")
            source = chain_to_turn.get(parent_id) if parent_id else None
            if source is not None:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "reply", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.REPLY,
                        source_uids=(source,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"rule": "parentId"},
                        timestamp_iso=rec.get("timestamp"),
                    )
                )
                edge_idx += 1
            elif prev_turn is not None:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "sequence", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.SEQUENCE,
                        source_uids=(prev_turn,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.DERIVED,
                        payload={"rule": "file line order (parentId unresolvable)"},
                        timestamp_iso=rec.get("timestamp"),
                    )
                )
                edge_idx += 1

        for rec in records:
            rtype = rec.get("type")
            rec_id = rec.get("id")
            ts_iso = rec.get("timestamp")

            if rtype == "message":
                message = rec.get("message") or {}
                role = message.get("role", "")
                text, structured = self._text_blocks(message.get("content"))
                image_blocks = [b for b in structured if b.get("type") == "image"]

                t_uid = turn_uid(session_uid, turn_idx)
                reply_edge(t_uid, rec)
                native_model = message.get("model") or current_model

                native_fields: dict[str, Any] = {
                    "record_id": rec_id,
                    "api": message.get("api"),
                    "provider": message.get("provider"),
                    "stopReason": message.get("stopReason"),
                    "stopDetails": message.get("stopDetails"),
                    "thinkingLevel": current_thinking,
                    "thinkingConfigured": current_thinking_configured,
                    "attribution": message.get("attribution"),
                    "completedAt": message.get("completedAt"),
                    "timestamp_unix_ms": message.get("timestamp"),
                    "duration": message.get("duration"),
                    "ttft": message.get("ttft"),
                    "errorId": message.get("errorId"),
                    "errorStatus": message.get("errorStatus"),
                    "errorMessage": message.get("errorMessage"),
                    "retryRecovery": message.get("retryRecovery"),
                    "useless": message.get("useless"),
                    "prunedAt": message.get("prunedAt"),
                    "providerPayload": message.get("providerPayload"),
                    "responseId": message.get("responseId"),
                    "steering": message.get("steering"),
                    "credentialId": message.get("credentialId"),
                    "synthetic": message.get("synthetic"),
                    "userInitiated": message.get("userInitiated"),
                }
                if image_blocks:
                    native_fields["image_blocks"] = image_blocks
                if role == "toolResult":
                    native_fields["toolName"] = message.get("toolName")
                    native_fields["toolCallId"] = message.get("toolCallId")
                    native_fields["details"] = message.get("details")

                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role=role,
                        content=text,
                        timestamp_iso=ts_iso,
                        native_model=native_model,
                        native_fields={k: v for k, v in native_fields.items() if v is not None},
                    )
                )

                # -- thinking blocks -> reasoning traces --
                for block in structured:
                    if block.get("type") != "thinking":
                        continue
                    sess.reasoning.append(
                        ReasoningTrace(
                            trace_uid=reasoning_uid(session_uid, reasoning_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            content=block.get("thinking", ""),
                            subject=None,
                            model=native_model,
                            timestamp_iso=ts_iso,
                        )
                    )
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "reasoning", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.REASONING,
                            source_uids=(t_uid,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"block": "thinking"},
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                    reasoning_idx += 1

                # -- toolCall blocks -> tool entities (edge on execution start) --
                for block in structured:
                    if block.get("type") != "toolCall":
                        continue
                    call_id = block.get("id", "")
                    exec_uid = tool_uid(session_uid, tool_idx)
                    sess.tools.append(
                        ToolCall(
                            exec_uid=exec_uid,
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            index=tool_idx,
                            tool_name=block.get("name", "tool"),
                            call_id=call_id or None,
                            command_or_args=json.dumps(
                                block.get("arguments", {}), ensure_ascii=False
                            ),
                            raw_output=None,
                            timestamp_iso=ts_iso,
                            native_fields={
                                k: v
                                for k, v in {
                                    "thoughtSignature": block.get("thoughtSignature"),
                                    "intent": block.get("intent"),
                                    "partialArgs": block.get("partialArgs"),
                                    "streamIndex": block.get("streamIndex"),
                                    "customWireName": block.get("customWireName"),
                                }.items()
                                if v is not None
                            },
                        )
                    )
                    if call_id:
                        call_id_to_exec[call_id] = exec_uid
                        pending_tool_edge[call_id] = (t_uid, exec_uid)
                    tool_idx += 1

                # -- toolResult role -> result entity linked to its call --
                if role == "toolResult":
                    call_id = message.get("toolCallId", "")
                    parent_exec = call_id_to_exec.get(call_id)
                    is_err = bool(message.get("isError"))
                    sess.tools.append(
                        ToolCall(
                            exec_uid=tool_uid(session_uid, tool_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            index=tool_idx,
                            tool_name="tool_result",
                            call_id=call_id or None,
                            command_or_args=None,
                            raw_output=text or None,
                            exit_code=1 if is_err else 0,
                            is_error=is_err,
                            parent_exec_uid=parent_exec,
                            timestamp_iso=ts_iso,
                            native_fields={
                                "toolName": message.get("toolName"),
                                "has_details": message.get("details") is not None,
                            },
                        )
                    )
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.TOOL_RESULT,
                            source_uids=(parent_exec,) if parent_exec else (t_uid,),
                            target_uid=t_uid,
                            provenance=EdgeProvenance.NATIVE
                            if parent_exec
                            else EdgeProvenance.DERIVED,
                            payload={"toolCallId": call_id},
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                    tool_idx += 1

                # -- usage -> state snapshot --
                usage = message.get("usage")
                snapshot = message.get("contextSnapshot") or {}
                if isinstance(usage, dict):
                    sess.states.append(
                        StateSnapshot(
                            snapshot_uid=state_uid(session_uid, state_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            index=state_idx,
                            model=native_model,
                            cwd=cwd,
                            context_prompt_tokens=usage.get("input"),
                            context_nonmessage_tokens=snapshot.get("nonMessageTokens"),
                            cumulative_tokens=usage.get("totalTokens"),
                            compacted=None,
                            native_fields={
                                k: v
                                for k, v in {
                                    "output": usage.get("output"),
                                    "cacheRead": usage.get("cacheRead"),
                                    "cacheWrite": usage.get("cacheWrite"),
                                    "reasoningTokens": usage.get("reasoningTokens"),
                                    "cost": usage.get("cost"),
                                    "promptTokens": snapshot.get("promptTokens"),
                                    "duration": message.get("duration"),
                                    "ttft": message.get("ttft"),
                                    "stopReason": message.get("stopReason"),
                                }.items()
                                if v is not None
                            },
                            timestamp_iso=ts_iso,
                        )
                    )
                    state_idx += 1

                # -- human signal (plain user turns only) --
                if role == "user":
                    kind, markers = classify_human_turn(text)
                    sess.human.append(
                        HumanSignal(
                            human_uid=human_uid(session_uid, t_uid),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            text=text,
                            kind=kind,
                            markers=markers,
                            timestamp_iso=ts_iso,
                        )
                    )

                if rec_id:
                    chain_to_turn[rec_id] = t_uid
                prev_turn = t_uid
                turn_idx += 1

            elif rtype == "model_change":
                model = rec.get("model")
                if model:
                    current_model = model
                    seen_models.add(model)
                model_history.append(
                    {
                        "model": model,
                        "resolvedModelIsFallback": rec.get("resolvedModelIsFallback"),
                        "role": rec.get("role"),
                    }
                )
                if prev_turn is not None:
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "model_change", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.MODEL_CHANGE,
                            source_uids=(prev_turn,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={
                                "model": model,
                                "resolvedModelIsFallback": rec.get(
                                    "resolvedModelIsFallback"
                                ),
                            },
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                else:
                    # Session setup before the first turn: the model is kept
                    # (current_model + native_meta models), no turn to anchor to.
                    skipped["model_change:pre-first-turn"] = (
                        skipped.get("model_change:pre-first-turn", 0) + 1
                    )
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "custom" and rec.get("customType") == "session_exit":
                data = rec.get("data") or {}
                session_exits.append(
                    {
                        "reason": data.get("reason"),
                        "kind": data.get("kind"),
                        "pendingToolCalls": data.get("pendingToolCalls"),
                    }
                )
                skipped["custom:session_exit"] = skipped.get("custom:session_exit", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "custom" and rec.get("customType") == "tool_execution_start":
                data = rec.get("data") or {}
                call_id = data.get("toolCallId", "")
                pending = pending_tool_edge.pop(call_id, None)
                source = pending[0] if pending else prev_turn
                if source is not None:
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.TOOL_CALL,
                            source_uids=(source,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={
                                "tool_name": data.get("toolName"),
                                "call_id": call_id,
                                "startedAt": data.get("startedAt"),
                                "intent": data.get("intent"),
                            },
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "compaction":
                kept = rec.get("firstKeptEntryId")
                kept_turn = chain_to_turn.get(kept) if kept else None
                anchor = kept_turn or prev_turn
                sess.states.append(
                    StateSnapshot(
                        snapshot_uid=state_uid(session_uid, state_idx),
                        session_uid=session_uid,
                        turn_uid=anchor,
                        index=state_idx,
                        model=current_model,
                        cwd=cwd,
                        compacted=True,
                        native_fields={
                            k: v
                            for k, v in {
                                "summary": rec.get("summary"),
                                "shortSummary": rec.get("shortSummary"),
                                "details": rec.get("details"),
                                "tokensBefore": rec.get("tokensBefore"),
                                "tokensAfter": rec.get("tokensAfter"),
                                "method": rec.get("method"),
                                "fromExtension": rec.get("fromExtension"),
                                "preserveData": rec.get("preserveData"),
                                "providerReplayThroughEntryId": rec.get(
                                    "providerReplayThroughEntryId"
                                ),
                                "firstKeptEntryId": kept,
                            }.items()
                            if v is not None
                        },
                        timestamp_iso=ts_iso,
                    )
                )
                state_idx += 1
                if anchor is not None:
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "compaction", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.COMPACTION,
                            source_uids=(anchor,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"firstKeptEntryId": kept},
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "title_change":
                last_title = rec.get("title", last_title)
                last_title_source = rec.get("source", last_title_source)
                title_history.append(
                    {
                        "title": rec.get("title"),
                        "source": rec.get("source"),
                        "trigger": rec.get("trigger"),
                        "previousTitle": rec.get("previousTitle"),
                    }
                )
                skipped["title_change"] = skipped.get("title_change", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "ttsr_injection":
                for rule in rec.get("injectedRules") or []:
                    if rule not in ttsr_rules:
                        ttsr_rules.append(rule)
                skipped["ttsr_injection"] = skipped.get("ttsr_injection", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "model_usage":
                usage = rec.get("usage") or {}
                cost = usage.get("cost") or {}
                usage_telemetry.append(
                    {
                        "purpose": rec.get("purpose"),
                        "role": rec.get("role"),
                        "api": rec.get("api"),
                        "provider": rec.get("provider"),
                        "model": rec.get("model"),
                        "input": usage.get("input"),
                        "output": usage.get("output"),
                        "totalTokens": usage.get("totalTokens"),
                        "cost_total": cost.get("total"),
                        "stopReason": rec.get("stopReason"),
                    }
                )
                skipped["model_usage"] = skipped.get("model_usage", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "branch_summary":
                branch_summaries.append(
                    {
                        "fromId": rec.get("fromId"),
                        "summary": rec.get("summary"),
                        "details": rec.get("details"),
                    }
                )
                skipped["branch_summary"] = skipped.get("branch_summary", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype == "session_init":
                session_init_record = {
                    k: v
                    for k, v in {
                        "agent": rec.get("agent"),
                        "modelRole": rec.get("modelRole"),
                        "resolvedModel": rec.get("resolvedModel"),
                        "readOnly": rec.get("readOnly"),
                        "task": rec.get("task"),
                        "tools": rec.get("tools"),
                        "outputSchemaMode": rec.get("outputSchemaMode"),
                        "outputSchema": rec.get("outputSchema"),
                        "spawns": rec.get("spawns"),
                        "systemPrompt": rec.get("systemPrompt"),
                    }.items()
                    if v is not None
                }
                skipped["session_init"] = skipped.get("session_init", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            elif rtype in SKIPPED_TYPES or rtype in ("custom", "custom_message", "session"):
                if rtype in SKIPPED_TYPES:
                    skipped[rtype] = skipped.get(rtype, 0) + 1
                elif rtype == "session":
                    skipped["session"] = skipped.get("session", 0) + 1
                else:
                    skipped[f"{rtype}:{rec.get('customType')}"] = (
                        skipped.get(f"{rtype}:{rec.get('customType')}", 0) + 1
                    )
                if rtype == "thinking_level_change":
                    current_thinking = rec.get("thinkingLevel", current_thinking)
                    if rec.get("configured") is not None:
                        current_thinking_configured = rec.get("configured")
                if rtype == "session" and rec.get("cwd"):
                    cwd = rec["cwd"]
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

            else:
                skipped[rtype or "<missing>"] = skipped.get(rtype or "<missing>", 0) + 1
                if rec_id:
                    chain_to_turn[rec_id] = prev_turn

        # Tool calls whose execution-start record never came: derived edge so the
        # call is still linked instead of silently dangling.
        for call_id, (t_uid, _exec_uid) in pending_tool_edge.items():
            sess.edges.append(
                Edge(
                    edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                    session_uid=session_uid,
                    kind=EdgeKind.TOOL_CALL,
                    source_uids=(t_uid,),
                    target_uid=None,
                    provenance=EdgeProvenance.DERIVED,
                    payload={"call_id": call_id, "rule": "no tool_execution_start seen"},
                )
            )
            edge_idx += 1

        sess.native_meta["models"] = sorted(seen_models)
        sess.native_meta["model_history"] = model_history
        sess.native_meta["last_title"] = last_title or title
        sess.native_meta["last_title_source"] = last_title_source
        sess.native_meta["title_history"] = title_history
        sess.native_meta["session_version"] = session_version
        sess.native_meta["session_parent"] = session_parent
        sess.native_meta["session_init"] = session_init_record
        sess.native_meta["usage_telemetry"] = usage_telemetry
        sess.native_meta["session_exits"] = session_exits
        sess.native_meta["branch_summaries"] = branch_summaries
        sess.native_meta["ttsr_rules"] = ttsr_rules
        sess.native_meta["skipped_record_types"] = skipped

        sess.check()
        return sess
