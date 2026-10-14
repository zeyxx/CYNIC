"""Capture entities.

A CaptureRecord is the proof that a source was frozen: a content hash, a size, a
timestamp, and the method used to obtain a consistent copy. It never points at a
live file — the whole point of Layer 1 is that the bytes stop moving.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum


class CaptureMethod(str, Enum):
    """How a consistent copy was obtained."""

    SQLITE_VACUUM_INTO = "sqlite_vacuum_into"   # consistent even with an active WAL
    SQLITE_BACKUP_API = "sqlite_backup_api"
    DIRECTORY_COPY = "directory_copy"
    FILE_COPY = "file_copy"


@dataclass(frozen=True)
class CaptureRecord:
    """One frozen source. Content-addressed, immutable, reproducible."""

    capture_uid: str            # "<harness>:<sha256 prefix>"
    harness: str
    source_kind: str            # "sqlite" | "directory" | "file"
    source_path: str            # where the bytes came from (for provenance only)
    captured_path: str          # the frozen copy — never a symlink
    content_hash: str           # sha256:<hex> over the frozen bytes
    size_bytes: int
    captured_at_iso: str
    method: CaptureMethod
    entry_count: int | None = None      # for directories: number of files
    native_meta: dict = field(default_factory=dict)


@dataclass(frozen=True)
class CaptureManifest:
    """The set of captures taken in one run. This is what makes a run reproducible."""

    manifest_uid: str
    created_at_iso: str
    records: tuple[CaptureRecord, ...]

    def by_harness(self, harness: str) -> CaptureRecord | None:
        for r in self.records:
            if r.harness == harness:
                return r
        return None
