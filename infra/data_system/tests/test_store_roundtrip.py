"""Store round-trip test — write then read must be lossless for canonical facts."""

from __future__ import annotations

import os
import sqlite3

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.adapters.store_sqlite import SqliteCanonicalStore

HERMES_DB = "/home/user/.hermes/state.db"


@pytest.fixture()
def store(tmp_path):
    s = SqliteCanonicalStore(str(tmp_path / "canonical.sqlite"))
    yield s
    s.close()


def _session_id() -> str | None:
    if not os.path.exists(HERMES_DB):
        return None
    con = sqlite3.connect(f"file:{HERMES_DB}?immutable=1", uri=True)
    try:
        row = con.execute(
            """
            SELECT session_id FROM messages
            WHERE session_id IN (SELECT session_id FROM messages WHERE tool_calls IS NOT NULL AND tool_calls != '')
            GROUP BY session_id ORDER BY count(*) DESC LIMIT 1
            """
        ).fetchone()
        return row[0] if row else None
    finally:
        con.close()


pytestmark = pytest.mark.skipif(not os.path.exists(HERMES_DB), reason="no hermes db")


def test_roundtrip_is_lossless(store):
    sid = _session_id()
    assert sid
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    before = sess.stats()
    store.write(sess)

    reloaded = store.read(sess.session_uid)
    assert reloaded is not None
    reloaded.check()
    after = reloaded.stats()
    for key in ("turns", "tools", "reasoning", "edges", "states", "human"):
        assert before[key] == after[key], f"{key} changed across the round-trip"


def test_write_is_idempotent(store):
    sid = _session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    store.write(sess)
    store.write(sess)  # must not duplicate
    reloaded = store.read(sess.session_uid)
    assert reloaded.stats()["turns"] == sess.stats()["turns"]
    assert store.count("sessions") == 1


def test_store_never_invents(store):
    """Unobserved fields stay NULL, not defaulted."""
    sid = _session_id()
    store.write(HermesNormalizer(HERMES_DB).normalize(sid))
    # hermes does not expose cwd/sandbox -> must be NULL, not ''
    row = store._con.execute("SELECT count(*) FROM states WHERE cwd IS NOT NULL").fetchone()[0]
    assert row == 0, "hermes exposes no cwd; it must stay NULL (anti-fabrication)"
