"""Shared project-root resolution helpers."""

from __future__ import annotations

import os
import sys
from pathlib import Path


def _repo_root_from_candidate(
    candidate: Path,
    *,
    required_directories: tuple[str, ...],
) -> Path | None:
    current = candidate.resolve()
    if current.is_file():
        current = current.parent
    for parent in (current, *current.parents):
        if all((parent / name).exists() for name in required_directories):
            return parent
    return None


def _is_runtime_mirror_root(
    root: Path,
    *,
    home: str | None = None,
    project_slug: str | None = None,
) -> bool:
    runtime_home = Path(home or Path.home()).expanduser().resolve()
    slug = project_slug or os.environ.get("CYNIC_PROJECT_SLUG", "CYNIC")
    return root == (runtime_home / ".cynic" / "repos" / slug)


def _local_repo_root() -> Path:
    candidate = Path(__file__).resolve()
    for current in (candidate.parent, *candidate.parents):
        if all((current / name).exists() for name in ("organs", "infra")):
            return current
    return candidate.parent


def _runtime_repo_root(
    *,
    home: str | None = None,
    project_slug: str | None = None,
    required_directories: tuple[str, ...] = ("organs", "infra"),
) -> Path | None:
    runtime_home = Path(home or Path.home()).expanduser().resolve()
    slug = project_slug or os.environ.get("CYNIC_PROJECT_SLUG", "CYNIC")
    candidate = runtime_home / ".cynic" / "repos" / slug
    if all((candidate / name).exists() for name in required_directories):
        return candidate
    return None


def resolve_repo_root(
    start: Path,
    fallback_depth: int = 4,
    fallback_directories: tuple[str, ...] = ("organs", "infra"),
    prefer_runtime_mirror: bool = False,
) -> Path:
    candidate = start.resolve()
    candidate_root = _repo_root_from_candidate(
        candidate,
        required_directories=fallback_directories,
    )

    local_root = _local_repo_root()
    if candidate_root is not None and _is_runtime_mirror_root(candidate_root):
        return candidate_root

    explicit_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if explicit_root:
        explicit = Path(explicit_root).expanduser().resolve()
        if explicit.exists() and all((explicit / name).exists() for name in fallback_directories) and explicit != local_root:
            return explicit

    if prefer_runtime_mirror:
        runtime_root = _runtime_repo_root(required_directories=fallback_directories)
        if runtime_root is not None and runtime_root != local_root:
            return runtime_root

    if candidate_root is not None:
        return candidate_root

    if candidate.is_file():
        candidate = candidate.parent
    current = candidate
    for _ in range(max(fallback_depth, 0)):
        if current.parent == current:
            break
        current = current.parent
    return current


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


def bootstrap_project_root(__file__: str, fallback_depth: int = 4) -> Path:
    root = resolve_project_root_from_script(__file__, fallback_depth=fallback_depth)
    if str(root) not in sys.path:
        sys.path.insert(0, str(root))
    return root
