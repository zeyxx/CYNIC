"""Capture use cases — freeze sources into an immutable manifest."""

from __future__ import annotations

import datetime
import json
import os

from ...kernel.ids import short_hash
from ..domain.records import CaptureManifest, CaptureRecord
from ..ports import SourceCapturerPort


def _pick_capturer(capturers: list[SourceCapturerPort], source_path: str) -> SourceCapturerPort | None:
    for c in capturers:
        if c.can_capture(source_path):
            return c
    return None


def capture_sources(
    sources: dict[str, str],
    dest_dir: str,
    capturers: list[SourceCapturerPort],
) -> CaptureManifest:
    """Freeze every ``harness -> source_path`` into ``dest_dir``.

    Fails closed: if a source cannot be captured, the run stops with an explicit
    error rather than silently falling back to reading live data.
    """
    records: list[CaptureRecord] = []
    for harness, source_path in sources.items():
        capturer = _pick_capturer(capturers, source_path)
        if capturer is None:
            raise RuntimeError(f"no capturer can handle {harness} source: {source_path}")
        records.append(capturer.capture(harness, source_path, dest_dir))

    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    manifest_uid = "cap:" + short_hash("|".join(r.content_hash for r in records), 16)
    return CaptureManifest(manifest_uid=manifest_uid, created_at_iso=now, records=tuple(records))


def write_manifest(manifest: CaptureManifest, path: str) -> None:
    payload = {
        "schema": "cynic.capture.manifest.v1",
        "manifest_uid": manifest.manifest_uid,
        "created_at_iso": manifest.created_at_iso,
        "records": [
            {
                "capture_uid": r.capture_uid,
                "harness": r.harness,
                "source_kind": r.source_kind,
                "source_path": r.source_path,
                "captured_path": r.captured_path,
                "content_hash": r.content_hash,
                "size_bytes": r.size_bytes,
                "captured_at_iso": r.captured_at_iso,
                "method": r.method.value,
                "entry_count": r.entry_count,
                "native_meta": r.native_meta,
            }
            for r in manifest.records
        ],
    }
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, sort_keys=True)
