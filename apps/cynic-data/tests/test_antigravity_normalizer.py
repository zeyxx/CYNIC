"""Antigravity normalizer contract test — runs against the real archive (read-only).

Antigravity has NO parent pointers (parent_references empty in all DBs), so
sequence edges are DERIVED from steps.idx order; tool edges are DERIVED too
(628 call ids, zero appear in any result step — proven, never linked).
One DB file is dead on disk and must be reported, never silently dropped.
Skips cleanly if the archive is absent.
"""

from __future__ import annotations

import os
from functools import lru_cache

import pytest

from cynic_data.canonical.adapters.normalizers.antigravity import AntigravityNormalizer
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

AG_ROOT = "/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations"


def _has_archive() -> bool:
    return os.path.isdir(AG_ROOT)


@lru_cache(maxsize=None)
def _first_valid():
    """Return the first session with named tool calls AND both tool edge kinds (cached)."""
    norm = AntigravityNormalizer()
    for ref in norm.session_refs():
        try:
            s = norm.normalize(ref)
            has_named = any(t.call_id and t.tool_name for t in s.tools)
            has_calls = any(e.kind is EdgeKind.TOOL_CALL for e in s.edges)
            has_results = any(e.kind is EdgeKind.TOOL_RESULT for e in s.edges)
            if has_named and has_calls and has_results:
                return s
        except Exception:
            continue
    return None


ag_mark = pytest.mark.skipif(not _has_archive(), reason="antigravity archive not present")


@ag_mark
def test_sessions_enumerate_as_trajectory_dbs():
    refs = AntigravityNormalizer().session_refs()
    assert len(refs) > 0, f"expected ~499 readable DBs, got {len(refs)}"
    assert all(r for r in refs)


@ag_mark
def test_dead_db_is_reported_never_silent():
    norm = AntigravityNormalizer()
    dead = norm.broken()
    assert dead, "expected the dead DB file to be reported"
    assert not set(dead) & set(norm.session_refs())


@ag_mark
def test_yields_valid_session():
    found = _first_valid()
    assert found is not None, "no antigravity session parsed"
    sess = found
    assert sess is not None
    sess.check()


@ag_mark
def test_sequence_edges_are_derived():
    """No parent pointers -> sequence edges must be DERIVED."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    seq = [e for e in sess.edges if e.kind is EdgeKind.SEQUENCE]
    assert seq, "expected derived sequence edges"
    assert all(e.provenance is EdgeProvenance.DERIVED for e in seq)


@ag_mark
def test_tool_edges_are_derived_not_native():
    """No call<->result id linkage exists -> tool edges must be DERIVED."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    calls = [e for e in sess.edges if e.kind is EdgeKind.TOOL_CALL]
    results = [e for e in sess.edges if e.kind is EdgeKind.TOOL_RESULT]
    assert calls and results, "expected tool edges"
    assert all(e.provenance is EdgeProvenance.DERIVED for e in calls + results)


@ag_mark
def test_tool_calls_carry_native_ids_and_args():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    named = [t for t in sess.tools if t.call_id and t.tool_name]
    assert named, "expected named tool calls"
    assert any(t.command_or_args for t in named), "args JSON must survive"


@ag_mark
def test_error_records_are_flagged():
    """Error-record types and error_details presence must set is_error."""
    norm = AntigravityNormalizer()
    n_err = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        n_err += sum(1 for t in s.tools if t.is_error)
    assert n_err > 0, f"expected the archived error records, got {n_err}"


@ag_mark
def test_queries_become_human_signals():
    norm = AntigravityNormalizer()
    n = 0
    for ref in norm.session_refs():
        n += len(norm.normalize(ref).human)
    assert n > 0, f"expected query texts as human signals, got {n}"


@ag_mark
def test_every_session_validates():
    """Full sweep: no session may produce an invalid canonical session."""
    broken = []
    norm = AntigravityNormalizer()
    for ref in norm.session_refs():
        try:
            norm.normalize(ref).check()
        except Exception as exc:
            broken.append(f"{ref[:40]}: {type(exc).__name__}: {exc}")
    assert not broken, "invalid sessions:\n" + "\n".join(broken[:10])


@ag_mark
def test_absent_source_is_empty_corpus():
    assert AntigravityNormalizer("/nonexistent").session_refs() == []
