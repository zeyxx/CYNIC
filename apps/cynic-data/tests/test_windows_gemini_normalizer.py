"""Windows-Gemini normalizer contract test — runs against the real archive (read-only).

Windows-Gemini has NO parent pointer, so sequence edges are DERIVED; tool
edges are NATIVE (inline toolCalls with native status). A session is a
sessionId group: merged chat files plus an inventoried (never content-linked)
tool-outputs directory. Skips cleanly if the archive is absent.
"""

from __future__ import annotations

import glob
import os
from functools import lru_cache

import pytest

from cynic_data.canonical.adapters.normalizers.windows_gemini import WindowsGeminiNormalizer
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

WG_ROOT = "/var/lib/cynic/vault/archive/windows_sessions_2026/zeyxm_dot_gemini"


def _has_archive() -> bool:
    return os.path.isdir(os.path.join(WG_ROOT, "tmp"))


@lru_cache(maxsize=None)
def _first_valid():
    """Return the first session that normalizes without error (cached)."""
    norm = WindowsGeminiNormalizer()
    for ref in norm.session_refs():
        try:
            return norm.normalize(ref)
        except Exception:
            continue
    return None


wg_mark = pytest.mark.skipif(not _has_archive(), reason="windows_gemini archive not present")


@wg_mark
def test_sessions_enumerate_by_session_id():
    refs = WindowsGeminiNormalizer().session_refs()
    assert len(refs) > 0, f"expected the archived sessions, got {len(refs)}"
    assert all(r for r in refs)


@wg_mark
def test_yields_valid_session():
    found = _first_valid()
    assert found is not None, "no windows_gemini session parsed"
    sess = found
    assert sess is not None
    sess.check()


@wg_mark
def test_sequence_edges_are_derived():
    """No parent pointer -> sequence edges must be DERIVED."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    seq = [e for e in sess.edges if e.kind is EdgeKind.SEQUENCE]
    assert seq, "expected derived sequence edges"
    assert all(e.provenance is EdgeProvenance.DERIVED for e in seq)


@wg_mark
def test_tool_calls_and_results_are_native():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    calls = [e for e in sess.edges if e.kind is EdgeKind.TOOL_CALL]
    results = [e for e in sess.edges if e.kind is EdgeKind.TOOL_RESULT]
    assert calls and results, "expected inline tool edges"
    assert all(e.provenance is EdgeProvenance.NATIVE for e in calls + results)


@wg_mark
def test_error_status_is_captured():
    """status error/cancelled must survive as is_error (never a silent success)."""
    norm = WindowsGeminiNormalizer()
    n_err = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        n_err += sum(1 for t in s.tools if t.is_error)
    assert n_err > 0, f"expected the archived error/cancelled calls, got {n_err}"


@wg_mark
def test_thoughts_become_reasoning_with_subject():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.stats()["reasoning"] > 0, "expected thoughts somewhere in the corpus"
    assert sess.reasoning, "thoughts must be preserved as reasoning"
    assert any(r.subject for r in sess.reasoning), "thought subjects must survive"


@wg_mark
def test_tokens_become_state():
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.states, "tokens must produce state snapshots"
    assert any(s.context_prompt_tokens for s in sess.states)


@wg_mark
def test_tool_outputs_inventoried_not_linked():
    """Full outputs have no per-call key: inventoried in meta, never linked."""
    found = _first_valid()
    assert found is not None
    sess = found
    assert sess is not None
    inv = sess.native_meta.get("tool_outputs") or {}
    assert inv.get("files", 0) > 0, "expected an inventoried tool-outputs dir"
    assert inv.get("manifest", "").startswith("sha256:")
    for t in sess.tools:
        assert "tool-outputs" not in (t.command_or_args or ""), "no fabricated file links"


@wg_mark
def test_every_session_validates():
    """Full sweep: no session may produce an invalid canonical session."""
    broken = []
    norm = WindowsGeminiNormalizer()
    for ref in norm.session_refs():
        try:
            norm.normalize(ref).check()
        except Exception as exc:
            broken.append(f"{ref[:40]}: {type(exc).__name__}: {exc}")
    assert not broken, "invalid sessions:\n" + "\n".join(broken[:10])


@wg_mark
def test_absent_source_is_empty_corpus():
    assert WindowsGeminiNormalizer("/nonexistent").session_refs() == []
