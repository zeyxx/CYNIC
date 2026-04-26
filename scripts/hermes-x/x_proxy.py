"""
CYNIC X Proxy — mitmproxy addon: capture → extract → enrich → dataset.

Passively captures X/Twitter GraphQL responses, extracts tweets,
enriches with signal scores and author tiers, appends to dataset.jsonl.
The ingest daemon reads the dataset and forwards to the kernel.

Usage (standalone):
    mitmdump -s x_proxy.py --listen-host 127.0.0.1 -p 8888

Production: hermes-proxy.service (systemd, domain-filtered, organ dir).

SECURITY: --listen-host 127.0.0.1 mandatory (KC4).
           --set allow_hosts in systemd service (SoC: only X traffic decrypted).
"""

import hashlib
import json
import os
import logging
import re
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse, parse_qs

from mitmproxy import http

logger = logging.getLogger("x-proxy")

CAPTURE_OPS = {
    "SearchTimeline", "UserTweets", "TweetDetail",
    "HomeTimeline", "HomeLatestTimeline", "ListLatestTweetsTimeline",
}

# Env vars set by systemd service; fallback to local for dev
CAPTURE_DIR = Path(os.environ.get(
    "X_CAPTURE_DIR", Path(__file__).parent / "captures"
))
DATASET_PATH = Path(os.environ.get(
    "X_DATASET_PATH", CAPTURE_DIR / "dataset.jsonl"
))

# ── Signal scoring: -5 (noise) to +7 (strong signal) ──

_SPAM = re.compile(
    r"top\s+\d+\s+memecoins?|next\s+100x|guaranteed\s+moon|"
    r"free\s+airdrop|check\s+my\s+pin|DM\s+for\s+alpha|"
    r"buy\s+now\s+before|not\s+financial\s+advice\s+but\s+you\s+should",
    re.IGNORECASE,
)
_QUALITY = re.compile(
    r"on[- ]?chain\s+data|liquidity\s+pool|market\s+cap|"
    r"token\s+economics|audit|smart\s+contract|"
    r"rug\s*pull|honeypot|dev\s+wallet|thread|analysis|research",
    re.IGNORECASE,
)

_NARRATIVES = [
    (re.compile(r"rug|scam|honeypot|dev sold", re.I), "warning"),
    (re.compile(r"bullish|moon|pump|gem|100x", re.I), "hype"),
    (re.compile(r"analysis|thread|research|data", re.I), "analysis"),
    (re.compile(r"community|dao|governance|vote", re.I), "community"),
    (re.compile(r"launch|mint|deploy|live", re.I), "launch"),
]


def _signal_score(text: str, tweet: dict) -> int:
    score = 0
    if _SPAM.search(text):
        score -= 3
    if tweet.get("is_retweet"):
        score -= 1
    if _QUALITY.search(text):
        score += 2
    if tweet.get("author_followers", 0) > 10000:
        score += 1
    if tweet.get("author_verified"):
        score += 1
    views = tweet.get("view_count", 0)
    likes = tweet.get("favorite_count", 0)
    if views > 0 and likes / views > 0.02:
        score += 1
    if len(text) > 200 and not tweet.get("is_retweet"):
        score += 1
    if any(tweet.get(k, 0) > 0 for k in ("reply_count", "bookmark_count")):
        score += 1
    return max(-5, min(7, score))


def _author_tier(tweet: dict) -> str:
    followers = tweet.get("author_followers", 0)
    statuses = tweet.get("author_statuses_count", 0)
    if followers > 100000:
        return "whale"
    if followers > 10000 and tweet.get("author_verified"):
        return "influencer"
    if statuses > 0 and statuses > followers * 50:
        return "bot"
    if followers > 500:
        return "active"
    return "unknown"


def _narratives(text: str) -> list[str]:
    return [label for pat, label in _NARRATIVES if pat.search(text)]


def _coordination(tweets: list[dict]) -> dict[str, int]:
    counts: Counter = Counter()
    for t in tweets:
        norm = re.sub(r"\s+", " ", t.get("text", "").strip().lower())
        if len(norm) > 30:
            counts[norm] += 1
    return {k: v for k, v in counts.items() if v >= 3}


# ── Tweet extraction from GraphQL responses ──

def _extract_tweets(data: dict) -> list[dict]:
    resp = data.get("response", data)
    for path in [
        ["data", "search_by_raw_query", "search_timeline", "timeline", "instructions"],
        ["data", "user", "result", "timeline_v2", "timeline", "instructions"],
        ["data", "home", "home_timeline_urt", "instructions"],
        ["data", "list", "tweets_timeline", "timeline", "instructions"],
    ]:
        node = resp
        for key in path:
            node = node.get(key, {}) if isinstance(node, dict) else {}
        if isinstance(node, list) and node:
            tweets = []
            for instr in node:
                if instr.get("type") != "TimelineAddEntries":
                    continue
                for entry in instr.get("entries", []):
                    t = _parse_entry(entry)
                    if t:
                        tweets.append(t)
            return tweets
    return []


def _parse_entry(entry: dict) -> dict | None:
    try:
        content = entry.get("content", {})
        etype = content.get("entryType") or content.get("__typename")
        if etype == "TimelineTimelineItem":
            item = content.get("itemContent", {})
            if item.get("itemType") != "TimelineTweet":
                return None
            return _parse_result(item.get("tweet_results", {}).get("result", {}))
        if etype == "TimelineTimelineModule":
            items = content.get("items", [])
            if items:
                first = items[0].get("item", {}).get("itemContent", {})
                return _parse_result(first.get("tweet_results", {}).get("result", {}))
    except (KeyError, TypeError, AttributeError):
        pass
    return None


def _parse_result(result: dict) -> dict | None:
    if not result:
        return None
    tn = result.get("__typename", "")
    if tn == "TweetWithVisibilityResults":
        result = result.get("tweet", {})
    elif tn == "TweetTombstone":
        return None
    legacy = result.get("legacy", {})
    if not legacy:
        return None
    uroot = result.get("core", {}).get("user_results", {}).get("result", {})
    ucore = uroot.get("core", {})
    uleg = uroot.get("legacy", {})
    views = result.get("views", {})

    def safe_int(v):
        try:
            return int(v)
        except (ValueError, TypeError):
            return 0

    return {
        "id": legacy.get("id_str"),
        "text": legacy.get("full_text", ""),
        "created_at": legacy.get("created_at", ""),
        "author": ucore.get("screen_name", "") or uleg.get("screen_name", ""),
        "author_name": ucore.get("name", "") or uleg.get("name", ""),
        "author_followers": uleg.get("followers_count", 0),
        "author_statuses_count": uleg.get("statuses_count", 0),
        "author_favourites_count": uleg.get("favourites_count", 0),
        "author_verified": uroot.get("is_blue_verified", False),
        "retweet_count": legacy.get("retweet_count", 0),
        "favorite_count": legacy.get("favorite_count", 0),
        "reply_count": legacy.get("reply_count", 0),
        "bookmark_count": legacy.get("bookmark_count", 0),
        "quote_count": legacy.get("quote_count", 0),
        "view_count": safe_int(views.get("count", 0)),
        "cashtags": [s["text"] for s in legacy.get("entities", {}).get("symbols", [])],
        "hashtags": [h["text"] for h in legacy.get("entities", {}).get("hashtags", [])],
        "mentions": [m["screen_name"] for m in legacy.get("entities", {}).get("user_mentions", [])],
        "is_retweet": "retweeted_status_result" in legacy or legacy.get("full_text", "").startswith("RT @"),
        "is_reply": bool(legacy.get("in_reply_to_status_id_str")),
        "lang": legacy.get("lang", ""),
    }


# ── Enrichment: raw tweet → dataset row ──

def _enrich(tweet: dict, operation: str, variables: dict, coord_map: dict) -> dict:
    text = tweet.get("text", "")
    norm = re.sub(r"\s+", " ", text.strip().lower())
    coord_count = coord_map.get(norm, 0) if len(norm) > 30 else 0
    views = tweet.get("view_count", 0)
    likes = tweet.get("favorite_count", 0)
    rts = tweet.get("retweet_count", 0)
    replies = tweet.get("reply_count", 0)
    engagement = (likes + rts + replies) / views if views > 0 else 0.0

    return {
        "tweet_id": tweet.get("id", ""),
        "dedupe_key": hashlib.blake2b(
            f"{tweet.get('id', '')}{tweet.get('author', '')}".encode(), digest_size=8,
        ).hexdigest(),
        "text": text,
        "created_at": tweet.get("created_at", ""),
        "capture_ts": datetime.now(timezone.utc).isoformat(),
        "operation": operation,
        "search_query": variables.get("rawQuery", ""),
        "interaction_type": (
            "retweet" if tweet.get("is_retweet") else
            "reply" if tweet.get("is_reply") else "original"
        ),
        "is_self_thread": False,
        "lang": tweet.get("lang", ""),
        "possibly_sensitive": False,
        # Author
        "author_screen_name": tweet.get("author", ""),
        "author_display_name": tweet.get("author_name", ""),
        "author_user_id": "",
        "author_followers_count": tweet.get("author_followers", 0),
        "author_following_count": 0,
        "author_statuses_count": tweet.get("author_statuses_count", 0),
        "author_favourites_count": tweet.get("author_favourites_count", 0),
        "author_account_created_at": "",
        "author_bio": "",
        "author_is_blue_verified": tweet.get("author_verified", False),
        "author_verified": tweet.get("author_verified", False),
        "author_default_profile": False,
        "author_default_profile_image": False,
        "author_profile_banner": "",
        "author_tier": _author_tier(tweet),
        # Metrics
        "views": views,
        "likes": likes,
        "retweets": rts,
        "replies": replies,
        "quotes": tweet.get("quote_count", 0),
        "bookmarks": tweet.get("bookmark_count", 0),
        "engagement_rate": round(engagement, 6),
        "viewer_favorited": False,
        "viewer_retweeted": False,
        "viewer_bookmarked": False,
        # Entities
        "cashtags": tweet.get("cashtags", []),
        "hashtags": tweet.get("hashtags", []),
        "mentions": tweet.get("mentions", []),
        "urls": [],
        "media": [],
        "has_media": False,
        "quoted_tweet": None,
        "retweeted_tweet": None,
        # Enrichment
        "signal_score": _signal_score(text, tweet),
        "narratives": _narratives(text),
        "is_coordinated": coord_count >= 3,
        "coordination_count": coord_count,
        "sampling_bias": "proxy-passive",
    }


# ── Dedup cache ──

class _DedupeCache:
    def __init__(self, max_size: int = 50000):
        self._seen: set[str] = set()
        self._max = max_size

    def is_new(self, key: str) -> bool:
        if key in self._seen:
            return False
        if len(self._seen) >= self._max:
            evict = len(self._seen) // 4
            for k in list(self._seen)[:evict]:
                self._seen.discard(k)
        self._seen.add(key)
        return True

    def load(self, path: Path):
        if not path.exists():
            return
        try:
            with open(path) as f:
                for line in f:
                    try:
                        self._seen.add(json.loads(line)["dedupe_key"])
                    except (json.JSONDecodeError, KeyError):
                        continue
            logger.info("dedup: loaded %d existing keys", len(self._seen))
        except OSError:
            pass


# ── Main addon ──

class XProxy:
    """mitmproxy addon: capture → extract → enrich → append dataset."""

    def __init__(self):
        CAPTURE_DIR.mkdir(parents=True, exist_ok=True)
        DATASET_PATH.parent.mkdir(parents=True, exist_ok=True)
        self._dedup = _DedupeCache()
        self._dedup.load(DATASET_PATH)
        self._stats = {"captured": 0, "enriched": 0, "deduped": 0}
        logger.info("x-proxy loaded — %d existing tweets, dataset: %s",
                     len(self._dedup._seen), DATASET_PATH)

    def response(self, flow: http.HTTPFlow) -> None:
        url = flow.request.url
        if "/i/api/graphql/" not in url:
            return

        op_name = self._extract_op(url)
        if not op_name or op_name not in CAPTURE_OPS:
            return
        if flow.response is None or flow.response.status_code != 200:
            return

        try:
            data = json.loads(flow.response.content)
        except (json.JSONDecodeError, TypeError):
            return

        variables = self._extract_vars(url)
        ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")

        # Raw capture
        (CAPTURE_DIR / f"{ts}_{op_name}.json").write_text(json.dumps({
            "operation": op_name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "variables": variables,
            "response": data,
        }, indent=2, ensure_ascii=False))
        self._stats["captured"] += 1

        # Extract + enrich
        raw_tweets = _extract_tweets(data)
        if not raw_tweets:
            return

        coord_map = _coordination(raw_tweets)
        new = []
        for t in raw_tweets:
            enriched = _enrich(t, op_name, variables, coord_map)
            if self._dedup.is_new(enriched["dedupe_key"]):
                new.append(enriched)
            else:
                self._stats["deduped"] += 1

        if new:
            with open(DATASET_PATH, "a") as f:
                for row in new:
                    f.write(json.dumps(row, ensure_ascii=False) + "\n")
            self._stats["enriched"] += len(new)

        logger.info("x-proxy: %s +%d new (%d total, %d dedup)",
                     op_name, len(new), self._stats["enriched"], self._stats["deduped"])

    @staticmethod
    def _extract_op(url: str) -> str | None:
        try:
            parts = url.split("/i/api/graphql/")[1].split("?")[0].split("/")
            return parts[1] if len(parts) >= 2 else None
        except (IndexError, AttributeError):
            return None

    @staticmethod
    def _extract_vars(url: str) -> dict:
        try:
            return json.loads(parse_qs(urlparse(url).query).get("variables", ["{}"])[0])
        except (json.JSONDecodeError, KeyError):
            return {}


addons = [XProxy()]
