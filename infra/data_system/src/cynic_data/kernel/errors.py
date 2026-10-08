"""Framework errors."""

from __future__ import annotations


class DataSystemError(Exception):
    """Base error for the data-centric framework."""


class ContractViolation(DataSystemError):
    """A layer or adapter broke its declared contract."""


class CaptureNotFound(DataSystemError):
    """The requested capture record does not exist."""


class SourceSchemaDrift(DataSystemError):
    """The source is missing a table the contract declares as in-scope.

    Fail-closed: an auxiliary read that cannot find its table must NOT degrade
    to an empty value (which would silently drop session truth). A missing
    in-scope table means the source is not the shape the contract was derived
    from, and the run must stop rather than publish a quiet loss."""
