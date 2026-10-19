"""Content addressing and stable identifiers.

Content addressing is sha256 over canonical bytes. Identifiers are deterministic
so that a re-run of the pipeline is idempotent.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass


def content_hash(payload: bytes) -> str:
    """Return the canonical sha256 digest of a payload, prefixed with its algorithm."""
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def short_hash(text: str, length: int = 12) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:length]


@dataclass(frozen=True)
class SessionUid:
    """A harness-qualified session identifier: ``<harness>:<raw_id>``."""

    harness: str
    raw_id: str

    def __str__(self) -> str:
        return f"{self.harness}:{self.raw_id}"


def turn_uid(session_uid: str, index: int) -> str:
    return f"{session_uid}:t{index:04d}"


def tool_uid(session_uid: str, index: int) -> str:
    return f"{session_uid}:x{index:04d}"


def reasoning_uid(session_uid: str, index: int) -> str:
    return f"{session_uid}:r{index:04d}"


def edge_uid(session_uid: str, kind: str, ordinal: int) -> str:
    return f"{session_uid}:e{ordinal:04d}:{kind}"


def state_uid(session_uid: str, index: int) -> str:
    return f"{session_uid}:s{index:04d}"


def human_uid(session_uid: str, turn: str) -> str:
    return f"{session_uid}:h:{short_hash(turn, 10)}"
