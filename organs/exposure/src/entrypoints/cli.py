#!/usr/bin/env python3
from __future__ import annotations

import sys
import os
from pathlib import Path

def _resolve_project_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()

    start = Path(__file__).resolve()
    for candidate in [start, *start.parents]:
        if (candidate / "infra").is_dir() and (candidate / "organs").is_dir() and (candidate / ".git").exists():
            return candidate

    return Path(__file__).resolve().parent


PROJECT_ROOT = _resolve_project_root()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))
SRC_ROOT = PROJECT_ROOT / "organs" / "exposure" / "src"
if str(SRC_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_ROOT))

from core.cli_runtime import main


if __name__ == "__main__":
    raise SystemExit(main(PROJECT_ROOT))
