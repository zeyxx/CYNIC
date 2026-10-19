"""Claude Code + Codex normalizer contract tests — against the real archives.

Each harness must yield a VALID canonical session and preserve what it natively
knows while leaving the rest None (anti-fabrication). Skips cleanly if the real
archives are absent.
"""

from __future__ import annotations

import glob
import os
from functools import lru_cache

import pytest

from cynic_data.canonical.adapters.normalizers.claude_code import ClaudeCodeNormalizer
from cynic_data.canonical.adapters.normalizers.codex import CodexNormalizer
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

CC_ROOT = "/home/user/.claude/projects"
CX_ROOT = "/home/user/.codex/sessions"


def _cc_files() -> tuple[str, ...]:
    """Tuple, not list: lru_cache requires hashable arguments."""
    return tuple(sorted(glob.glob(f"{CC_ROOT}/**/*.jsonl", recursive=True)))


def _cx_files() -> tuple[str, ...]:
    """Tuple, not list: lru_cache requires hashable arguments."""
    return tuple(sorted(glob.glob(f"{CX_ROOT}/**/*.jsonl", recursive=True)))


@lru_cache(maxsize=None)
def _first_valid(normalizer, files):
    """Return the first session that normalizes without error.

    Contract tests must not depend on corpus size or which session happens to be
    the largest. Any valid session proves the contract.
    """
    for f in files:
        try:
            return normalizer().normalize(f)
        except Exception:
            continue
    return None


# ---------------------------------------------------------------- claude_code

cc_mark = pytest.mark.skipif(not _cc_files(), reason="claude code archive not present")


@cc_mark
def test_claude_code_yields_valid_session():
    found = _first_valid(ClaudeCodeNormalizer, _cc_files())
    assert found is not None, "no claude code session parsed"
    assert found is not None
    sess = found
    assert sess is not None
    assert sess is not None
    sess.check()


@cc_mark
def test_claude_code_reply_edges_are_native():
    """Claude Code HAS parentUuid -> reply edges must be NATIVE, not derived."""
    found = _first_valid(ClaudeCodeNormalizer, _cc_files())
    assert found is not None
    sess = found
    assert sess is not None
    replies = [e for e in sess.edges if e.kind is EdgeKind.REPLY]
    assert replies, "expected native reply edges from parentUuid"
    assert all(e.provenance is EdgeProvenance.NATIVE for e in replies)


@cc_mark
def test_claude_code_tool_results_link_back():
    found = _first_valid(ClaudeCodeNormalizer, _cc_files())
    assert found is not None
    sess = found
    assert sess is not None
    results = [t for t in sess.tools if t.tool_name == "tool_result"]
    assert results, "expected tool results"
    assert any(t.parent_exec_uid for t in results), "results must link to their call"


@cc_mark
def test_claude_code_preserves_native_model():
    found = _first_valid(ClaudeCodeNormalizer, _cc_files())
    assert found is not None
    sess = found
    assert sess is not None
    models = {t.native_model for t in sess.turns if t.native_model}
    assert models, "message.model must be preserved on the turn"


# ---------------------------------------------------------------------- codex

cx_mark = pytest.mark.skipif(not _cx_files(), reason="codex archive not present")


@cx_mark
def test_codex_yields_valid_session():
    found = _first_valid(CodexNormalizer, _cx_files())
    assert found, "no codex session parsed"
    assert found is not None
    sess = found
    assert sess is not None
    sess.check()


@cx_mark
def test_codex_sequence_edges_are_derived():
    """Codex has NO parent pointer -> sequence edges must be DERIVED."""
    found = _first_valid(CodexNormalizer, _cx_files())
    assert found is not None
    sess = found
    assert sess is not None
    seq = [e for e in sess.edges if e.kind is EdgeKind.SEQUENCE]
    assert seq, "expected derived sequence edges"
    assert all(e.provenance is EdgeProvenance.DERIVED for e in seq)


@cx_mark
def test_codex_turn_context_becomes_state():
    """turn_context carries model/approval/sandbox -> a real StateSnapshot."""
    found = _first_valid(CodexNormalizer, _cx_files())
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.states, "turn_context must produce state snapshots"
    assert any(s.model for s in sess.states), "state must carry the native model"


@cx_mark
def test_codex_reasoning_is_captured():
    found = _first_valid(CodexNormalizer, _cx_files())
    assert found is not None
    sess = found
    assert sess is not None
    assert sess.reasoning, "codex summary must be preserved as reasoning"


@cx_mark
def test_codex_tool_result_canonical_name():
    """Tool results use the canonical name across all harnesses."""
    found = _first_valid(CodexNormalizer, _cx_files())
    assert found is not None
    sess = found
    assert sess is not None
    names = {t.tool_name for t in sess.tools}
    assert "tool_result" in names, "canonical tool_result name must be used"


# ------------------------------------------------------------------ both

@pytest.mark.skipif(not (_cc_files() and _cx_files()), reason="archives not present")
def test_every_session_of_both_harnesses_validates():
    """Full sweep: no file may produce an invalid canonical session."""
    broken = []
    for norm, files in ((ClaudeCodeNormalizer, _cc_files()), (CodexNormalizer, _cx_files())):
        for f in files:
            try:
                norm().normalize(f).check()
            except Exception as exc:
                broken.append(f"{f.split('/')[-1][:40]}: {type(exc).__name__}: {exc}")
    assert not broken, "invalid sessions:\n" + "\n".join(broken[:10])