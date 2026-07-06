#!/usr/bin/env python3
"""Compatibility wrapper for the organ-jupyter CLI."""

import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[4]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.jupyter.src.entrypoints.cli import main


if __name__ == "__main__":
    main()
