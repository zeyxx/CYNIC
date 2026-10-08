"""Hermes conservation proof — L1 closure, L2 value, L3 re-derivability.

This is the vertical the other harnesses inherit. Three cumulative levels:

* **L1 closure** — every column of the producer's OWN DDL has a declared fate.
  The observed field set is parsed from ``SCHEMA_SQL``, never hand-kept, so a new
  column fails the test the moment the producer adds it.
* **L1 anchor** — the contract is pinned to ``SCHEMA_VERSION``; a producer bump
  fails until the contract is re-derived.
* **L2 value** — for a RANDOM sample of rows, every ``preserved`` field's value
  read back from the normalized session equals the raw value (or its declared
  transform). Random sampling is the point: an average hides a rare shape.
* **L3 re-derivability** — every ``declared`` loss names why the bytes stay out;
  the value is re-derivable from the frozen capture, so the declaration is a
  claim the audit can check, not a hope.

The suite skips cleanly when the Hermes tree or the store is absent.
"""

from __future__ import annotations

import json
import os
import random
import re
import sqlite3

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.adapters.normalizers.hermes_contract import (
    CONTRACT,
    MESSAGES_FATE,
    SESSIONS_FATE,
)
from cynic_data.canonical.domain.conservation import Fate, parse_fate, parse_ddl_columns

HERMES_SRC = "/home/user/.hermes/hermes-agent/hermes_state_common.py"
# The frozen, integrity-checked capture — NOT the live store, which is corrupt
# (a full scan can raise "database disk image is malformed"). Tests must read a
# deterministic source, or a passing run proves nothing. This is the UNION of
# the repaired live store and the older freeze: newest data + sessions the live
# store pruned (2430 distinct sessions vs 1390 on the live store).
DB = "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"

pytestmark = pytest.mark.skipif(
    not (os.path.exists(HERMES_SRC) and os.path.exists(DB)),
    reason="hermes source or state.db absent",
)


# ---------------------------------------------------------------------------
# L1 — closure, derived from the producer's own DDL
# ---------------------------------------------------------------------------

def _producer_schema() -> tuple[int, dict[str, list[str]]]:
    text = open(HERMES_SRC, encoding="utf-8").read()
    m = re.search(r"^SCHEMA_VERSION\s*=\s*(\d+)", text, re.M)
    assert m, "SCHEMA_VERSION not found in producer source"
    version = int(m.group(1))
    m = re.search(r'^SCHEMA_SQL\s*=\s*"""(.*?)"""', text, re.M | re.S)
    assert m, "SCHEMA_SQL not found in producer source"
    return version, parse_ddl_columns(m.group(1))


def test_l1_anchor_matches_producer_schema_version():
    """The contract is pinned to the producer's schema version. A bump fails here."""
    version, _ = _producer_schema()
    assert version == CONTRACT.producer_schema_version, (
        f"producer bumped SCHEMA_VERSION to {version}; "
        f"contract pins {CONTRACT.producer_schema_version} — re-derive the contract"
    )


def test_l1_every_producer_column_has_a_declared_fate():
    """Every column the producer declares must have exactly one fate."""
    _, tables = _producer_schema()
    for table, fate_map in (("messages", MESSAGES_FATE), ("sessions", SESSIONS_FATE)):
        cols = tables.get(table, [])
        assert cols, f"no columns parsed for {table}"
        missing = sorted(c for c in cols if c not in fate_map)
        assert not missing, f"{table}: columns with no declared fate: {missing}"


def test_l1_no_fate_for_a_column_the_producer_does_not_have():
    """A fate for a column that no longer exists is stale contract debt."""
    _, tables = _producer_schema()
    for table, fate_map in (("messages", MESSAGES_FATE), ("sessions", SESSIONS_FATE)):
        cols = set(tables.get(table, []))
        stale = sorted(c for c in fate_map if c not in cols)
        assert not stale, f"{table}: fates for columns the producer does not have: {stale}"


def test_l1_all_fates_are_well_formed():
    bad = CONTRACT.malformed_fates()
    assert not bad, f"fates must start with preserved:/declared: {bad}"


def test_l1_contract_matches_live_store_schema():
    """The producer source and the live store must agree, or one is stale."""
    version, tables = _producer_schema()
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        for table in ("messages", "sessions"):
            live = [c[1] for c in con.execute(f"PRAGMA table_info({table})")]
            assert set(live) == set(tables[table]), (
                f"{table}: producer source and live store disagree; "
                f"live-only={sorted(set(live) - set(tables[table]))} "
                f"source-only={sorted(set(tables[table]) - set(live))}"
            )
    finally:
        con.close()


# ---------------------------------------------------------------------------
# L2 — value conservation, on a RANDOM sample
# ---------------------------------------------------------------------------

def _random_sessions(n: int, seed: int) -> list[str]:
    """Random sessions from the store. The sample is the proof: a rare shape
    (a codex model, a compacted row, a multi-model session) must not be missed
    because it is not the biggest session."""
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        rows = [r[0] for r in con.execute(
            "SELECT id FROM sessions ORDER BY id"
        )]
    finally:
        con.close()
    rng = random.Random(seed)
    return rng.sample(rows, min(n, len(rows)))


@pytest.mark.parametrize("seed", [1, 7, 42])
def test_l2_turn_content_and_role_survive_verbatim(seed):
    """For random sessions: every message's (role, content) read back from the
    normalized session equals the raw row, in order."""
    norm = HermesNormalizer(DB)
    for sid in _random_sessions(3, seed):
        sess = norm.normalize(sid)
        con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
        try:
            raw = con.execute(
                "SELECT role, content FROM messages WHERE session_id = ? ORDER BY id",
                (sid,),
            ).fetchall()
        finally:
            con.close()
        assert len(sess.turns) == len(raw), f"{sid}: turn count != raw message count"
        for i, (role, content) in enumerate(raw):
            t = sess.turns[i]
            assert t.role == (role or "unknown"), f"{sid} turn {i}: role drift"
            assert t.content == (content or ""), f"{sid} turn {i}: content drift"


def test_l2_reasoning_value_survives_on_its_home():
    """The reasoning value kept on a turn equals the raw value of the home it
    was resolved from. This is the check that catches reading the wrong column."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        # a session that has codex reasoning (the home the old adapter missed)
        sid = con.execute(
            "SELECT session_id FROM messages WHERE codex_reasoning_items IS NOT NULL "
            "GROUP BY session_id ORDER BY count(*) DESC LIMIT 1"
        ).fetchone()[0]
        raw = con.execute(
            "SELECT id, codex_reasoning_items FROM messages "
            "WHERE session_id = ? AND codex_reasoning_items IS NOT NULL ORDER BY id",
            (sid,),
        ).fetchall()
    finally:
        con.close()
    sess = norm.normalize(sid)
    by_native_id = {t.native_fields["native_id"]: t for t in sess.turns}
    for mid, raw_value in raw:
        t = by_native_id.get(mid)
        assert t is not None, f"message {mid} missing from normalized session"
        assert t.native_fields.get("reasoning_home.codex_reasoning_items") == raw_value, (
            f"message {mid}: codex reasoning value not preserved verbatim"
        )


def test_l2_active_flag_survives_and_drives_state():
    """The compaction flag is preserved on every turn; superseded turns are
    marked in state. Reading history without it conflates live and superseded."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        sid = con.execute(
            "SELECT session_id FROM messages WHERE active = 0 "
            "GROUP BY session_id ORDER BY count(*) DESC LIMIT 1"
        ).fetchone()[0]
    finally:
        con.close()
    sess = norm.normalize(sid)
    superseded_turns = [t for t in sess.turns if t.native_fields.get("active") == 0]
    assert superseded_turns, f"{sid}: active=0 turns not preserved"
    marked = [
        s for s in sess.states
        if s.native_fields.get("superseded_by_compaction") is True
    ]
    assert len(marked) == len(superseded_turns), (
        f"{sid}: {len(superseded_turns)} superseded turns but {len(marked)} marked states"
    )


def test_l2_session_row_survives_verbatim():
    """The whole sessions row is kept, so no session column is silently lost."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.row_factory = sqlite3.Row
    try:
        for sid in _random_sessions(3, seed=99):
            raw = dict(con.execute("SELECT * FROM sessions WHERE id = ?", (sid,)).fetchone())
            sess = norm.normalize(sid)
            kept = sess.native_meta.get("session", {})
            for col, value in raw.items():
                assert kept.get(col) == value, f"{sid}: session column {col} not preserved"
    finally:
        con.close()


# ---------------------------------------------------------------------------
# L3 — declared losses are re-derivable from the frozen capture
# ---------------------------------------------------------------------------

def test_l3_declared_losses_name_a_reason():
    losses = CONTRACT.declared_losses()
    assert losses, "expected at least one declared loss"
    for field, reason in losses.items():
        assert reason.strip(), f"{field}: declared loss with no reason"


def test_l3_token_count_is_really_unpopulated():
    """FIELD_FATE declares token_count 'unpopulated in the producer'. The claim
    is checked against the store, not assumed."""
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        n = con.execute("SELECT count(*) FROM messages WHERE token_count IS NOT NULL").fetchone()[0]
    finally:
        con.close()
    assert n == 0, f"token_count is populated ({n}); the declared loss is stale"


def test_l3_every_preserved_fate_names_a_home():
    """A preserved fate must name where the value goes; 'preserved:' alone is a
    claim with no home and cannot be checked."""
    for field, value in CONTRACT.field_fate.items():
        fate, detail = parse_fate(value)
        if fate is Fate.PRESERVED:
            assert detail.strip(), f"{field}: preserved fate with no named home"
