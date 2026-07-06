#!/usr/bin/env python3
"""Compatibility entrypoint for telemetry organ."""

from __future__ import annotations

import sys
import os
from pathlib import Path

def _resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate

    return Path(__file__).resolve().parent


PROJECT_ROOT = _resolve_project_root()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.telemetry.src.entrypoints.cli import main as main_entrypoint  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(main_entrypoint())
