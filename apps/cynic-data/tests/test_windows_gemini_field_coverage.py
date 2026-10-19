"""Windows-Gemini field-level conservation proof — closed world.

Mirrors the OMP proof: every raw field path observed in the corpus maps to
exactly one fate in ``FIELD_FATE``. A new harness field fails here first.
"""

from __future__ import annotations

import glob
import json
import os
from collections import Counter

import pytest

from cynic_data.canonical.adapters.normalizers.windows_gemini import (
    FIELD_FATE,
    WindowsGeminiNormalizer,
    _load_chat,
)

WG_ROOT = "/var/lib/cynic/vault/archive/windows_sessions_2026/zeyxm_dot_gemini"


def _chat_files() -> list[str]:
    return sorted(glob.glob(f"{WG_ROOT}/tmp/*/chats/session-*.json"))


def _observed_paths() -> Counter:
    paths: Counter = Counter()
    for f in _chat_files():
        try:
            data, _ = _load_chat(f)
        except (json.JSONDecodeError, OSError):
            continue
        if not isinstance(data, dict):
            paths["<non-dict-root>"] += 1
            continue
        for k in data.keys():
            if k == "messages":
                continue
            paths[f"*.{k}"] += 1
        for m in data.get("messages", []):
            if not isinstance(m, dict):
                continue
            for k in m.keys():
                if k == "content":
                    c = m[k]
                    if isinstance(c, list):
                        for b in c:
                            if isinstance(b, dict):
                                if "text" in b:
                                    paths["user_block.text"] += 1
                                if "inlineData" in b:
                                    paths["user_block.inlineData"] += 1
                                if "functionResponse" in b:
                                    paths["user_block.functionResponse"] += 1
                    else:
                        paths["message.content"] += 1
                elif k in ("thoughts", "tokens", "toolCalls"):
                    paths[f"message.{k}"] += 1
                else:
                    paths[f"message.{k}"] += 1
            for th in m.get("thoughts") or []:
                if isinstance(th, dict):
                    for k in th.keys():
                        paths[f"thought.{k}"] += 1
            toks = m.get("tokens")
            if isinstance(toks, dict):
                for k in toks.keys():
                    paths[f"tokens.{k}"] += 1
            for tc in m.get("toolCalls") or []:
                if not isinstance(tc, dict):
                    continue
                for k in tc.keys():
                    if k == "result":
                        paths["toolCall.result"] += 1
                        for item in tc["result"] or []:
                            if isinstance(item, dict) and isinstance(item.get("functionResponse"), dict):
                                fr = item["functionResponse"]
                                for fk in fr.keys():
                                    paths[f"functionResponse.{fk}"] += 1
                    else:
                        paths[f"toolCall.{k}"] += 1
    return paths


wg_mark = pytest.mark.skipif(not _chat_files(), reason="windows_gemini archive not present")


@wg_mark
def test_closed_world_every_observed_path_has_a_fate():
    paths = _observed_paths()
    assert paths, "no raw paths observed"
    missing = sorted(p for p in paths if p not in FIELD_FATE)
    assert not missing, "raw fields with no declared fate:\n" + "\n".join(missing)


@wg_mark
def test_declared_fates_all_start_with_declared_or_preserved():
    bad = [k for k, v in FIELD_FATE.items() if not v.startswith(("preserved:", "declared:"))]
    assert not bad, f"fates must be preserved:/declared:: {bad}"


@wg_mark
def test_summaries_survive_in_meta():
    """Human-authored session summaries must not be lost."""
    norm = WindowsGeminiNormalizer()
    n = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        if s.native_meta.get("summary"):
            n += 1
    assert n > 0, f"expected archived summaries in meta, got {n}"


@wg_mark
def test_masked_results_preserved_as_recorded():
    """Redaction markers are harness truth: preserved, never resolved."""
    norm = WindowsGeminiNormalizer()
    n_masked = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        for t in s.tools:
            if t.raw_output and ("<tool_output_masked>" in t.raw_output or "[TRUNCATED]" in t.raw_output):
                n_masked += 1
    assert n_masked > 100, f"expected masked outputs preserved inline, got {n_masked}"


@wg_mark
def test_tokens_values_match_raw():
    """State snapshots must carry the exact raw token counts."""
    norm = WindowsGeminiNormalizer()
    checked = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        by_turn = {t.turn_uid: t for t in s.turns}
        for st in s.states:
            turn = by_turn.get(st.turn_uid or "")
            assert turn is not None
            checked += 1
            assert st.context_prompt_tokens is not None
    assert checked > 4000, f"proof too thin: {checked} states"
