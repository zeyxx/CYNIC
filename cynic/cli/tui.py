"""
CYNIC CLI — `tui` command (Textual terminal interface).
"""
from __future__ import annotations

import sys

from cynic.cli.utils import _c
from typing import Optional


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
           python -m cynic.cli tui --organism (organism dashboard)
    """
    extra = sys.argv[2:]
    url: Optional[str] = None
    use_organism = False

    if "--url" in extra:
        idx = extra.index("--url")
        if idx + 1 < len(extra):
            url = extra[idx + 1]

    if "--organism" in extra:
        use_organism = True

    try:
        if use_organism:
            from cynic.cli.tui_dashboard import run_tui
            import asyncio
            asyncio.run(run_tui(cynic_url=url or "http://localhost:8000"))
        else:
            from cynic.tui.app import run as tui_run
            tui_run(base_url=url)
    except ImportError as e:
        print()
        print(_c("red", "  *GROWL* textual not installed."))
        print(_c("dim", "  pip install 'textual>=0.44'"))
        print()
        sys.exit(1)
    except httpx.RequestError as e:
        print()
        print(_c("red", f"  *GROWL* Error: {e}"))
        print()
        sys.exit(1)
