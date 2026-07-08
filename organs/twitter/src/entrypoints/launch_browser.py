#!/usr/bin/env python3
"""Launch Chrome for organ-twitter Hermes X browser sessions."""

from __future__ import annotations

import os
from pathlib import Path


def main() -> int:
    account = os.environ.get("HERMES_ACCOUNT", "cynic")
    profile = Path(
        os.environ.get(
            "HERMES_CHROME_PROFILE",
            str(Path.home() / ".cynic/organs/hermes/x/chrome-profiles/cynic"),
        )
    )
    cdp_port = os.environ.get("HERMES_CDP_PORT", "40769")
    proxy_port = os.environ.get("HERMES_PROXY_PORT", "8888")
    display = os.environ.get("DISPLAY", ":0")
    chrome = os.environ.get("CHROME_BIN", "/usr/bin/google-chrome")

    print(f"[organ-twitter launch-browser] account={account}")
    print(f"  profile={profile}")
    print(f"  cdp_port={cdp_port}")
    print(f"  proxy=127.0.0.1:{proxy_port}")
    print(f"  display={display}")

    profile.mkdir(parents=True, exist_ok=True)
    os.environ["DISPLAY"] = display
    os.execv(
        chrome,
        [
            chrome,
            f"--user-data-dir={profile}",
            f"--remote-debugging-port={cdp_port}",
            f"--remote-allow-origins=http://127.0.0.1:{cdp_port}",
            f"--proxy-server=127.0.0.1:{proxy_port}",
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-component-update",
            "--allow-insecure-localhost",
            "--ignore-certificate-errors",
            "about:blank",
        ],
    )
    return 127


if __name__ == "__main__":
    raise SystemExit(main())
