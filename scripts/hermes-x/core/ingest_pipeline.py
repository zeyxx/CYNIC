#!/usr/bin/env python3
"""
Robust, versioned, evolvable ingestion pipeline for Organ-X.

Extracts full tweet richness (URLs, hashtags, mentions, media, user metadata),
normalizes, validates, writes versioned datasets with field completeness metrics.

K15: Produces dataset.vX.jsonl consumed by CCM, domain-router, agents.
"""

import json
import sys
from pathlib import Path
from datetime import datetime, timezone
from collections import defaultdict
import hashlib


class HermesXIngestPipeline:
    """Robust ingestion with versioning and field tracking."""

    def __init__(self, account_id="cynic", version=1):
        self.account_id = account_id
        self.version = version
        self.stats = {
            "total_extracted": 0,
            "total_normalized": 0,
            "total_skipped": 0,
            "field_presence": defaultdict(int),
            "errors": [],
        }

    def extract_tweets_from_capture(self, data: dict) -> list:
        """Extract tweet objects from X.com API response (URT format)."""
        tweets = []
        response = data.get("response", {}).get("data", {})

        # HomeTimeline format
        if "home" in response:
            try:
                entries = response["home"]["home_timeline_urt"]["instructions"][0]["entries"]
                for entry in entries:
                    tweet = entry["content"]["itemContent"]["tweet_results"]["result"]
                    tweets.append(tweet)
            except (KeyError, IndexError, TypeError):
                pass

        # TweetDetail format
        elif "threaded_conversation_with_injections_v2" in response:
            try:
                instructions = response["threaded_conversation_with_injections_v2"]["instructions"]
                for instr in instructions:
                    if "entries" in instr:
                        for entry in instr["entries"]:
                            try:
                                tweet = entry["content"]["itemContent"]["tweet_results"]["result"]
                                tweets.append(tweet)
                            except (KeyError, TypeError):
                                pass
            except (KeyError, TypeError):
                pass

        # UserTweets format
        elif "user" in response:
            try:
                user_data = response["user"].get("result", {})
                if "timeline_v2" in user_data:
                    entries = user_data["timeline_v2"]["timeline"]["instructions"][0]["entries"]
                    for entry in entries:
                        try:
                            tweet = entry["content"]["itemContent"]["tweet_results"]["result"]
                            tweets.append(tweet)
                        except (KeyError, TypeError):
                            pass
            except (KeyError, TypeError):
                pass

        self.stats["total_extracted"] += len(tweets)
        return tweets

    def normalize_tweet(self, tweet: dict) -> dict:
        """Convert X.com API tweet to ingestion-ready format with full richness."""
        legacy = tweet.get("legacy", {})
        core = tweet.get("core", {})
        user_result = core.get("user_results", {}).get("result", {})
        user_legacy = user_result.get("legacy", {})

        # Required core fields
        tweet_id = tweet.get("rest_id")
        text = legacy.get("full_text")
        author_id = legacy.get("user_id_str")

        if not (tweet_id and text and author_id):
            return None

        # Extract rich metadata
        entities = legacy.get("entities", {})
        extended_entities = legacy.get("extended_entities", {})

        # URLs
        urls = entities.get("urls", [])
        url_list = [
            {
                "url": u.get("url"),
                "expanded_url": u.get("expanded_url"),
                "display_url": u.get("display_url"),
                "indices": u.get("indices"),
            }
            for u in urls
        ]

        # Hashtags
        hashtags = entities.get("hashtags", [])
        hashtag_list = [h.get("text") for h in hashtags]

        # Mentions
        mentions = entities.get("user_mentions", [])
        mention_list = [
            {
                "user_id": m.get("id_str"),
                "screen_name": m.get("screen_name"),
                "name": m.get("name"),
            }
            for m in mentions
        ]

        # Media
        media = extended_entities.get("media", []) or entities.get("media", [])
        media_list = [
            {
                "type": m.get("type"),
                "media_url": m.get("media_url_https"),
                "url": m.get("url"),
            }
            for m in media
        ]

        # User metadata
        record = {
            # Core
            "tweet_id": tweet_id,
            "text": text,
            "author_id": author_id,
            "created_at": legacy.get("created_at"),
            # Engagement
            "engagement": {
                "likes": legacy.get("favorite_count", 0),
                "replies": legacy.get("reply_count", 0),
                "retweets": legacy.get("retweet_count", 0),
                "bookmarks": legacy.get("bookmark_count", 0),
                "quotes": legacy.get("quote_count", 0),
            },
            # Rich metadata
            "urls": url_list if url_list else None,
            "hashtags": hashtag_list if hashtag_list else None,
            "mentions": mention_list if mention_list else None,
            "media": media_list if media_list else None,
            # User
            "author": {
                "handle": user_legacy.get("screen_name"),
                "name": user_legacy.get("name"),
                "followers_count": user_legacy.get("followers_count"),
                "following_count": user_legacy.get("friends_count"),
                "verified": user_legacy.get("verified"),
                "bio": user_legacy.get("description"),
                "location": user_legacy.get("location"),
                "created_at": user_legacy.get("created_at"),
            },
            # Metadata
            "lang": legacy.get("lang"),
            "possibly_sensitive": legacy.get("possibly_sensitive", False),
            "conversation_id": legacy.get("conversation_id_str"),
            "account_id": self.account_id,
            "extracted_at": datetime.now(timezone.utc).isoformat(),
            "dataset_version": self.version,
        }

        # Track field presence
        for field in ["urls", "hashtags", "mentions", "media"]:
            if record.get(field):
                self.stats["field_presence"][field] += 1

        return record

    def extract_and_normalize(self, captures_dir: Path):
        """Generator: extract and normalize tweets, track errors."""
        captures = sorted(list(captures_dir.glob("*.json")))

        for capture_file in captures:
            try:
                with open(capture_file) as f:
                    data = json.load(f)

                tweets = self.extract_tweets_from_capture(data)

                for tweet in tweets:
                    record = self.normalize_tweet(tweet)
                    if record:
                        self.stats["total_normalized"] += 1
                        yield record
                    else:
                        self.stats["total_skipped"] += 1

            except json.JSONDecodeError:
                self.stats["errors"].append(f"{capture_file.name}: Invalid JSON")
            except Exception as e:
                self.stats["errors"].append(f"{capture_file.name}: {str(e)[:60]}")

    def write_versioned_dataset(self, records, output_dir: Path):
        """Write dataset.vX.jsonl and metrics.vX.json with full richness."""
        output_dir.mkdir(parents=True, exist_ok=True)

        dataset_file = output_dir / f"dataset.v{self.version}.jsonl"
        metrics_file = output_dir / f"dataset.v{self.version}.metrics.json"

        with open(dataset_file, "w") as f:
            for record in records:
                f.write(json.dumps(record) + "\n")

        # Compute metrics
        metrics = {
            "version": self.version,
            "account_id": self.account_id,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "total_tweets": self.stats["total_normalized"],
            "field_completeness": {
                "core": {
                    "tweet_id": "100%",
                    "text": "100%",
                    "author_id": "100%",
                },
                "richness": {
                    "urls": f"{100*self.stats['field_presence'].get('urls', 0)/max(1, self.stats['total_normalized']):.1f}%",
                    "hashtags": f"{100*self.stats['field_presence'].get('hashtags', 0)/max(1, self.stats['total_normalized']):.1f}%",
                    "mentions": f"{100*self.stats['field_presence'].get('mentions', 0)/max(1, self.stats['total_normalized']):.1f}%",
                    "media": f"{100*self.stats['field_presence'].get('media', 0)/max(1, self.stats['total_normalized']):.1f}%",
                },
            },
            "stats": {
                "extracted": self.stats["total_extracted"],
                "normalized": self.stats["total_normalized"],
                "skipped": self.stats["total_skipped"],
                "errors": len(self.stats["errors"]),
            },
        }

        with open(metrics_file, "w") as f:
            json.dump(metrics, f, indent=2)

        return dataset_file, metrics_file

    def run(self, captures_dir: Path, output_dir: Path):
        """Full pipeline: extract, normalize, write versioned dataset."""
        records = self.extract_and_normalize(captures_dir)
        return self.write_versioned_dataset(records, output_dir)


if __name__ == "__main__":
    import os

    account_id = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("HERMES_ACCOUNT", "cynic")
    version = int(sys.argv[2]) if len(sys.argv) > 2 else 2

    captures_dir = Path(os.path.expanduser("~/.cynic/organs/hermes/x/captures"))
    output_dir = Path(os.path.expanduser(f"~/.cynic/organs/hermes/x/datasets/{account_id}"))

    print(f"{'='*70}")
    print(f"HERMES-X INGEST PIPELINE v{version}")
    print(f"{'='*70}")
    print(f"Account: {account_id}")
    print(f"Captures: {captures_dir.name}")
    print(f"Output: {output_dir}")

    pipeline = HermesXIngestPipeline(account_id=account_id, version=version)
    dataset_file, metrics_file = pipeline.run(captures_dir, output_dir)

    print(f"\n✓ Dataset: {dataset_file.name}")
    print(f"✓ Metrics: {metrics_file.name}")
    print(f"\nStats:")
    print(f"  Extracted: {pipeline.stats['total_extracted']}")
    print(f"  Normalized: {pipeline.stats['total_normalized']}")
    print(f"  Skipped: {pipeline.stats['total_skipped']}")
    print(f"\nField richness:")
    for field, count in pipeline.stats['field_presence'].items():
        pct = 100*count/max(1, pipeline.stats['total_normalized'])
        print(f"  {field}: {pct:.0f}%")
    print(f"\nReady for: CCM, domain-router, agents")
