"""Capture ports."""

from __future__ import annotations

from typing import Protocol

from .domain.records import CaptureRecord


class SourceCapturerPort(Protocol):
    """Freeze one source into an immutable, content-addressed copy."""

    source_kind: str

    def can_capture(self, source_path: str) -> bool: ...

    def capture(self, harness: str, source_path: str, dest_dir: str) -> CaptureRecord: ...
