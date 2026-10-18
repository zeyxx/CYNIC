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
from ....kernel.errors import SourceSchemaDrift
from ._human import classify_human_turn
from .hermes_contract import CONTRACT

HARNESS = "hermes"

# Tables this normalizer READS. If any is absent from the source, an auxiliary
# read would otherwise degrade to an empty value and silently drop session
# truth — so their absence is a hard failure (fail-closed), not a quiet default.
_REQUIRED_TABLES = (
    "messages",
    "sessions",
    "session_model_usage",
    "system_prompts",
    "async_delegations",
    "state_meta",
)

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


def _json_safe(value):
    """Make a raw column value JSON-serializable without losing information.

    ``messages.display_identity`` is a BLOB (a fixed-width sha256 digest the
    producer uses for indexed display-generation lookup). ``json.dumps`` refuses
    ``bytes``, so a turn carrying this column would crash the canonical write for
    the whole session (measured: 85 675 rows). Encode the bytes as hex — lossless
    and reversible with ``bytes.fromhex`` — and leave every other type untouched.
    """
    if isinstance(value, (bytes, bytearray)):
        return bytes(value).hex()
    return value


def _parse_uid_list(raw: str | None) -> list[str]:
    """Parse a producer JSON array of uids (``absorbed_message_uids``).

    Returns ``[]`` for absent/malformed input — a parse failure must never crash
    the session, and an empty list is the honest "nothing declared".
    """
    if not raw:
        return []
    try:
        parsed = json.loads(raw)
    except (json.JSONDecodeError, TypeError):
        return []
    if not isinstance(parsed, list):
        return []
    return [str(u) for u in parsed if u]


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

        Fail-closed: if the source is missing any table this normalizer reads,
        raise ``SourceSchemaDrift``. A missing in-scope table would otherwise
        make an auxiliary read return empty and silently drop session truth.
        """
        con = sqlite3.connect(f"file:{self.db_path}?mode=ro", uri=True)
        con.text_factory = lambda b: b.decode("utf-8", "replace")
        present = {
            r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type = 'table'")
        }
        missing = [t for t in _REQUIRED_TABLES if t not in present]
        if missing:
            con.close()
            raise SourceSchemaDrift(
                f"{self.db_path}: source is missing in-scope table(s) {missing}; "
                f"the contract was derived from a different schema"
            )
        return con

    def session_refs(self) -> list[str]:
        """Every session the producer declares — the ``sessions`` table, unioned
        with any ``messages.session_id`` that has no ``sessions`` row.

        Enumerating from ``messages`` alone silently dropped 366 sessions that
        carry NO messages but DO carry session-level truth (347 with tokens/cost,
        344 with ``session_model_usage`` telemetry, 24 with a title). The
        ``sessions`` table is the producer's own session registry; an orphan
        message (1 row on this store) is included too so its content is not lost.
        """
        if not os.path.exists(self.db_path):
            return []
        con = self._connect()
        try:
            rows = con.execute(
                "SELECT id FROM sessions "
                "UNION "
                "SELECT DISTINCT session_id FROM messages WHERE session_id IS NOT NULL "
                "ORDER BY 1"
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
                "SELECT * FROM session_model_usage WHERE session_id = ? "
                "ORDER BY first_seen",
                (session_ref,),
            ).fetchall()
            rows = con.execute(
                f"SELECT {', '.join(_MESSAGE_COLUMNS)} FROM messages "
                "WHERE session_id = ? ORDER BY id ASC",
                (session_ref,),
            ).fetchall()
            # Producer's logical display order. Compaction generations copy the
            # protected tail into a new generation with HIGHER ids, so physical
            # id order is NOT the logical order the producer shows. display_order
            # groups the generations of one logical message; the read path orders
            # by it and dedupes by display_identity. We keep it to re-derive the
            # logical position (measured: 30 sessions where id-order diverges).
            try:
                display_orders = {
                    r[0]: r[1]
                    for r in con.execute(
                        "SELECT id, display_order FROM messages WHERE session_id = ?",
                        (session_ref,),
                    )
                }
            except sqlite3.Error:
                display_orders = {}

            # system prompt TEXT lives in ``system_prompts``, keyed by hash;
            # ``sessions.system_prompt`` is usually NULL. Resolve the exact text.
            # Table presence is guaranteed by _connect() (fail-closed).
            system_prompt_resolved = None
            sp_hash = session_row["system_prompt_hash"] if session_row is not None else None
            if sp_hash:
                sp = con.execute(
                    "SELECT hash, prompt FROM system_prompts WHERE hash = ?", (sp_hash,)
                ).fetchone()
                if sp is not None:
                    system_prompt_resolved = {"hash": sp["hash"], "prompt": sp["prompt"]}

            # delegation graph: rows whose origin session is this one. The live
            # process owner is operational, not session truth, so it is dropped
            # here (declared out in the contract), the rest kept verbatim.
            delegations = [
                {k: v for k, v in dict(r).items() if k not in ("owner_pid", "owner_started_at")}
                for r in con.execute(
                    "SELECT * FROM async_delegations WHERE origin_session = ? "
                    "ORDER BY dispatched_at",
                    (session_ref,),
                ).fetchall()
            ]

            # store provenance (store instance id, creation time, ...)
            store_meta = {
                r[0]: r[1]
                for r in con.execute("SELECT key, value FROM state_meta").fetchall()
            }
        finally:
            con.close()

        # Dense rank of the producer's display_order: the logical index. A session
        # without compaction has display_order == id, so this equals the physical
        # index there; where generations exist, it is the producer's true order.
        _distinct_orders = sorted({o for o in display_orders.values() if o is not None})
        _order_rank = {o: i for i, o in enumerate(_distinct_orders)}

        models_in_session = list(dict.fromkeys(u["model"] for u in usage))
        # the whole telemetry row, verbatim: per-model token/cost accounting is
        # the independent oracle for the token claims (session counters can drift)
        model_usage = [dict(u) for u in usage]
        session_meta = dict(session_row) if session_row is not None else {}
        native_model = session_meta.get("model")

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=f"{self.db_path}#{session_ref}",
            native_meta={
                "native_model": native_model,
                "models_in_session": models_in_session,
                "model_usage": model_usage,
                "multi_model": len(models_in_session) > 1,
                # the whole session row, verbatim: 62 columns, no silent loss
                "session": session_meta,
                # the exact system prompt text (from system_prompts via hash)
                "system_prompt_resolved": system_prompt_resolved,
                # the delegation graph for this origin session (rows verbatim)
                "async_delegations": delegations,
                # store provenance (instance id, created_at)
                "store_meta": store_meta,
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
                # the producer's logical position (display_order rank), so a
                # reader never mistakes physical id order for the shown order
                "logical_index": _order_rank.get(row["display_order"]),
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
                # BLOB in the producer; hex-encoded so the canonical write survives
                "display_identity": _json_safe(row["display_identity"]),
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
                    content=row["content"],  # None and "" are distinct raw states
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
                        # None and "" are DISTINCT raw states (operator invariant);
                        # never collapse one into the other.
                        raw_output=row["content"],
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

            # ---- n-ary compaction edge: which turns this row absorbed ----
            # absorbed_message_uids names the logical messages folded into this
            # one (a JSON array of message_uid values). It is the producer's own
            # declaration of an n-ary relation — preserve it as an explicit edge,
            # never as loose text, so the fold is navigable.
            absorbed = _parse_uid_list(row["absorbed_message_uids"])
            if absorbed:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "compaction_absorb", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.COMPACTION,
                        source_uids=tuple(absorbed),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.NATIVE,
                        payload={"absorbed_message_uids": absorbed},
                        timestamp_iso=ts_iso,
                    )
                )
                edge_idx += 1

            # ---- human signal ----
            if role == "user":
                kind, markers = classify_human_turn(row["content"] or "")
                sess.human.append(
                    HumanSignal(
                        human_uid=human_uid(session_uid, t_uid),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        # keep the raw value: None stays None, "" stays ""
                        text=row["content"],
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
