#!/usr/bin/env python3
"""Compatibility wrapper for the contract audit controller."""

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

from organs.supervisor.src.controllers import audit_organs as _impl


def _sync_state() -> None:
    """Keep implementation globals aligned with caller-side monkeypatches."""

    for key, value in globals().items():
        if key.startswith("_") or key in {"_sync_state", "_sync_call", "_export_constants", "_impl", "Path", "Any", "sys", "PROJECT_ROOT", "wraps"}:
            continue
        setattr(_impl, key, value)


def _export_constants() -> None:
    for key, value in _impl.__dict__.items():
        if key.startswith("_") or callable(value):
            continue
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
