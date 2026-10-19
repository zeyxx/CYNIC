"""OMP field-level conservation proof — closed world.

Record-type counts prove every record became an entity or a declared skip.
This module proves the inside: every raw FIELD PATH observed in the corpus maps
to exactly one fate in ``FIELD_FATE`` (preserved home or declared reason).
A new harness field fails here before it can slip in silently.
"""

from __future__ import annotations

import glob
import json
from collections import Counter

import pytest

from cynic_data.canonical.adapters.normalizers.omp import FIELD_FATE, OmpNormalizer

OMP_ROOT = "/home/user/.omp/agent/sessions"

ENVELOPE = {"id": "*.id", "parentId": "*.parentId", "timestamp": "*.timestamp", "type": "*.type"}


def _observed_paths() -> Counter:
    """Enumerate every raw field path present in the corpus (read-only)."""
    paths: Counter = Counter()

    def add(prefix: str, o: object, depth: int = 0) -> None:
        if depth > 3 or not isinstance(o, dict):
            return
        for k, v in o.items():
            p = prefix + "." + str(k)
            paths[p] += 1
            if isinstance(v, dict) and k in ("usage", "contextSnapshot", "details", "cost", "data"):
                add(p, v, depth + 1)

    for f in sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)):
        with open(f, errors="replace") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    o = json.loads(line)
                except json.JSONDecodeError:
                    continue
                t = o.get("type")
                for k in o.keys():
                    if k in ("message", "data"):
                        continue
                    paths[ENVELOPE.get(k, f"{t}.{k}")] += 1
                if t == "message":
                    m = o.get("message") or {}
                    for k in m.keys():
                        if k == "content":
                            c = m[k]
                            if isinstance(c, list):
                                for b in c:
                                    if isinstance(b, dict):
                                        paths[f"message.content[]:{b.get('type')}"] += 1
                            elif isinstance(c, str):
                                paths["message.content:<str>"] += 1
                            elif c is None:
                                paths["message.content:<none>"] += 1
                        else:
                            paths[f"message.{k}"] += 1
                    for b in (m.get("content") or []):
                        if isinstance(b, dict) and b.get("type") == "toolCall":
                            for k in b.keys():
                                if k != "type":
                                    paths[f"toolCall.{k}"] += 1
                elif t in ("custom", "custom_message"):
                    paths[f"{t}:{o.get('customType')}"] += 1
                    if isinstance(o.get("data"), dict):
                        add(f"{t}:{o.get('customType')}.data", o["data"], 0)
    return paths


omp_mark = pytest.mark.skipif(
    not glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True),
    reason="omp archive not present",
)


@omp_mark
def test_closed_world_every_observed_path_has_a_fate():
    paths = _observed_paths()
    assert paths, "no raw paths observed"
    missing = sorted(p for p in paths if p not in FIELD_FATE and not _is_inventory_marker(p))
    assert not missing, "raw fields with no declared fate:\n" + "\n".join(missing)


def _is_inventory_marker(p: str) -> bool:
    # custom:<type> bare markers count records; their .data.* paths carry fate.
    return (p.startswith("custom:") or p.startswith("custom_message:")) and ".data" not in p


@omp_mark
def test_declared_fates_all_start_with_declared_or_preserved():
    bad = [k for k, v in FIELD_FATE.items() if not v.startswith(("preserved:", "declared:"))]
    assert not bad, f"fates must be preserved:/declared:: {bad}"


@omp_mark
def test_system_prompt_survives_in_meta():
    """The session context that explains the whole session must not be lost."""
    found = False
    norm = OmpNormalizer()
    for f in sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)):
        s = norm.normalize(f)
        init = s.native_meta.get("session_init") or {}
        if init.get("systemPrompt"):
            found = True
            assert len(init["systemPrompt"]) > 1000, "system prompt truncated?"
            break
    assert found, "no session_init with systemPrompt in corpus"


@omp_mark
def test_error_fields_survive_on_turns():
    """Failure/retry signal must ride on the turn, not vanish."""
    norm = OmpNormalizer()
    seen = Counter()
    for f in sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)):
        s = norm.normalize(f)
        for t in s.turns:
            for k in ("errorId", "errorMessage", "errorStatus", "retryRecovery"):
                if t.native_fields.get(k) is not None:
                    seen[k] += 1
    # Contract: error fields must survive on turns. Snapshot: errorId=586, errorMessage=210.
    assert seen["errorId"] > 0, f"errorId loss: {dict(seen)}"
    assert seen["errorMessage"] > 0, f"errorMessage loss: {dict(seen)}"


@omp_mark
def test_compaction_details_survive_in_state():
    norm = OmpNormalizer()
    n = 0
    for f in sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)):
        s = norm.normalize(f)
        for st in s.states:
            if st.compacted and st.native_fields.get("details") is not None:
                n += 1
    # Contract: compaction details must survive in state. Snapshot: n=15.
    assert n > 0, f"compaction.details loss: {n}"


@omp_mark
def test_custom_args_proven_redundant():
    """FIELD_FATE declares custom.data.args a strict subset of toolCall.arguments.

    Proven here on the whole corpus, not assumed: the block carries the wire
    args plus the intent gloss (``i``), so dropping the subset is lossless.
    """
    n_sub = n_bad = n_missing = 0
    for f in sorted(glob.glob(f"{OMP_ROOT}/**/*.jsonl", recursive=True)):
        blocks: dict[str, object] = {}
        customs: dict[str, object] = {}
        with open(f, errors="replace") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    o = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if o.get("type") == "message":
                    for b in ((o.get("message") or {}).get("content") or []):
                        if isinstance(b, dict) and b.get("type") == "toolCall" and b.get("id"):
                            blocks[b["id"]] = b.get("arguments")
                elif o.get("type") == "custom" and o.get("customType") == "tool_execution_start":
                    d = o.get("data") or {}
                    if d.get("toolCallId"):
                        customs[d["toolCallId"]] = d.get("args")
        for cid, ca in customs.items():
            if ca is None:
                n_missing += 1
            elif (
                cid in blocks
                and isinstance(ca, dict)
                and isinstance(blocks[cid], dict)
                and _is_redundant(ca, blocks[cid])  # type: ignore[arg-type]
            ):
                n_sub += 1
            else:
                n_bad += 1
    assert n_bad == 0, f"custom.args not a subset of block.arguments in {n_bad} cases"
    assert n_sub > 10000, f"proof too thin: {n_sub} subset relations"


def _is_redundant(custom_args: dict, block_args: dict) -> bool:
    """Every custom arg is carried by the block: equal, or the harness's
    200-char + ellipsis display truncation of the block value."""
    for k, v in custom_args.items():
        b = block_args.get(k)
        if b == v:
            continue
        if isinstance(v, str) and isinstance(b, str) and v.endswith("…") and b.startswith(v[:-1]):
            continue
        return False
    return True
