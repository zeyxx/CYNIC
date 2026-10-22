"""Layer-2 gap fixes — skip-and-record capture, resume, lineage, run ledger."""

from __future__ import annotations

import pytest

from cynic_data.canonical.adapters.store_sqlite import SqliteCanonicalStore
from cynic_data.canonical.domain.session import NormalizedSession, session_content_hash
from cynic_data.canonical.usecases.normalize import normalize_source
from cynic_data.capture.usecases.capture_source import capture_sources


class FakeNormalizer:
    harness = "fake"
    source_kind = "fake"

    def __init__(self, refs):
        self._refs = refs

    def can_read(self, source_ref: str) -> bool:
        return True

    def conservation(self):  # type: ignore[no-untyped-def]  # None: explicit unknown
        return None

    def session_refs(self) -> list[str]:
        return list(self._refs)

    def normalize(self, session_ref: str) -> NormalizedSession:
        return NormalizedSession(
            session_uid=f"fake:{session_ref}", harness="fake", source_ref=session_ref
        )


def _store(tmp_path):
    return SqliteCanonicalStore(str(tmp_path / "canon.sqlite"))


def test_hash_is_stable_across_runs():
    s = NormalizedSession(session_uid="x", harness="h", source_ref="r")
    assert session_content_hash(s) == session_content_hash(s)


def test_second_normalize_skips_everything(tmp_path):
    store = _store(tmp_path)
    try:
        first = normalize_source(FakeNormalizer(["a", "b"]), store)
        assert (first.sessions_ok, first.sessions_skipped) == (2, 0)
        second = normalize_source(FakeNormalizer(["a", "b"]), store)
        assert (second.sessions_ok, second.sessions_skipped) == (0, 2)
    finally:
        store.close()


def test_run_ledger_records_both_runs(tmp_path):
    store = _store(tmp_path)
    try:
        normalize_source(FakeNormalizer(["a"]), store)
        normalize_source(FakeNormalizer(["a"]), store)
        assert store.count("runs") == 2
    finally:
        store.close()


def test_manifest_uid_links_session_to_capture(tmp_path):
    store = _store(tmp_path)
    try:
        normalize_source(FakeNormalizer(["a"]), store, manifest_uid="cap:abc123")
        row = store._con.execute(
            "SELECT manifest_uid, content_hash FROM sessions WHERE session_uid='fake:a'"
        ).fetchone()
        assert row[0] == "cap:abc123"
        assert (row[1] or "").startswith("sha256:")
    finally:
        store.close()


def test_capture_non_strict_skips_and_records(tmp_path):
    manifest = capture_sources(
        sources={"broken": "/does/not/exist.db"},
        dest_dir=str(tmp_path / "cap"),
        capturers=[],
        strict=False,
    )
    assert manifest.records == ()
    assert len(manifest.failures) == 1
    assert manifest.failures[0][0] == "broken"


def test_capture_strict_still_fails_closed(tmp_path):
    with pytest.raises(RuntimeError):
        capture_sources(
            sources={"broken": "/does/not/exist.db"},
            dest_dir=str(tmp_path / "cap"),
            capturers=[],
            strict=True,
        )
