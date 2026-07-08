#!/usr/bin/env python3
from __future__ import annotations

import sys
from pathlib import Path


def _bootstrap() -> None:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            if str(candidate) not in sys.path:
                sys.path.insert(0, str(candidate))
            return
    sys.path.insert(0, str(start.parent))


_bootstrap()

try:
    from dotenv import load_dotenv

    load_dotenv(Path.home() / ".cynic-env")
except ImportError:
    pass

from organs.domain.src.entrypoints.runtime import main


if __name__ == "__main__":
    raise SystemExit(main())
