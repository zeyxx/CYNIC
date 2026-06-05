#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: @TalariaBuild strategy consumer — aggregates post metrics → /judge → editorial crystals.

K15 Consumer: reads dataset.jsonl for @talariabuild posts → POST /judge domain=general
Systemd: talaria-strategy.timer (daily at 08:00)
Promotion date: 2026-06-05 (migrated from @cynicoracle to @TalariaBuild)
Stability: new

Design:
  - Reads dataset.jsonl, filters @talariabuild posts
  - Aggregates engagement metrics by post_type (original/reply/qrt)
  - Builds a structured editorial analysis prompt
  - POSTs to /judge — Dogs produce strategic verdict → crystal
  - Min 5 new posts since last judgment (avoid judging noise)

K15 Falsification: run with MIN_NEW_POSTS=1, verify /verdicts shows
  a verdict with domain=general and agent_id=talaria-strategy.

Environment:
    CYNIC_REST_ADDR          — kernel address
    CYNIC_API_KEY            — kernel auth token
    HERMES_DATASET           — path to dataset.jsonl
    TALARIA_HANDLE           — X handle to track (default: talariabuild)
    STRATEGY_MIN_NEW_POSTS   — min new posts before judging (default: 5)
    STRATEGY_STATE_FILE      — path to state file
    STRATEGY_REQUEST_TIMEOUT — /judge timeout seconds (default: 45)
"""

from __future__ import annotations

import json
import logging
import os
import re
import sys
import time
import urllib.request
import urllib.error
from collections import defaultdict
from pathlib import Path

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

_raw_addr: str = os.environ.get("CYNIC_REST_ADDR", "")
CYNIC_REST_ADDR: str = (
    f"http://{_raw_addr}" if _raw_addr and not _raw_addr.startswith("http") else _raw_addr
)
CYNIC_API_KEY: str = os.environ.get("CYNIC_API_KEY", "")
HANDLE: str = os.environ.get("TALARIA_HANDLE", "talariabuild").lower()
DEFAULT_DATASET = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"
DATASET: Path = Path(os.environ.get("HERMES_DATASET", str(DEFAULT_DATASET)))
MIN_NEW_POSTS: int = int(os.environ.get("STRATEGY_MIN_NEW_POSTS", "5"))
REQUEST_TIMEOUT: int = int(os.environ.get("STRATEGY_REQUEST_TIMEOUT", "45"))
STATE_FILE: Path = Path(os.environ.get(
    "STRATEGY_STATE_FILE",
    str(Path.home() / ".cynic" / "talaria_strategy_state.json"),
))
AGENT_ID = "talaria-strategy"

logging.basicConfig(
    format="%(asctime)s talaria-strategy %(levelname)s %(message)s",
    level=logging.INFO,
    stream=sys.stdout,
)
log = logging.getLogger(AGENT_ID)

# ---------------------------------------------------------------------------
# State
# ---------------------------------------------------------------------------

def load_state() -> dict:
    if not STATE_FILE.exists():
        return {"seen_ids": [], "last_judged_at": 0.0}
    try:
        return json.loads(STATE_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return {"seen_ids": [], "last_judged_at": 0.0}


def save_state(state: dict) -> None:
    STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    # Keep only last 500 IDs to bound file size
    state["seen_ids"] = list(state.get("seen_ids", []))[-500:]
    STATE_FILE.write_text(json.dumps(state))

# ---------------------------------------------------------------------------
# Post classification
# ---------------------------------------------------------------------------

def post_type(row: dict) -> str:
    if row.get("retweeted_tweet"):
        return "retweet"
    quoted = row.get("quoted_tweet")
    if quoted and isinstance(quoted, dict) and quoted.get("tweet_id"):
        return "qrt"
    if isinstance(quoted, str) and quoted:
        return "qrt"
    if row.get("in_reply_to_screen_name"):
        return "reply"
    return "original"


# ---------------------------------------------------------------------------
# Dataset reading
# ---------------------------------------------------------------------------

def load_own_posts() -> list[dict]:
    """Read all @talariabuild posts from dataset.jsonl (skip retweets)."""
    if not DATASET.exists():
        log.error("dataset not found: %s", DATASET)
        return []

    posts = []
    with DATASET.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get("author_screen_name", "").lower() != HANDLE:
                continue
            if row.get("retweeted_tweet"):
                continue
            row["_post_type"] = post_type(row)
            posts.append(row)

    return posts

# ---------------------------------------------------------------------------
# Aggregation
# ---------------------------------------------------------------------------

def aggregate(posts: list[dict]) -> dict:
    """Aggregate engagement metrics by post_type."""
    by_type: dict[str, list[dict]] = defaultdict(list)
    for p in posts:
        by_type[p["_post_type"]].append(p)

    summary: dict = {}
    for ptype, items in by_type.items():
        n = len(items)
        views = [i.get("views", 0) for i in items]
        likes = [i.get("likes", 0) for i in items]
        eng = [i.get("engagement_rate", 0.0) for i in items]
        replies_c = [i.get("replies", 0) for i in items]
        quotes_c = [i.get("quotes", 0) for i in items]

        summary[ptype] = {
            "count": n,
            "avg_views": round(sum(views) / n, 1),
            "avg_likes": round(sum(likes) / n, 2),
            "avg_engagement": round(sum(eng) / n, 4),
            "avg_replies": round(sum(replies_c) / n, 2),
            "avg_quotes": round(sum(quotes_c) / n, 2),
            "top_post": _top_post(items),
        }

    return summary


def _top_post(items: list[dict]) -> dict:
    """Return the highest-engagement post."""
    best = max(items, key=lambda x: x.get("engagement_rate", 0.0))
    return {
        "tweet_id": best.get("tweet_id", ""),
        "text": best.get("text", "")[:120],
        "engagement_rate": best.get("engagement_rate", 0.0),
        "views": best.get("views", 0),
        "likes": best.get("likes", 0),
    }

# ---------------------------------------------------------------------------
# Prompt building
# ---------------------------------------------------------------------------

def build_prompt(posts: list[dict], summary: dict, new_count: int) -> str:
    total = len(posts)
    lines = [
        f"@talariabuild editorial performance analysis — {total} total posts, {new_count} new since last judgment.",
        "",
        "POST TYPE BREAKDOWN:",
    ]
    for ptype in ("original", "qrt", "reply"):
        s = summary.get(ptype)
        if not s:
            continue
        lines.append(
            f"  {ptype} ({s['count']} posts): "
            f"avg_views={s['avg_views']:.0f} avg_likes={s['avg_likes']:.1f} "
            f"avg_eng={s['avg_engagement']:.2%} avg_replies={s['avg_replies']:.1f}"
        )
        top = s["top_post"]
        lines.append(f"    top: \"{top['text'][:80]}\" → eng={top['engagement_rate']:.2%} views={top['views']}")

    lines += [
        "",
        "QUESTION: Based on this data, what editorial strategy should @talariabuild follow?",
        "Consider: which post type maximizes reach, which format drives replies/discussion,",
        "and whether the cynical/aphoristic style is working. Be specific and falsifiable.",
    ]
    return "\n".join(lines)

# ---------------------------------------------------------------------------
# Kernel helpers
# ---------------------------------------------------------------------------

def _headers() -> dict[str, str]:
    h = {"Accept": "application/json", "Content-Type": "application/json"}
    if CYNIC_API_KEY:
        h["Authorization"] = f"Bearer {CYNIC_API_KEY}"
    return h


def judge_strategy(prompt: str) -> dict | None:
    if not CYNIC_REST_ADDR or not CYNIC_API_KEY:
        log.error("CYNIC_REST_ADDR or CYNIC_API_KEY not set")
        return None

    payload = {
        "content": prompt,
        "domain": "general",
        "agent_id": AGENT_ID,
    }
    body = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"{CYNIC_REST_ADDR.rstrip('/')}/judge",
        data=body,
        headers=_headers(),
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=REQUEST_TIMEOUT) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as exc:
        log.error("POST /judge HTTP %d: %s", exc.code, exc.reason)
        return None
    except urllib.error.URLError as exc:
        log.warning("POST /judge network error: %s — will retry next cycle", exc.reason)
        return None
    except Exception as exc:  # noqa: BLE001
        log.error("POST /judge unexpected: %s", exc)
        return None

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def run() -> int:
    log.info("talaria_strategy_consumer starting (min_new=%d, timeout=%ds)", MIN_NEW_POSTS, REQUEST_TIMEOUT)

    if not CYNIC_REST_ADDR or not CYNIC_API_KEY:
        log.error("CYNIC_REST_ADDR or CYNIC_API_KEY not set")
        return 1

    state = load_state()
    seen_ids: set[str] = set(state.get("seen_ids", []))

    posts = load_own_posts()
    if not posts:
        log.info("no @%s posts in dataset — nothing to do", HANDLE)
        return 0

    new_posts = [p for p in posts if p.get("tweet_id", "") not in seen_ids]
    log.info("total=%d new=%d", len(posts), len(new_posts))

    if len(new_posts) < MIN_NEW_POSTS:
        log.info("only %d new posts, need %d — skipping judgment", len(new_posts), MIN_NEW_POSTS)
        return 0

    summary = aggregate(posts)
    prompt = build_prompt(posts, summary, len(new_posts))
    log.info("built prompt (%d chars), calling /judge...", len(prompt))

    verdict = judge_strategy(prompt)
    if verdict is None:
        log.warning("judgment failed — will retry next cycle (state not updated)")
        return 1

    verdict_kind = verdict.get("verdict", "?")
    q_total = verdict.get("q_score", {}).get("total", 0.0)
    log.info("verdict=%s q=%.3f", verdict_kind, q_total)

    # Update state: mark all current posts as seen
    state["seen_ids"] = list(seen_ids | {p["tweet_id"] for p in posts if p.get("tweet_id")})
    state["last_judged_at"] = time.time()
    save_state(state)

    return 0


if __name__ == "__main__":
    sys.exit(run())
