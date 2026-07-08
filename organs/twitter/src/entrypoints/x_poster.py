#!/usr/bin/env python3
"""Tier 2 INFRASTRUCTURE: organ-owned human-gated X reply poster.

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
import logging
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

try:
    from playwright.async_api import async_playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False
try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
from organs.twitter.src.adapters.x_poster_runtime import (
    COMMS_LEDGER,
    get_account as _get_account,
    get_cdp_url as _get_cdp_url,
    load_env as _load_env,
    log_to_ledger as _log_to_ledger,
    post_reply as _post_reply,
    post_to_kernel as _post_to_kernel,
)

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("x-poster")

BROWSER_STATE_PATH = HERMES_X_DIR.parent / "browser-state.json"


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

    _load_env()

    cdp_url = _get_cdp_url(BROWSER_STATE_PATH, logger)
    if not cdp_url:
        print("ERROR: no CDP endpoint — is hermes-browser.service running?")
        sys.exit(1)

    organ_dir = Path(args.organ_dir)
    account = _get_account(BROWSER_STATE_PATH)

    entry: dict = {
        "id": str(uuid.uuid4()),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "action": "skipped",
        "account": account,
        "tweet_url": args.url,
        "text": args.text,
        "dry_run": args.dry_run,
    }

    posted = asyncio.run(
        _post_reply(
            async_playwright=async_playwright,
            cdp_url=cdp_url,
            tweet_url=args.url,
            text=args.text,
            dry_run=args.dry_run,
            account=account,
            logger=logger,
        )
    )

    if dry_run := args.dry_run:
        entry["action"] = "dry_run"
    elif posted:
        entry["action"] = "posted"
    else:
        entry["action"] = "skipped"

    _log_to_ledger(organ_dir, entry, logger)

    if posted:
        _post_to_kernel(entry, logger)
        print(f"\n✓ Posted and logged to comms_ledger (id={entry['id'][:8]})")
    else:
        print(f"\n✓ Logged as '{entry['action']}' (id={entry['id'][:8]})")


if __name__ == "__main__":
    main()
