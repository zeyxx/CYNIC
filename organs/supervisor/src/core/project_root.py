#!/usr/bin/env python3
"""Project root resolution helpers shared across supervisor modules."""

from __future__ import annotations

from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root


def resolve_project_root(
    start: Path,
    fallback_depth: int = 4,
    fallback_directories: tuple[str, ...] = ("organs", "infra"),
) -> Path:
    return resolve_repo_root(start, fallback_depth=fallback_depth, fallback_directories=fallback_directories)


def resolve_project_root_from_script(__file__: str, fallback_depth: int = 4) -> Path:
    return resolve_project_root(
        Path(__file__).resolve(),
        fallback_depth=fallback_depth,
        fallback_directories=("organs", "infra"),
    )
