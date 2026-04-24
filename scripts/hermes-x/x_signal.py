"""
CYNIC X Signal Extractor — transforms raw X API captures into structured signals.

Reads JSON captures from x_proxy.py, extracts tweet data, computes metrics,
and outputs signals ready for POST /judge.

Usage:
    python x_signal.py captures/20260424_011500_SearchTimeline.json
    python x_signal.py --watch captures/    # watch directory for new files
"""

import json
import sys
import time
import os
import logging
from datetime import datetime, timezone
from pathlib import Path

logger = logging.getLogger("x-signal")

# --- Tweet extraction from X GraphQL response ---

def extract_tweets_from_search(data: dict) -> list[dict]:
    """Extract structured tweets from SearchTimeline response."""
    tweets = []
    try:
        instructions = (
            data.get("response", data)
            .get("data", {})
            .get("search_by_raw_query", {})
            .get("search_timeline", {})
            .get("timeline", {})
            .get("instructions", [])
        )
    except AttributeError:
        return tweets

    for instruction in instructions:
        if instruction.get("type") != "TimelineAddEntries":
            continue
        for entry in instruction.get("entries", []):
            tweet = _extract_tweet_from_entry(entry)
            if tweet:
                tweets.append(tweet)

    return tweets


def extract_tweets_from_user(data: dict) -> list[dict]:
    """Extract structured tweets from UserTweets response."""
    tweets = []
    try:
        instructions = (
            data.get("response", data)
            .get("data", {})
            .get("user", {})
            .get("result", {})
            .get("timeline_v2", {})
            .get("timeline", {})
            .get("instructions", [])
        )
    except AttributeError:
        return tweets

    for instruction in instructions:
        if instruction.get("type") != "TimelineAddEntries":
            continue
        for entry in instruction.get("entries", []):
            tweet = _extract_tweet_from_entry(entry)
            if tweet:
                tweets.append(tweet)

    return tweets


def _extract_tweet_from_entry(entry: dict) -> dict | None:
    """Extract a single tweet from a timeline entry."""
    try:
        content = entry.get("content", {})
        entry_type = content.get("entryType") or content.get("__typename")

        # Handle tweet items
        if entry_type == "TimelineTimelineItem":
            item = content.get("itemContent", {})
            if item.get("itemType") != "TimelineTweet":
                return None
            result = item.get("tweet_results", {}).get("result", {})
            return _parse_tweet_result(result)

        # Handle tweet modules (e.g., conversation threads)
        if entry_type == "TimelineTimelineModule":
            items = content.get("items", [])
            if items:
                first = items[0].get("item", {}).get("itemContent", {})
                result = first.get("tweet_results", {}).get("result", {})
                return _parse_tweet_result(result)

    except (KeyError, TypeError, AttributeError):
        pass
    return None


def _parse_tweet_result(result: dict) -> dict | None:
    """Parse a tweet_results.result object into structured data."""
    if not result:
        return None

    # Handle TweetWithVisibilityResults wrapper
    typename = result.get("__typename", "")
    if typename == "TweetWithVisibilityResults":
        result = result.get("tweet", {})
    elif typename == "TweetTombstone":
        return None

    legacy = result.get("legacy", {})
    if not legacy:
        return None

    user_root = result.get("core", {}).get("user_results", {}).get("result", {})
    # X stores screen_name/name under user.core (not user.legacy)
    user_core = user_root.get("core", {})
    user_legacy = user_root.get("legacy", {})
    views = result.get("views", {})

    return {
        "id": legacy.get("id_str"),
        "text": legacy.get("full_text", ""),
        "created_at": legacy.get("created_at", ""),
        "author": user_core.get("screen_name", "") or user_legacy.get("screen_name", ""),
        "author_name": user_core.get("name", "") or user_legacy.get("name", ""),
        "author_followers": user_legacy.get("followers_count", 0),
        "author_verified": user_root.get("is_blue_verified", False),
        "retweet_count": legacy.get("retweet_count", 0),
        "favorite_count": legacy.get("favorite_count", 0),
        "reply_count": legacy.get("reply_count", 0),
        "bookmark_count": legacy.get("bookmark_count", 0),
        "view_count": _safe_int(views.get("count", 0)),
        "cashtags": [s["text"] for s in legacy.get("entities", {}).get("symbols", [])],
        "hashtags": [h["text"] for h in legacy.get("entities", {}).get("hashtags", [])],
        "mentions": [m["screen_name"] for m in legacy.get("entities", {}).get("user_mentions", [])],
        "is_retweet": "retweeted_status_result" in legacy or legacy.get("full_text", "").startswith("RT @"),
        "is_reply": bool(legacy.get("in_reply_to_status_id_str")),
        "lang": legacy.get("lang", ""),
    }


def _safe_int(v) -> int:
    try:
        return int(v)
    except (ValueError, TypeError):
        return 0


# --- Signal aggregation ---

def aggregate_signal(operation: str, variables: dict, tweets: list[dict]) -> dict:
    """Aggregate extracted tweets into a structured signal for the kernel."""
    query = variables.get("rawQuery", "unknown")
    now = datetime.now(timezone.utc).isoformat()

    if not tweets:
        return {
            "source": f"x-{operation.lower()}",
            "query": query,
            "timestamp": now,
            "tweet_count": 0,
            "signal": "empty",
        }

    total_views = sum(t["view_count"] for t in tweets)
    total_likes = sum(t["favorite_count"] for t in tweets)
    total_rts = sum(t["retweet_count"] for t in tweets)
    total_replies = sum(t["reply_count"] for t in tweets)
    total_engagement = total_likes + total_rts + total_replies
    engagement_rate = total_engagement / total_views if total_views > 0 else 0

    unique_authors = list({t["author"] for t in tweets if t["author"]})
    all_cashtags = {}
    for t in tweets:
        for ct in t["cashtags"]:
            all_cashtags[ct.upper()] = all_cashtags.get(ct.upper(), 0) + 1

    # Content summary — first 3 non-RT tweets, truncated
    originals = [t for t in tweets if not t["is_retweet"]][:3]
    content_sample = [
        {"author": t["author"], "text": t["text"][:200], "likes": t["favorite_count"], "views": t["view_count"]}
        for t in originals
    ]

    return {
        "source": f"x-{operation.lower()}",
        "query": query,
        "timestamp": now,
        "tweet_count": len(tweets),
        "metrics": {
            "total_views": total_views,
            "total_likes": total_likes,
            "total_retweets": total_rts,
            "total_replies": total_replies,
            "engagement_rate": round(engagement_rate, 5),
            "unique_authors": len(unique_authors),
            "original_vs_rt": sum(1 for t in tweets if not t["is_retweet"]),
        },
        "authors": {
            "names": unique_authors[:10],
            "max_followers": max((t["author_followers"] for t in tweets), default=0),
            "verified_count": sum(1 for t in tweets if t["author_verified"]),
        },
        "cashtags": all_cashtags,
        "content_sample": content_sample,
    }


# --- CLI ---

def process_capture(path: Path) -> dict | None:
    """Process a single capture file into a signal."""
    try:
        raw = json.loads(path.read_text())
    except (json.JSONDecodeError, OSError) as e:
        logger.error("Failed to read %s: %s", path, e)
        return None

    operation = raw.get("operation", "Unknown")
    variables = raw.get("variables", {})

    if "Search" in operation:
        tweets = extract_tweets_from_search(raw)
    elif "User" in operation:
        tweets = extract_tweets_from_user(raw)
    else:
        tweets = extract_tweets_from_search(raw)  # best-effort fallback

    signal = aggregate_signal(operation, variables, tweets)
    logger.info("Processed %s: %d tweets, %d views",
                path.name, signal["tweet_count"], signal.get("metrics", {}).get("total_views", 0))
    return signal


def watch_directory(capture_dir: Path, callback):
    """Watch capture directory for new files and process them."""
    seen = set()
    logger.info("Watching %s for new captures...", capture_dir)
    while True:
        for f in sorted(capture_dir.glob("*.json")):
            if f.name not in seen:
                seen.add(f.name)
                signal = process_capture(f)
                if signal:
                    callback(signal)
        time.sleep(2)


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")

    if len(sys.argv) < 2:
        print("Usage: python x_signal.py <capture.json | --watch <dir>>")
        sys.exit(1)

    if sys.argv[1] == "--watch":
        watch_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("captures")
        watch_directory(watch_dir, lambda s: print(json.dumps(s, indent=2)))
    else:
        signal = process_capture(Path(sys.argv[1]))
        if signal:
            print(json.dumps(signal, indent=2))
