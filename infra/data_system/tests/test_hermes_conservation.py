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
    SESSION_MODEL_USAGE_FATE,
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


def test_l1_every_producer_table_is_decided():
    """Every table the producer declares must be DECIDED: either covered by a
    fate map, or explicitly named out of scope with a reason. No table may be
    silently unmodeled. This is the closure the hardcoded 3-table list lacked —
    it exempted 10 producer tables, including ``system_prompts`` (the exact
    prompt text of 1734 sessions) and ``async_delegations``."""
    _, tables = _producer_schema()
    unmodeled = CONTRACT.unmodeled_tables(tables)
    assert not unmodeled, (
        f"producer tables neither covered by a fate map nor declared out of "
        f"scope: {unmodeled}"
    )


def test_l1_every_in_scope_column_has_a_declared_fate():
    """Every column of every IN-SCOPE producer table must have exactly one fate.
    Derived from the producer's own DDL, so a new column fails the moment the
    producer adds it — for any table, not just the three hand-listed ones."""
    _, tables = _producer_schema()
    holes = CONTRACT.uncovered_columns(tables)
    assert not holes, f"in-scope columns with no declared fate: {holes}"


def test_l1_out_of_scope_tables_name_a_reason():
    """An out-of-scope declaration must carry a non-empty reason — an empty
    string would be a silent skip wearing a decision's clothes."""
    for table, reason in CONTRACT.out_of_scope.items():
        assert reason.strip(), f"{table}: out-of-scope with no reason"


def test_l1_no_out_of_scope_table_the_producer_does_not_have():
    """An out-of-scope entry for a table the producer no longer declares is
    stale contract debt."""
    _, tables = _producer_schema()
    stale = CONTRACT.stale_out_of_scope(tables)
    assert not stale, f"out-of-scope tables the producer does not have: {stale}"


def test_l1_no_fate_for_a_column_the_producer_does_not_have():
    """A fate for a column that no longer exists is stale contract debt."""
    _, tables = _producer_schema()
    known = {
        f"{table}.{col}"
        for table, cols in tables.items()
        for col in cols
    }
    stale = sorted(k for k in CONTRACT.field_fate if k not in known)
    assert not stale, f"fates for columns the producer does not have: {stale}"


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


# ---------------------------------------------------------------------------
# L2 — the blind spots the reverse-engineering of the producer surfaced.
# Each test pins a real defect found by reading the writer, so it can never
# come back silently.
# ---------------------------------------------------------------------------

def test_l2_display_identity_blob_is_preserved_and_json_safe():
    """display_identity is a BLOB in the producer. It must survive as a
    lossless, JSON-serializable value (hex) — a raw ``bytes`` crashes
    ``json.dumps`` and would lose the whole session (measured: 85 675 rows)."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        sid = con.execute(
            "SELECT session_id FROM messages WHERE typeof(display_identity) = 'blob' LIMIT 1"
        ).fetchone()[0]
        raw = con.execute(
            "SELECT id, display_identity FROM messages "
            "WHERE session_id = ? AND display_identity IS NOT NULL ORDER BY id",
            (sid,),
        ).fetchall()
    finally:
        con.close()
    sess = norm.normalize(sid)
    by_id = {t.native_fields["native_id"]: t for t in sess.turns}
    for mid, blob in raw:
        kept = by_id[mid].native_fields.get("display_identity")
        assert kept is not None, f"message {mid}: display_identity lost"
        # hex round-trips to the exact bytes
        assert bytes.fromhex(kept) == bytes(blob), f"message {mid}: display_identity corrupted"
    # and the whole session must be persistable (json.dumps must not raise)
    import json as _json
    for t in sess.turns:
        _json.dumps(t.native_fields)


def test_l2_logical_index_matches_the_producers_display_order():
    """Physical id order is NOT the logical order when compaction generations
    exist. ``logical_index`` must equal the dense rank of the producer's
    display_order (measured: 30 sessions diverge)."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        sid = con.execute(
            """
            SELECT session_id FROM messages
            GROUP BY session_id
            HAVING count(DISTINCT display_order) < count(*)
            ORDER BY count(*) DESC LIMIT 1
            """
        ).fetchone()[0]
        orders = {
            r[0]: r[1]
            for r in con.execute(
                "SELECT id, display_order FROM messages WHERE session_id = ?", (sid,)
            )
        }
    finally:
        con.close()
    dense = {o: i for i, o in enumerate(sorted(set(orders.values())))}
    sess = norm.normalize(sid)
    for t in sess.turns:
        assert t.native_fields["logical_index"] == dense[orders[t.native_fields["native_id"]]]


def test_l2_model_usage_telemetry_is_preserved_verbatim():
    """session_model_usage is the independent oracle for token accounting. Every
    row and column must be kept verbatim under native_meta.model_usage."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.row_factory = sqlite3.Row
    try:
        sid = con.execute(
            "SELECT session_id FROM session_model_usage GROUP BY session_id "
            "HAVING count(*) > 1 LIMIT 1"
        ).fetchone()[0]
        raw = [dict(r) for r in con.execute(
            "SELECT * FROM session_model_usage WHERE session_id = ? ORDER BY first_seen", (sid,)
        )]
    finally:
        con.close()
    kept = norm.normalize(sid).native_meta["model_usage"]
    assert len(kept) == len(raw), f"{sid}: model_usage row count changed"
    for got, want in zip(kept, raw):
        for col, value in want.items():
            assert got.get(col) == value, f"{sid}: model_usage.{col} not preserved"


def test_l2_absorbed_message_uids_become_an_nary_edge():
    """absorbed_message_uids declares which logical messages a compaction folded
    in. It must be an explicit n-ary edge, never loose text."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        row = con.execute(
            "SELECT session_id, absorbed_message_uids FROM messages "
            "WHERE absorbed_message_uids IS NOT NULL AND absorbed_message_uids != '' LIMIT 1"
        ).fetchone()
    finally:
        con.close()
    sid, absorbed_json = row
    import json as _json
    absorbed = _json.loads(absorbed_json)
    sess = norm.normalize(sid)
    nary = [e for e in sess.edges if (e.payload or {}).get("absorbed_message_uids")]
    assert nary, f"{sid}: absorbed_message_uids produced no n-ary edge"
    assert any(set(e.source_uids) == set(absorbed) for e in nary), (
        f"{sid}: n-ary edge sources {[e.source_uids for e in nary]} != declared {absorbed}"
    )


# ---------------------------------------------------------------------------
# Enumeration & session-level conservation — the loss the adversarial audit found.
# ---------------------------------------------------------------------------

def test_session_refs_cover_the_producer_session_registry():
    """session_refs() must enumerate the producer's ``sessions`` table, not only
    ``messages``. Enumerating from messages alone silently dropped 366 sessions
    that carry NO messages but DO carry session-level truth (tokens, cost,
    model_usage, title)."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        n_sessions = con.execute("SELECT count(*) FROM sessions").fetchone()[0]
        distinct_msg_sessions = con.execute(
            "SELECT count(DISTINCT session_id) FROM messages"
        ).fetchone()[0]
    finally:
        con.close()
    refs = set(norm.session_refs())
    assert n_sessions > distinct_msg_sessions, "fixture: expected message-less sessions"
    # every session row is enumerated
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        all_ids = {r[0] for r in con.execute("SELECT id FROM sessions")}
    finally:
        con.close()
    missing = all_ids - refs
    assert not missing, f"{len(missing)} sessions not enumerated, e.g. {sorted(missing)[:3]}"


def test_message_less_session_keeps_its_telemetry():
    """A session with no messages still carries truth: token counters, cost and
    model_usage. It must normalize without error and keep that truth."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.row_factory = sqlite3.Row
    try:
        sid = con.execute(
            "SELECT id FROM sessions WHERE id NOT IN (SELECT DISTINCT session_id FROM messages) "
            "AND id IN (SELECT session_id FROM session_model_usage) LIMIT 1"
        ).fetchone()[0]
        raw = dict(con.execute("SELECT * FROM sessions WHERE id = ?", (sid,)).fetchone())
    finally:
        con.close()
    sess = norm.normalize(sid)
    assert sess.turns == [], "fixture: expected a message-less session"
    assert sess.native_meta["session"] == raw, f"{sid}: session row not preserved"
    assert sess.native_meta.get("model_usage"), f"{sid}: model_usage lost on a message-less session"


def test_orphan_message_session_is_enumerated_and_preserved():
    """A message whose session_id has no ``sessions`` row is still source truth;
    it must be enumerated and its content preserved (never dropped)."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        row = con.execute(
            "SELECT session_id FROM messages "
            "WHERE session_id NOT IN (SELECT id FROM sessions) LIMIT 1"
        ).fetchone()
    finally:
        con.close()
    if row is None:
        pytest.skip("no orphan-message session on this capture")
    sid = row[0]
    assert sid in set(norm.session_refs()), "orphan session not enumerated"
    sess = norm.normalize(sid)
    assert sess.turns, f"{sid}: orphan session has content but produced no turns"


def test_l2_human_text_equals_raw_content_exactly():
    """HumanSignal.text must be the raw content value, never coerced (no
    ``content or ""``). None and "" are distinct raw states."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        sid = con.execute(
            "SELECT session_id FROM messages WHERE role = 'user' "
            "GROUP BY session_id ORDER BY count(*) DESC LIMIT 1"
        ).fetchone()[0]
        raw = {
            r[0]: r[1]
            for r in con.execute(
                "SELECT id, content FROM messages WHERE session_id = ? AND role = 'user'",
                (sid,),
            )
        }
    finally:
        con.close()
    sess = norm.normalize(sid)
    by_turn = {t.turn_uid: t.native_fields["native_id"] for t in sess.turns}
    for h in sess.human:
        mid = by_turn[h.turn_uid]
        assert h.text == raw[mid], f"{sid}: human text {h.text!r} != raw {raw[mid]!r}"


def test_l2_null_content_is_not_coerced_to_empty_string(tmp_path):
    """SYNTHETIC: the real capture has ZERO NULL/empty user or tool content, so
    the frozen-source test above cannot distinguish `content` from
    `content or ""` — it is vacuous on this data. A minimal synthetic capture
    pins the invariant directly: a NULL raw value must survive as None, an
    empty string as "", and the two must never be conflated."""
    src = sqlite3.connect(DB)
    try:
        msgs_ddl = src.execute(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='messages'"
        ).fetchone()[0]
        sess_ddl = src.execute(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='sessions'"
        ).fetchone()[0]
        usage_ddl = src.execute(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='session_model_usage'"
        ).fetchone()[0]
    finally:
        src.close()
    out = tmp_path / "synthetic.sqlite"
    con = sqlite3.connect(out)
    try:
        con.execute(sess_ddl)
        con.execute(msgs_ddl)
        con.execute(usage_ddl)
        con.execute(
            "INSERT INTO sessions (id, source, started_at) VALUES ('syn', 'cli', 1.0)"
        )
        # tool with NULL content, tool with '' content, user with NULL content
        con.executemany(
            "INSERT INTO messages (session_id, role, content, timestamp) VALUES (?,?,?,?)",
            [
                ("syn", "tool", None, 1.0),
                ("syn", "tool", "", 2.0),
                ("syn", "user", None, 3.0),
            ],
        )
        con.commit()
    finally:
        con.close()

    sess = HermesNormalizer(str(out)).normalize("syn")
    tools = [t for t in sess.tools if t.tool_name == "tool_result"]
    assert len(tools) == 2, f"expected 2 tool turns, got {len(tools)}"
    assert tools[0].raw_output is None, "NULL tool content must stay None, not ''"
    assert tools[1].raw_output == "", "empty tool content must stay ''"
    assert sess.human and sess.human[0].text is None, (
        "NULL user content must stay None, not ''"
    )


# ---------------------------------------------------------------------------
# L2 — the tables the closure audit surfaced (system_prompts, async_delegations,
# state_meta). Each carries real session truth that was previously unmodeled.
# ---------------------------------------------------------------------------

def test_l2_system_prompt_text_is_resolved_from_system_prompts():
    """``sessions.system_prompt`` is usually NULL; the real prompt text lives in
    ``system_prompts`` keyed by ``sessions.system_prompt_hash``. Normalization
    must resolve it into ``native_meta.system_prompt_resolved`` — the text of
    1734 sessions is otherwise unmodeled."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    con.row_factory = sqlite3.Row
    try:
        sid = con.execute(
            "SELECT id FROM sessions WHERE system_prompt_hash IS NOT NULL "
            "AND system_prompt_hash != '' LIMIT 1"
        ).fetchone()[0]
        raw = dict(con.execute("SELECT * FROM sessions WHERE id = ?", (sid,)).fetchone())
        sp = con.execute(
            "SELECT prompt FROM system_prompts WHERE hash = ?",
            (raw["system_prompt_hash"],),
        ).fetchone()
    finally:
        con.close()
    resolved = norm.normalize(sid).native_meta.get("system_prompt_resolved")
    assert resolved is not None, f"{sid}: system prompt not resolved"
    assert resolved["hash"] == raw["system_prompt_hash"], f"{sid}: hash mismatch"
    if sp is not None:
        assert resolved["prompt"] == sp["prompt"], f"{sid}: prompt text drift"


def test_l2_async_delegation_rows_are_attached_verbatim():
    """A session that dispatched async subagents must carry its delegation rows
    (minus the live process owner, which is operational, not session truth)."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    con.row_factory = sqlite3.Row
    try:
        sid = con.execute(
            "SELECT origin_session FROM async_delegations "
            "WHERE origin_session IN (SELECT id FROM sessions) "
            "GROUP BY origin_session ORDER BY count(*) DESC LIMIT 1"
        ).fetchone()[0]
        raw = [
            {k: v for k, v in dict(r).items() if k not in ("owner_pid", "owner_started_at")}
            for r in con.execute(
                "SELECT * FROM async_delegations WHERE origin_session = ? "
                "ORDER BY dispatched_at",
                (sid,),
            )
        ]
    finally:
        con.close()
    kept = norm.normalize(sid).native_meta.get("async_delegations")
    assert kept is not None, f"{sid}: async_delegations key missing"
    assert len(kept) == len(raw), f"{sid}: delegation row count {len(kept)} != {len(raw)}"
    for got, want in zip(kept, raw):
        assert got == want, f"{sid}: delegation row drift"


def test_l2_store_meta_is_carried():
    """``state_meta`` (store instance id, creation time) is store provenance and
    must be carried verbatim."""
    norm = HermesNormalizer(DB)
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        raw = {r[0]: r[1] for r in con.execute("SELECT key, value FROM state_meta")}
        sid = con.execute("SELECT id FROM sessions LIMIT 1").fetchone()[0]
    finally:
        con.close()
    kept = norm.normalize(sid).native_meta.get("store_meta")
    assert kept is not None, "store_meta key missing"
    assert kept == raw, f"store_meta drift: {kept} != {raw}"
