"""
CYNIC root conftest — global pytest fixtures and Windows encoding fix.

Must stay at cynic/ (one level above tests/) so it's loaded before
any test module is imported.
"""
from __future__ import annotations

import io
import os
import sys

# ── Windows UTF-8 encoding fix ─────────────────────────────────────────────
# On Windows, sys.stdout defaults to CP1252 (Code Page 1252).
# Any character outside CP1252 (emoji, box-drawing chars, pytest symbols)
# causes a UnicodeEncodeError that propagates back as exit code 1,
# even when ALL tests pass.
#
# Fix: rewrap stdout/stderr with UTF-8 + errors='replace' before pytest
# writes a single character of output.
# PYTHONUTF8=1 sets this before Python starts (export PYTHONUTF8=1),
# but conftest catches the case where the env var wasn't set.
if sys.platform == "win32":
    os.environ.setdefault("PYTHONUTF8", "1")
    if hasattr(sys.stdout, "buffer") and sys.stdout.encoding.lower() != "utf-8":
        sys.stdout = io.TextIOWrapper(
            sys.stdout.buffer, encoding="utf-8", errors="replace", line_buffering=True
        )
    if hasattr(sys.stderr, "buffer") and sys.stderr.encoding.lower() != "utf-8":
        sys.stderr = io.TextIOWrapper(
            sys.stderr.buffer, encoding="utf-8", errors="replace", line_buffering=True
        )
