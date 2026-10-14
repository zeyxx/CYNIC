"""Capture layer tests — immutability, content addressing, consistency."""

from __future__ import annotations

import os
import sqlite3

import pytest

from cynic_data.capture.adapters.capturer_directory import DirectorySourceCapturer
from cynic_data.capture.adapters.capturer_sqlite import SqliteSourceCapturer
from cynic_data.capture.domain.records import CaptureMethod
from cynic_data.capture.usecases.capture_source import capture_sources


def _make_sqlite(path: str) -> None:
    con = sqlite3.connect(path)
    con.execute("CREATE TABLE messages (id INTEGER PRIMARY KEY, session_id TEXT, role TEXT, content TEXT)")
    con.execute("INSERT INTO messages (session_id, role, content) VALUES ('s1','user','hello')")
    con.execute("INSERT INTO messages (session_id, role, content) VALUES ('s1','assistant','world')")
    con.commit()
    con.close()


def test_sqlite_capturer_produces_consistent_snapshot(tmp_path):
    src = tmp_path / "live.db"
    _make_sqlite(str(src))
    cap = SqliteSourceCapturer()
    assert cap.can_capture(str(src))

    rec = cap.capture("hermes", str(src), str(tmp_path / "captures"))
    assert rec.method is CaptureMethod.SQLITE_VACUUM_INTO
    assert rec.content_hash.startswith("sha256:")
    assert rec.size_bytes > 0
    assert rec.entry_count == 2

    # the frozen copy must be a real SQLite database, readable and consistent
    con = sqlite3.connect(f"file:{rec.captured_path}?immutable=1", uri=True)
    try:
        assert con.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
        assert con.execute("SELECT count(*) FROM messages").fetchone()[0] == 2
    finally:
        con.close()


def test_snapshot_is_immutable_under_source_mutation(tmp_path):
    """The whole point: after capture, mutating the source must not change the capture."""
    src = tmp_path / "live.db"
    _make_sqlite(str(src))
    cap = SqliteSourceCapturer()
    rec = cap.capture("hermes", str(src), str(tmp_path / "captures"))

    # mutate the source AFTER capture
    con = sqlite3.connect(str(src))
    con.execute("INSERT INTO messages (session_id, role, content) VALUES ('s2','user','later')")
    con.commit()
    con.close()

    con = sqlite3.connect(f"file:{rec.captured_path}?immutable=1", uri=True)
    try:
        assert con.execute("SELECT count(*) FROM messages").fetchone()[0] == 2, \
            "capture must not change when the source mutates"
    finally:
        con.close()


def test_directory_capturer_hashes_tree(tmp_path):
    src = tmp_path / "tree"
    (src / "a").mkdir(parents=True)
    (src / "a" / "f1.txt").write_text("one")
    (src / "f2.txt").write_text("two")
    cap = DirectorySourceCapturer()
    assert cap.can_capture(str(src))
    rec = cap.capture("codex", str(src), str(tmp_path / "captures"))
    assert rec.entry_count == 2
    assert rec.size_bytes == len("one") + len("two")
    assert rec.content_hash.startswith("sha256:")


def test_directory_capturer_skips_symlinks(tmp_path):
    """A capture must never re-introduce a link to live data."""
    src = tmp_path / "tree"
    src.mkdir()
    (src / "real.txt").write_text("real")
    outside = tmp_path / "outside.txt"
    outside.write_text("live")
    os.symlink(str(outside), str(src / "link.txt"))

    cap = DirectorySourceCapturer()
    rec = cap.capture("codex", str(src), str(tmp_path / "captures"))
    assert rec.entry_count == 1, "symlinks must be excluded from the capture"
    assert not os.path.islink(os.path.join(rec.captured_path, "link.txt"))


def test_capture_sources_fails_closed(tmp_path):
    """No capturer -> explicit error, never a silent fallback to live data."""
    missing = tmp_path / "nope.db"
    with pytest.raises(RuntimeError, match="no capturer"):
        capture_sources(
            sources={"hermes": str(missing)},
            dest_dir=str(tmp_path / "captures"),
            capturers=[SqliteSourceCapturer()],
        )


def test_manifest_is_deterministic(tmp_path):
    src = tmp_path / "live.db"
    _make_sqlite(str(src))
    m1 = capture_sources({"hermes": str(src)}, str(tmp_path / "c1"), [SqliteSourceCapturer()])
    m2 = capture_sources({"hermes": str(src)}, str(tmp_path / "c2"), [SqliteSourceCapturer()])
    assert m1.manifest_uid == m2.manifest_uid, "same bytes -> same manifest uid"
