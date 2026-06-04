#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Notification Poller — reads @TalariaBuild X notifications via CDP.

Navigates to x.com/notifications in hermes-browser (real Chrome, already logged in
as @TalariaBuild). Writes each new interaction to pending/{tweet_id}.json.
Tracks consecutive empty cycles for monitor-blind detection.

K15 Consumer: talaria_alerter.py (reads pending/)
Systemd: hermes-notification-poller.timer (every 5min)
Failure mode: CDP unavailable → exit 0 (timer retries)
"""

__version__ = "0.1.0"

import asyncio
import json
import logging
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    from playwright.async_api import async_playwright, TimeoutError as PwTimeout
except ImportError:
    print("ERROR: playwright not installed. Run: pip install playwright && playwright install chromium")
    sys.exit(1)

from hermes_paths import PENDING_DIR, PROCESSED_DIR, POLLER_STATE, HERMES_X_DIR

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("notification-poller")

BROWSER_STATE = HERMES_X_DIR.parent / "browser-state.json"
CDP_TOTAL_TIMEOUT_MS = 9000
DOM_WAIT_TIMEOUT_MS  = 6000


# ── Pure helpers (testable without Playwright) ──────────────────────────────

def parse_notification_text(text: str) -> dict:
    """Classify notification type from inner text. Returns {type, text}."""
    lower = text.lower()
    if "retweeted" in lower:
        notif_type = "retweet"
    elif "liked" in lower:
        notif_type = "like"
    elif "replied" in lower:
        notif_type = "reply"
    elif "mentioned" in lower:
        notif_type = "mention"
    else:
        notif_type = "notification"
    lines = text.strip().splitlines()
    body = lines[-1].strip() if len(lines) > 1 else text.strip()
    return {"type": notif_type, "text": body[:280]}


def build_interaction_entry(
    tweet_id: str,
    notif_type: str,
    author: str,
    text: str,
    source: str,
    keywords: list | None = None,
) -> dict:
    """Build a canonical interaction entry dict."""
    author_clean = author if author.startswith("@") else f"@{author}"
    handle = author_clean.lstrip("@")
    return {
        "schema_version": 1,
        "tweet_id": tweet_id,
        "type": notif_type,
        "author": author_clean,
        "author_followers": None,
        "text": text[:280],
        "url": f"https://x.com/{handle}/status/{tweet_id}",
        "detected_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": source,
        "keywords_matched": keywords or [],
    }


def is_already_seen(tweet_id: str, pending: Path, processed: Path) -> bool:
    return (pending / f"tweet_{tweet_id}.json").exists() or \
           (processed / f"tweet_{tweet_id}.json").exists()


def write_pending(entry: dict, pending: Path) -> None:
    pending.mkdir(parents=True, exist_ok=True)
    out = pending / f"tweet_{entry['tweet_id']}.json"
    out.write_text(json.dumps(entry, ensure_ascii=False))


def load_poller_state(state_file: Path) -> dict:
    if state_file.exists():
        try:
            return json.loads(state_file.read_text())
        except (json.JSONDecodeError, OSError):
            pass
    return {"consecutive_empty_cycles": 0}


def update_poller_state(state_file: Path, found_count: int) -> dict:
    state = load_poller_state(state_file)
    if found_count > 0:
        state["consecutive_empty_cycles"] = 0
    else:
        state["consecutive_empty_cycles"] = state.get("consecutive_empty_cycles", 0) + 1
    state_file.write_text(json.dumps(state))
    return state


def should_emit_blind_alert(state_file: Path) -> bool:
    state = load_poller_state(state_file)
    return state.get("consecutive_empty_cycles", 0) >= 3


def write_blind_alert(pending: Path) -> None:
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    entry = {
        "schema_version": 1,
        "tweet_id": f"MONITOR_BLIND_{ts}",
        "type": "monitor_blind",
        "author": "@system",
        "author_followers": None,
        "text": "3 consecutive empty notification cycles. Check: hermes-browser active? X session valid? DOM selector changed?",
        "url": "",
        "detected_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source": "notification_poller",
        "keywords_matched": [],
    }
    fname = f"MONITOR_BLIND_{ts}.json"
    (pending / fname).write_text(json.dumps(entry, ensure_ascii=False))
    logger.warning("MONITOR BLIND emitted: %s", fname)


def _get_cdp_url() -> str | None:
    if not BROWSER_STATE.exists():
        logger.warning("browser-state.json not found at %s", BROWSER_STATE)
        return None
    try:
        state = json.loads(BROWSER_STATE.read_text())
        return state.get("cdp_url")
    except Exception as e:
        logger.error("failed to read browser-state.json: %s", e)
        return None


async def _extract_notifications(page) -> list:
    """Navigate to /notifications and extract all visible items."""
    await page.goto("https://x.com/notifications", wait_until="domcontentloaded", timeout=CDP_TOTAL_TIMEOUT_MS)
    try:
        await page.wait_for_selector('[data-testid="notification"]', timeout=DOM_WAIT_TIMEOUT_MS)
    except PwTimeout:
        logger.info("No notification elements found within %dms", DOM_WAIT_TIMEOUT_MS)
        return []

    items = await page.locator('[data-testid="notification"]').all()
    results = []
    for item in items:
        try:
            link_el = item.locator('a[href*="/status/"]').first
            href = await link_el.get_attribute("href", timeout=500)
            if not href:
                continue
            parts = href.strip("/").split("/")
            if len(parts) < 3 or parts[1] != "status":
                continue
            author = parts[0]
            tweet_id = parts[2]
            inner = await item.inner_text(timeout=500)
            parsed = parse_notification_text(inner)
            results.append({
                "tweet_id": tweet_id,
                "author": author,
                "type": parsed["type"],
                "text": parsed["text"],
            })
        except Exception as e:
            logger.debug("skipping notification item: %s", e)
            continue
    return results


async def run() -> int:
    cdp_url = _get_cdp_url()
    if not cdp_url:
        logger.warning("CDP unavailable, exiting cleanly")
        return 0

    http_endpoint = cdp_url.replace("ws://", "http://")
    found_count = 0

    try:
        async with async_playwright() as p:
            browser = await p.chromium.connect_over_cdp(http_endpoint)
            contexts = browser.contexts
            context = contexts[0] if contexts else await browser.new_context()
            page = await context.new_page()

            raw_items = await _extract_notifications(page)
            await page.close()

            PENDING_DIR.mkdir(parents=True, exist_ok=True)
            PROCESSED_DIR.mkdir(parents=True, exist_ok=True)

            for raw in raw_items:
                if is_already_seen(raw["tweet_id"], PENDING_DIR, PROCESSED_DIR):
                    logger.debug("skip seen: %s", raw["tweet_id"])
                    continue
                entry = build_interaction_entry(
                    tweet_id=raw["tweet_id"],
                    notif_type=raw["type"],
                    author=raw["author"],
                    text=raw["text"],
                    source="notification_poller",
                )
                write_pending(entry, PENDING_DIR)
                found_count += 1
                logger.info("new: %s by @%s (%s)", raw["tweet_id"], raw["author"], raw["type"])

    except Exception as e:
        logger.error("CDP error: %s", str(e)[:120])
        return 0

    state = update_poller_state(POLLER_STATE, found_count)
    logger.info("done: %d new, consecutive_empty=%d", found_count, state["consecutive_empty_cycles"])

    if should_emit_blind_alert(POLLER_STATE):
        write_blind_alert(PENDING_DIR)
        update_poller_state(POLLER_STATE, found_count=1)

    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(run()))
