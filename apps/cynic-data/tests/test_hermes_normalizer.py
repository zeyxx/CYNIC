"""Hermes normalizer contract test — reads the FROZEN capture (read-only).

Proves the reference adapter: graph edges, state, human signal, tool linkage.
α9: reads the frozen capture, never the live store (which is corrupt and grows).
Skips cleanly if the capture is absent.
"""

from __future__ import annotations

import os
import sqlite3

import pytest

from cynic_data.canonical.adapters.normalizers.hermes import HermesNormalizer
from cynic_data.canonical.domain.entities import EdgeProvenance

HERMES_DB = "/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite"


def _richest_session_id() -> str | None:
    """Pick a session that exercises the most entities."""
    if not os.path.exists(HERMES_DB):
        return None
    con = sqlite3.connect(f"file:{HERMES_DB}?immutable=1", uri=True)
    con.text_factory = lambda b: b.decode("utf-8", "replace")
    try:
        row = con.execute(
            """
            SELECT session_id, count(*) AS n
            FROM messages
            WHERE session_id IN (
                SELECT session_id FROM messages WHERE reasoning_content IS NOT NULL AND reasoning_content != ''
            )
            AND session_id IN (
                SELECT session_id FROM messages WHERE tool_calls IS NOT NULL AND tool_calls != ''
            )
            GROUP BY session_id
            ORDER BY n DESC
            LIMIT 1
            """
        ).fetchone()
        return row[0] if row else None
    finally:
        con.close()


pytestmark = pytest.mark.skipif(
    not os.path.exists(HERMES_DB), reason="hermes state.db not present"
)


def test_normalizer_produces_a_valid_session():
    sid = _richest_session_id()
    assert sid, "no rich session found"
    norm = HermesNormalizer(HERMES_DB)
    sess = norm.normalize(sid)
    sess.check()  # invariants must hold


def test_graph_edges_are_present_and_typed():
    sid = _richest_session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    assert len(sess.edges) > 0, "hermes must yield derived causal edges"
    # hermes has no native parent pointer -> sequence edges are DERIVED
    seq = [e for e in sess.edges if e.kind.value == "sequence"]
    assert seq, "expected derived sequence edges"
    assert all(e.provenance is EdgeProvenance.DERIVED for e in seq)


def test_human_signal_is_captured_and_classified():
    sid = _richest_session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    assert len(sess.human) > 0, "human turns must be first-class"
    # every human signal references a real turn
    turn_ids = {t.turn_uid for t in sess.turns}
    for h in sess.human:
        assert h.turn_uid in turn_ids
    # classification is conservative: unknown is allowed, but the field exists
    kinds = {h.kind.value for h in sess.human}
    assert kinds <= {"unknown", "redirect", "constraint", "rejection", "approval"}


def test_tool_results_link_back_to_calls():
    sid = _richest_session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    results = [t for t in sess.tools if t.tool_name == "tool_result"]
    if results:
        linked = [t for t in results if t.parent_exec_uid]
        assert linked, "tool results must link back to their call when tool_call_id exists"


def test_reasoning_is_captured():
    sid = _richest_session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    assert len(sess.reasoning) > 0, "hermes reasoning_content must be preserved"


def test_stats_report_the_coverage():
    sid = _richest_session_id()
    sess = HermesNormalizer(HERMES_DB).normalize(sid)
    stats = sess.stats()
    assert stats["turns"] > 0
    assert stats["edges"] > 0
    assert stats["human"] > 0
