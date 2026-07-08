#!/usr/bin/env python3
"""Machine-readable organ-fleet CLI membrane."""

from __future__ import annotations

from pathlib import Path

try:
    from organs.fleet.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()

from organs.fleet.src.core.cli_runtime import main as core_main


def main(argv: list[str] | None = None) -> int:
    return core_main(ROOT, argv)


if __name__ == "__main__":
    raise SystemExit(main())
