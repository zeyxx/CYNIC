"""
CYNIC test runner — Windows-safe UTF-8 wrapper around pytest.

Usage:
    python run_tests.py [pytest args...]

Examples:
    python run_tests.py tests/
    python run_tests.py tests/ -v -k test_phi
    python run_tests.py tests/test_axiom_monitor.py -v

Why this exists:
    On Windows, the terminal defaults to CP1252. When pytest outputs any
    character outside CP1252 (emoji, box-drawing, ✓/✗), Python raises
    UnicodeEncodeError → exit code 1, even if all tests pass.

    This wrapper:
    1. Sets PYTHONUTF8=1 before spawning pytest (UTF-8 everywhere)
    2. Captures output via subprocess (bytes, decoded with errors='replace')
    3. Prints clean output and exits with pytest's real return code
"""
from __future__ import annotations

import os
import subprocess
import sys


def main() -> int:
    args = sys.argv[1:] or ["tests/"]
    env = {**os.environ, "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8"}
    r = subprocess.run(
        [sys.executable, "-m", "pytest"] + args,
        env=env,
        capture_output=True,
        text=True,
        errors="replace",
    )
    if r.stdout:
        print(r.stdout, end="")
    if r.stderr:
        print(r.stderr, end="", file=sys.stderr)
    return r.returncode


if __name__ == "__main__":
    sys.exit(main())
