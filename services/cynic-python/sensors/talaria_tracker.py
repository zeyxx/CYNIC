#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: @TalariaBuild post tracker — dataset.jsonl filter → /observe.

K15 Consumer: talaria_strategy_consumer.py aggregates metrics → /judge → editorial strategy crystals

Systemd: talaria-tracker.service (timer, every 15min)
Promotion: 2026-06-05 (migrated from @cynicoracle to @TalariaBuild)
Stability: 0 days

Reads the hermes-x dataset.jsonl, filters posts from @talariabuild,
and stores them in /observe domain=talaria with engagement metrics.
Maintains a cursor to avoid reprocessing.

Metrics tracked per post:
  views, likes, retweets, replies, quotes, bookmarks, engagement_rate
  post_type: original | reply | qrt | retweet

Environment:
    CYNIC_REST_ADDR — kernel address (no scheme, normalized at read)
    CYNIC_API_KEY   — kernel auth token
    HERMES_DATASET  — path to dataset.jsonl (default: ~/.cynic/organs/hermes/x/dataset.jsonl)
    TALARIA_HANDLE  — X handle to track (default: talariabuild)
"""

import json
import logging
import os
import sys
import time
from pathlib import Path

import requests

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
logger = logging.getLogger("talaria-tracker")

AGENT_ID = "talaria-tracker"
DOMAIN = "talaria"
CURSOR_FILE = Path.home() / ".cynic" / "talaria_tracker_cursor.json"
DEFAULT_DATASET = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"


def _load_env() -> tuple[str, str]:
    """Load CYNIC_REST_ADDR and CYNIC_API_KEY from env or ~/.cynic-env."""
    addr = os.environ.get("CYNIC_REST_ADDR", "")
    key = os.environ.get("CYNIC_API_KEY", "")
    if addr and key:
        return _normalize_addr(addr), key

    env_file = Path.home() / ".cynic-env"
    if env_file.exists():
        for line in env_file.read_text().splitlines():
            line = line.strip()
            if line.startswith("#") or "=" not in line:
                continue
            k, _, v = line.partition("=")
            v = v.strip().strip('"').strip("'")
            if k.strip() == "CYNIC_REST_ADDR" and not addr:
                addr = v
            elif k.strip() == "CYNIC_API_KEY" and not key:
                key = v

    return _normalize_addr(addr), key


def _normalize_addr(raw: str) -> str:
    if raw and not raw.startswith("http"):
        return f"http://{raw}"
    return raw


def _load_cursor() -> set[str]:
    """Load seen tweet_ids from cursor file."""
    if not CURSOR_FILE.exists():
        return set()
    try:
        data = json.loads(CURSOR_FILE.read_text())
        return set(data.get("seen_ids", []))
    except (json.JSONDecodeError, OSError):
        return set()


def _save_cursor(seen_ids: set[str]) -> None:
    """Persist seen tweet_ids. Keep last 10000 to bound file size."""
    CURSOR_FILE.parent.mkdir(parents=True, exist_ok=True)
    ids_list = list(seen_ids)[-10000:]
    CURSOR_FILE.write_text(json.dumps({"seen_ids": ids_list}))


def _post_type(row: dict) -> str:
    """Classify post as original | reply | qrt | retweet."""
    if row.get("retweeted_tweet"):
        return "retweet"
    if row.get("quoted_tweet"):
        return "qrt"
    if row.get("in_reply_to_screen_name"):
        return "reply"
    return "original"


def _post_observe(row: dict, addr: str, key: str) -> bool:
    """POST a @talariabuild tweet to /observe domain=talaria."""
    tweet_id = row.get("tweet_id", "")
    text = row.get("text", "")[:280]
    ptype = _post_type(row)
    quoted = row.get("quoted_tweet")
    quoted_author = quoted.get("author_screen_name", "") if isinstance(quoted, dict) else ""
    target = row.get("in_reply_to_screen_name") or quoted_author if ptype in ("reply", "qrt") else ""

    context = (
        f"[{ptype}] {text} | "
        f"views={row.get('views', 0)} likes={row.get('likes', 0)} "
        f"rt={row.get('retweets', 0)} replies={row.get('replies', 0)} "
        f"quotes={row.get('quotes', 0)} bk={row.get('bookmarks', 0)} "
        f"eng={row.get('engagement_rate', 0.0):.4f}"
    )

    payload: dict = {
        "tool": AGENT_ID,
        "target": tweet_id,
        "domain": DOMAIN,
        "status": "captured",
        "context": context,
        "agent_id": AGENT_ID,
        "tags": [ptype, f"target:{target}"] if target else [ptype],
        # Extended metrics for strategy consumer
        "meta": {
            "tweet_id": tweet_id,
            "post_type": ptype,
            "reply_target": target,
            "views": row.get("views", 0),
            "likes": row.get("likes", 0),
            "retweets": row.get("retweets", 0),
            "replies": row.get("replies", 0),
            "quotes": row.get("quotes", 0),
            "bookmarks": row.get("bookmarks", 0),
            "engagement_rate": row.get("engagement_rate", 0.0),
            "created_at": row.get("created_at", ""),
            "narratives": row.get("narratives", []),
        },
    }

    try:
        resp = requests.post(
            f"{addr}/observe",
            json=payload,
            headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
            timeout=10,
        )
        if resp.status_code == 200:
            logger.info("stored tweet_id=%s type=%s eng=%.4f", tweet_id, ptype, row.get("engagement_rate", 0.0))
            return True
        logger.warning("POST /observe %s → %d", tweet_id, resp.status_code)
        return False
    except requests.RequestException as e:
        logger.warning("POST /observe failed: %s", e)
        return False


def run() -> int:
    """Scan dataset.jsonl for @talariabuild tweets, POST new ones to /observe."""
    addr, key = _load_env()
    if not addr or not key:
        logger.error("CYNIC_REST_ADDR or CYNIC_API_KEY not set")
        return 1

    handle = os.environ.get("TALARIA_HANDLE", "talariabuild").lower()
    dataset = Path(os.environ.get("HERMES_DATASET", str(DEFAULT_DATASET)))

    if not dataset.exists():
        logger.error("dataset not found: %s", dataset)
        return 1

    seen_ids = _load_cursor()
    new_count = 0
    error_count = 0

    with dataset.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue

            # Filter to our own posts only
            if row.get("author_screen_name", "").lower() != handle:
                continue

            tweet_id = row.get("tweet_id", "")
            if not tweet_id or tweet_id in seen_ids:
                continue

            # Skip retweets of others — only track our original content
            if row.get("retweeted_tweet"):
                seen_ids.add(tweet_id)
                continue

            if _post_observe(row, addr, key):
                seen_ids.add(tweet_id)
                new_count += 1
                time.sleep(0.2)  # K25: yield between posts
            else:
                error_count += 1

    _save_cursor(seen_ids)
    logger.info("done: %d new posts stored, %d errors", new_count, error_count)
    return 0 if error_count == 0 else 1


if __name__ == "__main__":
    sys.exit(run())
