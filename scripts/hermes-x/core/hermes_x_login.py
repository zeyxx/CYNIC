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
import tomllib
from pathlib import Path

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("hermes-x-login")

# Import get_x_credentials after adding scripts to path
sys.path.insert(0, str(Path(__file__).parent))
from get_x_credentials import get_x_credentials, load_accounts_config
from hermes_paths import CHROME_PROFILE as DEFAULT_CHROME_PROFILE

# Playwright import
try:
    from playwright.sync_api import sync_playwright, TimeoutError as PlaywrightTimeout
except ImportError:
    logger.error("Playwright not installed. Run: pip install playwright")
    sys.exit(2)


def get_chrome_profile() -> Path:
    """Derive Chrome profile path from HERMES_ACCOUNT or accounts.toml."""
    account_id = os.getenv("HERMES_ACCOUNT", "").strip()

    if account_id:
        accounts = load_accounts_config()
        if "accounts" in accounts and account_id in accounts["accounts"]:
            account = accounts["accounts"][account_id]
            profile = account.get("profile")
            if profile:
                return Path(profile).expanduser()

    # Fallback to default (CYNIC) profile
    return DEFAULT_CHROME_PROFILE


CHROME_PROFILE = get_chrome_profile()
X_URL = "https://x.com"
TIMEOUT_MS = 30000


def login_to_x():
    """
    Headless login to X.com using Playwright.

    Returns:
        bool: True if authenticated, False otherwise
    """
    username, password = get_x_credentials()

    CHROME_PROFILE.mkdir(parents=True, exist_ok=True)

    logger.info(f"Logging in as {username}...")

    with sync_playwright() as p:
        try:
            # Launch Chrome with persistent user-data-dir
            browser = p.chromium.launch(
                headless=True,
                args=[
                    f"--user-data-dir={CHROME_PROFILE}",
                    "--disable-blink-features=AutomationControlled",
                ],
            )

            page = browser.new_page()
            page.set_default_timeout(TIMEOUT_MS)

            # Navigate to X
            logger.info("Navigating to x.com...")
            page.goto(X_URL, wait_until="domcontentloaded")

            # Check if already authenticated (no login form)
            try:
                page.wait_for_selector('input[name="text"]', timeout=2000)
                logger.info("Already authenticated (no login form found)")
                browser.close()
                return True
            except PlaywrightTimeout:
                pass  # Not logged in, proceed with login

            # Fill login form
            logger.info("Filling login form...")

            # Wait for username field
            page.wait_for_selector('input[name="text"]', timeout=TIMEOUT_MS)
            page.fill('input[name="text"]', username)
            page.press('input[name="text"]', "Enter")

            # Wait for password field
            page.wait_for_selector('input[name="password"]', timeout=TIMEOUT_MS)
            page.fill('input[name="password"]', password)
            page.press('input[name="password"]', "Enter")

            # Wait for authentication redirect (home timeline or check auth state)
            logger.info("Waiting for authentication...")
            try:
                page.wait_for_url("**/home", timeout=TIMEOUT_MS)
                logger.info("✓ Successfully authenticated (home timeline loaded)")
            except PlaywrightTimeout:
                # Might not reach /home, check if we're authenticated by checking for authenticated UI
                try:
                    page.wait_for_selector('[data-testid="primaryColumn"]', timeout=5000)
                    logger.info("✓ Successfully authenticated (primary column visible)")
                except PlaywrightTimeout:
                    logger.error("✗ Login failed: did not reach authenticated state")
                    browser.close()
                    return False

            # Verify cookies were saved
            cookies = browser.contexts[0].cookies()
            if cookies:
                logger.info(f"✓ Cookies saved to persistent profile ({len(cookies)} cookies)")
            else:
                logger.warning("⚠ No cookies detected (may still work on next browser launch)")

            browser.close()
            return True

        except PlaywrightTimeout as e:
            logger.error(f"✗ Timeout during login: {e}")
            return False
        except Exception as e:
            logger.error(f"✗ Login failed: {e}")
            return False


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
