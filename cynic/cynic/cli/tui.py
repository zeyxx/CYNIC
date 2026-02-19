"""
CYNIC CLI — `tui` command (Textual terminal interface).
"""
from __future__ import annotations

import sys
from typing import Optional

from cynic.cli.utils import _c


def cmd_tui() -> None:
    """
    Launch the CYNIC interactive TUI (Textual terminal interface).

    Displays: Dogs panel (reputation bars), Judgment stream (live),
    Pending Actions (accept/reject with keyboard).

    Reads local files every 2 s; queries server every 5 s (optional).
    Gracefully degrades when server is offline.

    Keys:
      a / r      accept / reject selected action
      j / k / ↑↓ navigate actions
      1-5        rate last judgment (L3 feedback)
      ctrl+r     force refresh
      q          quit

    Environment:
      CYNIC_URL  base URL  (default: http://localhost:8042)

    Usage: python -m cynic.cli tui [--url URL]
    """
    extra = sys.argv[2:]
    url: str | None = None
    if "--url" in extra:
        idx = extra.index("--url")
        if idx + 1 < len(extra):
            url = extra[idx + 1]

    try:
        from cynic.tui.app import run as tui_run
    except ImportError:
        print()
        print(_c("red", "  *GROWL* textual not installed."))
        print(_c("dim", "  pip install 'textual>=0.44'"))
        print()
        sys.exit(1)

    tui_run(base_url=url)
