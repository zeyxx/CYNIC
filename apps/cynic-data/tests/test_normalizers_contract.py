"""Claude Code normalizer contract tests — against the real archives.

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
from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

CC_ROOT = "/home/user/.claude/projects"


def _cc_files() -> tuple[str, ...]:
    """Tuple, not list: lru_cache requires hashable arguments."""
    return tuple(sorted(glob.glob(f"{CC_ROOT}/**/*.jsonl", recursive=True)))


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


# ------------------------------------------------------------- claude only

@pytest.mark.skipif(not _cc_files(), reason="archives not present")
def test_every_session_of_claude_validates():
    """Full sweep: no file may produce an invalid canonical session."""
    broken = []
    for f in _cc_files():
        try:
            ClaudeCodeNormalizer().normalize(f).check()
        except Exception as exc:
            broken.append(f"{f.split('/')[-1][:40]}: {type(exc).__name__}: {exc}")
    assert not broken, "invalid sessions:\n" + "\n".join(broken[:10])