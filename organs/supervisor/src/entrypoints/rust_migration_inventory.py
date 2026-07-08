#!/usr/bin/env python3
"""Print the supervisor Rust migration inventory as JSON."""

from __future__ import annotations

import sys
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return candidate
    return start.parent


ROOT = _bootstrap_repo_path()

from organs.supervisor.src.core.rust_migration_inventory import inventory_json


def main() -> int:
    print(inventory_json(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
