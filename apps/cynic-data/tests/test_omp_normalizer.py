"""OMP normalizer contract test — runs against the real archive (read-only).

OMP is the NATIVE-pointer family: every record carries ``parentId``, so reply
edges must be NATIVE. Tool results ride as their own ``toolResult`` role linked
via ``toolCallId``; execution starts are explicitly stated by
``custom/tool_execution_start`` records. Skips cleanly if the archive is absent.
"""

from __future__ import annotations

import glob
import os
from functools import lru_cache

import pytest

from cynic_data.canonical.adapters.normalizers.omp import OmpNormalizer
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

OMP_ROOT = "/home/user/.omp/agent/sessions"


def _omp_files() -> tuple[str, ...]:
    """Tuple, not list: lru_cache requires hashable arguments."""
    return tuple(sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)))


@lru_cache(maxsize=None)
def _first_valid():
    """Return the first session that normalizes without error (cached)."""
    norm = OmpNormalizer()
    for f in _omp_files():
        try:
            return norm.normalize(f)
        except Exception:
            continue
    """Return the first session that exercises the contract (cached).

    The first session that merely *parses* may be a trivial Q&A with no
    tools, thinking blocks, or usage snapshots — asserting contract
    properties on it is vacuous. Select the first session carrying tool
    activity, reasoning traces, and token-carrying states instead.
    """
    norm = OmpNormalizer()
    for f in _omp_files():
        try:
            sess = norm.normalize(f)
        except Exception:
            continue
        if sess.tools and sess.reasoning and any(
            s.context_prompt_tokens for s in sess.states
        ):
            return sess
    return None


omp_mark = pytest.mark.skipif(not _omp_files(), reason="omp archive not present")


@omp_mark
def test_omp_yields_valid_session():
    found = _first_valid()
    assert found is not None, "no omp session parsed"
    sess = found
    assert sess is not None
    sess.check()


@omp_mark
def test_omp_reply_edges_are_native():
    """OMP HAS parentId -> reply edges must be NATIVE, not derived."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    replies = [e for e in sess.edges if e.kind is EdgeKind.REPLY]
    assert replies, "expected native reply edges from parentId"
    assert all(e.provenance is EdgeProvenance.NATIVE for e in replies)


@omp_mark
def test_omp_tool_results_link_back():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    results = [t for t in sess.tools if t.tool_name == "tool_result"]
    assert results, "expected tool results"
    assert any(t.parent_exec_uid for t in results), "results must link to their call"


@omp_mark
def test_omp_tool_calls_are_native():
    """tool_execution_start records state execution -> NATIVE tool_call edges."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    calls = [e for e in sess.edges if e.kind is EdgeKind.TOOL_CALL]
    assert calls, "expected tool_call edges"
    assert any(e.provenance is EdgeProvenance.NATIVE for e in calls)


@omp_mark
def test_omp_thinking_becomes_reasoning():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.stats()["reasoning"] > 0, "expected thinking blocks somewhere in the corpus"
    assert sess.reasoning, "thinking blocks must be preserved as reasoning"


@omp_mark
def test_omp_usage_becomes_state():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.states, "assistant usage must produce state snapshots"
    assert any(s.context_prompt_tokens for s in sess.states), (
        "state must carry the native prompt tokens"
    )


@omp_mark
def test_omp_preserves_native_model():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    models = {t.native_model for t in sess.turns if t.native_model}
    assert models, "model_change / message.model must be preserved on the turn"


@omp_mark
def test_every_omp_session_validates():
    """Full sweep: no file may produce an invalid canonical session."""
    broken = []
    norm = OmpNormalizer()
    for f in _omp_files():
        try:
            norm.normalize(f).check()
        except Exception as exc:
            broken.append(f"{f.split('/')[-1][:40]}: {type(exc).__name__}: {exc}")
    assert not broken, "invalid sessions:\n" + "\n".join(broken[:10])


@omp_mark
def test_absent_source_is_empty_corpus():
    assert OmpNormalizer("/nonexistent").session_refs() == []
