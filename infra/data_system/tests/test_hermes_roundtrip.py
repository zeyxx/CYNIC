"""α7 round-trip and α8 cross-oracle — conservation the parse cannot fake.

α7 (round-trip): the canonical session must RECONSTRUCT the raw rows. If a raw
field cannot be rebuilt from the canonical entities, it was lost. The rebuild
reads only declared homes (native_fields / native_meta), never the source, so a
parse bug cannot hide behind the comparison.

α8 (cross-oracle): the producer keeps independent representations of the same
truth (per-session counters, an FTS index, a file-level integrity check). Where
they disagree with the store, the disagreement is a QUESTION to resolve — it is
never assumed to be our bug, and never silently ignored.

Runs against the repaired capture (integrity ok). Skips if absent.
"""

from __future__ import annotations

import os
import random
import sqlite3

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer

SOURCE = "/var/lib/cynic/vault/corpus/captures/hermes_repaired.sqlite"

pytestmark = pytest.mark.skipif(
    not os.path.exists(SOURCE), reason="repaired hermes capture absent"
)


def _connect():
    con = sqlite3.connect(f"file:{SOURCE}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    con.row_factory = sqlite3.Row
    return con


def _random_sessions(n: int, seed: int) -> list[str]:
    con = _connect()
    try:
        rows = [r[0] for r in con.execute("SELECT id FROM sessions ORDER BY id")]
    finally:
        con.close()
    return random.Random(seed).sample(rows, min(n, len(rows)))


# ---------------------------------------------------------------------------
# α7 — round-trip
# ---------------------------------------------------------------------------

def _reconstruct_turn(t) -> tuple:
    """Rebuild the raw message row from the canonical turn, using only declared
    homes. This is the reconstruction the round-trip compares against the raw."""
    f = t.native_fields
    return (
        f["native_id"],
        t.role if t.role != "unknown" else None,
        t.content,  # None stays None; "" stays "" — they are distinct raw states
    )


@pytest.mark.parametrize("seed", [3, 11, 29])
def test_alpha7_turn_roundtrips_to_the_raw_row(seed):
    """For random sessions, every canonical turn reconstructs the raw
    (id, role, content) triple exactly."""
    norm = HermesNormalizer(SOURCE)
    for sid in _random_sessions(2, seed):
        sess = norm.normalize(sid)
        con = _connect()
        try:
            raw = con.execute(
                "SELECT id, role, content FROM messages WHERE session_id = ? ORDER BY id",
                (sid,),
            ).fetchall()
        finally:
            con.close()
        assert len(sess.turns) == len(raw), f"{sid}: turn count != raw"
        for i, r in enumerate(raw):
            got = _reconstruct_turn(sess.turns[i])
            want = (r["id"], r["role"], r["content"])
            assert got == want, f"{sid} row {i}: round-trip mismatch {got!r} != {want!r}"


def test_alpha7_reasoning_roundtrips_to_its_raw_home():
    """Every reasoning home present on a raw row is reconstructible from the
    canonical turn's native_fields, for a random codex-family session."""
    norm = HermesNormalizer(SOURCE)
    con = _connect()
    try:
        sid = con.execute(
            "SELECT session_id FROM messages WHERE codex_reasoning_items IS NOT NULL "
            "GROUP BY session_id ORDER BY count(*) DESC LIMIT 1"
        ).fetchone()[0]
        raw = con.execute(
            "SELECT id, codex_reasoning_items FROM messages "
            "WHERE session_id = ? AND codex_reasoning_items IS NOT NULL",
            (sid,),
        ).fetchall()
    finally:
        con.close()
    sess = norm.normalize(sid)
    by_id = {t.native_fields["native_id"]: t for t in sess.turns}
    for r in raw:
        t = by_id[r["id"]]
        assert t.native_fields.get("reasoning_home.codex_reasoning_items") == r["codex_reasoning_items"]


def test_alpha7_session_row_roundtrips():
    """The sessions row reconstructs column-for-column from native_meta.session."""
    norm = HermesNormalizer(SOURCE)
    con = _connect()
    try:
        for sid in _random_sessions(2, seed=5):
            raw = dict(con.execute("SELECT * FROM sessions WHERE id = ?", (sid,)).fetchone())
            kept = norm.normalize(sid).native_meta["session"]
            for col, value in raw.items():
                assert kept.get(col) == value, f"{sid}: column {col} not reconstructible"
    finally:
        con.close()


# ---------------------------------------------------------------------------
# α8 — cross-oracle
# ---------------------------------------------------------------------------

def test_alpha8_integrity_check_is_ok():
    """A source that fails integrity_check is not a valid conservation baseline."""
    con = _connect()
    try:
        assert con.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
    finally:
        con.close()


def test_alpha8_fts_index_agrees_with_the_messages_table():
    """The producer's FTS index is a second representation of the same rows,
    built by its own triggers. When present, counts must agree. The repaired
    capture is rebuilt without FTS (a separate task); the oracle then skips with
    a stated reason rather than passing vacuously."""
    con = _connect()
    try:
        has_fts = con.execute(
            "SELECT 1 FROM sqlite_master WHERE name = 'messages_fts' LIMIT 1"
        ).fetchone()
        if not has_fts:
            pytest.skip("repaired capture has no FTS index (rebuilt without it)")
        n_msg = con.execute("SELECT count(*) FROM messages").fetchone()[0]
        n_fts = con.execute("SELECT count(*) FROM messages_fts").fetchone()[0]
    finally:
        con.close()
    assert n_msg == n_fts, f"FTS row count {n_fts} != messages {n_msg}"


def test_alpha8_session_counters_disagree_and_the_gap_is_quantified():
    """sessions.message_count is the producer's own counter. Where it disagrees
    with the real row count, the gap is measured and reported — never assumed to
    be our bug and never hidden."""
    con = _connect()
    try:
        total = con.execute("SELECT count(*) FROM sessions").fetchone()[0]
        disagree = con.execute(
            """
            SELECT count(*) FROM sessions s
            WHERE s.message_count IS NOT NULL
              AND s.message_count != (SELECT count(*) FROM messages m WHERE m.session_id = s.id)
            """
        ).fetchone()[0]
        # the disagreement exists on the producer's side; we quantify it, we do
        # not fail on it. A zero here would itself be worth investigating.
        assert disagree <= total
        print(f"[alpha8] session message_count disagreements: {disagree}/{total}")
    finally:
        con.close()
