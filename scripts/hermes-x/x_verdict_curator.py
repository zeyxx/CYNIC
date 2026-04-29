#!/usr/bin/env python3
"""
CYNIC Verdict Curator — Manual X Posting Helper

Reads verdicts, filters by confidence + signal quality, formats for human posting.
Tracks posted verdicts to avoid duplicates.

Usage:
    python3 x_verdict_curator.py --output verdicts_to_post.json

Output: JSON with ready-to-copy tweet text + verdict details
"""

__version__ = "0.1.0"

import argparse
import json
import logging
from pathlib import Path
from datetime import datetime
from typing import Optional

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("x-verdict-curator")

ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"
VERDICTS_DIR = ORGAN_DIR / "verdicts"
DATASET_PATH = ORGAN_DIR / "dataset.jsonl"
POSTED_TRACKER = ORGAN_DIR / ".posted_verdicts.json"


def load_dataset_index() -> dict:
    """Load dataset.jsonl and index by tweet_id for quick lookup."""
    index = {}
    if not DATASET_PATH.exists():
        logger.warning("Dataset not found: %s", DATASET_PATH)
        return index

    try:
        with open(DATASET_PATH) as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    tweet = json.loads(line)
                    tweet_id = tweet.get("tweet_id")
                    if tweet_id:
                        index[str(tweet_id)] = tweet
                except json.JSONDecodeError:
                    continue
    except IOError as e:
        logger.warning("Failed to load dataset: %s", e)

    return index


def load_posted_tracker() -> dict:
    """Load tracking of already-posted verdicts."""
    if not POSTED_TRACKER.exists():
        return {}
    try:
        with open(POSTED_TRACKER) as f:
            return json.load(f)
    except (IOError, json.JSONDecodeError):
        return {}


def save_posted_tracker(tracker: dict):
    """Save posted verdicts tracking."""
    try:
        with open(POSTED_TRACKER, "w") as f:
            json.dump(tracker, f, indent=2)
    except IOError as e:
        logger.warning("Failed to save posted tracker: %s", e)


def extract_claim_summary(tweet: dict) -> str:
    """Extract first 40 chars of tweet text as claim summary."""
    text = tweet.get("text", "")[:40]
    if len(tweet.get("text", "")) > 40:
        text += "…"
    return text.strip()


def verdict_type_from_score(q_score: float) -> str:
    """Map q_score to verdict type label."""
    if q_score > 0.528:
        return "HOWL"
    elif q_score > 0.382:
        return "GROWL"
    elif q_score > 0.236:
        return "WAG"
    else:
        return "BARK"


def should_post_verdict(verdict: dict, signal_score: int) -> bool:
    """
    Filter verdict: post if:
    - HOWL (q_score > 0.528), OR
    - BARK on high-signal tweets (signal_score >= 5)
    """
    q_score = verdict.get("q_score", {}).get("total", 0)

    # HOWL always
    if q_score > 0.528:
        return True

    # BARK on high-signal
    if q_score <= 0.236 and signal_score >= 5:
        return True

    return False


def format_post(
    verdict_id: str,
    tweet_id: str,
    claim_summary: str,
    q_score: float,
    signal_score: int,
    dogs_used: int,
) -> str:
    """Format verdict as X post (280 char limit)."""
    verdict_type = verdict_type_from_score(q_score)

    # Build compact post
    post = f"Verdict: {verdict_type} ({q_score:.2f}) on \"{claim_summary}\"\n"
    post += f"Dogs: {dogs_used}/5 | Signal: {signal_score}\n"
    post += f"Axioms → cynic.ai/v/{verdict_id[:8]}"

    return post


def curate_verdicts(output_file: Optional[str] = None) -> dict:
    """Read verdicts, filter, format for posting."""
    logger.info("Loading dataset index...")
    dataset = load_dataset_index()
    logger.info("Loaded %d tweets from dataset", len(dataset))

    logger.info("Loading posted tracker...")
    posted = load_posted_tracker()
    logger.info("Already posted: %d verdicts", len(posted))

    verdicts_to_post = []

    if not VERDICTS_DIR.exists():
        logger.warning("Verdicts directory not found: %s", VERDICTS_DIR)
        return {"verdicts": [], "count": 0}

    logger.info("Scanning verdicts...")
    verdict_files = sorted(VERDICTS_DIR.glob("*.json"))
    logger.info("Found %d verdicts", len(verdict_files))

    for verdict_file in verdict_files:
        tweet_id = verdict_file.stem

        # Skip if already posted
        if tweet_id in posted:
            continue

        try:
            with open(verdict_file) as f:
                verdict_data = json.load(f)
        except (IOError, json.JSONDecodeError) as e:
            logger.warning("Failed to load verdict %s: %s", verdict_file, e)
            continue

        verdict = verdict_data.get("verdict", {})
        signal_score = verdict_data.get("signal_score", 0)

        # Filter by criteria (pass the verdict object, not verdict_data)
        if not should_post_verdict(verdict, signal_score):
            continue

        # Extract data
        q_score = verdict.get("q_score", {}).get("total", 0)
        dogs_used = len(verdict.get("dog_scores", []))
        verdict_id = verdict.get("verdict_id", "unknown")

        # Get claim summary from dataset
        tweet = dataset.get(tweet_id, {})
        claim_summary = extract_claim_summary(tweet)
        if not claim_summary:
            claim_summary = f"Tweet {tweet_id[:8]}"

        # Format post
        post_text = format_post(
            verdict_id=verdict_id,
            tweet_id=tweet_id,
            claim_summary=claim_summary,
            q_score=q_score,
            signal_score=signal_score,
            dogs_used=dogs_used,
        )

        # Check character count
        if len(post_text) > 280:
            logger.warning("Post too long (%d chars): %s", len(post_text), tweet_id)
            # Truncate claim
            chars_over = len(post_text) - 280
            claim_summary = claim_summary[:-chars_over-3] + "…"
            post_text = format_post(
                verdict_id=verdict_id,
                tweet_id=tweet_id,
                claim_summary=claim_summary,
                q_score=q_score,
                signal_score=signal_score,
                dogs_used=dogs_used,
            )

        verdicts_to_post.append({
            "tweet_id": tweet_id,
            "verdict_id": verdict_id,
            "q_score": round(q_score, 3),
            "verdict_type": verdict_type_from_score(q_score),
            "signal_score": signal_score,
            "dogs_used": dogs_used,
            "post_text": post_text,
            "char_count": len(post_text),
            "verdict_url": f"https://cynic.ai/verdicts/{verdict_id}",
            "curated_at": datetime.now().isoformat() + "Z",
        })

    logger.info("Curated %d verdicts ready to post", len(verdicts_to_post))

    # Save to output
    output = {
        "count": len(verdicts_to_post),
        "curated_at": datetime.now().isoformat() + "Z",
        "verdicts": verdicts_to_post,
    }

    if output_file:
        try:
            with open(output_file, "w") as f:
                json.dump(output, f, indent=2)
            logger.info("Saved to %s", output_file)
        except IOError as e:
            logger.warning("Failed to save output: %s", e)

    return output


def mark_posted(tweet_id: str, posted_tweet_id: str):
    """Mark a verdict as posted on X."""
    tracker = load_posted_tracker()
    tracker[tweet_id] = {
        "posted_tweet_id": posted_tweet_id,
        "posted_at": datetime.now().isoformat() + "Z",
    }
    save_posted_tracker(tracker)
    logger.info("Marked as posted: %s → %s", tweet_id, posted_tweet_id)


def main():
    parser = argparse.ArgumentParser(
        description="CYNIC Verdict Curator — format verdicts for manual X posting"
    )
    parser.add_argument(
        "--output",
        default="verdicts_to_post.json",
        help="Output file (default: verdicts_to_post.json)",
    )
    parser.add_argument(
        "--mark-posted",
        nargs=2,
        metavar=("TWEET_ID", "POSTED_TWEET_ID"),
        help="Mark a verdict as posted: --mark-posted <tweet_id> <posted_tweet_id>",
    )
    args = parser.parse_args()

    if args.mark_posted:
        tweet_id, posted_tweet_id = args.mark_posted
        mark_posted(tweet_id, posted_tweet_id)
    else:
        output = curate_verdicts(output_file=args.output)
        print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
