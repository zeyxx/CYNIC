#!/usr/bin/env python3
"""
Headless X.com login via Playwright — multi-account aware.

Authenticates Chrome browser with X credentials (from accounts.toml or env).
Uses account-specific Chrome profile so cookies survive restarts.

Usage:
    python3 hermes_x_login.py                                        # Uses HERMES_ACCOUNT or defaults to cynic
    HERMES_ACCOUNT=personal python3 hermes_x_login.py               # Specific account
    CYNIC_X_PASSWORD="..." python3 hermes_x_login.py                # Legacy env var

Exit codes:
    0 - Success (authenticated)
    1 - Credentials unavailable or login failed
    2 - Playwright/browser issue
"""

import sys
import os
import logging
from pathlib import Path

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("hermes-login")

from organs.twitter.src.adapters.get_x_credentials import get_x_credentials, load_accounts_config
from organs.twitter.src.adapters.hermes_paths import CHROME_PROFILE as DEFAULT_CHROME_PROFILE
from organs.twitter.src.adapters.x_login_runtime import (
    get_chrome_profile as _get_chrome_profile,
    load_playwright as _runtime_load_playwright,
    login_to_x as _login_to_x,
)


def get_chrome_profile() -> Path:
    """Derive Chrome profile path from HERMES_ACCOUNT or accounts.toml."""
    return _get_chrome_profile(
        account_id=os.getenv("HERMES_ACCOUNT", "").strip(),
        accounts=load_accounts_config(),
        default_profile=DEFAULT_CHROME_PROFILE,
    )


CHROME_PROFILE = get_chrome_profile()
X_URL = "https://x.com"
TIMEOUT_MS = 30000


def _load_playwright():
    return _runtime_load_playwright()


def login_to_x():
    """
    Headless login to X.com using Playwright.

    Returns:
        bool: True if authenticated, False otherwise
    """
    username, password = get_x_credentials()
    return _login_to_x(
        username=username,
        password=password,
        chrome_profile=CHROME_PROFILE,
        logger=logger,
        playwright_loader=_load_playwright,
    )


def main():
    """Run headless login."""
    try:
        success = login_to_x()
        if success:
            logger.info("=" * 60)
            logger.info("LOGIN COMPLETE — Chrome authenticated and ready")
            logger.info("=" * 60)
            return 0
        else:
            logger.error("=" * 60)
            logger.error("LOGIN FAILED — Check credentials and try again")
            logger.error("=" * 60)
            return 1
    except KeyboardInterrupt:
        logger.info("\nLogin cancelled by user")
        return 1
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
