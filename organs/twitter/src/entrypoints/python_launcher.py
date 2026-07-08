#!/usr/bin/env python3
"""Resolve the organ Python interpreter and exec a target entrypoint."""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path


def _candidate_env(name: str) -> str | None:
    value = os.environ.get(name, "").strip()
    return value or None


def resolve_python() -> str:
    for name in ("CYNIC_ORGAN_PYTHON_BIN", "HERMES_PYTHON_BIN", "CYNIC_PYTHON_BIN"):
        candidate = _candidate_env(name)
        if candidate and os.access(candidate, os.X_OK):
            return candidate

    for name in ("CYNIC_VENV", "HERMES_VENV"):
        root = _candidate_env(name)
        if root:
            candidate = Path(root) / "bin" / "python3"
            if candidate.exists() and os.access(candidate, os.X_OK):
                return str(candidate)

    default_venv = Path.home() / ".cynic-venvs" / "cynic" / "bin" / "python3"
    if default_venv.exists() and os.access(default_venv, os.X_OK):
        return str(default_venv)

    candidate = shutil.which("python3")
    if candidate:
        return candidate
    raise SystemExit("No usable python3 interpreter found for organ-twitter")


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args:
        print("usage: python_launcher.py <entrypoint.py> [args...]", file=sys.stderr)
        return 2
    os.execv(resolve_python(), [resolve_python(), *args])
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
