#!/usr/bin/env python3
"""Resolve and exec mitmdump for organ-discord passive proxy."""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path


def resolve_mitmdump() -> str:
    for name in ("CYNIC_MITMDUMP_BIN", "HERMES_MITMDUMP_BIN"):
        candidate = os.environ.get(name, "").strip()
        if candidate and os.access(candidate, os.X_OK):
            return candidate

    for name in ("CYNIC_VENV", "HERMES_VENV"):
        root = os.environ.get(name, "").strip()
        if root:
            candidate = Path(root) / "bin" / "mitmdump"
            if candidate.exists() and os.access(candidate, os.X_OK):
                return str(candidate)

    default = Path.home() / ".cynic-venvs" / "cynic" / "bin" / "mitmdump"
    if default.exists() and os.access(default, os.X_OK):
        return str(default)

    candidate = shutil.which("mitmdump")
    if candidate:
        return candidate
    raise SystemExit("No usable mitmdump executable found for organ-discord passive proxy")


def main() -> int:
    binary = resolve_mitmdump()
    os.execv(binary, [binary, *sys.argv[1:]])
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
