"""
CYNIC Exception Hierarchy

All CYNIC-specific exceptions inherit from CynicError.
Critical paths use specific exceptions instead of bare 'except Exception'.

φ-Law: VERIFY — typed errors, not silent swallowing.
"""
from __future__ import annotations


class CynicError(Exception):
    """Base exception for all CYNIC errors."""


class StorageError(CynicError):
    """Storage backend failure (SurrealDB, PostgreSQL, filesystem)."""


class LLMError(CynicError):
    """LLM call failure (timeout, rate limit, model not found)."""


class JudgmentError(CynicError):
    """Judgment pipeline failure (dog error, consensus failure)."""


class ConfigError(CynicError):
    """Configuration validation failure."""


class CircuitOpenError(CynicError):
    """Circuit breaker is open — fast-fail to prevent cascade."""
