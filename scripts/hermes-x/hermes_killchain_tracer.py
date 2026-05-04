#!/usr/bin/env python3
"""
CYNIC Hermes Kill-Chain Tracer — Map behavior_log clicks to X.com captures

Maps the kill-chain: behavior_log click → visible X.com tweets → inferred engagement

Input:
  - behavior_log.jsonl: 29K clicks with timestamps
  - captures/*.json: 689 X.com GraphQL responses with tweets
  - learned_weights.json: keyword weights, temporal peaks, depth threshold

Output:
  - killchain.jsonl: For each click, which tweets were visible, ranked by predicted engagement

Usage:
    python3 hermes_killchain_tracer.py --organ-dir ~/.cynic/organs/hermes/x
                                       --behavior-log ~/.cynic/organs/hermes/behavior/behavior_log.jsonl
                                       --max-window-sec 5

Falsifiable hypothesis:
  "Clicks correlate with high-signal tweets (score > 0.5) visible at click timestamp"
  Metric: Coverage (% clicks matched to captures), Precision (signal score of matched tweets)
"""

__version__ = "0.1.0"

import json
import logging
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional, List, Dict, Any
from dataclasses import dataclass, asdict
from collections import defaultdict

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("killchain-tracer")


@dataclass
class ClickEvent:
    """A single click from behavior_log."""
    ts: str  # ISO timestamp
    x: int
    y: int
    window_name: str
    dt: datetime = None  # Parsed timestamp


@dataclass
class CapturedTweet:
    """A tweet extracted from a capture file."""
    tweet_id: str
    text: str
    author_handle: str
    author_name: str
    author_followers: int
    author_verified: bool
    engagement_likes: int
    engagement_retweets: int
    engagement_replies: int
    engagement_bookmarks: int
    signal_score: float  # From x_proxy.py enrichment


@dataclass
class KillChainLink:
    """Result of kill-chain analysis for a single click."""
    click_ts: str
    click_x: int
    click_y: int
    window_name: str

    # Capture window (±5 sec)
    capture_count: int
    matched_captures: List[str]  # filenames

    # Tweets visible in that window
    visible_tweets: int
    top_tweets: List[Dict[str, Any]]  # sorted by predicted_engagement, top 5

    # Metrics
    coverage_pct: float  # % of visible tweets with valid enrichment
    max_signal_score: float
    avg_signal_score: float


class KillChainTracer:
    """Trace kill-chains from clicks to tweets."""

    def __init__(self, organ_dir: Path, behavior_log: Path, max_window_sec: int = 5):
        self.organ_dir = Path(organ_dir)
        self.behavior_log = Path(behavior_log)
        self.max_window_sec = max_window_sec
        self.captures_dir = self.organ_dir / "captures"
        self.dataset_path = self.organ_dir / "dataset.jsonl"

        self.clicks: List[ClickEvent] = []
        self.capture_files: Dict[str, Path] = {}  # ts_str -> filepath
        self.learned_weights: Dict[str, Any] = {}
        self.tweet_enrichments: Dict[str, dict] = {}  # tweet_id -> enrichment from dataset.jsonl

    def load_clicks(self) -> bool:
        """Load clicks from behavior_log.jsonl."""
        if not self.behavior_log.exists():
            logger.error("behavior_log not found: %s", self.behavior_log)
            return False

        try:
            with open(self.behavior_log) as f:
                for line in f:
                    try:
                        event = json.loads(line)
                        if event.get("type") == "click":
                            try:
                                dt = datetime.fromisoformat(event["ts"].replace("Z", "+00:00"))
                                click = ClickEvent(
                                    ts=event["ts"],
                                    x=event.get("x", 0),
                                    y=event.get("y", 0),
                                    window_name=event.get("window_name", "unknown"),
                                    dt=dt
                                )
                                self.clicks.append(click)
                            except Exception as e:
                                logger.debug("Failed to parse click: %s", str(e)[:80])
                    except json.JSONDecodeError:
                        pass
        except Exception as e:
            logger.error("Failed to load clicks: %s", e)
            return False

        logger.info("✓ Loaded %d clicks from behavior_log", len(self.clicks))
        return len(self.clicks) > 0

    def load_captures(self) -> bool:
        """Index capture files by timestamp."""
        if not self.captures_dir.exists():
            logger.error("Captures directory not found: %s", self.captures_dir)
            return False

        for filepath in self.captures_dir.glob("*.json"):
            # Extract timestamp from filename: YYYYMMDD_HHMMSS_Operation.json
            parts = filepath.stem.split("_")
            if len(parts) >= 2:
                try:
                    date_part = parts[0]  # YYYYMMDD
                    time_part = parts[1]  # HHMMSS
                    ts_str = f"{date_part[:4]}-{date_part[4:6]}-{date_part[6:8]}T{time_part[:2]}:{time_part[2:4]}:{time_part[4:6]}Z"
                    self.capture_files[ts_str] = filepath
                except Exception:
                    pass

        logger.info("✓ Indexed %d capture files", len(self.capture_files))
        return len(self.capture_files) > 0

    def load_enrichments(self) -> bool:
        """Load pre-computed enrichments from dataset.jsonl (x_proxy output).

        This replaces naive signal_score computation with x_proxy's sophisticated
        analysis including author_tier, narratives, coordination_count.
        """
        if not self.dataset_path.exists():
            logger.warning("dataset.jsonl not found; will recompute signal scores from scratch")
            return False

        try:
            deduped = {}  # tweet_id -> enrichment (last occurrence wins)
            count = 0
            with open(self.dataset_path) as f:
                for line in f:
                    try:
                        row = json.loads(line)
                        tweet_id = row.get("tweet_id", "")
                        if tweet_id:
                            deduped[tweet_id] = {
                                "signal_score": row.get("signal_score", 0),
                                "narratives": row.get("narratives", []),
                                "author_tier": row.get("author_tier", "unknown"),
                                "coordination_count": row.get("coordination_count", 0),
                            }
                            count += 1
                    except json.JSONDecodeError:
                        pass

            self.tweet_enrichments = deduped
            logger.info("✓ Loaded %d enrichments from dataset.jsonl (deduped to %d unique tweets)",
                       count, len(deduped))
            return len(deduped) > 0
        except Exception as e:
            logger.error("Failed to load enrichments: %s", e)
            return False

    def load_learned_weights(self) -> bool:
        """Load learned_weights.json."""
        weights_file = self.organ_dir / "learned_weights.json"
        if not weights_file.exists():
            logger.warning("learned_weights.json not found; using fallback")
            self.learned_weights = {
                "keyword_weights": {
                    "code": 0.147, "architecture": 0.103, "python": 0.088,
                    "rust": 0.074, "api": 0.059, "algorithm": 0.052
                },
                "temporal_peaks": {str(h): 0.038 for h in range(24)},  # Default
                "depth_threshold": 11.1
            }
            return True

        try:
            with open(weights_file) as f:
                self.learned_weights = json.load(f)
            logger.info("✓ Loaded learned weights")
            return True
        except Exception as e:
            logger.error("Failed to load learned weights: %s", e)
            return False

    def extract_tweets_from_capture(self, filepath: Path) -> List[CapturedTweet]:
        """Extract all tweets from a single capture file."""
        tweets = []

        try:
            with open(filepath) as f:
                data = json.load(f)
        except Exception as e:
            logger.debug("Failed to load capture %s: %s", filepath.name, str(e)[:80])
            return tweets

        # Navigate to tweets (structure varies by operation type)
        operation = data.get("operation", "")
        response = data.get("response", {})

        try:
            if operation == "HomeTimeline":
                timeline = response.get("data", {}).get("home", {}).get("home_timeline_urt", {}).get("instructions", [])
            elif operation == "UserTweets":
                timeline = response.get("data", {}).get("user", {}).get("result", {}).get("timeline_v2", {}).get("timeline", {}).get("instructions", [])
            elif operation == "SearchTimeline":
                timeline = response.get("data", {}).get("search_by_raw_query", {}).get("search_timeline", {}).get("timeline", {}).get("instructions", [])
            elif operation == "TweetDetail":
                # TweetDetail has conversation thread
                timeline = response.get("data", {}).get("threaded_conversation_with_injections_v2", {}).get("instructions", [])
            elif operation in ["Likes", "Bookmarks"]:
                timeline = response.get("data", {}).get("user", {}).get("result", {}).get("timeline_v2", {}).get("timeline", {}).get("instructions", [])
            else:
                timeline = []

            if not timeline:
                timeline = []

            # Extract tweets from timeline instructions
            for instruction in (timeline if isinstance(timeline, list) else []):
                entries = instruction.get("entries", []) if isinstance(instruction, dict) else []

                for entry in entries:
                    if isinstance(entry, dict) and "content" in entry:
                        content = entry["content"]
                        item_content = content.get("itemContent", {})

                        # Extract tweet from TimelineTweet or other types
                        tweet_results = item_content.get("tweet_results", {})
                        result = tweet_results.get("result", {})

                        if isinstance(result, dict) and "legacy" in result:
                            try:
                                tweet = self._parse_tweet(result)
                                if tweet:
                                    tweets.append(tweet)
                            except Exception as e:
                                logger.debug("Failed to parse tweet: %s", str(e)[:80])

        except Exception as e:
            logger.debug("Error extracting tweets from %s: %s", filepath.name, str(e)[:80])

        return tweets

    def _parse_tweet(self, tweet_data: dict) -> Optional[CapturedTweet]:
        """Parse a single tweet from X.com GraphQL response."""
        try:
            legacy = tweet_data.get("legacy", {})
            user_info = tweet_data.get("core", {}).get("user_results", {}).get("result", {})
            user_legacy = user_info.get("legacy", {})

            tweet_id = legacy.get("id_str", "")
            text = legacy.get("full_text", "")
            author_handle = user_legacy.get("screen_name", "unknown")
            author_name = user_legacy.get("name", "")
            author_followers = user_legacy.get("followers_count", 0)
            author_verified = user_legacy.get("verified", False)

            engagement_likes = legacy.get("favorite_count", 0)
            engagement_retweets = legacy.get("retweet_count", 0)
            engagement_replies = legacy.get("reply_count", 0)
            engagement_bookmarks = legacy.get("bookmark_count", 0)

            # Use pre-computed signal score from x_proxy enrichment if available,
            # otherwise fall back to local computation
            if tweet_id in self.tweet_enrichments:
                signal_score = self.tweet_enrichments[tweet_id]["signal_score"]
            else:
                signal_score = self._compute_signal_score(text, author_followers, engagement_likes)

            if not tweet_id:
                return None

            return CapturedTweet(
                tweet_id=tweet_id,
                text=text,
                author_handle=author_handle,
                author_name=author_name,
                author_followers=author_followers,
                author_verified=author_verified,
                engagement_likes=engagement_likes,
                engagement_retweets=engagement_retweets,
                engagement_replies=engagement_replies,
                engagement_bookmarks=engagement_bookmarks,
                signal_score=signal_score
            )
        except Exception as e:
            logger.debug("Failed to parse tweet fields: %s", str(e)[:80])
            return None

    def _compute_signal_score(self, text: str, followers: int, likes: int) -> float:
        """
        Compute signal score using learned patterns.
        Range: -5 (spam) to +7 (high quality)
        """
        score = 0.5
        text_lower = text.lower()

        # Spam detection (-3 to -1)
        spam_keywords = ["100x", "moon", "airdrop", "giveaway", "follow", "retweet"]
        spam_count = sum(1 for kw in spam_keywords if kw in text_lower)
        if spam_count > 0:
            score -= min(3, spam_count * 0.5)

        # Quality signals (+2)
        quality_keywords = ["on-chain", "smart contract", "analysis", "research", "data"]
        quality_count = sum(1 for kw in quality_keywords if kw in text_lower)
        if quality_count > 0:
            score += min(2, quality_count * 0.4)

        # Author tier
        if followers > 100000:
            score += 2  # whale
        elif followers > 10000:
            score += 1
        elif followers < 100:
            score -= 1  # likely bot

        # Engagement ratio
        words = len(text.split())
        if words > 0:
            engagement_ratio = likes / max(1, words)
            if engagement_ratio > 0.05:
                score += 2
            elif engagement_ratio > 0.02:
                score += 1

        # Absolute engagement
        if likes > 1000:
            score += 1

        return max(-5, min(7, score))

    def find_captures_for_click(self, click: ClickEvent) -> List[Path]:
        """Find capture files within ±max_window_sec of click timestamp."""
        window_start = click.dt - timedelta(seconds=self.max_window_sec)
        window_end = click.dt + timedelta(seconds=self.max_window_sec)

        matching = []
        for ts_str, filepath in self.capture_files.items():
            try:
                capture_dt = datetime.fromisoformat(ts_str.replace("Z", "+00:00"))
                if window_start <= capture_dt <= window_end:
                    matching.append(filepath)
            except Exception:
                pass

        return sorted(matching)

    def analyze_click(self, click: ClickEvent) -> KillChainLink:
        """Analyze a single click: find visible tweets and score them."""
        captures = self.find_captures_for_click(click)

        # Extract all tweets from matching captures
        all_tweets: List[CapturedTweet] = []
        for cap_file in captures:
            tweets = self.extract_tweets_from_capture(cap_file)
            all_tweets.extend(tweets)

        # Score tweets using learned weights
        scored_tweets = []
        for tweet in all_tweets:
            predicted_engagement = self._predict_engagement(tweet)
            scored_tweets.append({
                "tweet_id": tweet.tweet_id,
                "text": tweet.text[:100],
                "author": tweet.author_handle,
                "followers": tweet.author_followers,
                "signal_score": tweet.signal_score,
                "engagement_likes": tweet.engagement_likes,
                "predicted_engagement": predicted_engagement,
            })

        # Sort by predicted engagement
        scored_tweets.sort(key=lambda x: x["predicted_engagement"], reverse=True)

        # Compute metrics
        coverage_pct = len([t for t in all_tweets if t.signal_score is not None]) / max(1, len(all_tweets)) * 100
        signal_scores = [t.signal_score for t in all_tweets]
        max_signal = max(signal_scores) if signal_scores else 0
        avg_signal = sum(signal_scores) / len(signal_scores) if signal_scores else 0

        return KillChainLink(
            click_ts=click.ts,
            click_x=click.x,
            click_y=click.y,
            window_name=click.window_name,
            capture_count=len(captures),
            matched_captures=[cap.name for cap in captures],
            visible_tweets=len(all_tweets),
            top_tweets=scored_tweets[:5],
            coverage_pct=coverage_pct,
            max_signal_score=max_signal,
            avg_signal_score=avg_signal,
        )

    def _predict_engagement(self, tweet: CapturedTweet) -> float:
        """
        Predict likelihood T. will engage with this tweet.
        Combines: signal_score + keyword_match + author_tier + temporal
        Range: 0.0 (unlikely) to 1.0 (very likely)
        """
        score = 0.5
        text_lower = tweet.text.lower()

        # Keyword signals from learned_weights
        keyword_weights = self.learned_weights.get("keyword_weights", {})
        for keyword, weight in keyword_weights.items():
            if keyword in text_lower:
                score += weight * 0.5  # Scale to 0.0-1.0 range

        # Author tier: T. engages more with technical authors
        if "dev" in tweet.author_handle.lower() or "engineer" in tweet.author_name.lower():
            score += 0.2

        if tweet.author_followers > 100000:
            score += 0.15  # Whale

        # Signal score contribution
        # Normalize from [-5, 7] to [0, 1]
        normalized_signal = (tweet.signal_score + 5) / 12
        score += normalized_signal * 0.3

        # Engagement boost
        if tweet.engagement_likes > 500:
            score += 0.1

        return min(1.0, max(0.0, score))

    def trace_all(self) -> List[KillChainLink]:
        """Trace all clicks."""
        links = []

        for i, click in enumerate(self.clicks):
            if (i + 1) % 100 == 0:
                logger.info("Processing click %d/%d...", i + 1, len(self.clicks))

            link = self.analyze_click(click)
            links.append(link)

        return links

    def save_killchain(self, links: List[KillChainLink]) -> None:
        """Save kill-chain results to killchain.jsonl."""
        output_file = self.organ_dir / "killchain.jsonl"

        try:
            with open(output_file, "w") as f:
                for link in links:
                    f.write(json.dumps(asdict(link)) + "\n")

            logger.info("✓ Saved %d kill-chain links to %s", len(links), output_file)
        except Exception as e:
            logger.error("Failed to save killchain: %s", e)

    def analyze_coverage(self, links: List[KillChainLink]) -> None:
        """Print coverage statistics."""
        matched = sum(1 for link in links if link.capture_count > 0)
        coverage = matched / len(links) * 100 if links else 0

        avg_visible = sum(link.visible_tweets for link in links) / len(links) if links else 0
        avg_signal = sum(link.avg_signal_score for link in links) / len(links) if links else 0

        logger.info("\n=== Kill-Chain Coverage ===")
        logger.info("Total clicks analyzed: %d", len(links))
        logger.info("Clicks with visible tweets: %d (%.1f%%)", matched, coverage)
        logger.info("Avg tweets visible per click: %.1f", avg_visible)
        logger.info("Avg signal score: %.2f", avg_signal)

        # Show distribution
        signal_distribution = {
            "low (<0)": sum(1 for link in links if link.avg_signal_score < 0),
            "medium (0-3)": sum(1 for link in links if 0 <= link.avg_signal_score < 3),
            "high (3+)": sum(1 for link in links if link.avg_signal_score >= 3),
        }

        logger.info("Signal score distribution:")
        for bucket, count in signal_distribution.items():
            pct = count / len(links) * 100 if links else 0
            logger.info("  %s: %d (%.1f%%)", bucket, count, pct)


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes Kill-Chain Tracer")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    parser.add_argument("--behavior-log", type=Path, default=Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl")
    parser.add_argument("--max-window-sec", type=int, default=5, help="Time window around click (±N sec)")
    args = parser.parse_args()

    logger.info("Hermes Kill-Chain Tracer v%s", __version__)
    tracer = KillChainTracer(args.organ_dir, args.behavior_log, args.max_window_sec)

    # Load data
    if not tracer.load_clicks():
        return 1
    if not tracer.load_captures():
        return 1
    tracer.load_enrichments()  # Optional: use pre-computed enrichments from x_proxy
    if not tracer.load_learned_weights():
        return 1

    # Trace kill-chains
    logger.info("Tracing %d clicks → captures → tweets...", len(tracer.clicks))
    links = tracer.trace_all()

    # Save and analyze
    tracer.save_killchain(links)
    tracer.analyze_coverage(links)

    logger.info("✓ Kill-chain trace complete.")
    return 0


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
