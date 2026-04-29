"""Twitter-specialized Dog for Hermes organ judgment.

Scores tweets on 6 axioms using TwitterScorer.
Can be called alongside token-domain Dogs for multi-dog consensus.
"""

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Optional

from twitter_heuristics import TwitterScorer
from twitter_signal_extractor import TwitterSignalExtractor


@dataclass
class TwitterDogVerdict:
    """Verdict from twitter-specialized Dog."""
    fidelity: float
    phi: float
    verify: float
    culture: float
    burn: float
    sovereignty: float
    q_score: float
    dog_id: str = "twitter-dog"
    latency_ms: int = 0
    reasoning: dict = None


class TwitterDog:
    """Heuristic Dog specialized for twitter domain judgment."""

    def __init__(self):
        self.scorer = TwitterScorer()
        self.extractor = TwitterSignalExtractor()
        self.id = "twitter-dog"

    def judge(self, tweet: dict, signal_score: Optional[int] = None) -> TwitterDogVerdict:
        """Judge a tweet using twitter-domain heuristics.

        Args:
            tweet: Raw tweet JSON from dataset.jsonl
            signal_score: Hermes signal_score (optional, for context)

        Returns:
            TwitterDogVerdict with axiom scores
        """
        # Extract signals from tweet
        signals = self.extractor.extract(tweet)

        # Score using TwitterScorer
        scores = self.scorer.score(signals)

        # Build reasoning
        reasoning = {
            "fidelity": f"Community sentiment: {scores.fidelity:.3f} (rug_allegations={signals.has_rug_allegations})",
            "phi": f"Engagement patterns: {scores.phi:.3f} (velocity={signals.tweet_velocity:.2f} tweets/day)",
            "verify": f"Verifiable metrics: {scores.verify:.3f} (engagement_rate={signals.engagement_rate:.1%})",
            "culture": f"Ecosystem norms: {scores.culture:.3f} (LP_discussion={signals.liquidity_discussion_active})",
            "burn": f"Marketing efficiency: {scores.burn:.3f} (has_spam={signals.has_buy_pressure_spam})",
            "sovereignty": f"Community decentralization: {scores.sovereignty:.3f} (followers={signals.follower_count})",
        }

        return TwitterDogVerdict(
            fidelity=scores.fidelity,
            phi=scores.phi,
            verify=scores.verify,
            culture=scores.culture,
            burn=scores.burn,
            sovereignty=scores.sovereignty,
            q_score=scores.q_score,
            latency_ms=0,  # Heuristic, no inference
            reasoning=reasoning,
        )

    def judge_from_dataset(self, dataset_path: Path, limit: int = 100) -> list[tuple[str, TwitterDogVerdict]]:
        """Judge tweets from dataset.jsonl.

        Returns: [(tweet_id, verdict), ...]
        """
        results = []

        with open(dataset_path) as f:
            for i, line in enumerate(f):
                if limit and i >= limit:
                    break

                if not line.strip():
                    continue

                try:
                    tweet = json.loads(line)
                    tweet_id = tweet.get("tweet_id")
                    signal_score = tweet.get("signal_score")

                    verdict = self.judge(tweet, signal_score)
                    results.append((tweet_id, verdict))
                except json.JSONDecodeError:
                    continue

        return results

    def compare_to_token_domain(
        self,
        dataset_path: Path,
        token_verdicts_dir: Path = None,
        limit: int = 100,
    ) -> dict:
        """Score tweets with twitter-domain Dog and compare to token-domain verdicts.

        Returns summary of differences.
        """
        if token_verdicts_dir is None:
            token_verdicts_dir = Path.home() / ".cynic/organs/hermes/x/verdicts"

        twitter_verdicts = self.judge_from_dataset(dataset_path, limit=limit)

        comparison = {
            "total_tweets": len(twitter_verdicts),
            "twitter_scores": [],
            "domain_mismatch_count": 0,
            "samples": [],
        }

        for tweet_id, twitter_verdict in twitter_verdicts[:20]:
            comparison["twitter_scores"].append(twitter_verdict.q_score)

            # Check if token verdict exists (for older tweets)
            token_verdict_path = token_verdicts_dir / f"{tweet_id}.json"
            if token_verdict_path.exists():
                with open(token_verdict_path) as f:
                    token_verdict = json.load(f)
                    token_q = token_verdict["verdict"]["q_score"]["total"]
                    delta = abs(twitter_verdict.q_score - token_q)
                    comparison["domain_mismatch_count"] += 1
                    comparison["samples"].append({
                        "tweet_id": tweet_id,
                        "twitter_q": twitter_verdict.q_score,
                        "token_q": token_q,
                        "delta": delta,
                    })

        return comparison


if __name__ == "__main__":
    dog = TwitterDog()
    dataset_path = Path.home() / ".cynic/organs/hermes/x/dataset.jsonl"

    print("Scoring first 10 tweets with Twitter-Dog...")
    print("-" * 80)
    print(f"{'Tweet ID':<20} {'Signal':<8} {'Q-Score':<10} {'Verdict':<10}")
    print("-" * 80)

    verdicts = dog.judge_from_dataset(dataset_path, limit=10)

    for tweet_id, verdict in verdicts:
        verdict_type = "HOWL" if verdict.q_score > 0.528 else "GROWL" if verdict.q_score > 0.382 else "BARK"
        print(f"{tweet_id:<20} {' ':<8} {verdict.q_score:.3f}     {verdict_type:<10}")
