#!/usr/bin/env python3
"""Compatibility entrypoint for runtime principles.

Executable logic lives under organs/supervisor/src/entrypoints/runtime_principals.py.
This file is a compatibility shim that delegates to the implementation module.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return candidate
    raise RuntimeError("Could not resolve repository root for runtime principals shim")


PROJECT_ROOT = _bootstrap_repo_path()


def _load_runtime_env() -> None:
    from organs.contracts.src.adapters.env import load_env_file

    load_env_file(Path.home() / ".cynic-env")


_load_runtime_env()

RUNTIME_PRINCIPALS_REQUIRED_SCRIPT_COMMANDS = (
    "status",
    "doctor",
    "plan",
    "update",
    "organism",
    "actions",
    "activate",
    "bootstrap-request",
    "bootstrap",
    "mcp-reload",
    "gate-2",
    "deploy",
    "bridge",
    "storage",
    "fleet",
    "mesh",
    "repos",
    "apply",
    "install-defaults",
    "print-launchers",
    "print-profile",
    "print-cynic-cli",
    "handicap",
    "hf",
)

from organs.supervisor.src.entrypoints.runtime_principals import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals import main as _main


if __name__ == "__main__":
    raise SystemExit(_main())
