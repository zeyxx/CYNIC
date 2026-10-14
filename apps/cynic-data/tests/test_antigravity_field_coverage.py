"""Antigravity field-level conservation proof — closed world.

Protobuf has no field names, so the closed world is at field-NUMBER
granularity: every top-level field number and every f5 subfield number
observed in the corpus must map to exactly one fate in ``FIELD_FATE``.
A new harness field fails here first.
"""

from __future__ import annotations

import os
import sqlite3
from collections import Counter

import pytest

from cynic_data.canonical.adapters.normalizers.antigravity import (
    FIELD_FATE,
    AntigravityNormalizer,
    _msg,
    _walk,
)

AG_ROOT = "/var/lib/cynic/vault/archive/linux_sessions_2026/gemini_antigravity/conversations"


def _db_files() -> list[str]:
    import glob

    return sorted(glob.glob(f"{AG_ROOT}/*.db"))


def _observed() -> tuple[Counter, Counter, Counter]:
    """Returns (top_fields, f5_fields, columns)."""
    top: Counter = Counter()
    f5: Counter = Counter()
    cols: Counter = Counter()
    for db in _db_files():
        try:
            con = sqlite3.connect(f"file:{db}?immutable=1", uri=True)
        except sqlite3.Error:
            continue
        try:
            info = {c[1] for c in con.execute("PRAGMA table_info(steps)")}
            for c in info:
                cols[c] += 1
            rows = con.execute(
                "SELECT step_type, step_payload, metadata, error_details,"
                " permissions, task_details, render_info FROM steps"
            ).fetchall()
        except sqlite3.Error:
            con.close()
            continue
        con.close()
        for st, pay, *blobs in rows:
            if pay is None:
                continue
            t = _walk(bytes(pay))
            if t is None:
                top[("<unwalkable>", st)] += 1
                continue
            for f in t:
                top[(st, f)] += 1
            env = _msg(t, 5)
            if isinstance(env, dict):
                for f in env:
                    f5[(st, f)] += 1
            for blob in blobs:
                if blob is not None and _walk(bytes(blob)) is None:
                    top[("<opaque-blob>", st)] += 1
    return top, f5, cols


ag_mark = pytest.mark.skipif(not _db_files(), reason="antigravity archive not present")


@ag_mark
def test_closed_world_every_observed_field_has_a_fate():
    top, f5, cols = _observed()
    assert top, "no raw fields observed"
    missing = []
    for st, f in top:
        if st in ("<unwalkable>", "<opaque-blob>"):
            missing.append(f"{st} type={f}")
        elif f in (1, 4, 5):
            continue  # envelope: covered by steps.* + f5.* fates
        elif f"f{f}.*" not in FIELD_FATE:
            missing.append(f"type={st} f{f}")
    for st, f in f5:
        if f"f5.f{f}" not in FIELD_FATE and f"f5.f{f}" not in {
            # grouped fates cover ranges explicitly
        }:
            # fates group some envelope fields (f6/f7/f8, f21/f22/f23, ...)
            grouped = (
                (f in (6, 7, 8) and "f5.f6/f7/f8" in FIELD_FATE)
                or (f in (21, 22, 23) and "f5.f21/f22/f23" in FIELD_FATE)
                or (f in (30, 31, 32) and "f5.f30/f31/f32" in FIELD_FATE)
            )
            if not grouped:
                missing.append(f"type={st} f5.f{f}")
    for c in cols:
        if f"steps.{c}" not in FIELD_FATE:
            missing.append(f"column steps.{c}")
    assert not missing, "raw fields with no declared fate:\n" + "\n".join(sorted(set(missing)))


@ag_mark
def test_declared_fates_all_start_with_declared_or_preserved():
    bad = [k for k, v in FIELD_FATE.items() if not v.startswith(("preserved:", "declared:"))]
    assert not bad, f"fates must be preserved:/declared:: {bad}"


@ag_mark
def test_status_column_matches_payload():
    """steps.status == payload f4 on every step (no silent divergence)."""
    norm = AntigravityNormalizer()
    checked = mismatched = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        for t in s.turns:
            checked += 1
            if "envelope_mismatch" in t.native_fields:
                mismatched += 1
    assert checked > 50000, f"proof too thin: {checked} steps"
    assert mismatched == 0, f"{mismatched} steps diverge"


@ag_mark
def test_timestamps_cover_all_turns():
    norm = AntigravityNormalizer()
    missing = 0
    for ref in norm.session_refs():
        for t in norm.normalize(ref).turns:
            if t.timestamp_iso is None:
                missing += 1
    assert missing == 0, f"{missing} turns without timestamp"


@ag_mark
def test_no_native_tool_edges_anywhere():
    """The no-linkage proof holds corpus-wide: zero NATIVE tool edges."""
    from cynic_data.canonical.domain.entities import EdgeKind, EdgeProvenance

    norm = AntigravityNormalizer()
    native = 0
    for ref in norm.session_refs():
        s = norm.normalize(ref)
        native += sum(
            1
            for e in s.edges
            if e.kind in (EdgeKind.TOOL_CALL, EdgeKind.TOOL_RESULT)
            and e.provenance is EdgeProvenance.NATIVE
        )
    assert native == 0, f"{native} fabricated NATIVE tool edges"
