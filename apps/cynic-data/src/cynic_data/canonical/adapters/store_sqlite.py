"""SQLite adapter for CanonicalStorePort.

The canonical layer is AUTHORITATIVE: this store must never lose a fact and never
invent one. Writes are idempotent by primary key; unobserved fields stay NULL.

Payloads larger than 1 GiB are externalized to a content-addressed blob store
(the Python sqlite3 binding refuses parameters above INT_MAX bytes). The database
then holds the hash in place of the text; the blob store holds the bytes. Nothing
is lost, and the content is retrievable by its hash.
"""

from __future__ import annotations

import datetime
import json
import sqlite3

from ..domain.entities import (
    CorrectionKind,
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    ReasoningTrace,
    StateSnapshot,
    ToolCall,
    Turn,
)
from ..domain.session import NormalizedSession
from .content_store import ContentBlobStore
from .store_schema import SCHEMA_SQL


def _j(value) -> str | None:
    """JSON-encode a mapping/sequence, or None."""
    if value is None:
        return None
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


class SqliteCanonicalStore:
    def __init__(self, db_path: str, blob_root: str | None = None, bulk_load: bool = False) -> None:
        self.db_path = db_path
        self.blob_root = blob_root or db_path + ".blobs"
        self._blobs = ContentBlobStore(self.blob_root)
        self._con = sqlite3.connect(db_path)
        if bulk_load:
            # Bulk load: no WAL, no fsync. Safe because the source is a frozen
            # capture — if the load dies, we re-capture and re-run.
            self._con.execute("PRAGMA journal_mode = OFF;")
            self._con.execute("PRAGMA synchronous = OFF;")
            self._con.execute("PRAGMA cache_size = -200000;")  # 200 MB
        else:
            self._con.execute("PRAGMA journal_mode = WAL;")
            self._con.execute("PRAGMA synchronous = NORMAL;")
        self._con.executescript(SCHEMA_SQL)
        self._con.commit()

    def close(self) -> None:
        self._con.close()

    # -- payload handling ----------------------------------------------------

    def _put_payload(self, text: str | None) -> str | None:
        """Store a payload, externalizing it if too large. Returns the value to
        persist in the database: the text itself (inline) or its content hash.

        None is stored as NULL, "" as "" — the two are distinct raw states and
        collapsing them would be a silent loss."""
        if text is None:
            return None
        digest, inlined = self._blobs.put(text)
        if inlined:
            return text
        # externalized: record the blob, store the hash in the base
        self._con.execute(
            "INSERT OR IGNORE INTO blobs (content_hash, size_bytes, created_at_iso) VALUES (?,?,?)",
            (digest, len(text.encode("utf-8")), datetime.datetime.now(datetime.timezone.utc).isoformat()),
        )
        return digest

    def _get_payload(self, value: str | None) -> str | None:
        """Resolve a stored value: inline text or a blob hash."""
        if value is None:
            return None
        if value.startswith("sha256:"):
            return self._blobs.get(value)
        return value

    # -- write ---------------------------------------------------------------

    def write(self, session: NormalizedSession) -> None:
        cur = self._con.cursor()
        cur.execute(
            "INSERT OR REPLACE INTO sessions (session_uid, harness, source_ref, native_meta) VALUES (?,?,?,?)",
            (session.session_uid, session.harness, session.source_ref, _j(session.native_meta)),
        )
        # idempotent re-write: clear this session's rows first
        for table in ("turns", "tools", "reasoning", "edges", "states", "human_signals"):
            cur.execute(f"DELETE FROM {table} WHERE session_uid = ?", (session.session_uid,))

        cur.executemany(
            "INSERT INTO turns (turn_uid, session_uid, idx, role, content, timestamp_iso, native_model, native_fields) VALUES (?,?,?,?,?,?,?,?)",
            [
                (t.turn_uid, t.session_uid, t.index, t.role, self._put_payload(t.content), t.timestamp_iso, t.native_model, _j(t.native_fields))
                for t in session.turns
            ],
        )
        cur.executemany(
            "INSERT INTO tools (exec_uid, session_uid, turn_uid, idx, tool_name, call_id, command_or_args, raw_output, exit_code, is_error, parent_exec_uid, timestamp_iso, native_fields) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            [
                (t.exec_uid, t.session_uid, t.turn_uid, t.index, t.tool_name, t.call_id,
                 self._put_payload(t.command_or_args), self._put_payload(t.raw_output),
                 t.exit_code, int(t.is_error), t.parent_exec_uid, t.timestamp_iso, _j(t.native_fields))
                for t in session.tools
            ],
        )
        cur.executemany(
            "INSERT INTO reasoning (trace_uid, session_uid, turn_uid, content, subject, model, timestamp_iso) VALUES (?,?,?,?,?,?,?)",
            [(r.trace_uid, r.session_uid, r.turn_uid, self._put_payload(r.content), r.subject, r.model, r.timestamp_iso) for r in session.reasoning],
        )
        cur.executemany(
            "INSERT INTO edges (edge_uid, session_uid, kind, source_uids, target_uid, provenance, payload, timestamp_iso) VALUES (?,?,?,?,?,?,?,?)",
            [
                (e.edge_uid, e.session_uid, e.kind.value, _j(list(e.source_uids)), e.target_uid,
                 e.provenance.value, _j(e.payload), e.timestamp_iso)
                for e in session.edges
            ],
        )
        cur.executemany(
            "INSERT INTO states (snapshot_uid, session_uid, turn_uid, idx, model, cwd, sandbox_policy, approval_policy, context_prompt_tokens, context_nonmessage_tokens, cumulative_tokens, compacted, native_fields, timestamp_iso) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            [
                (st.snapshot_uid, st.session_uid, st.turn_uid, st.index, st.model, st.cwd,
                 st.sandbox_policy, st.approval_policy, st.context_prompt_tokens,
                 st.context_nonmessage_tokens, st.cumulative_tokens,
                 None if st.compacted is None else int(st.compacted), _j(st.native_fields), st.timestamp_iso)
                for st in session.states
            ],
        )
        cur.executemany(
            "INSERT INTO human_signals (human_uid, session_uid, turn_uid, text, kind, markers, timestamp_iso) VALUES (?,?,?,?,?,?,?)",
            [(h.human_uid, h.session_uid, h.turn_uid, self._put_payload(h.text), h.kind.value, _j(list(h.markers)), h.timestamp_iso) for h in session.human],
        )
        self._con.commit()

    # -- read ----------------------------------------------------------------

    def read(self, session_uid: str) -> NormalizedSession | None:
        cur = self._con.cursor()
        row = cur.execute("SELECT harness, source_ref, native_meta FROM sessions WHERE session_uid = ?", (session_uid,)).fetchone()
        if not row:
            return None
        harness, source_ref, native_meta = row
        s = NormalizedSession(
            session_uid=session_uid,
            harness=harness,
            source_ref=source_ref,
            native_meta=json.loads(native_meta) if native_meta else {},
        )
        for r in cur.execute("SELECT turn_uid, session_uid, idx, role, content, timestamp_iso, native_model, native_fields FROM turns WHERE session_uid = ? ORDER BY idx", (session_uid,)):
            s.turns.append(Turn(r[0], r[1], r[2], r[3], self._get_payload(r[4]), r[5], r[6], json.loads(r[7]) if r[7] else {}))
        for r in cur.execute("SELECT exec_uid, session_uid, turn_uid, idx, tool_name, call_id, command_or_args, raw_output, exit_code, is_error, parent_exec_uid, timestamp_iso, native_fields FROM tools WHERE session_uid = ? ORDER BY idx", (session_uid,)):
            s.tools.append(ToolCall(r[0], r[1], r[2], r[3], r[4], r[5], self._get_payload(r[6]), self._get_payload(r[7]), r[8], bool(r[9]), r[10], r[11], json.loads(r[12]) if r[12] else {}))
        for r in cur.execute("SELECT trace_uid, session_uid, turn_uid, content, subject, model, timestamp_iso FROM reasoning WHERE session_uid = ?", (session_uid,)):
            s.reasoning.append(ReasoningTrace(r[0], r[1], r[2], self._get_payload(r[3]) or "", r[4], r[5], r[6]))
        for r in cur.execute("SELECT edge_uid, session_uid, kind, source_uids, target_uid, provenance, payload, timestamp_iso FROM edges WHERE session_uid = ?", (session_uid,)):
            s.edges.append(Edge(r[0], r[1], EdgeKind(r[2]), tuple(json.loads(r[3])), r[4], EdgeProvenance(r[5]), json.loads(r[6]) if r[6] else {}, r[7]))
        for r in cur.execute("SELECT snapshot_uid, session_uid, turn_uid, idx, model, cwd, sandbox_policy, approval_policy, context_prompt_tokens, context_nonmessage_tokens, cumulative_tokens, compacted, native_fields, timestamp_iso FROM states WHERE session_uid = ? ORDER BY idx", (session_uid,)):
            s.states.append(StateSnapshot(r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10], None if r[11] is None else bool(r[11]), json.loads(r[12]) if r[12] else {}, r[13]))
        for r in cur.execute("SELECT human_uid, session_uid, turn_uid, text, kind, markers, timestamp_iso FROM human_signals WHERE session_uid = ?", (session_uid,)):
            s.human.append(HumanSignal(r[0], r[1], r[2], self._get_payload(r[3]), CorrectionKind(r[4]), tuple(json.loads(r[5])) if r[5] else (), r[6]))
        return s

    def count(self, table: str) -> int:
        if table not in {"sessions", "turns", "tools", "reasoning", "edges", "states", "human_signals", "blobs"}:
            raise ValueError(f"unknown table {table}")
        return self._con.execute(f"SELECT count(*) FROM {table}").fetchone()[0]

    def session_uids(self) -> list[str]:
        return [r[0] for r in self._con.execute("SELECT session_uid FROM sessions ORDER BY session_uid")]
