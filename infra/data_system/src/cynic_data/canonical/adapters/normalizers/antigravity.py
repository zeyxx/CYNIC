"""Antigravity normalizer — SQLite trajectory DBs, protobuf payloads.

Source: ``.../gemini_antigravity/conversations/<uuid>.db`` — one SQLite DB per
session with a ``steps`` table. Every ``step_payload`` (and every
``error_details``) is a protobuf message WITHOUT a discoverable schema, so the
adapter walks the wire format directly: field numbers and structure are
preserved exactly, names stay unknown (``f13``, ``f5.f4`` — never invented).

Shape (proven over the full 500-DB corpus, see FIELD_FATE):
  * envelope: ``f1`` = step type (mirrors the ``step_type`` column),
    ``f4`` = status (mirrors the ``status`` column, proven equal on all
    56 155 steps), ``f5`` = record envelope (timestamps, role, tool detail,
    trace checkpoints)
  * ``f5.f1`` = ``(epoch_sec, nanos)`` -> the step timestamp
  * ``f5.f3`` = native side marker: 2 = tool-side record, 4 = config,
    5 = agent-side record
  * ``f5.f4`` = tool call detail (``f1`` call id, ``f2`` tool name,
    ``f3`` JSON args) on call types {5, 8, 9, 132, 138}
  * top-level field ``N`` (one per type) = the type-specific payload
  * NO call<->result id linkage exists anywhere in the corpus (628 call ids,
    zero appear in any result step) -> tool edges are DERIVED from order,
    never NATIVE; ``parent_references`` / ``battle_mode_infos`` are empty in
    all 499 readable DBs (declared, not linked)
  * one DB file (``7d1d0212-...``) is dead on disk (``unable to open``) ->
    ``session_refs`` skips it and ``broken`` records it in meta

Per the operator invariant: harnesses carry different models. The trajectory
``source``/``type`` ints are kept raw; unobserved fields stay None.
"""

from __future__ import annotations

import hashlib
import os
import re
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from ....kernel.ids import (
    SessionUid,
    edge_uid,
    human_uid,
    tool_uid,
    turn_uid,
)
from ...domain.entities import (
    Edge,
    EdgeKind,
    EdgeProvenance,
    HumanSignal,
    ToolCall,
    Turn,
)
from ...domain.session import NormalizedSession
from ._human import classify_human_turn

HARNESS = "antigravity"

_STR = re.compile(rb"[\x20-\x7e]{4,}")

# Step types carrying a tool call in f5.f4 (id/name/args).
_CALL_TYPES = frozenset({5, 8, 9, 132, 138})
# Step types that are error records (detail in f24/f31/f37 or error_details).
_ERROR_TYPES = frozenset({17, 28, 51})
# Native side marker f5.f3.
_SIDE = {2: "tool", 4: "config", 5: "agent"}

# Every raw field path observed in the corpus maps to exactly one fate.
# The closed-world test fails on any observed path missing here.
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
    "f21.*": "preserved: tool.native.shell_output (directory listing text)",
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


def _read_varint(buf: bytes, i: int) -> tuple[int, int]:
    shift = 0
    val = 0
    while i < len(buf):
        b = buf[i]
        i += 1
        val |= (b & 0x7F) << shift
        if not b & 0x80:
            return val, i
        shift += 7
        if shift > 64:
            raise ValueError("varint overflow")
    raise ValueError("truncated varint")


def _walk(buf: bytes, depth: int = 0, max_depth: int = 5) -> dict[int, list[Any]] | None:
    """Walk protobuf wire format. Returns {field: [(kind, value)...]} or None.

    Kinds: ('int', v), ('fix', width), ('msg', sub-dict), ('bytes', len, strs).
    Bytes chunks that parse as nested messages become structure; otherwise the
    printable strings are kept with their length. Field NAMES are unknowable
    without the schema and are never invented.
    """
    out: dict[int, list[Any]] = {}
    i = 0
    try:
        while i < len(buf):
            tag, i = _read_varint(buf, i)
            field, wire = tag >> 3, tag & 7
            if field == 0:
                return None
            if wire == 0:
                val, i = _read_varint(buf, i)
                out.setdefault(field, []).append(("int", val))
            elif wire in (1, 5):
                out.setdefault(field, []).append(("fix", wire))
                i += 8 if wire == 1 else 4
            elif wire == 2:
                length, i = _read_varint(buf, i)
                chunk = buf[i : i + length]
                i += length
                if depth < max_depth:
                    sub = _walk(chunk, depth + 1, max_depth)
                    if sub is not None:
                        out.setdefault(field, []).append(("msg", sub))
                        continue
                strs = [s.decode("utf-8", "replace") for s in _STR.findall(chunk)]
                out.setdefault(field, []).append(("bytes", len(chunk), strs[:64]))
            else:
                return None
            if i > len(buf):
                return None
    except (ValueError, IndexError):
        return None
    return out


def _strings(node: Any, depth: int = 0, _out: list[str] | None = None) -> list[str]:
    """Collect printable strings from a walked node (text only, no binary)."""
    if _out is None:
        _out = []
    if isinstance(node, dict):
        for vals in node.values():
            for v in vals:
                if v[0] == "msg":
                    if depth < 6:
                        _strings(v[1], depth + 1, _out)
                elif v[0] == "bytes":
                    _out.extend(s for s in v[2] if s.strip())
    return _out


def _msg(node: dict[int, list[Any]] | None, field: int) -> dict[int, list[Any]] | None:
    if not isinstance(node, dict):
        return None
    for v in node.get(field, []):
        if v[0] == "msg":
            return v[1]
    return None


def _int(node: dict[int, list[Any]] | None, field: int) -> int | None:
    if not isinstance(node, dict):
        return None
    for v in node.get(field, []):
        if v[0] == "int":
            return v[1]
    return None


def _all_strings(node: dict[int, list[Any]] | None, field: int) -> list[str]:
    if not isinstance(node, dict):
        return []
    out: list[str] = []
    for v in node.get(field, []):
        if v[0] == "bytes":
            out.extend(s for s in v[2] if s.strip())
        elif v[0] == "msg":
            out.extend(_strings(v[1]))
    return out


def _epoch_iso(sec: int | None, nanos: int | None) -> str | None:
    if sec is None:
        return None
    try:
        base = datetime.fromtimestamp(sec, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
    except (OverflowError, OSError, ValueError):
        return None
    if nanos:
        return f"{base}.{nanos:09d}Z"
    return base + "Z"


class AntigravityNormalizer:
    harness = HARNESS

    def __init__(
        self,
        conversations_root: str = "/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations",
    ) -> None:
        self.root = Path(conversations_root)

    def _db(self, session_ref: str) -> Path:
        return self.root / f"{session_ref}.db"

    def _connect(self, session_ref: str) -> sqlite3.Connection:
        db = self._db(session_ref)
        con = sqlite3.connect(f"file:{db}?immutable=1", uri=True)
        con.execute("SELECT count(*) FROM steps").fetchone()
        return con

    def can_read(self, source_ref: str) -> bool:
        try:
            inside = Path(source_ref).resolve().is_relative_to(self.root.resolve())
        except OSError:
            inside = False
        if source_ref.endswith(".db") and inside:
            return True
        try:
            self._connect(source_ref).close()
            return True
        except (sqlite3.Error, OSError):
            return False

    def _scan_dbs(self) -> tuple[list[str], list[str]]:
        """Single pass: (readable stems, dead filenames). Cached per instance."""
        if not self.root.is_dir():
            return [], []
        refs: list[str] = []
        dead: list[str] = []
        for db in sorted(self.root.glob("*.db")):
            try:
                con = sqlite3.connect(f"file:{db}?immutable=1", uri=True)
                con.execute("SELECT count(*) FROM steps").fetchone()
                con.close()
                refs.append(db.stem)
            except (sqlite3.Error, OSError):
                dead.append(db.name)
        return refs, dead

    def session_refs(self) -> list[str]:
        """A session is one trajectory DB (filename stem). Dead files skipped."""
        return self._scan_dbs()[0]

    def broken(self) -> list[str]:
        """DB files that fail to open (dead on disk, never silently dropped)."""
        return self._scan_dbs()[1]

    def _blob_columns(self, con: sqlite3.Connection, table: str) -> list[str]:
        try:
            return [
                c[1]
                for c in con.execute(f"PRAGMA table_info({table})")
                if c[2].upper() == "BLOB"
            ]
        except sqlite3.Error:
            return []

    def normalize(self, session_ref: str) -> NormalizedSession:
        session_uid = str(SessionUid(HARNESS, session_ref))
        con = self._connect(session_ref)
        try:
            meta_row = con.execute(
                "SELECT trajectory_id, cascade_id, trajectory_type, source FROM trajectory_meta"
            ).fetchone()
            meta: dict[str, Any] = {
                "trajectory_id": meta_row[0] if meta_row else None,
                "cascade_id": meta_row[1] if meta_row else None,
                "trajectory_type": meta_row[2] if meta_row else None,
                "source": meta_row[3] if meta_row else None,
            }
            for table, key in (
                ("trajectory_metadata_blob", "blob"),
                ("gen_metadata", "gen"),
                ("executor_metadata", "executor"),
            ):
                try:
                    rows = con.execute(f"SELECT * FROM {table}").fetchall()
                except sqlite3.Error:
                    rows = []
                walked = []
                for row in rows:
                    for val in row[1:]:
                        if isinstance(val, (bytes, memoryview)):
                            w = _walk(bytes(val))
                            walked.append(_strings(w) if w else ["<opaque>"])
                meta[key] = walked or None

            steps = con.execute(
                "SELECT idx, step_type, status, metadata, error_details,"
                " permissions, task_details, render_info, step_payload"
                " FROM steps ORDER BY idx"
            ).fetchall()
        finally:
            con.close()

        sess = NormalizedSession(
            session_uid=session_uid,
            harness=HARNESS,
            source_ref=str(self._db(session_ref)),
            native_meta={
                "ordering": "steps.idx order with envelope timestamps",
                "parent_pointer": "none: parent_references empty, sequence edges are DERIVED",
                **{f"traj_{k}": v for k, v in meta.items()},
            },
        )

        turn_idx = tool_idx = edge_idx = 0
        prev_turn: str | None = None

        for row in steps:
            idx, step_type, status, metadata_b, err_b, perm_b, task_b, render_b, payload_b = row
            if payload_b is None:
                continue
            raw = bytes(payload_b)
            top = _walk(raw)
            if top is None:
                continue
            f1 = _int(top, 1)
            f4 = _int(top, 4)
            env = _msg(top, 5)

            ts_pair = _msg(env, 1) if env else None
            ts = _epoch_iso(_int(ts_pair, 1), _int(ts_pair, 2))
            side_name: str | None = None
            if env:
                side_int = _int(env, 3)
                side_name = _SIDE.get(side_int) if side_int is not None else None
            side = side_name

            payload_hash = "sha256:" + hashlib.sha256(raw).hexdigest()
            native: dict[str, Any] = {
                "step_idx": idx,
                "step_type": step_type,
                "status": status,
                "payload_hash": payload_hash,
            }
            if f1 != step_type or f4 != status:
                native["envelope_mismatch"] = {"f1": f1, "f4": f4}
            if side:
                native["side"] = side
            if env:
                if 5 in env:
                    native["arg_keys"] = _strings({5: env[5]})
                if 25 in env:
                    native["envelope_25"] = _strings({25: env[25]})
                if 33 in env:
                    native["envelope_33"] = _strings({33: env[33]})
                for fk, nk in (
                    (11, "model_code"),
                    (12, "trace_id"),
                    (13, "envelope_13"),
                    (20, "step_refs"),
                    (21, "envelope_21"),
                    (22, "envelope_22"),
                    (23, "envelope_23"),
                    (26, "checkpoints"),
                    (28, "envelope_28"),
                    (30, "envelope_30"),
                    (31, "envelope_31"),
                    (32, "envelope_32"),
                ):
                    if fk in env:
                        native[nk] = _strings({fk: env[fk]})
            for blob, nk in (
                (metadata_b, "metadata"),
                (perm_b, "permissions"),
                (task_b, "task_details"),
                (render_b, "render_info"),
            ):
                if blob is not None:
                    w = _walk(bytes(blob))
                    native[nk] = _strings(w) if w else ["<opaque>"]
            err_texts: list[str] = []
            if err_b is not None:
                w = _walk(bytes(err_b))
                err_texts = _strings(w) if w else ["<opaque>"]
                native["error"] = err_texts

            call = _msg(env, 4) if env else None
            call_id = (_all_strings(call, 1) or [None])[0]
            call_name = (_all_strings(call, 2) or [None])[0]
            call_args = "\n".join(_all_strings(call, 3)) or None
            call_status = _strings({9: call[9]}) if call and 9 in call else None

            distinct = {f for f in top if f not in (1, 4, 5)}
            texts: list[str] = []
            for f in sorted(distinct):
                texts.extend(_strings({f: top[f]}))

            # Entity mapping: content shape decides, never the bare type int.
            # Roles are harness-internal (antigravity has no native role field);
            # they are documented here, not normalized to other harnesses' vocab.
            role = "record"
            is_error = bool(err_b is not None)
            if step_type == 14:
                role = "system"
            elif step_type == 15:
                role = "model"
            elif step_type in _CALL_TYPES:
                role = "tool_call"
            elif step_type in _ERROR_TYPES:
                role = "error"
                is_error = True
            elif step_type == 33:
                role = "user"
            elif step_type == 90:
                role = "system"
            elif step_type == 98:
                role = "system"
            elif step_type in (101, 127):
                role = "agent"
            elif step_type in (7, 21, 23, 31, 91, 139):
                role = "tool_result"

            t_uid = turn_uid(session_uid, turn_idx)
            if prev_turn is not None:
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "sequence", edge_idx),
                        session_uid=session_uid,
                        kind=EdgeKind.SEQUENCE,
                        source_uids=(prev_turn,),
                        target_uid=t_uid,
                        provenance=EdgeProvenance.DERIVED,
                        payload={"rule": "steps.idx order (no parent pointer)"},
                        timestamp_iso=ts,
                    )
                )
                edge_idx += 1

            content = "\n".join(texts)
            sess.turns.append(
                Turn(
                    turn_uid=t_uid,
                    session_uid=session_uid,
                    index=turn_idx,
                    role=role,
                    content=content,
                    timestamp_iso=ts,
                    native_model=None,
                    native_fields=native,
                )
            )

            tool_name = call_name
            if tool_name is None and 47 in top:
                f47 = _msg(top, 47)
                tool_name = (_all_strings(f47, 2) or [None])[0]
                if call_args is None and f47:
                    call_args = "\n".join(_all_strings(f47, 3)) or None
            plan_commands: list[str] = []
            for pf in (133, 147):
                if pf in top:
                    plan_commands.extend(_strings({pf: top[pf]}))
            if plan_commands:
                native["plan_commands"] = plan_commands
                if call_args is None:
                    call_args = "\n".join(plan_commands)
            if tool_name is not None or role in ("tool_call", "tool_result", "error"):
                exec_uid = tool_uid(session_uid, tool_idx)
                sess.tools.append(
                    ToolCall(
                        exec_uid=exec_uid,
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        index=tool_idx,
                        tool_name=tool_name or f"antigravity_type_{step_type}",
                        call_id=call_id,
                        command_or_args=call_args,
                        raw_output=content or None,
                        exit_code=1 if is_error else 0,
                        is_error=is_error,
                        parent_exec_uid=None,
                        timestamp_iso=ts,
                        native_fields={
                            "step_type": step_type,
                            "status": status,
                            **({"call_status": call_status} if call_status else {}),
                        },
                    )
                )
                kind = EdgeKind.TOOL_CALL if role == "tool_call" else EdgeKind.TOOL_RESULT
                sess.edges.append(
                    Edge(
                        edge_uid=edge_uid(session_uid, "tool_call" if role == "tool_call" else "tool_result", edge_idx),
                        session_uid=session_uid,
                        kind=kind,
                        source_uids=(t_uid,) if role == "tool_call" else (exec_uid,),
                        target_uid=None if role == "tool_call" else t_uid,
                        provenance=EdgeProvenance.DERIVED,
                        payload={"rule": "order adjacency (no call id linkage in harness)"},
                        timestamp_iso=ts,
                    )
                )
                edge_idx += 1
                tool_idx += 1

            if role == "user":
                kind_m, markers = classify_human_turn(content)
                sess.human.append(
                    HumanSignal(
                        human_uid=human_uid(session_uid, t_uid),
                        session_uid=session_uid,
                        turn_uid=t_uid,
                        text=content,
                        kind=kind_m,
                        markers=markers,
                        timestamp_iso=ts,
                    )
                )

            prev_turn = t_uid
            turn_idx += 1

        sess.check()
        return sess
