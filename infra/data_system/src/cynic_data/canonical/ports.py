"""Canonical ports — what the canonical layer needs from the outside world."""

from __future__ import annotations

from typing import Protocol

from .domain.session import NormalizedSession


class NormalizerPort(Protocol):
    """A harness adapter: one bound source -> canonical sessions.

    Each harness has its own models and its own shape. A normalizer does NOT have
    to produce a uniform result — it must produce a *correct* one, and leave
    unobserved fields as None.

    The adapter, not the caller, decides what a session is:
      * Hermes     — a SQLite database; a session is a ``messages.session_id`` value
      * Codex      — a directory of rollout JSONL files; a session is one file
      * ClaudeCode — a directory tree of session JSONL files; a session is one file

    The application layer never supplies session ids: where sessions live and what
    identifies one is harness knowledge that belongs to the adapter. Only the
    normalized domain object crosses the port.
    """

    harness: str

    def can_read(self, source_ref: str) -> bool:
        """Whether this adapter can normalize the given source reference."""
        ...

    def session_refs(self) -> list[str]:
        """Enumerate this source's sessions. Empty list when the source is absent.

        Returned values are opaque: they mean nothing outside this adapter.
        """
        ...

    def normalize(self, session_ref: str) -> NormalizedSession:
        """Normalize one session of this adapter's bound source."""
        ...


class CanonicalStorePort(Protocol):
    """Persistence for canonical entities. A store never invents data."""

    def write(self, session: NormalizedSession) -> None: ...

    def read(self, session_uid: str) -> NormalizedSession | None: ...

    def count(self, table: str) -> int: ...
