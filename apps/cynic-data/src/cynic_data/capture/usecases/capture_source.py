"""Capture use cases — freeze sources into an immutable manifest."""

from __future__ import annotations

import datetime

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


# write_manifest moved to adapters/manifest_writer.py (FileManifestWriter)
# so this use case stays pure (no os/json I/O).
