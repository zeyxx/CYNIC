#!/usr/bin/env python3
"""Activate Codex access to CYNIC Browser Hub MCP bridge."""

from __future__ import annotations

import argparse
import sys
try:
    from organs.browser.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


ROOT = bootstrap_repo_path()
EXPOSURE_SRC = ROOT / "organs/exposure/src"

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
if str(EXPOSURE_SRC) not in sys.path:
    sys.path.insert(0, str(EXPOSURE_SRC))

from organs.browser.src.core.activation import (
    CodexBrowserActivationOptions,
    activate_codex_browser_mcp,
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Activate Codex access to the CYNIC Browser Hub MCP bridge."
    )
    parser.add_argument(
        "--start-services",
        action="store_true",
        help="Start organ-browser.service and organ-browser-hub.service with systemd --user.",
    )
    parser.add_argument(
        "--skip-diagnose",
        action="store_true",
        help="Do not run diagnose_browser_mcp.py after activation.",
    )
    parser.add_argument(
        "--install-deps",
        action="store_true",
        help="Create the organ-browser runtime venv and install the organ-browser Python package into it.",
    )
    args = parser.parse_args()

    return activate_codex_browser_mcp(
        ROOT,
        CodexBrowserActivationOptions(
            start_services=args.start_services,
            skip_diagnose=args.skip_diagnose,
            install_deps=args.install_deps,
        ),
    )


if __name__ == "__main__":
    raise SystemExit(main())
