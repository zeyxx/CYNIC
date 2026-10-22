"""Canonical ports — what the canonical layer needs from the outside world."""

from __future__ import annotations

from typing import Protocol

from .domain.conservation import ConservationContract
from .domain.session import NormalizedSession


class NormalizerPort(Protocol):
    """A harness adapter: one bound source -> canonical sessions.

    Each harness has its own models and its own shape. A normalizer does NOT have
    to produce a uniform result — it must produce a *correct* one, and leave
    unobserved fields as None.

    The adapter, not the caller, decides what a session is:
      * Hermes     — a SQLite database; a session is a ``messages.session_id`` value
      (Codex, ClaudeCode retired T6, capsules in ``docs/specs/harness-*.md``.)

    The application layer never supplies session ids: where sessions live and what
    identifies one is harness knowledge that belongs to the adapter. Only the
    normalized domain object crosses the port.
    """

    harness: str

    def can_read(self, source_ref: str) -> bool:
        """Whether this adapter can normalize the given source reference."""
        ...

    def conservation(self) -> ConservationContract | None:
        """The conservation contract for this harness, or None if none exists yet.

        α6: an adapter that declares a contract is held to it — its producer's DDL
        must match the contract's field set, and the schema version must match.
        An adapter without a contract is an explicit unknown, never an implicit
        "no loss". Returning None is allowed during migration; the audit reports it.
        """
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

    def write(
        self, session: NormalizedSession, manifest_uid: str | None = None
    ) -> None: ...

    def read(self, session_uid: str) -> NormalizedSession | None: ...

    def session_content_hash(self, session_uid: str) -> str | None:
        """Hash recorded at the last write, or None when never written."""
        ...

    def record_run(self, run: dict) -> None:
        """Persist a normalize-run ledger entry (crash visibility, resume proof)."""
        ...

    def count(self, table: str) -> int: ...
