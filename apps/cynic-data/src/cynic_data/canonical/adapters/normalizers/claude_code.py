"""Claude Code normalizer.

Source: ``/home/user/.claude/projects/<slug>/<session>.jsonl`` — one JSON object per
line, chronological.

Native shape (this harness is the EASY test, unlike Hermes):
  * ``type`` is ``user`` | ``assistant`` | ``file-history-snapshot``
  * ``parentUuid`` -> explicit tree pointer, so REPLY edges are NATIVE, not derived
  * ``isSidechain`` -> subagent turns are explicitly flagged (EdgeKind.SIDECHAIN)
  * ``message.content`` is a LIST of typed blocks: ``text`` | ``tool_use`` | ``tool_result``
  * ``message.usage`` -> per-turn token counters (the only harness that carries them inline)
  * ``toolUseResult`` on the user turn carries the raw result payload

Per the operator invariant: harnesses carry different models. ``message.model`` is
kept on the Turn; unobserved fields stay None.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ....kernel.ids import (
    SessionUid,
    edge_uid,
    human_uid,
    state_uid,
    tool_uid,
    turn_uid,
)
from ...domain.entities import (
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    StateSnapshot,
    ToolCall,
    Turn,
)
from ...domain.session import NormalizedSession
from ._human import classify_human_turn

HARNESS = "claude_code"


class ClaudeCodeNormalizer:
    harness = HARNESS

    def __init__(self, projects_root: str = "/home/user/.claude/projects") -> None:
        self.projects_root = Path(projects_root)

    def can_read(self, source_ref: str) -> bool:
        try:
            inside = Path(source_ref).resolve().is_relative_to(self.projects_root.resolve())
        except OSError:
            inside = False
        return source_ref.endswith(".jsonl") and inside

    def conservation(self):
        """α6: no contract yet — an explicit unknown, never an implicit no-loss.

        This harness's vertical is not done. Until it is, the audit reports this
        adapter as UNCONTRACTED rather than assuming conservation.
        """
        return None

    def session_refs(self) -> list[str]:
        """A Claude Code session is one session JSONL file in the projects tree.

        Subagent transcripts (``*/subagents/*.jsonl``) are included as separate
        sessions with ``is_subagent: true`` in native_meta. They carry rich
        content and are currently orphaned (no inline parent reference), but
        excluding them would be a silent loss.
        """
        if not self.projects_root.is_dir():
            return []
        return sorted(
            str(p)
            for p in self.projects_root.rglob("*.jsonl")
        )

    # -- helpers -----------------------------------------------------------

    @staticmethod
    def _blocks(message: dict[str, Any]) -> list[dict[str, Any]]:
        content = message.get("content")
        if isinstance(content, list):
            return [b for b in content if isinstance(b, dict)]
        if isinstance(content, str):
            return [{"type": "text", "text": content}]
        return []

    def normalize(self, session_ref: str) -> NormalizedSession:
        path = Path(session_ref)
        raw_id = path.stem
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

        is_subagent = "subagents" in path.parts
        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=str(path),
            native_meta={
                "ordering": "file line order",
                "parent_pointer": "native parentUuid",
                "sidechain_flag": "native isSidechain",
                "is_subagent": is_subagent,
            },
        )

        uuid_to_turn: dict[str, str] = {}
        call_id_to_exec: dict[str, str] = {}
        turn_idx = tool_idx = state_idx = edge_idx = 0
        prev_turn: str | None = None

        for rec in records:
            rtype = rec.get("type")
            if rtype not in ("user", "assistant"):
                # file-history-snapshot and friends carry no dialogue semantics
                continue

            t_uid = turn_uid(session_uid, turn_idx)
            ts_iso = rec.get("timestamp")
            message = rec.get("message") or {}
            blocks = self._blocks(message)
            is_sidechain = bool(rec.get("isSidechain"))

            # ---- edges: reply (native parentUuid) + sequence (file order) ----
            parent_uuid = rec.get("parentUuid")
            if parent_uuid and parent_uuid in uuid_to_turn:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "reply", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.REPLY,
                        source_uids=(uuid_to_turn[parent_uuid],),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"rule": "parentUuid"},
                        timestamp_iso=ts_iso,
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
                        payload={"rule": "file line order (no parentUuid match)"},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1

            # ---- turn ----
            usage = message.get("usage") or {}
            sess.turns.append(
                Turn(
                    turn_uid=t_uid,
                    session_uid=session_uid,
                    index=turn_idx,
                    role=rtype,
                    content="".join(
                        b.get("text", "") for b in blocks if b.get("type") == "text"
                    ),
                    timestamp_iso=ts_iso,
                    native_model=message.get("model"),
                    native_fields={
                        "uuid": rec.get("uuid"),
                        "parentUuid": parent_uuid,
                        "isSidechain": is_sidechain,
                        "stop_reason": message.get("stop_reason"),
                        "usage": usage or None,
                    },
                )
            )

            # ---- state snapshot: token usage + cwd + approval mode ----
            if usage or rec.get("cwd"):
                sess.states.append(
                    StateSnapshot(
                        snapshot_uid=state_uid(session_uid, state_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=state_idx,
                        model=message.get("model"),
                        cwd=rec.get("cwd"),
                        approval_policy=rec.get("permissionMode"),
                        context_prompt_tokens=usage.get("input_tokens"),
                        context_nonmessage_tokens=usage.get("cache_read_input_tokens"),
                        cumulative_tokens=None,
                        compacted=None,
                        native_fields={
                            "cache_creation_input_tokens": usage.get("cache_creation_input_tokens"),
                            "output_tokens": usage.get("output_tokens"),
                            "gitBranch": rec.get("gitBranch"),
                        },
                        timestamp_iso=ts_iso,
                    )
                )
                state_idx += 1

            # ---- tool_use (assistant) ----
            for block in blocks:
                if block.get("type") != "tool_use":
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
                        command_or_args=json.dumps(block.get("input", {}), ensure_ascii=False),
                        raw_output=None,
                        timestamp_iso=ts_iso,
                    )
                )
                if call_id:
                    call_id_to_exec[call_id] = exec_uid
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.TOOL_CALL,
                        source_uids=(t_uid,),
                        target_uid=None,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"tool_name": block.get("name", "tool"), "call_id": call_id},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1
                tool_idx += 1

            # ---- tool_result (user turn carries the result) ----
            for block in blocks:
                if block.get("type") != "tool_result":
                    continue
                call_id = block.get("tool_use_id", "")
                parent_exec = call_id_to_exec.get(call_id)
                result_payload = rec.get("toolUseResult")
                is_err = bool(block.get("is_error"))
                sess.tools.append(
                    ToolCall(
                        exec_uid=tool_uid(session_uid, tool_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=tool_idx,
                        tool_name="tool_result",
                        call_id=call_id or None,
                        command_or_args=None,
                        raw_output=json.dumps(result_payload, ensure_ascii=False)
                        if result_payload is not None
                        else None,
                        exit_code=1 if is_err else 0,
                        is_error=is_err,
                        parent_exec_uid=parent_exec,
                        timestamp_iso=ts_iso,
                    )
                )
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.TOOL_RESULT,
                        source_uids=(parent_exec,) if parent_exec else (t_uid,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE if parent_exec else EdgeProvenance.DERIVED,
                        payload={"tool_use_id": call_id},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1
                tool_idx += 1

            # ---- sidechain edge (subagent) ----
            if is_sidechain:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "sidechain", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.SIDECHAIN,
                        source_uids=(t_uid,),
                        target_uid=None,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"flag": "isSidechain"},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1

            # ---- human signal (user turns that are NOT tool results) ----
            if rtype == "user" and not any(b.get("type") == "tool_result" for b in blocks):
                text = "".join(b.get("text", "") for b in blocks if b.get("type") == "text")
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

            if rec.get("uuid"):
                uuid_to_turn[rec["uuid"]] = t_uid
            prev_turn = t_uid
            turn_idx += 1

        sess.check()
        return sess