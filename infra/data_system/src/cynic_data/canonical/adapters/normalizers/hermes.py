"""Hermes normalizer — the reference adapter.

Source: ``/home/user/.hermes/state.db`` (SQLite), table ``messages``.

Native shape (why this harness is the hardest test):
  * ordering by ``messages.id``
  * ``tool_calls`` JSON on assistant turns; ``tool_call_id`` on tool results
  * ``reasoning_content`` chain-of-thought
  * ``compacted`` / ``_compressed_summary`` / ``absorbed_message_uids`` state
  * NO parent pointer -> edges are DERIVED from ordering + tool_call linkage.
    This is exactly the distinction (native vs derived) the contract must model.

Per the operator invariant: harnesses carry different models. We keep the native
model name and native fields on every entity; unobserved fields stay None.
"""

from __future__ import annotations

import datetime
import json
import sqlite3

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

HARNESS = "hermes"


def _iso(ts_unix) -> str | None:
    if not ts_unix:
        return None
    try:
        return datetime.datetime.fromtimestamp(ts_unix).isoformat()
    except (OverflowError, OSError, ValueError):
        return None


class HermesNormalizer:
    harness = HARNESS

    def __init__(self, db_path: str) -> None:
        self.db_path = db_path

    def can_read(self, source_ref: str) -> bool:
        return source_ref.endswith(".db") or source_ref.endswith(".sqlite")

    def normalize(self, raw_session_id: str) -> NormalizedSession:
        session_uid = str(SessionUid(HARNESS, raw_session_id))
        con = sqlite3.connect(f"file:{self.db_path}?immutable=1", uri=True)
        try:
            rows = con.execute(
                """
                SELECT id, role, content, reasoning_content, tool_calls, tool_call_id,
                       finish_reason, timestamp, compacted
                FROM messages
                WHERE session_id = ?
                ORDER BY id ASC
                """,
                (raw_session_id,),
            ).fetchall()
        finally:
            con.close()

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=f"{self.db_path}#{raw_session_id}",
            native_meta={"native_model": None, "ordering": "messages.id"},
        )

        prev_turn: str | None = None
        call_id_to_turn: dict[str, str] = {}
        turn_idx = 0
        tool_idx = 0
        reason_idx = 0
        edge_idx = 0
        state_idx = 0

        for mid, role, content, reasoning_content, tool_calls_json, tool_call_id, finish_reason, ts_unix, compacted in rows:
            t_uid = turn_uid(session_uid, turn_idx)
            ts_iso = _iso(ts_unix)
            role = role or "unknown"

            # ---- edge: derived causal chain (id ordering) ----
            if prev_turn is not None:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "sequence", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.SEQUENCE,
                        source_uids=(prev_turn,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.DERIVED,
                        payload={"rule": "messages.id order"},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1

            # ---- turn ----
            sess.turns.append(
                Turn(
                    turn_uid=t_uid,
                    session_uid=session_uid,
                    index=turn_idx,
                    role=role,
                    content=content or "",
                    timestamp_iso=ts_iso,
                    native_model=None,
                    native_fields={"finish_reason": finish_reason, "native_id": mid},
                )
            )

            # ---- reasoning ----
            if reasoning_content:
                sess.reasoning.append(
                    ReasoningTrace(
                        trace_uid=reasoning_uid(session_uid, reason_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        content=reasoning_content,
                        subject="Hermes CoT",
                        timestamp_iso=ts_iso,
                    )
                )
                reason_idx += 1

            # ---- tool calls (assistant) ----
            if tool_calls_json:
                try:
                    tcalls = json.loads(tool_calls_json)
                except (json.JSONDecodeError, TypeError):
                    tcalls = []
                if isinstance(tcalls, list):
                    for tc in tcalls:
                        fn = tc.get("function", {}) if isinstance(tc, dict) else {}
                        call_id = tc.get("id", "") if isinstance(tc, dict) else ""
                        exec_uid = tool_uid(session_uid, tool_idx)
                        sess.tools.append(
                            ToolCall(
                                exec_uid=exec_uid,
                                session_uid=session_uid,
                                turn_uid=t_uid,
                                index=tool_idx,
                                tool_name=fn.get("name", "tool"),
                                call_id=call_id or None,
                                command_or_args=fn.get("arguments", ""),
                                raw_output=None,
                                timestamp_iso=ts_iso,
                            )
                        )
                        if call_id:
                            call_id_to_turn[call_id] = exec_uid
                        sess.edges.append(
                            Edge(
                                edge_uid=edge_uid(session_uid, "tool_call", edge_idx),
                                session_uid=session_uid,
                                kind=EdgeKind.TOOL_CALL,
                                source_uids=(t_uid,),
                                target_uid=None,
                                provenance=EdgeProvenance.NATIVE,
                                payload={"tool_name": fn.get("name", "tool"), "call_id": call_id},
                                timestamp_iso=ts_iso,
                            )
                        )
                        edge_idx += 1
                        tool_idx += 1

            # ---- tool result (role=tool) ----
            if role == "tool":
                parent = call_id_to_turn.get(tool_call_id or "")
                sess.tools.append(
                    ToolCall(
                        exec_uid=tool_uid(session_uid, tool_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=tool_idx,
                        tool_name="hermes_tool_result",
                        call_id=tool_call_id or None,
                        command_or_args=None,
                        raw_output=content or "",
                        is_error=bool(content and content.lower().startswith("error")),
                        parent_exec_uid=parent,
                        timestamp_iso=ts_iso,
                    )
                )
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "tool_result", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.TOOL_RESULT,
                        source_uids=(parent,) if parent else (t_uid,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE if parent else EdgeProvenance.DERIVED,
                        payload={"tool_call_id": tool_call_id},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1
                tool_idx += 1

            # ---- state snapshot (compaction) ----
            if compacted:
                sess.states.append(
                    StateSnapshot(
                        snapshot_uid=state_uid(session_uid, state_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=state_idx,
                        compacted=True,
                        timestamp_iso=ts_iso,
                    )
                )
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "compaction", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.COMPACTION,
                        source_uids=(t_uid,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"trigger": "hermes_context_compaction"},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1
                state_idx += 1

            # ---- human signal ----
            if role == "user":
                kind, markers = classify_human_turn(content or "")
                sess.human.append(
                    HumanSignal(
                        human_uid=human_uid(session_uid, t_uid),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        text=content or "",
                        kind=kind,
                        markers=markers,
                        timestamp_iso=ts_iso,
                    )
                )

            prev_turn = t_uid
            turn_idx += 1

        sess.check()
        return sess
