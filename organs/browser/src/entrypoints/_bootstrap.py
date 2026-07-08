"""Shared repository bootstrap for organ-browser entrypoints."""

from __future__ import annotations

import os
import sys
from pathlib import Path


def resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return Path(__file__).resolve().parent


def bootstrap_repo_path() -> Path:
    root = resolve_project_root()
    if str(root) not in sys.path:
        sys.path.insert(0, str(root))
    return root
