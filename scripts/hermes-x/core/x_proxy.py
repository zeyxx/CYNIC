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
import tomllib
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlparse, parse_qs

from mitmproxy import http

from hermes_paths import (
    CAPTURES_DIR as CAPTURE_DIR,
    DATASET as DATASET_PATH,
    SEARCH_RESULTS_LOG,
)

logger = logging.getLogger("x-proxy")


def _load_account_config() -> dict:
    """Load account configuration from accounts.toml."""
    config_path = Path.home() / ".config" / "cynic" / "accounts.toml"
    if not config_path.exists():
        return {}
    try:
        with open(config_path, "rb") as f:
            return tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as e:
        logger.warning("Failed to load accounts.toml: %s", e)
        return {}

CAPTURE_OPS = {
    "SearchTimeline", "UserTweets", "TweetDetail",
    "HomeTimeline", "HomeLatestTimeline", "ListLatestTweetsTimeline",
}
ACCOUNT_ID = os.environ.get("HERMES_ACCOUNT", "cynic")

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

# Narrative tags — vocabulary MUST match narrative_domains.yaml keys.
# Each tag maps to a domain via narrative_domains.yaml (priority 1 in assign_domain).
# Order matters: first match wins in _narratives(). Most specific patterns first.
_NARRATIVES = [
    # D4: Security/Exploits
    (re.compile(r"rug\s*pull|rugged|dev\s+sold|dev\s+dumped|LP\s+removed", re.I), "rug_pull"),
    (re.compile(r"honeypot|honey\s+pot", re.I), "honeypot"),
    (re.compile(r"exploit|hack(?:ed)?|drain(?:ed)?|bridge\s+attack|reentrancy", re.I), "exploit"),
    (re.compile(r"scam|fraud|fake\s+(?:token|project|team)", re.I), "scam"),
    (re.compile(r"vuln(?:erability)?|CVE-|zero[- ]day|bug\s+bounty", re.I), "vulnerability"),
    # D1: Solana/Tokens
    (re.compile(r"rug\s+warning|bundled\s+scam|cabal", re.I), "rug_warning"),
    (re.compile(r"pump\.fun|pumpfun|bonding\s+curve|graduated", re.I), "token_launch"),
    (re.compile(r"memecoin|meme\s+coin|[$](?:BONK|WIF|FARTCOIN)", re.I), "meme_token"),
    (re.compile(r"bullish|moon|pump(?!\.fun)|gem|100x|1000x", re.I), "pump_hype"),
    (re.compile(r"solana\s+token|sol\s+ecosystem|SPL\s+token", re.I), "solana_token"),
    # D2: Inference/LLM
    (re.compile(r"local\s+(?:LLM|AI|model)|self[- ]hosted\s+(?:AI|model)", re.I), "inference"),
    (re.compile(r"Qwen|llama\.cpp|vLLM|SGLang|GGUF|tok/s|LoRA|fine[- ]tun", re.I), "llm"),
    (re.compile(r"(?:AI|LLM)\s+agent|agent\s+(?:framework|protocol)", re.I), "agent"),
    (re.compile(r"open[- ](?:weights|source\s+model)|open[- ]model", re.I), "open_weights"),
    (re.compile(r"AI\s+(?:crypto|token|depin)|crypto\s+AI|GPU\s+network", re.I), "ai_crypto"),
    # D5: Macro/Markets
    (re.compile(r"macro|interest\s+rate|monetary\s+policy|fiscal|M2\s+", re.I), "macro"),
    (re.compile(r"ETF|institutional|blackrock|grayscale", re.I), "institutional"),
    (re.compile(r"market\s+(?:cap|structure|crash|rally)|bear|bull\s+market", re.I), "market"),
    (re.compile(r"regulat(?:ion|or)|SEC\s|MiCA|legislation", re.I), "regulation"),
    # D3: Sovereignty
    (re.compile(r"self[- ](?:custody|hosted|sovereign)|own\s+your\s+(?:data|keys)", re.I), "self_custody"),
    (re.compile(r"sovereign(?:ty)?|censorship[- ]resist", re.I), "sovereignty"),
    (re.compile(r"decentraliz(?:ed|ation)|permissionless|trustless", re.I), "decentralization"),
    # D3 extended: Prediction markets / MetaDAO / B&C
    (re.compile(r"polymarket|kalshi|prediction\s+market|forecast\s+market", re.I), "prediction_market"),
    (re.compile(r"metadao|futarchy|futardio", re.I), "futarchy"),
    (re.compile(r"p2p\s+wager|player\s+vs\.?\s+player\s+(?:bet|stake)|skill\s+wager", re.I), "p2p_wager"),
    (re.compile(r"sportsfi|sports?\s*(?:betting|prediction|wager)|sportsbetting", re.I), "sports_betting"),
    # D6: Epistemology
    (re.compile(r"falsif(?:y|iable)|verif(?:y|iable)|calibrat(?:e|ion)|epistemic", re.I), "epistemology"),
    (re.compile(r"bounded\s+judgment|signal\s+vs\s+noise|ground\s+truth", re.I), "bounded_judgment"),
]


def _signal_score(text: str, tweet: dict) -> int:
    """v2: Improved signal scoring with author-tier and engagement awareness.

    Baseline heuristics from v1, enhanced with:
    - Author tier weighting (whale +2, bot -1)
    - Engagement-aware (>5% ratio +2, >2% +1, +absolute >1000 +1)
    - Domain-aware (cashtags +1, narratives +1)
    - Harsher retweet penalty (-2 vs -1)
    """
    score = 0

    # Negative signals
    if _SPAM.search(text):
        score -= 3
    if tweet.get("is_retweet"):
        score -= 2  # v2: harsher penalty for retweets

    # Positive signals - quality
    if _QUALITY.search(text):
        score += 2

    # Author tier weighting (NEW in v2)
    # Note: author_tier is calculated from tweet metadata, not yet in tweet dict
    tier = _author_tier(tweet)
    if tier == "whale":
        score += 2
    elif tier == "bot":
        score -= 1
    # "influencer"/"active"/"unknown" get no bonus

    # Verification (keep from v1)
    if tweet.get("author_verified"):
        score += 1

    # Engagement: ratio + absolute (IMPROVED in v2)
    views = tweet.get("view_count", 0)
    likes = tweet.get("favorite_count", 0)
    engagement_sum = likes + tweet.get("reply_count", 0) + tweet.get("bookmark_count", 0)

    if views > 0:
        engagement_ratio = engagement_sum / views
        # v2: scale engagement ratio more aggressively
        if engagement_ratio > 0.05:  # 5% engagement is excellent
            score += 2
        elif engagement_ratio > 0.02:  # 2% engagement is good
            score += 1

    # Absolute engagement bonus (NEW in v2)
    if engagement_sum > 1000:
        score += 1

    # Text quality (v2: higher bar for length bonus)
    if len(text) > 200 and not tweet.get("is_retweet"):
        score += 1

    # Discussion signals
    replies = tweet.get("reply_count", 0)
    bookmarks = tweet.get("bookmark_count", 0)
    if replies > bookmarks and replies > 0:
        score += 1  # Discussion > saving
    elif bookmarks > 0:
        score += 1  # Some engagement

    # Domain signals (NEW in v2)
    cashtags = tweet.get("cashtags", [])
    if cashtags:
        score += 1  # Token mentions are domain-relevant

    narratives = _narratives(text)
    if narratives:
        score += 1  # Hermes already detected relevant pattern

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

# Labelled extraction paths for schema observability
EXTRACTION_PATHS = [
    # (label, path_keys)
    ("tweetdetail_v2",   ["data", "threaded_conversation_with_injections_v2", "instructions"]),  # New TweetDetail endpoint (April 2026+)
    ("user_timeline_v2", ["data", "user", "result", "timeline", "timeline", "instructions"]),     # Updated UserTweets (timeline_v2 removed)
    ("search_v1",        ["data", "search_by_raw_query", "search_timeline", "timeline", "instructions"]),  # Old search path
    ("user_timeline_v1", ["data", "user", "result", "timeline_v2", "timeline", "instructions"]),  # Old user path
    ("home_timeline",    ["data", "home", "home_timeline_urt", "instructions"]),                  # Old home path
    ("list_timeline",    ["data", "list", "tweets_timeline", "timeline", "instructions"]),       # Old list path
]

def _extract_tweets(data: dict) -> tuple[list[dict], str]:
    """Extract tweets from GraphQL response, return (tweets, path_label)."""
    resp = data.get("response", data)

    for path_label, path_keys in EXTRACTION_PATHS:
        node = resp
        for key in path_keys:
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
            return tweets, path_label

    # No path matched — log schema change signal
    data_keys = list(resp.get("data", {}).keys()) if isinstance(resp.get("data"), dict) else []
    if data_keys:
        logger.warning("SCHEMA_MISS: no extraction path matched. data.keys=%s", data_keys[:8])

    return [], "schema_miss"


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
    entities = legacy.get("entities", {})

    def safe_int(v):
        try:
            return int(v)
        except (ValueError, TypeError):
            return 0

    return {
        # Core
        "id": legacy.get("id_str"),
        "text": legacy.get("full_text", ""),
        "created_at": legacy.get("created_at", ""),
        "lang": legacy.get("lang", ""),
        "possibly_sensitive": legacy.get("possibly_sensitive", False),

        # Author (full extraction)
        # Fallback: when X API withholds user_results (suspended/restricted accounts),
        # use legacy.user_id_str as identifier. 18/21514 entries hit this (2026-05-17).
        "author": ucore.get("screen_name", "") or uleg.get("screen_name", "") or f"uid:{legacy.get('user_id_str', 'unknown')}",
        "author_name": ucore.get("name", "") or uleg.get("name", ""),
        "author_followers": uleg.get("followers_count", 0),
        "author_statuses_count": uleg.get("statuses_count", 0),
        "author_favourites_count": uleg.get("favourites_count", 0),
        "author_bio": uleg.get("description", ""),  # Author biography
        "author_verified": uroot.get("is_blue_verified", False),
        "author_default_profile": uleg.get("default_profile", False),
        "author_default_profile_image": uleg.get("default_profile_image", False),
        "author_profile_banner_url": uleg.get("profile_banner_url", ""),

        # Engagement metrics
        "retweet_count": legacy.get("retweet_count", 0),
        "favorite_count": legacy.get("favorite_count", 0),
        "reply_count": legacy.get("reply_count", 0),
        "bookmark_count": legacy.get("bookmark_count", 0),
        "quote_count": legacy.get("quote_count", 0),
        "view_count": safe_int(views.get("count", 0)),

        # Content entities
        "cashtags": [s["text"] for s in entities.get("symbols", [])],
        "hashtags": [h["text"] for h in entities.get("hashtags", [])],
        "mentions": [m["screen_name"] for m in entities.get("user_mentions", [])],
        "urls": [u["expanded_url"] for u in entities.get("urls", [])],

        # Media
        "has_media": "media" in entities or "extended_entities" in legacy,
        "media": [m.get("media_url_https", "") for m in entities.get("media", [])],

        # Conversation references
        "is_retweet": "retweeted_status_result" in legacy or legacy.get("full_text", "").startswith("RT @"),
        "is_reply": bool(legacy.get("in_reply_to_status_id_str")),
        "in_reply_to_screen_name": legacy.get("in_reply_to_screen_name", ""),
        "quoted_status_id": legacy.get("quoted_status_id_str", ""),

        # Edit control (if available)
        "edit_control": result.get("edit_control", {}) or None,
    }


# ── Enrichment: raw tweet → dataset row ──

def _enrich(tweet: dict, operation: str, variables: dict, coord_map: dict, source: str = "unknown", extraction_path: str = "unknown") -> dict:
    text = tweet.get("text", "")
    norm = re.sub(r"\s+", " ", text.strip().lower())
    coord_count = coord_map.get(norm, 0) if len(norm) > 30 else 0
    views = tweet.get("view_count", 0)
    likes = tweet.get("favorite_count", 0)
    rts = tweet.get("retweet_count", 0)
    replies = tweet.get("reply_count", 0)
    engagement = (likes + rts + replies) / views if views > 0 else 0.0

    return {
        # Schema & account
        "schema_version": 3,  # P17: bump on any field addition/removal
        "account_id": ACCOUNT_ID,

        # Core tweet
        "tweet_id": tweet.get("id", ""),
        "dedupe_key": hashlib.blake2b(
            f"{tweet.get('id', '')}{tweet.get('author', '')}{ACCOUNT_ID}".encode(), digest_size=8,
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
        "lang": tweet.get("lang", ""),
        "possibly_sensitive": tweet.get("possibly_sensitive", False),

        # Author (full extraction)
        "author_screen_name": tweet.get("author", ""),
        "author_display_name": tweet.get("author_name", ""),
        "author_followers_count": tweet.get("author_followers", 0),
        "author_statuses_count": tweet.get("author_statuses_count", 0),
        "author_favourites_count": tweet.get("author_favourites_count", 0),
        "author_bio": tweet.get("author_bio", ""),  # NOW EXTRACTED
        "author_is_blue_verified": tweet.get("author_verified", False),
        "author_verified": tweet.get("author_verified", False),
        "author_default_profile": tweet.get("author_default_profile", False),  # NOW EXTRACTED
        "author_default_profile_image": tweet.get("author_default_profile_image", False),  # NOW EXTRACTED
        "author_profile_banner": tweet.get("author_profile_banner_url", ""),  # NOW EXTRACTED
        "author_tier": _author_tier(tweet),

        # Engagement metrics
        "views": views,
        "likes": likes,
        "retweets": rts,
        "replies": replies,
        "quotes": tweet.get("quote_count", 0),
        "bookmarks": tweet.get("bookmark_count", 0),
        "engagement_rate": round(engagement, 6),

        # Viewer engagement (not available: passive proxy)
        "viewer_favorited": False,  # No auth in mitmproxy
        "viewer_retweeted": False,  # No auth in mitmproxy
        "viewer_bookmarked": False,  # No auth in mitmproxy

        # Entities (full extraction)
        "cashtags": tweet.get("cashtags", []),
        "hashtags": tweet.get("hashtags", []),
        "mentions": tweet.get("mentions", []),
        "urls": tweet.get("urls", []),  # NOW EXTRACTED
        "media": tweet.get("media", []),  # NOW EXTRACTED
        "has_media": tweet.get("has_media", False),  # NOW EXTRACTED

        # Conversation references
        "quoted_tweet": tweet.get("quoted_status_id", ""),  # NOW EXTRACTED (was None)
        "retweeted_tweet": None,  # Not in legacy for retweets
        "in_reply_to_screen_name": tweet.get("in_reply_to_screen_name", ""),  # NOW EXTRACTED
        "is_self_thread": False,  # Would need reply chain analysis

        # Enrichment
        "signal_score": _signal_score(text, tweet),
        "narratives": _narratives(text),
        "is_coordinated": coord_count >= 3,
        "coordination_count": coord_count,
        "sampling_bias": "proxy-passive",
        "source": source,
        "extraction_path": extraction_path,
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

    METRICS_FLUSH_INTERVAL = 300  # seconds between metric flushes

    def __init__(self):
        CAPTURE_DIR.mkdir(parents=True, exist_ok=True)
        DATASET_PATH.parent.mkdir(parents=True, exist_ok=True)
        self._dedup = _DedupeCache()
        self._dedup.load(DATASET_PATH)
        self._stats = {
            "responses_seen": 0,       # All GraphQL responses
            "ops_matched": 0,          # Matched CAPTURE_OPS
            "non_200": 0,              # Non-200 responses skipped
            "schema_miss": 0,          # Extraction path not found
            "tweets_extracted": 0,     # Raw tweets from GraphQL
            "tweets_written": 0,       # After dedup, written to dataset
            "tweets_deduped": 0,       # Rejected by dedup
        }
        self._boot_ts = datetime.now(timezone.utc).isoformat()
        self._last_flush = datetime.now(timezone.utc).timestamp()
        self._metrics_path = DATASET_PATH.parent / "proxy_metrics.jsonl"
        self._hub_url = os.environ.get("BROWSER_HUB_URL", "http://127.0.0.1:40770")

        # Load account config for read-only enforcement
        self._account_config = _load_account_config()
        self._account_role = self._get_account_role()

        logger.info("x-proxy loaded — %d existing tweets, dataset: %s, hub: %s, account: %s (role: %s)",
                     len(self._dedup._seen), DATASET_PATH, self._hub_url, ACCOUNT_ID, self._account_role)
        self._flush_metrics("boot")

    def _get_account_role(self) -> str:
        """Get the current account's role from accounts.toml."""
        try:
            accounts = self._account_config.get("accounts", {})
            account = accounts.get(ACCOUNT_ID, {})
            return account.get("role", "primary")
        except Exception:
            return "primary"

    def request(self, flow: http.HTTPFlow) -> None:
        """Intercept requests; block write operations for read_only accounts."""
        if self._account_role != "read_only":
            return  # No enforcement for non-read_only accounts

        url = flow.request.url
        if "/i/api/graphql/" not in url:
            return  # Not a GraphQL request

        # Block POST/PUT/DELETE on GraphQL endpoint for read-only accounts
        if flow.request.method in ("POST", "PUT", "DELETE"):
            logger.warning("Blocked action on read-only account: %s %s", flow.request.method, url)
            flow.response = http.Response.make(
                403,
                b"Forbidden: Read-only account cannot perform write operations",
                {"Content-Type": "text/plain"},
            )

    def _flush_metrics(self, event: str = "periodic") -> None:
        """Persist stats to JSONL — the only way to trace proxy activity across restarts."""
        row = {
            "ts": datetime.now(timezone.utc).isoformat(),
            "event": event,
            "boot_ts": self._boot_ts,
            **self._stats,
        }
        try:
            with open(self._metrics_path, "a") as f:
                f.write(json.dumps(row) + "\n")
        except OSError as e:
            logger.warning("metrics flush failed: %s", e)
        self._last_flush = datetime.now(timezone.utc).timestamp()

    def _maybe_flush(self) -> None:
        now = datetime.now(timezone.utc).timestamp()
        if now - self._last_flush >= self.METRICS_FLUSH_INTERVAL:
            self._flush_metrics("periodic")

    def response(self, flow: http.HTTPFlow) -> None:
        url = flow.request.url
        if "/i/api/graphql/" not in url:
            return

        self._stats["responses_seen"] += 1
        op_name = self._extract_op(url)
        if not op_name or op_name not in CAPTURE_OPS:
            self._maybe_flush()
            return
        self._stats["ops_matched"] += 1
        if flow.response is None or flow.response.status_code != 200:
            self._stats["non_200"] += 1
            self._maybe_flush()
            return

        try:
            data = json.loads(flow.response.content)
        except (json.JSONDecodeError, TypeError):
            return

        variables = self._extract_vars(url)
        source = self._get_attribution(flow.request.url, flow.request.timestamp_start)
        ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")

        # Raw capture
        (CAPTURE_DIR / f"{ts}_{op_name}.json").write_text(json.dumps({
            "operation": op_name,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "variables": variables,
            "response": data,
        }, indent=2, ensure_ascii=False))
        # ops_matched already incremented above; this is the raw JSON capture point

        # Extract + enrich
        raw_tweets, extraction_path = _extract_tweets(data)
        if not raw_tweets:
            if extraction_path == "schema_miss":
                self._stats["schema_miss"] += 1
            self._maybe_flush()
            return
        self._stats["tweets_extracted"] += len(raw_tweets)

        coord_map = _coordination(raw_tweets)
        new = []
        for t in raw_tweets:
            enriched = _enrich(t, op_name, variables, coord_map, source, extraction_path)
            if self._dedup.is_new(enriched["dedupe_key"]):
                new.append(enriched)
            else:
                self._stats["tweets_deduped"] += 1

        if new:
            with open(DATASET_PATH, "a") as f:
                for row in new:
                    f.write(json.dumps(row, ensure_ascii=False) + "\n")
            self._stats["tweets_written"] += len(new)

        # Log search results for click-to-tweet linkage (Phase 1A)
        tweet_ids = [row["tweet_id"] for row in new if row.get("tweet_id")]
        if tweet_ids:
            search_result = {
                "operation": op_name,
                "query": variables.get("rawQuery", "") or op_name.lower(),
                "returned_tweet_ids": tweet_ids,
                "tweet_count": len(tweet_ids),
                "timestamp": datetime.now(timezone.utc).isoformat(),
            }
            try:
                with open(SEARCH_RESULTS_LOG, "a") as f:
                    f.write(json.dumps(search_result) + "\n")
            except Exception as e:
                logger.warning("Failed to log search results: %s", e)

        logger.info("x-proxy: %s +%d new (%d total, %d dedup)",
                     op_name, len(new), self._stats["tweets_written"], self._stats["tweets_deduped"])
        self._maybe_flush()

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

    def _get_attribution(self, url: str, timestamp: float) -> str:
        """Query Hub for source attribution. Returns 'unknown' on failure."""
        try:
            import urllib.request
            from urllib.parse import urlencode
            req_url = f"{self._hub_url}/attribution?{urlencode({'url': url, 'ts': timestamp})}"
            with urllib.request.urlopen(req_url, timeout=0.5) as resp:
                data = json.loads(resp.read())
                return data.get("source", "unknown")
        except Exception:
            return "unknown"


addons = [XProxy()]
