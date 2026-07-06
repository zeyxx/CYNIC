#!/usr/bin/env python3
"""Compatibility wrapper for the supervisor topology validator."""

from __future__ import annotations

from functools import wraps
from pathlib import Path
import sys
from typing import Any

try:
    from organs.supervisor.controllers._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.supervisor.src.controllers import validate_repo_topology as _impl


def _sync_state() -> None:
    """Keep implementation globals aligned with caller-side monkeypatches."""

    for key, value in globals().items():
        if key.startswith("_") or key in {"_sync_state", "_sync_call", "_export_constants", "_is_public", "_impl", "Path", "sys", "PROJECT_ROOT", "Any", "wraps"}:
            continue
        setattr(_impl, key, value)


def _is_public(value: Any) -> bool:
    return not str(value).startswith("<module")


def _export_constants() -> None:
    for key, value in _impl.__dict__.items():
        if key.startswith("_"):
            continue
        if callable(value):
            continue
        if _is_public(value):
            globals()[key] = value


def _sync_call(name: str):
    target = getattr(_impl, name)
    if not callable(target):
        return target

    @wraps(target)
    def _wrapped(*args: Any, **kwargs: Any) -> Any:
        _sync_state()
        return target(*args, **kwargs)

    return _wrapped


def __getattr__(name: str) -> Any:
    if name.startswith("_"):
        raise AttributeError(name)
    return _sync_call(name)


_export_constants()


if __name__ == "__main__":
    raise SystemExit(__getattr__("main")())
