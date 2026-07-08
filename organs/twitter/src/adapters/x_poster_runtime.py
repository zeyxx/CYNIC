"""Runtime adapters for human-gated X reply posting."""

from __future__ import annotations

import json
import os
from pathlib import Path

from organs.contracts.src.adapters.env import load_env_file
from organs.contracts.src.surface_contracts import read_json_file_or_default
from organs.twitter.src.adapters.kernel_observe_transport import post_observe_payload


COMMS_LEDGER = "comms_ledger.jsonl"


def load_env(env_path: Path | None = None) -> None:
    load_env_file(env_path or Path.home() / ".cynic-env")


def browser_state(path: Path) -> dict:
    if not path.exists():
        return {}
    state = read_json_file_or_default(path, {})
    return state if isinstance(state, dict) else {}


def get_cdp_url(path: Path, logger) -> str | None:
    if not path.exists():
        logger.error("browser-state.json not found at %s", path)
        return None
    return browser_state(path).get("cdp_url")


def get_account(path: Path) -> str:
    return browser_state(path).get("username", "unknown")


def log_to_ledger(organ_dir: Path, entry: dict, logger) -> None:
    ledger_path = organ_dir / COMMS_LEDGER
    with ledger_path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(entry) + "\n")
    logger.info("ledger: %s -> %s (id=%s)", entry["action"], ledger_path, entry["id"][:8])


def post_to_kernel(entry: dict, logger) -> None:
    payload = (
        {
            "tool": "x_poster",
            "target": entry["tweet_url"],
            "domain": "comms",
            "context": (
                f"action:{entry['action']} account:{entry['account']} "
                f"text:{entry['text'][:120]}"
            ),
            "agent_id": "x-poster",
            "tags": ["comms-log", entry["action"]],
        }
    )
    result = post_observe_payload(payload, timeout=10)
    if result.skipped:
        logger.warning("kernel env not set - /observe skipped")
        return
    if result.posted:
        logger.info("kernel /observe: %s", result.status)
    else:
        logger.warning("kernel /observe failed: %s", result.error or result.status)


async def post_reply(
    *,
    async_playwright,
    cdp_url: str,
    tweet_url: str,
    text: str,
    dry_run: bool,
    account: str,
    logger,
) -> bool:
    async with async_playwright() as playwright:
        browser = await playwright.chromium.connect_over_cdp(cdp_url)
        context = browser.contexts[0] if browser.contexts else await browser.new_context()
        page = await context.new_page()

        try:
            logger.info("navigating to %s", tweet_url)
            await page.goto(tweet_url, wait_until="domcontentloaded", timeout=30000)
            await page.wait_for_selector('[data-testid="tweet"]', timeout=15000)
            await page.locator('[data-testid="reply"]').first.click()

            textbox = page.locator('[data-testid="tweetTextarea_0"]')
            await textbox.wait_for(state="visible", timeout=10000)

            print("\n" + "=" * 60)
            print("REPLY PREVIEW")
            print(f"Account : @{account}")
            print(f"To      : {tweet_url}")
            print(f"Text ({len(text)}/280):")
            print(f"  {text}")
            print("=" * 60)

            if dry_run:
                print("[DRY RUN] - not posting")
                return False

            confirm = input("\nPost this reply? [y/N] ").strip().lower()
            if confirm != "y":
                print("Skipped.")
                return False

            await textbox.click()
            await page.keyboard.type(text, delay=30)
            await page.wait_for_timeout(600)
            await page.locator('[data-testid="tweetButton"]').last.click()
            await page.wait_for_timeout(2500)
            logger.info("reply posted")
            return True
        except Exception as exc:
            logger.error("post_reply failed: %s", exc)
            return False
        finally:
            await page.close()
