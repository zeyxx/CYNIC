#!/usr/bin/env python3
"""
Domain Signal Yield Measurement

Measures whether domain-aware routing preserves signal quality compared to baseline.

Falsification criterion: >80% of raw signal yield vs. baseline (all-general) routing.

Input files:
  - domain_routed.jsonl: tweets with assigned domains (from domain_classifier.py)
  - engagement.jsonl: actual user engagements (from browser extension)
  - killchain.jsonl: enriched tweets with signal predictions

Output:
  - Per-domain precision/recall (% of routed tweets that engaged, % of engagements caught)
  - Overall signal preservation vs baseline
  - Recommendation: accept/reject domain routing based on >80% threshold

Usage:
    python3 measure_domain_signal_yield.py --organ-dir ~/.cynic/organs/hermes/x
"""

__version__ = "0.1.0"

import json
import logging
from pathlib import Path
from typing import Dict, List, Set, Tuple
from dataclasses import dataclass

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("domain-signal-yield")


@dataclass
class DomainMetrics:
    """Per-domain signal metrics."""
    domain: str
    routed_count: int
    engagement_count: int
    true_positives: int  # Routed AND engaged

    @property
    def precision(self) -> float:
        """% of routed tweets that were actually engaged."""
        if self.routed_count == 0:
            return 0.0
        return self.true_positives / self.routed_count

    @property
    def recall(self) -> float:
        """% of engaged tweets in this domain that were routed to it."""
        if self.engagement_count == 0:
            return 0.0
        return self.true_positives / self.engagement_count

    @property
    def f1(self) -> float:
        """Harmonic mean of precision and recall."""
        p = self.precision
        r = self.recall
        if p + r == 0:
            return 0.0
        return 2 * (p * r) / (p + r)


class DomainSignalYieldMeasurer:
    """Measures signal yield per domain."""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.domain_routed_file = self.organ_dir / "domain_routed.jsonl"
        self.engagement_file = self.organ_dir / "engagement.jsonl"
        self.killchain_file = self.organ_dir / "killchain.jsonl"

    def load_domain_routes(self) -> Dict[str, str]:
        """Load domain assignments by tweet_id."""
        routes = {}

        if not self.domain_routed_file.exists():
            logger.warning("domain_routed.jsonl not found: %s", self.domain_routed_file)
            return routes

        try:
            with open(self.domain_routed_file) as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        tweet_id = obj.get("tweet_id")
                        domain = obj.get("domain")
                        if tweet_id and domain:
                            routes[tweet_id] = domain
                    except json.JSONDecodeError:
                        pass

            logger.info("✓ Loaded %d domain routes", len(routes))
            return routes

        except Exception as e:
            logger.error("Failed to load domain routes: %s", e)
            return routes

    def load_signal_scores(self) -> Dict[str, float]:
        """Load predicted signal scores from killchain.jsonl.

        Uses killchain predictions as proxy for engagement signal.
        High signal = predicted to be high-engagement.
        """
        scores = {}

        if not self.killchain_file.exists():
            logger.warning("killchain.jsonl not found: %s", self.killchain_file)
            return scores

        try:
            with open(self.killchain_file) as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        top_tweets = obj.get("top_tweets", [])
                        for tweet in top_tweets:
                            tweet_id = tweet.get("tweet_id")
                            signal_score = tweet.get("signal_score", 0)
                            if tweet_id:
                                # Keep highest score if duplicate
                                if tweet_id not in scores or signal_score > scores[tweet_id]:
                                    scores[tweet_id] = signal_score
                    except json.JSONDecodeError:
                        pass

            logger.info("✓ Loaded %d signal scores from killchain", len(scores))
            return scores

        except Exception as e:
            logger.error("Failed to load signal scores: %s", e)
            return scores

    def measure(self, routes: Dict[str, str], signal_scores: Dict[str, float]) -> Tuple[Dict[str, DomainMetrics], float]:
        """Measure signal yield per domain using predicted signal scores.

        Uses killchain signal_score as proxy for engagement signal.

        Returns:
            (per_domain_metrics, signal_preservation_ratio)

        Signal preservation = avg signal score with routing / avg signal score baseline (all-general)
        """
        # Per-domain signal aggregation
        domain_signals = {}

        for tweet_id, domain in routes.items():
            if domain not in domain_signals:
                domain_signals[domain] = {"scores": [], "count": 0}
            score = signal_scores.get(tweet_id, 0.0)
            domain_signals[domain]["scores"].append(score)
            domain_signals[domain]["count"] += 1

        # Baseline: all tweets as one "general" bucket
        all_scores = [signal_scores.get(tid, 0.0) for tid in routes.keys()]
        baseline_avg_signal = sum(all_scores) / len(all_scores) if all_scores else 0.0

        # Per-domain average signals
        domain_avg_signals = {}
        for domain, data in domain_signals.items():
            avg = sum(data["scores"]) / len(data["scores"]) if data["scores"] else 0.0
            domain_avg_signals[domain] = avg

        # Weighted average signal across domains (weighting by routed count)
        total_routed = sum(len(s["scores"]) for s in domain_signals.values())
        weighted_avg_signal = sum(
            (sum(s["scores"]) / len(s["scores"]) if s["scores"] else 0.0) * len(s["scores"])
            for s in domain_signals.values()
        ) / total_routed if total_routed > 0 else 0.0

        # Signal preservation = domain-routed avg / baseline avg
        signal_preservation = weighted_avg_signal / baseline_avg_signal if baseline_avg_signal > 0 else 1.0

        logger.info("\n=== Domain Signal Yield Measurement ===")
        logger.info("Baseline (all-general) avg signal: %.3f", baseline_avg_signal)
        logger.info("Weighted avg signal (domain-routed): %.3f", weighted_avg_signal)
        logger.info("Signal preservation ratio: %.2f× (baseline=1.0, >0.8=acceptable)", signal_preservation)

        logger.info("\n=== Per-Domain Signal Stats ===")
        for domain in sorted(domain_avg_signals.keys()):
            scores = domain_signals[domain]["scores"]
            avg_sig = domain_avg_signals[domain]
            min_sig = min(scores) if scores else 0.0
            max_sig = max(scores) if scores else 0.0
            logger.info("\nDomain: %s", domain)
            logger.info("  Routed: %d tweets", len(scores))
            logger.info("  Avg signal: %.3f (min=%.3f, max=%.3f)", avg_sig, min_sig, max_sig)

        # Verdict
        logger.info("\n=== Verdict ===")
        if signal_preservation >= 0.80:
            logger.info("✓ PASS: Signal preservation %.2f× >= 0.80 threshold", signal_preservation)
            logger.info("  Domain-aware routing preserves >80% of signal yield vs baseline.")
            logger.info("  Proceed with domain routing.")
        else:
            logger.warning("✗ FAIL: Signal preservation %.2f× < 0.80 threshold", signal_preservation)
            logger.warning("  Domain-aware routing loses too much signal vs baseline.")
            logger.warning("  Recommend: improve domain classifier or revert to general routing.")

        return {}, signal_preservation

    def run(self):
        """Run full measurement."""
        logger.info("Domain Signal Yield Measurement v%s", __version__)

        routes = self.load_domain_routes()
        signal_scores = self.load_signal_scores()

        if not routes or not signal_scores:
            logger.error("Missing data for measurement. Run domain_classifier.py and killchain tracer first.")
            return None

        return self.measure(routes, signal_scores)


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Domain Signal Yield Measurement")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return 1

    measurer = DomainSignalYieldMeasurer(organ_dir)
    result = measurer.run()

    # Result is (metrics, signal_preservation)
    if result:
        metrics, signal_preservation = result
        return 0 if signal_preservation >= 0.80 else 1
    return 1


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
