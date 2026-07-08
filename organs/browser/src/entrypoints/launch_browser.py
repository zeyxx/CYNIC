#!/usr/bin/env python3
"""Launch Chrome for organ-browser CDP sessions."""

from __future__ import annotations

import os
from pathlib import Path


def main() -> int:
    chrome = os.environ.get("CHROME_BIN", "/usr/bin/google-chrome")
    profile = Path(
        os.environ.get(
            "ORGAN_BROWSER_CHROME_PROFILE",
            str(Path.home() / ".cynic/organs/browser/chrome-profiles/default"),
        )
    )
    cdp_port = os.environ.get("ORGAN_BROWSER_CDP_PORT", "40769")
    display = os.environ.get("DISPLAY", ":0")
    headless = os.environ.get("ORGAN_BROWSER_HEADLESS", "1").lower() not in {"0", "false", "no"}

    args = [
        chrome,
        f"--user-data-dir={profile}",
        "--remote-debugging-address=127.0.0.1",
        f"--remote-debugging-port={cdp_port}",
        f"--remote-allow-origins=http://127.0.0.1:{cdp_port}",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-component-update",
        "about:blank",
    ]
    if headless:
        args.insert(1, "--headless=new")
        args.insert(2, "--disable-gpu")
        args.insert(3, "--disable-dev-shm-usage")
    proxy_port = os.environ.get("ORGAN_BROWSER_PROXY_PORT")
    if proxy_port:
        args.insert(1, f"--proxy-server=127.0.0.1:{proxy_port}")

    profile.mkdir(parents=True, exist_ok=True)
    if not headless:
        os.environ["DISPLAY"] = display
    os.execv(chrome, args)
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
