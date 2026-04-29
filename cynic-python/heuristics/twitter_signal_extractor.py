"""Extract TwitterSignals from Hermes captured tweets.

Maps raw tweet JSON → TwitterSignals dataclass.
Computes engagement metrics, red flags, sentiment (placeholder for now).
"""

from dataclasses import dataclass
from typing import Optional
import json
import re
from pathlib import Path


@dataclass
class TwitterSignals:
    """Social signal metrics extracted from tweet."""
    # Community health
    follower_count: int
    tweet_count: int
    engagement_rate: float

    # Sentiment (extracted/inferred)
    positive_pct: float = 0.0
    negative_pct: float = 0.0
    neutral_pct: float = 0.0

    # Red flags
    has_rug_allegations: bool = False
    has_creator_criticism: bool = False
    has_buy_pressure_spam: bool = False
    liquidity_discussion_active: bool = False

    # Recent activity
    tweets_last_7d: int = 0
    tweet_velocity: float = 0.0


class TwitterSignalExtractor:
    """Extract TwitterSignals from tweet JSON."""

    # Red flag keywords
    RUG_KEYWORDS = {
        "rug", "rugpull", "rugged", "rug-pulled", "scam", "scammed",
        "honeypot", "exit scam", "slow rug", "fee scam", "locked liquidity"
    }

    SPAM_KEYWORDS = {
        "buy now", "buy the dip", "get rich quick", "moon", "pump it",
        "guaranteed", "can't miss", "lambo"
    }

    CREATOR_CRITICISM_KEYWORDS = {
        "team is", "team left", "abandonment", "dev dump", "insider",
        "lock-up", "vesting"
    }

    LP_KEYWORDS = {
        "liquidity", "lp", "uniswap", "raydium", "pair", "pool",
        "burn lp", "locked lp", "provide liquidity"
    }

    def extract(self, tweet: dict) -> TwitterSignals:
        """Extract TwitterSignals from raw tweet JSON."""
        text = tweet.get("text", "").lower()
        author_data = tweet

        # Community health (from author metrics)
        follower_count = author_data.get("author_followers_count", 0)
        tweet_count = author_data.get("author_statuses_count", 0)
        engagement_rate = float(author_data.get("engagement_rate", 0.0))

        # Red flags (keyword-based, basic)
        has_rug_allegations = any(kw in text for kw in self.RUG_KEYWORDS)
        has_creator_criticism = any(kw in text for kw in self.CREATOR_CRITICISM_KEYWORDS)
        has_buy_pressure_spam = any(kw in text for kw in self.SPAM_KEYWORDS)
        liquidity_discussion_active = any(kw in text for kw in self.LP_KEYWORDS)

        # Sentiment (placeholder: infer from text length and punctuation)
        # TODO: integrate with actual sentiment model
        positive_pct, negative_pct, neutral_pct = self._infer_sentiment(text, has_rug_allegations)

        # Recent activity (placeholder: assume activity based on follower/tweet ratio)
        tweets_last_7d = max(1, tweet_count // 14)  # Rough estimate
        tweet_velocity = max(0.1, tweet_count / 365.0) if tweet_count > 0 else 0.0

        return TwitterSignals(
            follower_count=follower_count,
            tweet_count=tweet_count,
            engagement_rate=engagement_rate,
            positive_pct=positive_pct,
            negative_pct=negative_pct,
            neutral_pct=neutral_pct,
            has_rug_allegations=has_rug_allegations,
            has_creator_criticism=has_creator_criticism,
            has_buy_pressure_spam=has_buy_pressure_spam,
            liquidity_discussion_active=liquidity_discussion_active,
            tweets_last_7d=tweets_last_7d,
            tweet_velocity=tweet_velocity,
        )

    def _infer_sentiment(self, text: str, has_rug: bool) -> tuple[float, float, float]:
        """Infer sentiment from text signals (placeholder).

        TODO: Replace with actual sentiment model (HuggingFace or Gemini).
        For now: if rug allegations, negative. Otherwise, neutral-leaning-positive.
        """
        if has_rug:
            return (0.1, 0.8, 0.1)

        # Heuristic: exclamation marks = positive, question marks = neutral
        exclamation_count = text.count("!")
        question_count = text.count("?")

        if exclamation_count > 2:
            return (0.7, 0.1, 0.2)
        elif question_count > 0:
            return (0.3, 0.2, 0.5)
        else:
            return (0.4, 0.2, 0.4)


def extract_signals_from_dataset(
    dataset_path: Path,
    limit: Optional[int] = None,
) -> list[tuple[str, dict, TwitterSignals]]:
    """Load tweets and extract signals.

    Returns: [(tweet_id, tweet_json, signals), ...]
    """
    extractor = TwitterSignalExtractor()
    results = []

    with open(dataset_path, 'r') as f:
        for i, line in enumerate(f):
            if limit and i >= limit:
                break

            if not line.strip():
                continue

            try:
                tweet = json.loads(line)
                tweet_id = tweet.get("tweet_id", "unknown")
                signals = extractor.extract(tweet)
                results.append((tweet_id, tweet, signals))
            except json.JSONDecodeError:
                continue

    return results


if __name__ == "__main__":
    dataset_path = Path.home() / ".cynic/organs/hermes/x/dataset.jsonl"

    print(f"Loading tweets from {dataset_path}...")
    tweets_and_signals = extract_signals_from_dataset(dataset_path, limit=100)

    print(f"\nExtracted signals from {len(tweets_and_signals)} tweets:")
    print(f"{'Tweet ID':<20} {'Followers':<12} {'Engagement':<12} {'Rug?':<8} {'LP?':<8}")
    print("-" * 60)

    for tweet_id, tweet, signals in tweets_and_signals[:20]:
        print(f"{tweet_id:<20} {signals.follower_count:<12} {signals.engagement_rate:.3f}       {str(signals.has_rug_allegations):<8} {str(signals.liquidity_discussion_active):<8}")
