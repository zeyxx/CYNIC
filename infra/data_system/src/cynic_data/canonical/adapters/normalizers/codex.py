"""Codex normalizer.

Source: ``/home/user/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<iso>-<uuid>.jsonl``
— one JSON object per line, each carrying ``type`` + ``payload``.

Native shape:
  * ``type`` is one of ``session_meta`` | ``event_msg`` | ``response_item`` |
    ``world_state`` | ``turn_context``
  * ``session_meta.payload`` carries ``session_id``, ``cwd``, ``cli_version``,
    ``model_provider`` — the session identity + native context
  * ``response_item.payload`` carries ``role`` | ``type`` and is the dialogue
    stream: ``message`` (text) | ``reasoning`` | ``function_call`` | ``function_call_output``
  * ``response_item.payload.summary`` is the reasoning chain-of-thought
  * ``turn_context.payload`` carries ``model``, ``cwd``, ``approval_policy``,
    ``file_system_sandbox_policy``, ``effort`` -> a genuine StateSnapshot
  * NO parent pointer -> sequence edges are DERIVED from line order

This harness names its tool results ``function_call_output`` instead of
``tool_result``; the canonical entity keeps the native name in ``tool_name``
rather than inventing a uniform label.
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

HARNESS = "codex"


class CodexNormalizer:
    harness = HARNESS

    def __init__(self, sessions_root: str = "/home/user/.codex/sessions") -> None:
        self.sessions_root = Path(sessions_root)

    def can_read(self, source_ref: str) -> bool:
        try:
            inside = Path(source_ref).resolve().is_relative_to(self.sessions_root.resolve())
        except OSError:
            inside = False
        return source_ref.endswith(".jsonl") and inside

    def session_refs(self) -> list[str]:
        """A Codex session is one rollout JSONL file under the sessions tree."""
        if not self.sessions_root.is_dir():
            return []
        return sorted(str(p) for p in self.sessions_root.rglob("*.jsonl"))

    @staticmethod
    def _text_of(payload: dict[str, Any]) -> str:
        content = payload.get("content")
        if isinstance(content, str):
            return content
        if isinstance(content, list):
            parts = []
            for part in content:
                if isinstance(part, dict):
                    parts.append(part.get("text") or part.get("input_text") or "")
                elif isinstance(part, str):
                    parts.append(part)
            return "".join(parts)
        return ""

    def normalize(self, session_ref: str) -> NormalizedSession:
        path = Path(session_ref)
        raw_id = path.stem.replace("rollout-", "")
        session_uid = str(SessionUid(HARNESS, raw_id))

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

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=str(path),
            native_meta={"ordering": "file line order", "parent_pointer": None},
        )

        call_id_to_exec: dict[str, str] = {}
        turn_idx = tool_idx = reason_idx = state_idx = edge_idx = 0
        prev_turn: str | None = None
        native_model: str | None = None
        cwd: str | None = None
        session_meta: dict[str, Any] = {}

        for rec in records:
            rtype = rec.get("type")
            payload = rec.get("payload") or {}
            ts_iso = rec.get("timestamp")

            if rtype == "session_meta":
                session_meta = payload
                cwd = payload.get("cwd")
                continue

            # ---- turn_context -> StateSnapshot (model, approval, sandbox) ----
            if rtype == "turn_context":
                native_model = payload.get("model") or native_model
                # Codex ships a rich sandbox *structure*, not a label. StateSnapshot
                # types this column as TEXT, so keep the human-readable kind in the
                # typed column and the full structure in native_fields: nothing is
                # lost, nothing is flattened into a string that was never said.
                sandbox_raw = payload.get("file_system_sandbox_policy")
                sandbox_kind = (
                    sandbox_raw.get("kind")
                    if isinstance(sandbox_raw, dict)
                    else sandbox_raw
                )
                sess.states.append(
                    StateSnapshot(
                        snapshot_uid=state_uid(session_uid, state_idx),
                        session_uid=session_uid,
                        turn_uid=prev_turn,
                        index=state_idx,
                        model=payload.get("model"),
                        cwd=payload.get("cwd") or cwd,
                        approval_policy=payload.get("approval_policy"),
                        sandbox_policy=sandbox_kind,
                        context_prompt_tokens=payload.get("model_context_window"),
                        native_fields={
                            "effort": payload.get("effort"),
                            "collaboration_mode": payload.get("collaboration_mode"),
                            "comp_hash": payload.get("comp_hash"),
                            "file_system_sandbox_policy": sandbox_raw,
                        },
                        timestamp_iso=ts_iso,
                    )
                )
                state_idx += 1
                continue

            if rtype != "response_item":
                # world_state / event_msg carry no canonical dialogue
                continue

            payload_type = payload.get("type")
            role = payload.get("role")

            # ---- reasoning ----
            if payload_type == "reasoning":
                summary = payload.get("summary")
                content = summary if isinstance(summary, str) else json.dumps(summary, ensure_ascii=False)
                encrypted = payload.get("encrypted_content")
                sess.reasoning.append(
                    ReasoningTrace(
                        trace_uid=reasoning_uid(session_uid, reason_idx),
                        session_uid=session_uid,
                        turn_uid=prev_turn,
                        content=content or "",
                        # ReasoningTrace carries no native_fields (fixed SQL columns):
                        # record the encrypted-reasoning fact in the subject instead.
                        subject="Codex CoT (encrypted payload)" if encrypted else "Codex CoT",
                        model=native_model,
                        timestamp_iso=ts_iso,
                    )
                )
                reason_idx += 1
                continue

            # ---- function_call -> tool call ----
            if payload_type in ("function_call", "local_shell_call"):
                call_id = payload.get("call_id") or payload.get("id") or ""
                exec_uid = tool_uid(session_uid, tool_idx)
                args = payload.get("arguments") or payload.get("action") or {}
                sess.tools.append(
                    ToolCall(
                        exec_uid=exec_uid,
                        session_uid=session_uid,
                        turn_uid=prev_turn,
                        index=tool_idx,
                        tool_name=payload.get("name", "tool"),
                        call_id=call_id or None,
                        command_or_args=args if isinstance(args, str)
                        else json.dumps(args, ensure_ascii=False),
                        raw_output=None,
                        timestamp_iso=ts_iso,
                    )
                )
                if call_id:
                    call_id_to_exec[str(call_id)] = exec_uid
                # An edge requires a non-empty source (NormalizedSession.check).
                # A tool call emitted before any turn has no source to point at:
                # record the ToolCall, do not fabricate an edge.
                if prev_turn:
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.TOOL_CALL,
                            source_uids=(prev_turn,),
                            target_uid=None,
                            provenance=EdgeProvenance.NATIVE,
                            payload={"tool_name": payload.get("name", "tool"), "call_id": call_id},
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1
                tool_idx += 1
                continue

            # ---- function_call_output -> tool result ----
            if payload_type in ("function_call_output", "local_shell_call_output"):
                call_id = str(payload.get("call_id") or "")
                parent_exec = call_id_to_exec.get(call_id)
                output = payload.get("output")
                sess.tools.append(
                    ToolCall(
                        exec_uid=tool_uid(session_uid, tool_idx),
                        session_uid=session_uid,
                        turn_uid=prev_turn,
                        index=tool_idx,
                        tool_name="tool_result",
                        call_id=call_id or None,
                        command_or_args=None,
                        raw_output=output if isinstance(output, str)
                        else json.dumps(output, ensure_ascii=False),
                        exit_code=None,
                        is_error=False,
                        parent_exec_uid=parent_exec,
                        timestamp_iso=ts_iso,
                    )
                )
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.TOOL_RESULT,
                        source_uids=(parent_exec,) if parent_exec else ((prev_turn,) if prev_turn else ()),
                        target_uid=prev_turn,
                        provenance=EdgeProvenance.NATIVE if parent_exec else EdgeProvenance.DERIVED,
                        payload={"call_id": call_id},
                        timestamp_iso=ts_iso,
                    )
                ) if (parent_exec or prev_turn) else None
                if parent_exec or prev_turn:
                    edge_idx += 1
                tool_idx += 1
                continue

            # ---- message -> Turn ----
            if payload_type == "message" and role:
                t_uid = turn_uid(session_uid, turn_idx)
                text = self._text_of(payload)

                if prev_turn is not None:
                    sess.edges.append(
                        Edge(
                            edge_uid=edge_uid(session_uid, "sequence", edge_idx),
                            session_uid=session_uid,
                            kind=EdgeKind.SEQUENCE,
                            source_uids=(prev_turn,),
                            target_uid=t_uid,
                            provenance=EdgeProvenance.DERIVED,
                            payload={"rule": "file line order"},
                            timestamp_iso=ts_iso,
                        )
                    )
                    edge_idx += 1

                sess.turns.append(
                    Turn(
                        turn_uid=t_uid,
                        session_uid=session_uid,
                        index=turn_idx,
                        role=role,
                        content=text,
                        timestamp_iso=ts_iso,
                        native_model=native_model,
                        native_fields={"payload_id": payload.get("id"), "phase": payload.get("phase")},
                    )
                )

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

                prev_turn = t_uid
                turn_idx += 1

        sess.native_meta["cli_version"] = session_meta.get("cli_version")
        sess.native_meta["model_provider"] = session_meta.get("model_provider")
        sess.check()
        return sess