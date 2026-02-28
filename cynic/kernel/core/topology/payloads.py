"""Topology change event payloads — data structures for real-time architecture updates."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class SourceChangedPayload:
    """File system detected changes in source code."""
    category: str  # "handlers", "dogs", "judge", "cli"
    files: list[str]  # ["cynic/api/handlers/direct.py", ...]
    timestamp: float


@dataclass
class TopologyDelta:
    """Computed delta between previous and current architecture."""
    added: list[str]  # New handler names
    removed: list[str]  # Removed handler names
    modified: list[str]  # Modified handler names


@dataclass
class TopologyChangedPayload:
    """Architecture changed — handlers added/removed/modified."""
    added_handlers: list[str]
    removed_handlers: list[str]
    modified_handlers: list[str]
    timestamp: float


@dataclass
class TopologyAppliedPayload:
    """Hot-reload succeeded — new handlers now wired and active."""
    handlers_added: int
    handlers_removed: int
    timestamp: float


@dataclass
class TopologyRollbackPayload:
    """Hot-reload failed — rolled back to previous state."""
    reason: str
    timestamp: float


@dataclass
class TopologySnapshotPayload:
    """Complete kernel architecture snapshot."""
    snapshot: dict  # Full KernelMirror snapshot
