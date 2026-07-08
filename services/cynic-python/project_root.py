"""Portable helpers for resolving the CYNIC repository root from services code."""

# Tier 3
from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path
import sys


def _iter_candidate_paths(start: Path) -> Iterable[Path]:
    base = start if start.is_dir() else start.parent
    yield base
    yield from base.parents


def _bootstrap_repo_root(start: Path) -> None:
    for candidate in _iter_candidate_paths(start):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            root_str = str(candidate)
            if root_str not in sys.path:
                sys.path.insert(0, root_str)
            return


_bootstrap_repo_root(Path(__file__).resolve())

from organs.contracts.src.project_root import resolve_repo_root as _resolve_repo_root


def resolve_repo_root(start: Path, fallback_depth: int = 4, fallback_directories: tuple[str, ...] = ("organs", "infra")) -> Path:
    return _resolve_repo_root(start, fallback_depth=fallback_depth, fallback_directories=fallback_directories)


def ensure_repo_on_sys_path(start: Path) -> Path:
    root = resolve_repo_root(start)
    root_str = str(root)
    if root_str not in sys.path:
        sys.path.insert(0, root_str)
    return root
