#!/usr/bin/env python3
"""
Reconstruct dataset.jsonl from raw X.com captures.

Parses 1,295 capture files (618MB), extracts tweets, normalizes to
ingest-ready format with complete metadata, and writes to dataset.jsonl.

K15: Output consumed by nightshift, search-executor, CCM, recovery daemon.
"""

import json
import sys
from pathlib import Path
from datetime import datetime
from collections import defaultdict

def extract_tweets_from_capture(data: dict) -> list:
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

    return tweets

def normalize_tweet(tweet: dict) -> dict:
    """Convert X.com API tweet to ingest-ready format."""
    legacy = tweet.get("legacy", {})
    core = tweet.get("core", {})
    user_result = core.get("user_results", {}).get("result", {})
    user_legacy = user_result.get("legacy", {})

    # Required fields for valid record
    tweet_id = tweet.get("rest_id")
    text = legacy.get("full_text")
    author_id = legacy.get("user_id_str")

    if not (tweet_id and text and author_id):
        return None

    record = {
        "tweet_id": tweet_id,
        "text": text,
        "author_id": author_id,
        "author_name": user_legacy.get("name", ""),
        "author_handle": user_legacy.get("screen_name", ""),
        "created_at": legacy.get("created_at"),
        "likes": legacy.get("favorite_count", 0),
        "replies": legacy.get("reply_count", 0),
        "retweets": legacy.get("retweet_count", 0),
        "bookmarks": legacy.get("bookmark_count", 0),
        "quotes": legacy.get("quote_count", 0),
        "lang": legacy.get("lang"),
        "possibly_sensitive": legacy.get("possibly_sensitive", False),
        "conversation_id": legacy.get("conversation_id_str"),
        "account_id": "cynic",  # K15: Tag for recovery daemon to know source
        "captured_at": datetime.now(datetime.UTC).isoformat(),
    }

    return record

def reconstruct(captures_dir: Path, output_file: Path, progress: bool = True) -> dict:
    """Reconstruct dataset from all captures in directory."""
    captures = sorted(list(captures_dir.glob("*.json")))

    stats = {
        "total_files": len(captures),
        "files_processed": 0,
        "files_failed": 0,
        "tweets_extracted": 0,
        "tweets_normalized": 0,
        "tweets_skipped": 0,
        "by_type": defaultdict(int),
        "errors": [],
    }

    with open(output_file, "w") as outf:
        for i, capture_file in enumerate(captures):
            try:
                with open(capture_file) as f:
                    data = json.load(f)

                stats["files_processed"] += 1

                # Determine capture type
                response = data.get("response", {}).get("data", {})
                if "home" in response:
                    capture_type = "HomeTimeline"
                elif "threaded_conversation_with_injections_v2" in response:
                    capture_type = "TweetDetail"
                elif "user" in response:
                    capture_type = "UserTweets"
                else:
                    capture_type = "Unknown"

                stats["by_type"][capture_type] += 1

                # Extract tweets
                tweets = extract_tweets_from_capture(data)
                stats["tweets_extracted"] += len(tweets)

                # Normalize and write
                for tweet in tweets:
                    record = normalize_tweet(tweet)
                    if record:
                        outf.write(json.dumps(record) + "\n")
                        stats["tweets_normalized"] += 1
                    else:
                        stats["tweets_skipped"] += 1

                if progress and (i + 1) % 100 == 0:
                    print(f"  [{i+1}/{len(captures)}] {i+1*618//len(captures)}MB processed...", file=sys.stderr)

            except json.JSONDecodeError:
                stats["files_failed"] += 1
                stats["errors"].append(f"{capture_file.name}: Invalid JSON")
            except Exception as e:
                stats["files_failed"] += 1
                stats["errors"].append(f"{capture_file.name}: {str(e)[:50]}")

    return stats

if __name__ == "__main__":
    import os

    # Account ID from command line or env
    account_id = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("HERMES_ACCOUNT", "cynic")

    captures_dir = Path(os.path.expanduser(f"~/.cynic/organs/hermes/x/captures"))
    datasets_dir = Path(os.path.expanduser(f"~/.cynic/organs/hermes/x/datasets/{account_id}"))
    datasets_dir.mkdir(parents=True, exist_ok=True)
    output_file = datasets_dir / "dataset.jsonl"

    print(f"Reconstructing {account_id} dataset from {captures_dir.name}...")
    print(f"Output: {output_file}")

    stats = reconstruct(captures_dir, output_file, progress=True)

    print(f"\n{'=' * 70}")
    print(f"RECONSTRUCTION COMPLETE")
    print(f"{'=' * 70}")
    print(f"Files processed: {stats['files_processed']} / {stats['total_files']}")
    print(f"Failed: {stats['files_failed']}")
    print(f"\nTweets:")
    print(f"  Extracted: {stats['tweets_extracted']}")
    print(f"  Normalized: {stats['tweets_normalized']}")
    print(f"  Skipped: {stats['tweets_skipped']}")
    print(f"\nCapture types:")
    for capture_type, count in stats["by_type"].items():
        print(f"  {capture_type}: {count}")

    if stats["errors"]:
        print(f"\nErrors: {len(stats['errors'])}")
        for err in stats["errors"][:10]:
            print(f"  - {err}")

    # K15: Report to kernel
    print(f"\nOutput: {output_file} ({output_file.stat().st_size / (1024*1024):.1f}MB)")
    print(f"Ready for: hermes-x-recovery, nightshift, search-executor, CCM")
