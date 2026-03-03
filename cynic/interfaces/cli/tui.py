"""
CYNIC CLI â€" `tui` command (Textual terminal interface).
"""
from __future__ import annotations

import sys


def cmd_tui() -> None:
    """
    Launch the CYNIC interactive TUI (Textual terminal interface).

    Displays: Dogs panel (reputation bars), Judgment stream (live),
    Pending Actions (accept/reject with keyboard).

    Reads local files every 2 s; queries server every 5 s (optional).
    Gracefully degrades when server is offline.

    Keys:
      a / r      accept / reject selected action
      j / k / â'â" navigate actions
      1-5        rate last judgment (L3 feedback)
      ctrl+r     force refresh
      q          quit

    Environment:
      CYNIC_URL  base URL  (default: http://localhost:8042)

    Usage: python -m cynic.interfaces.cli tui [--url URL]
           python -m cynic.interfaces.cli tui --organism (organism dashboard)
    """
    extra = sys.argv[2:]
    url: str | None = None
    use_organism = False

    if "--url" in extra:
        idx = extra.index("--url")
        if idx + 1 < len(extra):
            url = extra[idx + 1]

    if "--organism" in extra:
        use_organism = True

    try:
        if use_organism:
            import asyncio

            from cynic.interfaces.cli.tui_dashboard import run_tui
            asyncio.run(run_tui(cynic_url=url or "http://localhost:8000"))
        else:
            from cynic.interfaces.tui.app import run as tui_run
            tui_run(base_url=url)
    except ImportError:
        sys.exit(1)
    except httpx.RequestError:
        sys.exit(1)
