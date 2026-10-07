"""Hermes-Agent normalizer — the reference vertical.

Source: ``/home/user/.hermes/state.db`` (SQLite), tables ``sessions`` and
``messages``. The producer's schema is the source of truth; this adapter reads
every column that carries signal and declares the rest, per ``hermes_contract``.

Native shape (why this harness is the hardest test):
  * ordering by ``messages.id``
  * ``tool_calls`` JSON on assistant turns; ``tool_call_id`` on tool results
  * NO parent pointer -> edges are DERIVED from ordering + tool_call linkage.
  * **reasoning lives in four homes, chosen by the MODEL, not the harness**:
    ``reasoning``/``reasoning_content`` (plain CoT), ``reasoning_details``
    (structured text), ``codex_reasoning_items`` (encrypted CoT for codex-family
    models). The contract's ``reasoning`` concept resolves the home; a known
    model using an unexpected home is recorded as drift, never silently accepted.
  * **compaction rewrites history in place**: ``active=0`` marks a message a
    compaction replaced (173 696 rows on the live store). It is preserved and
    drives a COMPACTION edge, so live history is never conflated with superseded
    history.
  * the whole ``sessions`` row (62 columns) is kept verbatim under
    ``native_meta.session`` — no per-column entity home would otherwise be a
    silent loss.

Per the operator invariant: harnesses carry different models. We keep the native
model name and native fields on every entity; unobserved fields stay None.
"""

from __future__ import annotations

import datetime
import json
import os
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
from ...domain.conservation import ConceptResolution
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
from .hermes_contract import CONTRACT

HARNESS = "hermes"

# The columns read from ``messages``. Every other column has a declared fate in
# the contract, so nothing is dropped without a named home.
_MESSAGE_COLUMNS = (
    "id", "role", "content", "tool_call_id", "tool_calls", "tool_name",
    "effect_disposition", "timestamp", "finish_reason",
    "reasoning", "reasoning_content", "reasoning_details",
    "codex_reasoning_items", "codex_message_items",
    "platform_message_id", "observed", "_compressed_summary",
    "active", "compacted", "api_content", "display_kind", "display_metadata",
    "display_identity", "display_order", "message_uid",
    "absorbed_message_uids", "tool_call_uids", "tool_call_uid",
)

# The four homes the reasoning concept can live in.
_REASONING_HOMES = ("reasoning_content", "reasoning", "reasoning_details", "codex_reasoning_items")


def _tool_is_error(content: str | None) -> bool:
    """Native error signal: JSON `success: false` or non-empty `error` field."""
    if not content:
        return False
    try:
        d = json.loads(content)
    except (json.JSONDecodeError, TypeError):
        return False
    if not isinstance(d, dict):
        return False
    if d.get("success") is False:
        return True
    err = d.get("error")
    return bool(err)


def _iso(ts_unix) -> str | None:
    if not ts_unix:
        return None
    try:
        return datetime.datetime.fromtimestamp(ts_unix, tz=datetime.timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%S"
        ) + "Z"
    except (OverflowError, OSError, ValueError):
        return None


class HermesNormalizer:
    harness = HARNESS

    def __init__(self, db_path: str) -> None:
        self.db_path = db_path

    def can_read(self, source_ref: str) -> bool:
        return source_ref.endswith(".db") or source_ref.endswith(".sqlite")

    def conservation(self):
        """α6: this adapter's conservation contract, anchored on SCHEMA_VERSION."""
        return CONTRACT

    def _connect(self) -> sqlite3.Connection:
        """Read-only, and tolerant of invalid UTF-8.

        The producer stores arbitrary bytes in TEXT columns; at least 2 rows in
        the live store are not valid UTF-8. Default sqlite3 raises on decode and
        the whole session would be lost. We decode with ``replace`` so the row
        survives (lossy bytes are visible as U+FFFD, never a silent skip).
        """
        con = sqlite3.connect(f"file:{self.db_path}?mode=ro", uri=True)
        con.text_factory = lambda b: b.decode("utf-8", "replace")
        return con

    def session_refs(self) -> list[str]:
        """A Hermes session is a ``messages.session_id`` value in this database."""
        if not os.path.exists(self.db_path):
            return []
        con = self._connect()
        try:
            rows = con.execute(
                "SELECT DISTINCT session_id FROM messages "
                "WHERE session_id IS NOT NULL ORDER BY session_id"
            ).fetchall()
        finally:
            con.close()
        return [r[0] for r in rows]

    def normalize(self, session_ref: str) -> NormalizedSession:
        session_uid = str(SessionUid(HARNESS, session_ref))
        con = self._connect()
        con.row_factory = sqlite3.Row
        try:
            session_row = con.execute(
                "SELECT * FROM sessions WHERE id = ?", (session_ref,)
            ).fetchone()
            usage = con.execute(
                "SELECT model FROM session_model_usage WHERE session_id = ? "
                "GROUP BY model ORDER BY min(first_seen)",
                (session_ref,),
            ).fetchall()
            rows = con.execute(
                f"SELECT {', '.join(_MESSAGE_COLUMNS)} FROM messages "
                "WHERE session_id = ? ORDER BY id ASC",
                (session_ref,),
            ).fetchall()
        finally:
            con.close()

        models_in_session = [u["model"] for u in usage]
        session_meta = dict(session_row) if session_row is not None else {}
        native_model = session_meta.get("model")

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=f"{self.db_path}#{session_ref}",
            native_meta={
                "native_model": native_model,
                "models_in_session": models_in_session,
                "multi_model": len(models_in_session) > 1,
                # the whole session row, verbatim: 62 columns, no silent loss
                "session": session_meta,
                "ordering": "messages.id",
                "producer_schema_version": CONTRACT.producer_schema_version,
                "producer_schema_source": CONTRACT.producer_schema_source,
            },
        )

        reasoning: ConceptResolution | None = CONTRACT.concept("reasoning")

        prev_turn: str | None = None
        call_id_to_exec: dict[str, str] = {}
        turn_idx = 0
        tool_idx = 0
        reason_idx = 0
        edge_idx = 0
        state_idx = 0
        drift: list[str] = []

        for row in rows:
            row = dict(row)  # sqlite3.Row -> dict: Mapping interface for concept.resolve
            t_uid = turn_uid(session_uid, turn_idx)
            ts_iso = _iso(row["timestamp"])
            role = row["role"] or "unknown"

            # ---- sequence edge (derived causal chain) ----
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

            # ---- turn (all signal columns kept; dedicated homes named) ----
            turn_fields = {
                "native_id": row["id"],
                "finish_reason": row["finish_reason"],
                "effect_disposition": row["effect_disposition"],
                "codex_message_items": row["codex_message_items"],
                "platform_message_id": row["platform_message_id"],
                "observed": row["observed"],
                "_compressed_summary": row["_compressed_summary"],
                "active": row["active"],
                "api_content": row["api_content"],
                "display_kind": row["display_kind"],
                "display_metadata": row["display_metadata"],
                "display_identity": row["display_identity"],
                "display_order": row["display_order"],
                "message_uid": row["message_uid"],
                "absorbed_message_uids": row["absorbed_message_uids"],
                "tool_call_uids": row["tool_call_uids"],
            }
            # every reasoning home present on the row is preserved verbatim, so
            # the losing homes are never lost
            for home in _REASONING_HOMES:
                if row[home] is not None:
                    turn_fields[f"reasoning_home.{home}"] = row[home]

            sess.turns.append(
                Turn(
                    turn_uid=t_uid,
                    session_uid=session_uid,
                    index=turn_idx,
                    role=role,
                    content=row["content"] or "",
                    timestamp_iso=ts_iso,
                    native_model=native_model,
                    native_fields=turn_fields,
                )
            )

            # ---- reasoning: resolved across four homes, by model ----
            if reasoning is not None:
                home, value = reasoning.resolve(row)
                if value is not None:
                    expected = reasoning.expected_home(native_model)
                    if expected and expected != home:
                        drift.append(f"{native_model}: reasoning in {home}, expected {expected}")
                    sess.reasoning.append(
                        ReasoningTrace(
                            trace_uid=reasoning_uid(session_uid, reason_idx),
                            session_uid=session_uid,
                            turn_uid=t_uid,
                            content=value if isinstance(value, str) else json.dumps(value),
                            subject=f"Hermes CoT ({home})",
                            model=native_model,
                            timestamp_iso=ts_iso,
                        )
                    )
                    reason_idx += 1

            # ---- tool calls (assistant) ----
            if row["tool_calls"]:
                try:
                    tcalls = json.loads(row["tool_calls"])
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
                                native_fields={"tool_call_uid": row["tool_call_uid"]},
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
                                payload={"tool_name": fn.get("name", "tool"), "call_id": call_id},
                                timestamp_iso=ts_iso,
                            )
                        )
                        edge_idx += 1
                        tool_idx += 1

            # ---- tool result (role=tool) ----
            if role == "tool":
                parent = call_id_to_exec.get(row["tool_call_id"] or "")
                sess.tools.append(
                    ToolCall(
                        exec_uid=tool_uid(session_uid, tool_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=tool_idx,
                        tool_name="tool_result",
                        call_id=row["tool_call_id"] or None,
                        command_or_args=None,
                        raw_output=row["content"] or "",
                        is_error=_tool_is_error(row["content"]),
                        parent_exec_uid=parent,
                        timestamp_iso=ts_iso,
                        native_fields={
                            "native_tool_name": row["tool_name"],
                            "tool_call_uid": row["tool_call_uid"],
                        },
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
                        payload={"tool_call_id": row["tool_call_id"]},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1
                tool_idx += 1

            # ---- state snapshot: compaction + the active flag ----
            compacted = bool(row["compacted"])
            superseded = row["active"] == 0
            if compacted or superseded:
                sess.states.append(
                    StateSnapshot(
                        snapshot_uid=state_uid(session_uid, state_idx),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=state_idx,
                        compacted=True if compacted else None,
                        native_fields={
                            "active": row["active"],
                            "superseded_by_compaction": superseded,
                            "_compressed_summary": row["_compressed_summary"],
                        },
                        timestamp_iso=ts_iso,
                    )
                )
                if compacted:
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
                kind, markers = classify_human_turn(row["content"] or "")
                sess.human.append(
                    HumanSignal(
                        human_uid=human_uid(session_uid, t_uid),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        text=row["content"] or "",
                        kind=kind,
                        markers=markers,
                        timestamp_iso=ts_iso,
                    )
                )

            prev_turn = t_uid
            turn_idx += 1

        if drift:
            sess.native_meta["reasoning_drift"] = drift

        sess.check()
        return sess
