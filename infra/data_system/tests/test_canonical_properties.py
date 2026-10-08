"""Property-based and scale tests for the Hermes canonical vertical.

No external dependency: the properties are exercised with a seeded stdlib RNG so
a failure is reproducible from the seed alone. Each property is the black-box
contract the pipeline must hold for EVERY session, not just the sampled real
ones:

* **Lossless round-trip** — normalize -> store.write -> store.read returns the
  same canonical facts, for randomly shaped sessions (unicode, NULL vs "",
  multi-KB content, tool linkage, compaction flags).
* **No drop** — the turn count always equals the raw message count.
* **NULL != ""** — the two raw states never collapse.
* **Native tool linkage** — an assistant tool_call and its matching tool result
  become a NATIVE edge, deterministically.

The scale test normalizes the largest real session and bounds the time: a
pipeline that cannot finish a single big session is not production-grade.

α9: the real-source tests read the FROZEN capture, never the live store.
"""

from __future__ import annotations

import json
import os
import random
import sqlite3
import time

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.adapters.store_sqlite import SqliteCanonicalStore
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance
from cynic_data.canonical.usecases.normalize import normalize_source
from cynic_data.kernel.errors import SourceSchemaDrift

DB = "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"
_TABLES = (
    "sessions", "messages", "session_model_usage", "system_prompts",
    "async_delegations", "state_meta",
)

# Raw content shapes that have historically hidden losses.
_CONTENT_POOL = [
    None, "", "x", "héllo 🎉 ünïcode", '{"success": false, "error": "boom"}',
    '{"output": "ok", "exit_code": 0}', "a" * 4096, "line\nbreak\ttab",
    "0", "false", '{"nested": {"deep": [1, 2, 3]}}',
]


def _ddl() -> dict[str, str]:
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        return {
            t: con.execute(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name=?", (t,)
            ).fetchone()[0]
            for t in _TABLES
        }
    finally:
        con.close()


def _synthetic_capture(path: str, seed: int, n_messages: int) -> str:
    """Build a minimal but valid capture with randomized, adversarial rows."""
    rng = random.Random(seed)
    ddl = _ddl()
    con = sqlite3.connect(path)
    try:
        for sql in ddl.values():
            con.execute(sql)
        sid = f"syn_{seed}"
        con.execute(
            "INSERT INTO sessions (id, source, started_at) VALUES (?, 'cli', 1.0)", (sid,)
        )
        call_ids: list[str] = []
        rows = []
        for i in range(n_messages):
            role = rng.choice(["user", "assistant", "tool"])
            content = rng.choice(_CONTENT_POOL)
            tool_calls = None
            tool_call_id = None
            if role == "assistant" and rng.random() < 0.4:
                cid = f"call_{seed}_{i}"
                call_ids.append(cid)
                tool_calls = json.dumps(
                    [{"id": cid, "type": "function",
                      "function": {"name": "shell", "arguments": '{"cmd": "ls"}'}}]
                )
            elif role == "tool" and call_ids and rng.random() < 0.7:
                tool_call_id = rng.choice(call_ids)
            rows.append((
                sid, role, content, tool_call_id, tool_calls, "shell",
                float(i), i,                     # timestamp, display_order
                0 if rng.random() < 0.2 else 1,  # active
                1 if rng.random() < 0.2 else 0,  # compacted
                f"uid_{seed}_{i}", None,
            ))
        con.executemany(
            "INSERT INTO messages (session_id, role, content, tool_call_id, tool_calls, "
            "tool_name, timestamp, display_order, active, compacted, message_uid, "
            "absorbed_message_uids) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            rows,
        )
        con.commit()
    finally:
        con.close()
    return f"syn_{seed}"


@pytest.mark.parametrize("seed", [1, 2, 3, 4, 5, 6, 7, 8])
def test_roundtrip_is_lossless_for_randomized_captures(tmp_path, seed):
    """For every randomized capture: normalize -> write -> read is lossless and
    deterministic. This is the property the real-session sample cannot cover."""
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed, n_messages=40)
    norm = HermesNormalizer(db)

    sess = norm.normalize(sid)
    assert sess == norm.normalize(sid), f"seed {seed}: not deterministic"

    store = SqliteCanonicalStore(str(tmp_path / "canon.sqlite"))
    try:
        store.write(sess)
        back = store.read(sess.session_uid)
    finally:
        store.close()
    assert back is not None, f"seed {seed}: read returned None"
    back.check()

    assert back.stats() == sess.stats(), f"seed {seed}: stats drift"
    for a, b in zip(sess.turns, back.turns):
        assert a.role == b.role, f"seed {seed}: role drift"
        assert a.content == b.content, f"seed {seed}: content drift (NULL != \"\")"
        assert a.native_fields == b.native_fields, f"seed {seed}: native_fields drift"
    assert [h.text for h in sess.human] == [h.text for h in back.human], (
        f"seed {seed}: human text drift"
    )


@pytest.mark.parametrize("seed", [11, 12, 13])
def test_turn_count_always_equals_message_count(tmp_path, seed):
    """No message may be dropped: turns == raw rows, for any shape."""
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed, n_messages=60)
    con = sqlite3.connect(db)
    try:
        raw = con.execute(
            "SELECT count(*) FROM messages WHERE session_id = ?", (sid,)
        ).fetchone()[0]
    finally:
        con.close()
    assert len(HermesNormalizer(db).normalize(sid).turns) == raw, f"seed {seed}: dropped"


def test_null_and_empty_never_collapse_for_randomized_captures(tmp_path):
    """Across a randomized pool containing NULL and "", each turn's content AND
    each tool result's raw_output must equal its raw value exactly — the two
    states never merge."""
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed=99, n_messages=80)
    con = sqlite3.connect(db)
    con.row_factory = sqlite3.Row
    try:
        raw = {r["id"]: r["content"] for r in con.execute(
            "SELECT id, content FROM messages WHERE session_id = ?", (sid,))}
        raw_tool = {r["id"]: r["content"] for r in con.execute(
            "SELECT id, content FROM messages WHERE session_id = ? AND role = 'tool'", (sid,))}
    finally:
        con.close()
    sess = HermesNormalizer(db).normalize(sid)
    for t in sess.turns:
        assert t.content == raw[t.native_fields["native_id"]], (
            f"turn {t.native_fields['native_id']}: content coerced"
        )
    # tool results: raw_output is the raw content, never coerced (None stays None)
    tool_turns = {t.turn_uid: t.native_fields["native_id"] for t in sess.turns
                  if t.role == "tool"}
    checked = 0
    for tool in sess.tools:
        if tool.tool_name != "tool_result":
            continue
        assert tool.turn_uid is not None, "tool result without a turn"
        mid = tool_turns[tool.turn_uid]
        assert tool.raw_output == raw_tool[mid], f"tool {mid}: raw_output coerced"
        checked += 1
    assert checked > 0, "fixture produced no tool results to check"
    # the pool really did exercise both states
    values = set(raw.values())
    assert None in values and "" in values, "fixture did not exercise NULL vs ''"


def test_tool_call_linkage_is_native_and_deterministic(tmp_path):
    """An assistant tool_call with id K and a later tool result with
    tool_call_id=K become a NATIVE TOOL_RESULT edge, every run."""
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed=5, n_messages=40)
    sess = HermesNormalizer(db).normalize(sid)
    native = [e for e in sess.edges
              if e.kind is EdgeKind.TOOL_RESULT and e.provenance is EdgeProvenance.NATIVE]
    # determinism: identical edge set on a second run
    again = [e for e in HermesNormalizer(db).normalize(sid).edges
             if e.kind is EdgeKind.TOOL_RESULT and e.provenance is EdgeProvenance.NATIVE]
    assert [e.edge_uid for e in native] == [e.edge_uid for e in again]
    # every native tool-result edge points at a real tool call id
    con = sqlite3.connect(db)
    try:
        call_ids = {r[0] for r in con.execute(
            "SELECT DISTINCT tool_call_id FROM messages WHERE session_id = ? "
            "AND tool_call_id IS NOT NULL", (sid,))}
    finally:
        con.close()
    for e in native:
        assert (e.payload or {}).get("tool_call_id") in call_ids, "fabricated link"


# ---------------------------------------------------------------------------
# Scale — a single big session must finish in bounded time.
# ---------------------------------------------------------------------------

@pytest.mark.skipif(not os.path.exists(DB), reason="no frozen capture")
def test_largest_session_normalizes_within_budget():
    """The biggest real session must normalize in bounded time. A pipeline that
    stalls on one session is not production-grade. The ceiling is generous: it
    catches an O(n^2) regression, not host jitter."""
    con = sqlite3.connect(f"file:{DB}?immutable=1", uri=True)
    try:
        sid, n = con.execute(
            "SELECT session_id, count(*) c FROM messages GROUP BY session_id "
            "ORDER BY c DESC LIMIT 1"
        ).fetchone()
    finally:
        con.close()
    start = time.monotonic()
    sess = HermesNormalizer(DB).normalize(sid)
    elapsed = time.monotonic() - start
    assert len(sess.turns) == n, "turn count != raw message count on the largest session"
    print(f"\nlargest session {sid}: {n} messages in {elapsed:.2f}s")
    assert elapsed < 60.0, f"largest session ({n} msgs) took {elapsed:.1f}s (> 60s)"


# ---------------------------------------------------------------------------
# Fail-closed: a source missing an in-scope table must STOP, never publish an
# empty enrichment as if it were truth.
# ---------------------------------------------------------------------------

def test_missing_in_scope_table_fails_closed(tmp_path):
    """If the source lacks a table the normalizer reads, it must raise
    SourceSchemaDrift — an auxiliary read degrading to {} would silently drop
    session truth."""
    db = str(tmp_path / "cap.sqlite")
    _synthetic_capture(db, seed=1, n_messages=5)
    # drop one in-scope table
    con = sqlite3.connect(db)
    try:
        con.execute("DROP TABLE system_prompts")
        con.commit()
    finally:
        con.close()
    with pytest.raises(SourceSchemaDrift) as exc:
        HermesNormalizer(db).session_refs()
    assert "system_prompts" in str(exc.value)


def test_complete_source_does_not_raise(tmp_path):
    """Control: a source with every in-scope table normalizes fine."""
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed=1, n_messages=5)
    sess = HermesNormalizer(db).normalize(sid)
    assert sess.turns, "control source produced no turns"


# ---------------------------------------------------------------------------
# Observability: a run leaves a machine-readable JSON trace.
# ---------------------------------------------------------------------------

def test_normalize_source_emits_structured_json_events(tmp_path, capsys):
    """Every run must leave an auditable JSON trace (start/progress/done), not
    silence. The schema is stable and machine-parseable."""
    db = str(tmp_path / "cap.sqlite")
    _synthetic_capture(db, seed=3, n_messages=5)
    store = SqliteCanonicalStore(str(tmp_path / "canon.sqlite"))
    try:
        normalize_source(HermesNormalizer(db), store, progress_every=1)
    finally:
        store.close()
    lines = [json.loads(l) for l in capsys.readouterr().out.splitlines()
             if l.startswith("{")]
    events = [e["event"] for e in lines]
    assert lines and all(e["schema"] == "cynic.canonical.event.v1" for e in lines)
    assert "start" in events and "done" in events, f"missing lifecycle events: {events}"
    done = next(e for e in lines if e["event"] == "done")
    assert done["ok"] >= 1 and "elapsed_s" in done and "totals" in done


# ---------------------------------------------------------------------------
# Redaction boundary: Capture and Canonical PRESERVE verbatim. Redaction is a
# Projection concern, never applied here — this test pins that boundary.
# ---------------------------------------------------------------------------

def test_capture_and_canonical_do_not_redact(tmp_path):
    """A secret-looking value must survive byte-for-byte through normalize and
    the store. If this test ever fails, redaction leaked into the preservation
    layers — which would destroy conservation. Redaction belongs to Projection."""
    marker = "sk-live-DEADBEEF-secret-token"
    db = str(tmp_path / "cap.sqlite")
    sid = _synthetic_capture(db, seed=1, n_messages=0)
    con = sqlite3.connect(db)
    try:
        con.execute(
            "INSERT INTO messages (session_id, role, content, timestamp, display_order) "
            "VALUES (?, 'user', ?, 1.0, 1)",
            (sid, marker),
        )
        con.commit()
    finally:
        con.close()
    sess = HermesNormalizer(db).normalize(sid)
    assert sess.turns[0].content == marker, "canonical redacted a preserved value"
    store = SqliteCanonicalStore(str(tmp_path / "canon.sqlite"))
    try:
        store.write(sess)
        back = store.read(sess.session_uid)
    finally:
        store.close()
    assert back is not None and back.turns[0].content == marker, "store redacted"
