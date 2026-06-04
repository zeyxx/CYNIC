#!/usr/bin/env python3
"""
CYNIC Hermes X Poster — post a reply to an X thread via CDP browser session.

Architecture:
  Human spots opportunity → drafts reply → x_poster (this) → human gate → post → comms_ledger + kernel

K15: Human (producer) → x_poster (consumer) → kernel /observe domain=comms

Usage:
    python3 x_poster.py --url "https://x.com/user/status/123" --text "your reply"
    python3 x_poster.py --url "https://x.com/user/status/123" --text "..." --dry-run

Environment (loaded from ~/.cynic-env automatically):
    CYNIC_REST_ADDR  — kernel address (for /observe logging)
    CYNIC_API_KEY    — kernel auth token
"""

import argparse
import asyncio
import json
import logging
import os
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

try:
    from playwright.async_api import async_playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hermes_paths import HERMES_X_DIR

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("x-poster")

COMMS_LEDGER = "comms_ledger.jsonl"
BROWSER_STATE_PATH = HERMES_X_DIR.parent / "browser-state.json"


def load_env() -> None:
    env_file = Path.home() / ".cynic-env"
    if not env_file.exists():
        return
    for line in env_file.read_text().splitlines():
        if "=" in line and not line.startswith("#"):
            k, _, v = line.partition("=")
            k = k.strip()
            if k and k not in os.environ:
                os.environ[k] = v.strip()


def get_cdp_url() -> str | None:
    if not BROWSER_STATE_PATH.exists():
        logger.error("browser-state.json not found at %s", BROWSER_STATE_PATH)
        return None
    try:
        state = json.loads(BROWSER_STATE_PATH.read_text())
        return state.get("cdp_url")
    except Exception as e:
        logger.error("failed to read browser-state.json: %s", e)
        return None


def get_account() -> str:
    if not BROWSER_STATE_PATH.exists():
        return "unknown"
    try:
        return json.loads(BROWSER_STATE_PATH.read_text()).get("username", "unknown")
    except Exception:
        return "unknown"


def log_to_ledger(organ_dir: Path, entry: dict) -> None:
    ledger_path = organ_dir / COMMS_LEDGER
    with open(ledger_path, "a") as f:
        f.write(json.dumps(entry) + "\n")
    logger.info("ledger: %s → %s (id=%s)", entry["action"], ledger_path, entry["id"][:8])


def post_to_kernel(entry: dict) -> None:
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if not addr or not key:
        logger.warning("kernel env not set — /observe skipped")
        return
    import urllib.request, urllib.error
    if not addr.startswith("http"):
        addr = f"http://{addr}"
    body = json.dumps({
        "tool": "x_poster",
        "target": entry["tweet_url"],
        "domain": "comms",
        "context": (
            f"action:{entry['action']} account:{entry['account']} "
            f"text:{entry['text'][:120]}"
        ),
        "agent_id": "x-poster",
        "tags": ["comms-log", entry["action"]],
    }).encode()
    req = urllib.request.Request(
        f"{addr}/observe", data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            logger.info("kernel /observe: %s", resp.status)
    except Exception as e:
        logger.warning("kernel /observe failed: %s", e)


async def post_reply(cdp_url: str, tweet_url: str, text: str, dry_run: bool) -> bool:
    async with async_playwright() as pw:
        browser = await pw.chromium.connect_over_cdp(cdp_url)
        # Reuse the existing authenticated context (logged-in session)
        ctx = browser.contexts[0] if browser.contexts else await browser.new_context()
        page = await ctx.new_page()

        try:
            logger.info("navigating to %s", tweet_url)
            await page.goto(tweet_url, wait_until="domcontentloaded", timeout=30000)

            # Wait for the tweet to render
            await page.wait_for_selector('[data-testid="tweet"]', timeout=15000)

            # Click reply on the first (root) tweet
            reply_btn = page.locator('[data-testid="reply"]').first
            await reply_btn.click()

            # Reply drawer opens — wait for textarea
            textbox = page.locator('[data-testid="tweetTextarea_0"]')
            await textbox.wait_for(state="visible", timeout=10000)

            print("\n" + "=" * 60)
            print("REPLY PREVIEW")
            print(f"Account : @{get_account()}")
            print(f"To      : {tweet_url}")
            print(f"Text ({len(text)}/280):")
            print(f"  {text}")
            print("=" * 60)

            if dry_run:
                print("[DRY RUN] — not posting")
                return False

            confirm = input("\nPost this reply? [y/N] ").strip().lower()
            if confirm != "y":
                print("Skipped.")
                return False

            # Fill reply
            await textbox.click()
            await page.keyboard.type(text, delay=30)
            await page.wait_for_timeout(600)

            # Post — scoped to the reply drawer to avoid hitting the compose box
            post_btn = page.locator('[data-testid="tweetButton"]').last
            await post_btn.click()

            # Brief wait for network round-trip
            await page.wait_for_timeout(2500)
            logger.info("reply posted")
            return True

        except Exception as e:
            logger.error("post_reply failed: %s", e)
            return False
        finally:
            await page.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Post a reply to an X thread via CDP session")
    parser.add_argument("--url", required=True, help="Full URL of the tweet to reply to")
    parser.add_argument("--text", required=True, help="Reply text (max 280 chars)")
    parser.add_argument("--organ-dir", default=str(HERMES_X_DIR))
    parser.add_argument("--dry-run", action="store_true", help="Preview only — do not post")
    args = parser.parse_args()

    if not PLAYWRIGHT_AVAILABLE:
        print("ERROR: playwright not installed — run: pip3 install playwright")
        sys.exit(1)

    if len(args.text) > 280:
        print(f"ERROR: text is {len(args.text)} chars (max 280)")
        sys.exit(1)

    load_env()

    cdp_url = get_cdp_url()
    if not cdp_url:
        print("ERROR: no CDP endpoint — is hermes-browser.service running?")
        sys.exit(1)

    organ_dir = Path(args.organ_dir)

    entry: dict = {
        "id": str(uuid.uuid4()),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "action": "skipped",
        "account": get_account(),
        "tweet_url": args.url,
        "text": args.text,
        "dry_run": args.dry_run,
    }

    posted = asyncio.run(post_reply(cdp_url, args.url, args.text, args.dry_run))

    if dry_run := args.dry_run:
        entry["action"] = "dry_run"
    elif posted:
        entry["action"] = "posted"
    else:
        entry["action"] = "skipped"

    log_to_ledger(organ_dir, entry)

    if posted:
        post_to_kernel(entry)
        print(f"\n✓ Posted and logged to comms_ledger (id={entry['id'][:8]})")
    else:
        print(f"\n✓ Logged as '{entry['action']}' (id={entry['id'][:8]})")


if __name__ == "__main__":
    main()
