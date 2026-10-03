"""Canonical ports — what the canonical layer needs from the outside world."""

from __future__ import annotations

from typing import Protocol

from .domain.session import NormalizedSession


class NormalizerPort(Protocol):
    """A harness adapter: raw source -> canonical entities.

    Each harness has its own models and its own shape. A normalizer does NOT have
    to produce a uniform result — it must produce a *correct* one, and leave
    unobserved fields as None.
    """

    harness: str

    def can_read(self, source_ref: str) -> bool: ...

    def normalize(self, source_ref: str) -> NormalizedSession: ...


class CanonicalStorePort(Protocol):
    """Persistence for canonical entities. A store never invents data."""

    def write(self, session: NormalizedSession) -> None: ...

    def read(self, session_uid: str) -> NormalizedSession | None: ...

    def count(self, table: str) -> int: ...
