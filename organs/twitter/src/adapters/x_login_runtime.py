"""Playwright runtime adapter for Hermes X login."""

from __future__ import annotations

from pathlib import Path


X_URL = "https://x.com"
TIMEOUT_MS = 30000


def get_chrome_profile(
    *,
    account_id: str,
    accounts: dict,
    default_profile: Path,
) -> Path:
    """Derive Chrome profile path from selected Hermes account config."""
    if account_id and "accounts" in accounts and account_id in accounts["accounts"]:
        profile = accounts["accounts"][account_id].get("profile")
        if profile:
            return Path(profile).expanduser()
    return default_profile


def load_playwright():
    try:
        from playwright.sync_api import TimeoutError as PlaywrightTimeout
        from playwright.sync_api import sync_playwright
    except ImportError as exc:
        raise RuntimeError("Playwright not installed. Run: pip install playwright") from exc
    return sync_playwright, PlaywrightTimeout


def login_to_x(
    *,
    username: str,
    password: str,
    chrome_profile: Path,
    logger,
    playwright_loader=load_playwright,
) -> bool:
    """Headless login to X.com using Playwright."""
    sync_playwright, PlaywrightTimeout = playwright_loader()
    chrome_profile.mkdir(parents=True, exist_ok=True)
    logger.info(f"Logging in as {username}...")

    with sync_playwright() as playwright:
        try:
            browser = playwright.chromium.launch(
                headless=True,
                args=[
                    f"--user-data-dir={chrome_profile}",
                    "--disable-blink-features=AutomationControlled",
                ],
            )
            page = browser.new_page()
            page.set_default_timeout(TIMEOUT_MS)
            logger.info("Navigating to x.com...")
            page.goto(X_URL, wait_until="domcontentloaded")

            try:
                page.wait_for_selector('input[name="text"]', timeout=2000)
                logger.info("Already authenticated (no login form found)")
                browser.close()
                return True
            except PlaywrightTimeout:
                pass

            logger.info("Filling login form...")
            page.wait_for_selector('input[name="text"]', timeout=TIMEOUT_MS)
            page.fill('input[name="text"]', username)
            page.press('input[name="text"]', "Enter")
            page.wait_for_selector('input[name="password"]', timeout=TIMEOUT_MS)
            page.fill('input[name="password"]', password)
            page.press('input[name="password"]', "Enter")

            logger.info("Waiting for authentication...")
            try:
                page.wait_for_url("**/home", timeout=TIMEOUT_MS)
                logger.info("Successfully authenticated (home timeline loaded)")
            except PlaywrightTimeout:
                try:
                    page.wait_for_selector('[data-testid="primaryColumn"]', timeout=5000)
                    logger.info("Successfully authenticated (primary column visible)")
                except PlaywrightTimeout:
                    logger.error("Login failed: did not reach authenticated state")
                    browser.close()
                    return False

            cookies = browser.contexts[0].cookies()
            if cookies:
                logger.info(f"Cookies saved to persistent profile ({len(cookies)} cookies)")
            else:
                logger.warning("No cookies detected (may still work on next browser launch)")
            browser.close()
            return True
        except PlaywrightTimeout as exc:
            logger.error(f"Timeout during login: {exc}")
            return False
        except Exception as exc:
            logger.error(f"Login failed: {exc}")
            return False
