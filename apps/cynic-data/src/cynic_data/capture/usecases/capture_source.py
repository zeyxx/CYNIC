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
    *,
    strict: bool = True,
) -> CaptureManifest:
    """Freeze every ``harness -> source_path`` into ``dest_dir``.

    strict=True (default): fails closed — the run stops with an explicit
    error rather than silently falling back to reading live data.
    strict=False: an uncapturable source is skipped and recorded on the
    manifest's failures; one corrupt archive no longer kills the run.
    """
    records: list[CaptureRecord] = []
    failures: list[tuple[str, str]] = []
    for harness, source_path in sources.items():
        capturer = _pick_capturer(capturers, source_path)
        if capturer is None:
            reason = f"no capturer can handle {harness} source: {source_path}"
            if strict:
                raise RuntimeError(reason)
            failures.append((harness, reason))
            continue
        try:
            records.append(capturer.capture(harness, source_path, dest_dir))
        except Exception as exc:  # noqa: BLE001 — recorded on the manifest, never silent
            if strict:
                raise
            failures.append((harness, f"{type(exc).__name__}: {exc}"))

    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    manifest_uid = "cap:" + short_hash("|".join(r.content_hash for r in records), 16)
    return CaptureManifest(
        manifest_uid=manifest_uid,
        created_at_iso=now,
        records=tuple(records),
        failures=tuple(failures),
    )


# write_manifest moved to adapters/manifest_writer.py (FileManifestWriter)
# so this use case stays pure (no os/json I/O).
