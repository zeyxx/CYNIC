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
import shutil
import subprocess
import sys


def _find_python() -> str:
    """Find the best Python 3.13+ executable.

    Priority:
      1. sys.executable if it's 3.11+ (StrEnum support)
      2. 'py -3.13' launcher on Windows
      3. 'python3.13' on PATH
      4. Fallback to sys.executable (may fail on 3.9)
    """
    if sys.version_info >= (3, 11):
        return sys.executable

    # Windows py launcher
    if sys.platform == "win32":
        py = shutil.which("py")
        if py:
            try:
                r = subprocess.run([py, "-3.13", "-c", "import sys; print(sys.executable)"],
                                   capture_output=True, text=True, timeout=5)
                if r.returncode == 0 and r.stdout.strip():
                    return r.stdout.strip()
            except CynicError:
                pass

    # Unix-style python3.13
    p313 = shutil.which("python3.13")
    if p313:
        return p313

    return sys.executable


def main() -> int:
    args = sys.argv[1:] or ["tests/"]
    python = _find_python()
    env = {**os.environ, "PYTHONUTF8": "1", "PYTHONIOENCODING": "utf-8"}
    r = subprocess.run(
        [python, "-m", "pytest"] + args,
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
