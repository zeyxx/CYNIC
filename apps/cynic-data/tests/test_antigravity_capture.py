"""Antigravity capture proof — frozen layout is the only ingestion source.

The real archive is ~1.1 GB of SQLite DBs, so this module proves the SHAPE:
a synthetic DB in the frozen layout normalizes end-to-end (protobuf payloads
walked, tool call detail extracted, error flagged), live mutation does not
move the capture, and absent sources fail closed. The real-corpus freeze is
exercised by capture_antigravity.py, not by this suite.
"""

from __future__ import annotations

import os
import sqlite3
import struct

import pytest

from cynic_data.canonical.adapters.normalizers.antigravity import AntigravityNormalizer
from cynic_data.capture.adapters.capturer_directory import DirectorySourceCapturer
from cynic_data.capture.usecases.capture_source import capture_sources


def _enc_varint(v: int) -> bytes:
    out = b""
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out += struct.pack("B", b | 0x80)
        else:
            return out + struct.pack("B", b)


def _field(n: int, wire: int) -> bytes:
    return _enc_varint((n << 3) | wire)


def _str_field(n: int, s: str) -> bytes:
    b = s.encode()
    return _field(n, 2) + _enc_varint(len(b)) + b


def _msg_field(n: int, inner: bytes) -> bytes:
    return _field(n, 2) + _enc_varint(len(inner)) + inner


def _ts(sec: int, nanos: int) -> bytes:
    return _msg_field(1, _field(1, 0) + _enc_varint(sec) + _field(2, 0) + _enc_varint(nanos))


def _make_db(path: str) -> None:
    con = sqlite3.connect(path)
    con.execute(
        "CREATE TABLE steps (idx INTEGER, step_type INTEGER, status INTEGER,"
        " has_subtrajectory NUMERIC, metadata BLOB, error_details BLOB,"
        " permissions BLOB, task_details BLOB, render_info BLOB,"
        " step_payload BLOB, step_format INTEGER)"
    )
    con.execute(
        "CREATE TABLE trajectory_meta (trajectory_id TEXT, cascade_id TEXT,"
        " trajectory_type INTEGER, source INTEGER)"
    )
    con.execute("CREATE TABLE gen_metadata (idx INTEGER, data BLOB, size INTEGER)")
    con.execute("CREATE TABLE executor_metadata (idx INTEGER, data BLOB)")
    con.execute("CREATE TABLE parent_references (idx INTEGER, data BLOB)")
    con.execute("CREATE TABLE trajectory_metadata_blob (id TEXT, data BLOB)")
    con.execute("CREATE TABLE battle_mode_infos (idx INTEGER, data BLOB)")
    con.execute(
        "INSERT INTO trajectory_meta VALUES ('traj-1', 'casc-1', 4, 17)"
    )
    # step 0: tool call (type 8)
    call = _msg_field(
        4,
        _str_field(1, "toolu_test1") + _str_field(2, "read_file") + _str_field(3, '{"p": "x"}'),
    )
    env0 = _msg_field(
        5, _ts(1781500765, 100) + _field(3, 0) + _enc_varint(2) + call
    )
    pay0 = _field(1, 0) + _enc_varint(8) + _field(4, 0) + _enc_varint(3) + env0
    # step 1: model turn (type 15) with response text
    env1 = _msg_field(5, _ts(1781500766, 200) + _field(3, 0) + _enc_varint(2))
    pay1 = (
        _field(1, 0)
        + _enc_varint(15)
        + _field(4, 0)
        + _enc_varint(3)
        + env1
        + _msg_field(20, _str_field(3, "hello world"))
    )
    # step 2: error record (type 17) with detail + error_details
    env2 = _msg_field(5, _ts(1781500767, 300) + _field(3, 0) + _enc_varint(5))
    err_detail = _msg_field(24, _str_field(1, "quota reached"))
    pay2 = (
        _field(1, 0)
        + _enc_varint(17)
        + _field(4, 0)
        + _enc_varint(7)
        + env2
        + err_detail
    )
    err_blob = _msg_field(1, _str_field(2, "RESOURCE_EXHAUSTED"))
    con.execute(
        "INSERT INTO steps VALUES (0, 8, 3, 0, NULL, NULL, NULL, NULL, NULL, ?, 0)",
        (pay0,),
    )
    con.execute(
        "INSERT INTO steps VALUES (1, 15, 3, 0, NULL, NULL, NULL, NULL, NULL, ?, 0)",
        (pay1,),
    )
    con.execute(
        "INSERT INTO steps VALUES (2, 17, 7, 0, NULL, ?, NULL, NULL, NULL, ?, 0)",
        (err_blob, pay2),
    )
    con.commit()
    con.close()


def test_frozen_layout_normalizes_end_to_end(tmp_path):
    src = str(tmp_path / "live")
    os.makedirs(os.path.join(src, "conversations"))
    _make_db(os.path.join(src, "conversations", "probe-uuid.db"))
    manifest = capture_sources(
        sources={"antigravity": src},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("antigravity")
    assert rec is not None and rec.content_hash.startswith("sha256:")

    norm = AntigravityNormalizer(os.path.join(rec.captured_path, "conversations"))
    assert norm.session_refs() == ["probe-uuid"]
    assert norm.can_read("probe-uuid")
    s = norm.normalize("probe-uuid")
    s.check()
    assert len(s.turns) == 3
    assert [t.role for t in s.turns] == ["tool_call", "model", "error"]
    assert s.turns[0].timestamp_iso == "2026-06-15T05:19:25.000000100Z"
    assert len(s.tools) == 2, "call + error record carry tool entities"
    assert s.tools[0].call_id == "toolu_test1"
    assert s.tools[0].tool_name == "read_file"
    assert s.tools[1].is_error
    assert "hello world" in s.turns[1].content
    assert s.native_meta["traj_trajectory_id"] == "traj-1"


def test_capture_immutable_under_live_mutation(tmp_path):
    src = str(tmp_path / "live")
    os.makedirs(os.path.join(src, "conversations"))
    _make_db(os.path.join(src, "conversations", "probe-uuid.db"))
    manifest = capture_sources(
        sources={"antigravity": src},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("antigravity")
    assert rec is not None
    frozen = os.path.join(rec.captured_path, "conversations")
    before = AntigravityNormalizer(frozen).normalize("probe-uuid").stats()

    _make_db(os.path.join(src, "conversations", "intruder.db"))
    after = AntigravityNormalizer(frozen).normalize("probe-uuid").stats()
    assert after == before
    assert AntigravityNormalizer(frozen).session_refs() == ["probe-uuid"]


def test_absent_source_fails_closed(tmp_path):
    with pytest.raises(Exception):
        capture_sources(
            sources={"antigravity": "/nonexistent/ag-root"},
            dest_dir=str(tmp_path / "captures"),
            capturers=[DirectorySourceCapturer()],
        )
