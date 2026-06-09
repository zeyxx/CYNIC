"""Measure twitter-domain calibration vs current token-domain verdicts.

Compare TwitterScorer accuracy against current Dogs' verdicts on same tweets.
Measure: does twitter-aware scoring discriminate signal better?
"""

import json
import sys
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from hermes_paths import DATASET, VERDICTS_DIR

from twitter_heuristics import TwitterScorer
from twitter_signal_extractor import extract_signals_from_dataset


def load_verdict(tweet_id: str, verdicts_dir: Path) -> Optional[dict]:
    """Load verdict JSON for a tweet."""
    verdict_path = verdicts_dir / f"{tweet_id}.json"
    if verdict_path.exists():
        try:
            with open(verdict_path) as f:
                return json.load(f)
        except json.JSONDecodeError:
            return None
    return None


def measure_twitter_calibration(
    dataset_path: Path = None,
    verdicts_dir: Path = None,
    limit: int = 100,
) -> dict:
    """Compare twitter-domain vs token-domain scoring."""

    if dataset_path is None:
        dataset_path = DATASET
    if verdicts_dir is None:
        verdicts_dir = VERDICTS_DIR

    scorer = TwitterScorer()
    tweets_and_signals = extract_signals_from_dataset(dataset_path, limit=limit)

    results = {
        "total_tweets": len(tweets_and_signals),
        "tweets_with_verdicts": 0,
        "twitter_scores": [],
        "verdict_scores": [],
        "score_deltas": [],
        "discrimination_quality": {
            "twitter_score_range": (1.0, 0.0),  # (max, min)
            "verdict_score_range": (1.0, 0.0),
        },
    }

    print(f"{'Tweet ID':<20} {'Signal':<8} {'Twitter Score':<15} {'Verdict Score':<15} {'Delta':<10}")
    print("-" * 70)

    for tweet_id, tweet, signals in tweets_and_signals:
        verdict = load_verdict(tweet_id, verdicts_dir)
        if not verdict:
            continue

        results["tweets_with_verdicts"] += 1

        # Score with twitter heuristics
        twitter_scores = scorer.score(signals)
        twitter_q = twitter_scores.q_score

        # Get current verdict score (token-domain)
        verdict_q = verdict.get("verdict", {}).get("q_score", {}).get("total", 0.0)
        signal_score = tweet.get("signal_score", 0)

        delta = abs(twitter_q - verdict_q)
        results["twitter_scores"].append(twitter_q)
        results["verdict_scores"].append(verdict_q)
        results["score_deltas"].append(delta)

        # Track ranges
        results["discrimination_quality"]["twitter_score_range"] = (
            max(results["discrimination_quality"]["twitter_score_range"][0], twitter_q),
            min(results["discrimination_quality"]["twitter_score_range"][1], twitter_q),
        )
        results["discrimination_quality"]["verdict_score_range"] = (
            max(results["discrimination_quality"]["verdict_score_range"][0], verdict_q),
            min(results["discrimination_quality"]["verdict_score_range"][1], verdict_q),
        )

        if results["tweets_with_verdicts"] <= 20:
            print(f"{tweet_id:<20} {signal_score:<8} {twitter_q:.3f}           {verdict_q:.3f}          {delta:.3f}")

    # Compute aggregate metrics
    if results["twitter_scores"]:
        avg_twitter = sum(results["twitter_scores"]) / len(results["twitter_scores"])
        avg_verdict = sum(results["verdict_scores"]) / len(results["verdict_scores"])
        avg_delta = sum(results["score_deltas"]) / len(results["score_deltas"])

        results["summary"] = {
            "avg_twitter_score": avg_twitter,
            "avg_verdict_score": avg_verdict,
            "avg_score_delta": avg_delta,
            "twitter_range": results["discrimination_quality"]["twitter_score_range"][0] - results["discrimination_quality"]["twitter_score_range"][1],
            "verdict_range": results["discrimination_quality"]["verdict_score_range"][0] - results["discrimination_quality"]["verdict_score_range"][1],
        }

        print("\n" + "=" * 70)
        print(f"Summary (n={results['tweets_with_verdicts']}):")
        print(f"  Avg twitter-domain score:   {avg_twitter:.3f}")
        print(f"  Avg token-domain score:     {avg_verdict:.3f}")
        print(f"  Avg absolute delta:         {avg_delta:.3f}")
        print(f"  Twitter score range:        {results['discrimination_quality']['twitter_score_range'][0]:.3f} - {results['discrimination_quality']['twitter_score_range'][1]:.3f} (Δ={results['summary']['twitter_range']:.3f})")
        print(f"  Token score range:          {results['discrimination_quality']['verdict_score_range'][0]:.3f} - {results['discrimination_quality']['verdict_score_range'][1]:.3f} (Δ={results['summary']['verdict_range']:.3f})")

        print(f"\nInterpretation:")
        if results["summary"]["twitter_range"] > results["summary"]["verdict_range"]:
            print(f"  ✓ Twitter-domain scoring has BETTER discrimination (range {results['summary']['twitter_range']:.3f} vs {results['summary']['verdict_range']:.3f})")
        else:
            print(f"  ✗ Token-domain scoring has better discrimination")

        if avg_delta > 0.15:
            print(f"  ⚠ Large disagreement between domains (delta {avg_delta:.3f}) — Dogs confused on twitter content")
        else:
            print(f"  ~ Domains agree reasonably (delta {avg_delta:.3f})")

    return results


if __name__ == "__main__":
    results = measure_twitter_calibration(limit=100)
