#!/usr/bin/env python3
"""
Extract precise tweet data from raw Organ X captures.

Converts raw X API responses (512 JSON files in captures/) into structured
tweet objects with full metadata: author profiles, engagement metrics, timestamps.

Removes narrative bias by working from raw signals instead of hand-classified tags.

Usage:
    python3 extract_organ_x_tweets.py
    python3 extract_organ_x_tweets.py --output tweets_extracted.jsonl
"""

import json
import sys
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, asdict
from datetime import datetime
from collections import defaultdict


@dataclass
class AuthorProfile:
    """Twitter user metadata."""
    screen_name: str
    name: str
    followers_count: int
    verified: bool
    created_at: str  # ISO 8601
    description: Optional[str] = None
    is_bot: Optional[bool] = None  # Inferred: blue_verified, high followers, low content variety


@dataclass
class PublicMetrics:
    """Tweet engagement metrics."""
    like_count: int
    retweet_count: int
    reply_count: int
    quote_count: int


@dataclass
class Tweet:
    """Complete tweet with metadata."""
    id: str
    created_at: str  # ISO 8601
    text: str
    author: AuthorProfile
    public_metrics: PublicMetrics
    in_reply_to_user: Optional[str] = None
    in_reply_to_tweet_id: Optional[str] = None
    is_quote: bool = False
    quote_of_tweet_id: Optional[str] = None


def extract_tweets_from_captures(captures_dir: Path) -> List[Tweet]:
    """Walk through all capture files and extract tweets."""
    tweets = []
    seen_ids = set()

    for i, capture_file in enumerate(sorted(captures_dir.glob("*.json"))):
        if i % 50 == 0:
            print(f"  Processing {i}/{len(list(captures_dir.glob('*.json')))} captures...", file=sys.stderr)

        try:
            with open(capture_file) as f:
                data = json.load(f)
        except:
            continue

        operation = data.get("operation", "unknown")
        response = data.get("response", {})

        # Extract based on operation type
        if operation == "TweetDetail":
            _extract_from_tweet_detail(response, tweets, seen_ids)
        elif operation == "HomeTimeline":
            _extract_from_timeline(response, tweets, seen_ids)
        elif operation == "SearchTimeline":
            _extract_from_timeline(response, tweets, seen_ids)
        elif operation == "UserTweets":
            _extract_from_timeline(response, tweets, seen_ids)

    print(f"✓ Extracted {len(tweets)} unique tweets", file=sys.stderr)
    return tweets


def _extract_from_tweet_detail(response: Dict, tweets: List[Tweet], seen_ids: set) -> None:
    """Extract tweets from TweetDetail response."""
    instructions = response.get("data", {}).get("threaded_conversation_with_injections_v2", {}).get("instructions", [])

    for instr in instructions:
        if instr.get("type") == "TimelineAddEntries":
            entries = instr.get("entries", [])
            for entry in entries:
                content = entry.get("content", {})
                if content.get("__typename") == "TimelineTimelineItem":
                    tweet_obj = _extract_tweet_object(content)
                    if tweet_obj and tweet_obj.id not in seen_ids:
                        tweets.append(tweet_obj)
                        seen_ids.add(tweet_obj.id)


def _extract_from_timeline(response: Dict, tweets: List[Tweet], seen_ids: set) -> None:
    """Extract tweets from Timeline responses (Home, Search, UserTweets)."""
    data = response.get("data", {})

    # Try different timeline structures
    instructions = (
        data.get("home", {}).get("home_timeline_urt", {}).get("instructions", [])
        or data.get("search_by_raw_query", {}).get("search_timeline", {}).get("timeline", {}).get("instructions", [])
        or data.get("user", {}).get("result", {}).get("timeline_v2", {}).get("timeline", {}).get("instructions", [])
    )

    for instr in instructions:
        if instr.get("type") not in ["TimelineAddEntries", "TimelineShowMoreItem"]:
            continue

        entries = instr.get("entries", [])
        for entry in entries:
            content = entry.get("content", {})
            if content.get("__typename") in ["TimelineTimelineItem", "TimelineTweet"]:
                tweet_obj = _extract_tweet_object(content)
                if tweet_obj and tweet_obj.id not in seen_ids:
                    tweets.append(tweet_obj)
                    seen_ids.add(tweet_obj.id)


def _extract_tweet_object(content: Dict) -> Optional[Tweet]:
    """Extract Tweet dataclass from content object."""
    item_content = content.get("itemContent", {})
    if not item_content:
        return None

    # Navigate to tweet result
    tweet_result = item_content.get("tweet_results", {}).get("result", {})
    if not tweet_result:
        return None

    # Tweet metadata
    tweet_id = tweet_result.get("rest_id")
    created_at = tweet_result.get("core", {}).get("created_at")
    text = tweet_result.get("legacy", {}).get("full_text", "")

    if not tweet_id or not created_at:
        return None

    # Author metadata
    user_result = tweet_result.get("core", {}).get("user_results", {}).get("result", {})
    author = _extract_author(user_result)
    if not author:
        return None

    # Engagement metrics
    legacy = tweet_result.get("legacy", {})
    metrics = PublicMetrics(
        like_count=legacy.get("favorite_count", 0),
        retweet_count=legacy.get("retweet_count", 0),
        reply_count=legacy.get("reply_count", 0),
        quote_count=legacy.get("quote_count", 0),
    )

    # Reply/quote information
    in_reply_to_user = legacy.get("in_reply_to_screen_name")
    in_reply_to_tweet_id = legacy.get("in_reply_to_status_id_str")
    is_quote = bool(legacy.get("quoted_status_id_str"))
    quote_of_tweet_id = legacy.get("quoted_status_id_str")

    return Tweet(
        id=tweet_id,
        created_at=created_at,
        text=text,
        author=author,
        public_metrics=metrics,
        in_reply_to_user=in_reply_to_user,
        in_reply_to_tweet_id=in_reply_to_tweet_id,
        is_quote=is_quote,
        quote_of_tweet_id=quote_of_tweet_id,
    )


def _extract_author(user_result: Dict) -> Optional[AuthorProfile]:
    """Extract author metadata from user result."""
    core = user_result.get("core", {})
    legacy = user_result.get("legacy", {})

    screen_name = core.get("screen_name")
    if not screen_name:
        return None

    # Infer bot status (heuristic)
    is_bot = None
    if legacy.get("is_blue_verified") and legacy.get("followers_count", 0) > 100000:
        is_bot = False  # Verified + large following = likely real
    elif legacy.get("followers_count", 0) > 1000000 and legacy.get("statuses_count", 0) < 100:
        is_bot = True  # Massive followers, low tweets = likely bot

    return AuthorProfile(
        screen_name=screen_name,
        name=core.get("name", ""),
        followers_count=legacy.get("followers_count", 0),
        verified=legacy.get("is_blue_verified", False),
        created_at=legacy.get("created_at", ""),
        description=legacy.get("description"),
        is_bot=is_bot,
    )


def compute_sentiment_scores(tweets: List[Tweet]) -> Dict[str, float]:
    """Compute sentiment indicators from raw metrics (no narrative bias).

    Returns dict with sentiment lifting scores based on engagement patterns.
    """
    if not tweets:
        return {}

    # Aggregate metrics
    avg_likes = sum(t.public_metrics.like_count for t in tweets) / len(tweets) if tweets else 0
    avg_retweets = sum(t.public_metrics.retweet_count for t in tweets) / len(tweets) if tweets else 0
    avg_replies = sum(t.public_metrics.reply_count for t in tweets) / len(tweets) if tweets else 0

    # Engagement ratio indicators
    high_engagement_tweets = sum(
        1
        for t in tweets
        if (t.public_metrics.like_count + t.public_metrics.retweet_count)
        / max(1, t.public_metrics.reply_count)
        > 10
    )
    engagement_ratio = high_engagement_tweets / len(tweets) if tweets else 0

    # Author quality
    verified_authors = sum(1 for t in tweets if t.author.verified) / len(tweets) if tweets else 0
    high_follower_authors = sum(1 for t in tweets if t.author.followers_count > 10000) / len(tweets) if tweets else 0

    return {
        "avg_likes": avg_likes,
        "avg_retweets": avg_retweets,
        "avg_replies": avg_replies,
        "high_engagement_ratio": engagement_ratio,
        "verified_authors": verified_authors,
        "high_follower_authors": high_follower_authors,
    }


def save_tweets_jsonl(tweets: List[Tweet], output_file: Path) -> None:
    """Save tweets as JSONL (one per line, raw dict format)."""
    with open(output_file, "w") as f:
        for tweet in tweets:
            # Serialize to dict
            tweet_dict = {
                "id": tweet.id,
                "created_at": tweet.created_at,
                "text": tweet.text,
                "author": {
                    "screen_name": tweet.author.screen_name,
                    "name": tweet.author.name,
                    "followers_count": tweet.author.followers_count,
                    "verified": tweet.author.verified,
                    "created_at": tweet.author.created_at,
                    "description": tweet.author.description,
                    "is_bot": tweet.author.is_bot,
                },
                "public_metrics": {
                    "like_count": tweet.public_metrics.like_count,
                    "retweet_count": tweet.public_metrics.retweet_count,
                    "reply_count": tweet.public_metrics.reply_count,
                    "quote_count": tweet.public_metrics.quote_count,
                },
                "in_reply_to_user": tweet.in_reply_to_user,
                "in_reply_to_tweet_id": tweet.in_reply_to_tweet_id,
                "is_quote": tweet.is_quote,
                "quote_of_tweet_id": tweet.quote_of_tweet_id,
            }
            f.write(json.dumps(tweet_dict) + "\n")


def main():
    captures_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "captures"

    if not captures_dir.exists():
        print(f"❌ Captures directory not found: {captures_dir}")
        return 1

    print(f"📊 Extracting tweets from {len(list(captures_dir.glob('*.json')))} captures...")
    tweets = extract_tweets_from_captures(captures_dir)

    if not tweets:
        print("❌ No tweets extracted")
        return 1

    # Group by mentioned tokens
    token_mentions = defaultdict(list)
    for tweet in tweets:
        import re

        symbols = re.findall(r"\$(\w+)", tweet.text)
        for symbol in symbols:
            token_mentions[symbol].append(tweet)

    print(f"\n✓ Extracted {len(tweets)} tweets mentioning {len(token_mentions)} tokens")

    # Compute sentiment scores
    scores = compute_sentiment_scores(tweets)
    print(f"\nSignal metrics (raw):")
    for metric, value in scores.items():
        print(f"  {metric}: {value:.2f}")

    # Save
    output_file = Path("cynic-python/organ_x_tweets_extracted.jsonl")
    save_tweets_jsonl(tweets, output_file)
    print(f"\n✓ Tweets saved to: {output_file}")

    # Summary by token
    summary_file = Path("cynic-python/organ_x_token_mentions_summary.json")
    summary = {
        symbol: {
            "mention_count": len(mentions),
            "avg_engagement": (
                sum(m.public_metrics.like_count + m.public_metrics.retweet_count for m in mentions)
                / len(mentions)
            )
            if mentions
            else 0,
            "top_authors": list(set(m.author.screen_name for m in mentions))[:5],
        }
        for symbol, mentions in token_mentions.items()
    }
    with open(summary_file, "w") as f:
        json.dump(summary, f, indent=2)
    print(f"✓ Token summary saved to: {summary_file}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
