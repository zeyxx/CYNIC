#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Talaria Alerter — reads pending/, sends Telegram, moves to processed/.

Reads all *.json files in pending/. Sends each to TALARIA_OPS_CHAT_ID via Telegram Bot API.
On success → move to processed/. On 429 → stop batch (retry next cycle). Other errors → skip.
Batch cap: 20 messages per cycle (Telegram rate limit protection).

K15 Consumer: TALARIA_OPS_CHAT_ID (groupe Telegram Ops)
Systemd: hermes-notification-alerter.timer (every 5min, offset +30s)
Env required: CYNIC_TELEGRAM_BOT_TOKEN, TALARIA_OPS_CHAT_ID
"""

__version__ = "0.1.0"

import json
import logging
import os
import sys
from pathlib import Path

import requests

from hermes_paths import PENDING_DIR, PROCESSED_DIR

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("talaria-alerter")

BATCH_CAP = 20
TELEGRAM_TIMEOUT = 10

BOT_TOKEN = os.environ.get("CYNIC_TELEGRAM_BOT_TOKEN", "")
OPS_CHAT_ID = os.environ.get("TALARIA_OPS_CHAT_ID", "")


def _fmt_followers(count) -> str:
    if count is None:
        return ""
    if count >= 1000:
        return f"{count / 1000:.1f}K followers"
    return f"{count} followers"


def format_telegram_message(entry: dict) -> str:
    notif_type = entry.get("type", "notification")
    if notif_type == "monitor_blind":
        return (
            f"⚠️ MONITOR BLIND — notification_poller\n"
            f"{entry.get('text', '')}\n"
            f"📡 {entry.get('detected_at', '')} UTC"
        )
    author = entry.get("author", "@?")
    followers = _fmt_followers(entry.get("author_followers"))
    followers_str = f" ({followers})" if followers else ""
    text = entry.get("text", "")
    url = entry.get("url", "")
    source = entry.get("source", "?")
    detected_at = entry.get("detected_at", "")[:16].replace("T", " ")
    return (
        f"🔔 @TalariaBuild — {notif_type}\n"
        f"👤 {author}{followers_str}\n"
        f'💬 "{text}"\n'
        f"🔗 {url}\n"
        f"📡 {source} | {detected_at} UTC"
    )


def send_telegram(bot_token: str, chat_id: str, text: str) -> bool:
    """Send a message. Returns True on 200, False on any error."""
    try:
        resp = requests.post(
            f"https://api.telegram.org/bot{bot_token}/sendMessage",
            json={"chat_id": chat_id, "text": text},
            timeout=TELEGRAM_TIMEOUT,
        )
        if resp.status_code == 200:
            return True
        logger.warning("Telegram returned %d", resp.status_code)
        return False
    except Exception as e:
        logger.error("Telegram request failed: %s", e)
        return False


def process_pending(
    pending: Path,
    processed: Path,
    bot_token: str,
    chat_id: str,
) -> tuple[int, int]:
    """
    Process pending/*.json up to BATCH_CAP.
    Returns (sent_count, kept_count).
    Stops batch on 429.
    """
    processed.mkdir(parents=True, exist_ok=True)
    all_files = sorted(pending.glob("*.json"))
    batch = all_files[:BATCH_CAP]
    kept = len(all_files) - len(batch)
    sent = 0

    for f in batch:
        try:
            entry = json.loads(f.read_text())
        except (json.JSONDecodeError, OSError) as e:
            logger.warning("skipping invalid file %s: %s", f.name, e)
            continue

        msg = format_telegram_message(entry)
        try:
            resp = requests.post(
                f"https://api.telegram.org/bot{bot_token}/sendMessage",
                json={"chat_id": chat_id, "text": msg},
                timeout=TELEGRAM_TIMEOUT,
            )
        except Exception as e:
            logger.error("Telegram request failed for %s: %s", f.name, e)
            kept += 1
            continue

        if resp.status_code == 200:
            f.rename(processed / f.name)
            sent += 1
        elif resp.status_code == 429:
            logger.warning("429 rate limit — stopping batch")
            # current file + remaining unprocessed files are all kept
            kept += len(batch) - sent
            break
        else:
            logger.warning("Telegram %d for %s — keeping", resp.status_code, f.name)
            kept += 1

    return sent, kept


def main() -> int:
    if not BOT_TOKEN:
        logger.error("CYNIC_TELEGRAM_BOT_TOKEN not set")
        return 1
    if not OPS_CHAT_ID:
        logger.error("TALARIA_OPS_CHAT_ID not set")
        return 1

    PENDING_DIR.mkdir(parents=True, exist_ok=True)
    PROCESSED_DIR.mkdir(parents=True, exist_ok=True)

    pending_files = list(PENDING_DIR.glob("*.json"))
    if not pending_files:
        logger.info("pending/ empty, nothing to send")
        return 0

    sent, kept = process_pending(PENDING_DIR, PROCESSED_DIR, BOT_TOKEN, OPS_CHAT_ID)
    logger.info("done: %d sent, %d kept for next cycle", sent, kept)
    return 0


if __name__ == "__main__":
    sys.exit(main())
