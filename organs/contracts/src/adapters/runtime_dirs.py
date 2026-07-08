"""Shared runtime directory creation helpers."""

from __future__ import annotations

from pathlib import Path
from typing import Iterable


def ensure_parent_dirs(paths: Iterable[str | Path]) -> None:
    for path in paths:
        Path(path).expanduser().parent.mkdir(parents=True, exist_ok=True)


def ensure_dirs(paths: Iterable[str | Path]) -> None:
    for path in paths:
        Path(path).expanduser().mkdir(parents=True, exist_ok=True)
