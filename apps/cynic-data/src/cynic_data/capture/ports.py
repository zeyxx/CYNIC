"""Capture ports."""

from __future__ import annotations

from typing import Protocol

from .domain.records import CaptureManifest, CaptureRecord


class SourceCapturerPort(Protocol):
    """Freeze one source into an immutable, content-addressed copy."""

    source_kind: str

    def can_capture(self, source_path: str) -> bool: ...

    def capture(self, harness: str, source_path: str, dest_dir: str) -> CaptureRecord: ...


class ManifestWriterPort(Protocol):
    """Persist a capture manifest to disk (I/O lives behind this port)."""

    def write(self, manifest: "CaptureManifest", path: str) -> None: ...
