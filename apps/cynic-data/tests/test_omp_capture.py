"""OMP capture proof — the frozen copy is the only ingestion source.

Proves: a frozen OMP tree normalizes byte-identically (stats) to the live
tree, and later live mutations do not move the capture. Without this, the
field-level conservation proof would rest on mutable bytes.
"""

from __future__ import annotations

import glob
import json
import os

import pytest

from cynic_data.canonical.adapters.normalizers.omp import OmpNormalizer
from cynic_data.capture.adapters.capturer_directory import DirectorySourceCapturer
from cynic_data.capture.usecases.capture_source import capture_sources

OMP_ROOT = "/home/user/.omp/agent/sessions"

omp_mark = pytest.mark.skipif(
    not os.path.isdir(OMP_ROOT), reason="omp archive not present"
)


def _stats_for(root: str) -> dict:
    norm = OmpNormalizer(root)
    refs = norm.session_refs()
    assert refs, "no sessions enumerated"
    assert all(norm.can_read(r) for r in refs), "can_read must hold for every enumerated ref"
    totals: dict[str, int] = {"sessions": 0}
    for ref in refs:
        for k, v in norm.normalize(ref).stats().items():
            totals[k] = totals.get(k, 0) + v
        totals["sessions"] += 1
    return totals


@omp_mark
def test_frozen_capture_normalizes_identically_to_live(tmp_path):
    live = _stats_for(OMP_ROOT)

    manifest = capture_sources(
        sources={"omp": OMP_ROOT},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("omp")
    assert rec is not None
    assert rec.content_hash.startswith("sha256:")

    frozen = _stats_for(rec.captured_path)
    assert frozen == live, (
        "frozen capture must normalize identically to live:\n"
        + "\n".join(f"  {k}: live={live.get(k)} frozen={frozen.get(k)}" for k in live if live.get(k) != frozen.get(k))
    )


@omp_mark
def test_capture_immutable_under_live_mutation(tmp_path):
    """Prove immutability without mutating the live production tree.

    Copies a small subset of the live tree into a disposable source, captures
    it, then mutates the disposable copy. The live tree is never touched.
    """
    import shutil

    # Build a disposable source from a few live sessions (never mutate live).
    live = OmpNormalizer(OMP_ROOT)
    refs = live.session_refs()[:3]
    assert refs, "need at least one live session to copy"
    disposable = tmp_path / "disposable_src"
    disposable.mkdir()
    for ref in refs:
        src = os.path.join(OMP_ROOT, ref)
        if os.path.isfile(src):
            shutil.copy2(src, disposable)

    manifest = capture_sources(
        sources={"omp": str(disposable)},
        dest_dir=str(tmp_path / "captures"),
        capturers=[DirectorySourceCapturer()],
    )
    rec = manifest.by_harness("omp")
    assert rec is not None
    before = _stats_for(rec.captured_path)

    # Mutate the DISPOSABLE source after capture: add a session file.
    probe = disposable / ".cynic-proof-probe.jsonl"
    with open(probe, "w", encoding="utf-8") as fh:
        fh.write('{"type":"title","v":1,"title":"probe","updatedAt":"2026-01-01T00:00:00Z"}\n')
    after = _stats_for(rec.captured_path)
    assert after == before, "capture must not move when source mutates"
    # the probe file existed only in the disposable source: frozen refs must not contain it
    assert not any("proof-probe" in r for r in OmpNormalizer(rec.captured_path).session_refs())


@omp_mark
def test_absent_live_source_fails_closed(tmp_path):
    with pytest.raises(Exception):
        capture_sources(
            sources={"omp": "/nonexistent/omp-sessions"},
            dest_dir=str(tmp_path / "captures"),
            capturers=[DirectorySourceCapturer()],
        )
