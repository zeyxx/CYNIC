"""Framework errors."""

from __future__ import annotations


class DataSystemError(Exception):
    """Base error for the data-centric framework."""


class ContractViolation(DataSystemError):
    """A layer or adapter broke its declared contract."""


class CaptureNotFound(DataSystemError):
    """The requested capture record does not exist."""
