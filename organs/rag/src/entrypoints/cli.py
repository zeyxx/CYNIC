#!/usr/bin/env python3
"""Non-mutating control surface for organ-rag."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def _repo_root() -> Path:
    env_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if env_root:
        return Path(env_root).expanduser().resolve()
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start.parent


ROOT = _repo_root()
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from organs.rag.src.adapters.local_inventory import LocalRagInventory
from organs.rag.src.core.status import build_status


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="organ-rag control surface")
    parser.add_argument("command", nargs="?", choices=("status",), default="status")
    return parser


def main(argv: list[str] | None = None) -> int:
    build_parser().parse_args(argv)
    organ_root = ROOT / "organs" / "rag"
    status = build_status(LocalRagInventory(organ_root).files())
    print(json.dumps(status.to_dict(), indent=2, sort_keys=True))
    return 0 if status.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())

