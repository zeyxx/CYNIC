"""Core topology supervision entities.

This package is the target membrane for organ-supervisor. Legacy controller
modules must shrink toward thin wrappers around core services and adapters.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class GodObjectFinding:
    path: str
    lines: int
    limit: int
    replacement: str


@dataclass(frozen=True)
class TopologyExceptionSummary:
    noncanonical_organs_count: int
    exception_count: int
    god_object_count: int
