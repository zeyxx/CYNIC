"""Filesystem adapter for supervisor topology scans."""

from __future__ import annotations

from pathlib import Path


def count_lines(path: Path) -> int:
    try:
        return len(path.read_text(encoding="utf-8").splitlines())
    except UnicodeDecodeError:
        return 0
