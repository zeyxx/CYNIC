#!/usr/bin/env python3
"""
CYNIC Hermes Engagement Correlator

Correlates T.'s actual X.com engagement (from browser extension) with:
1. Agent predictions (hermes_observations.jsonl)
2. Kill-chain analysis (killchain.jsonl)
3. Kill-chain coverage

Output metrics:
  - Precision: % of agent predictions T. actually engaged with
  - Recall: % of T.'s actual engagements agent predicted
  - F1 score: Harmonic mean
  - Coverage: % of T.'s tweets in kill-chain

Usage:
    python3 engagement_correlator.py --organ-dir ~/.cynic/organs/hermes/x

Requires:
  - engagement.jsonl (from browser extension)
  - hermes_observations.jsonl (from organic agent)
  - killchain.jsonl (from tracer)
"""

__version__ = "0.1.0"

import json
import logging
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Set
from dataclasses import dataclass

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("engagement-correlator")


@dataclass
class Metrics:
    """Measurement results."""
    total_engaged: int
    total_predicted: int
    true_positives: int
    false_positives: int
    false_negatives: int

    @property
    def precision(self) -> float:
        """% of predicted tweets T. actually engaged with."""
        if self.total_predicted == 0:
            return 0.0
        return self.true_positives / self.total_predicted

    @property
    def recall(self) -> float:
        """% of T.'s actual engagements agent predicted."""
        if self.total_engaged == 0:
            return 0.0
        return self.true_positives / self.total_engaged

    @property
    def f1(self) -> float:
        """Harmonic mean of precision and recall."""
        p = self.precision
        r = self.recall
        if p + r == 0:
            return 0.0
        return 2 * (p * r) / (p + r)

    def __str__(self) -> str:
        return f"""
=== Engagement Correlation Results ===
Actual engagements (T.):        {self.total_engaged}
Agent predictions:              {self.total_predicted}

True positives (both):          {self.true_positives}
False positives (agent only):   {self.false_positives}
False negatives (T. only):      {self.false_negatives}

Precision (TP / Predicted):     {self.precision:.1%}
Recall (TP / Actual):           {self.recall:.1%}
F1 Score:                       {self.f1:.3f}

Interpretation:
  Precision > 60%:  Agent's predictions are reliable
  Recall > 50%:     Agent caught most of T.'s interests
  F1 > 0.55:        Strong learning signal
"""


class EngagementCorrelator:
    """Correlate engagement with predictions."""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.engagement_file = self.organ_dir / "engagement.jsonl"
        self.observations_file = self.organ_dir / "hermes_observations.jsonl"
        self.killchain_file = self.organ_dir / "killchain.jsonl"

    def load_engagement(self) -> Dict[str, dict]:
        """Load actual engagements by tweet_id."""
        engagements = {}

        if not self.engagement_file.exists():
            logger.warning("engagement.jsonl not found: %s", self.engagement_file)
            return engagements

        try:
            with open(self.engagement_file) as f:
                for line in f:
                    try:
                        event = json.loads(line)
                        tweet_id = event.get("tweet_id")
                        if tweet_id:
                            if tweet_id not in engagements:
                                engagements[tweet_id] = []
                            engagements[tweet_id].append(event)
                    except json.JSONDecodeError:
                        pass

            logger.info("✓ Loaded %d engaged tweets", len(engagements))
            return engagements

        except Exception as e:
            logger.error("Failed to load engagement: %s", e)
            return engagements

    def load_observations(self) -> Dict[str, dict]:
        """Load agent predictions by tweet_id."""
        observations = {}

        if not self.observations_file.exists():
            logger.warning("hermes_observations.jsonl not found: %s", self.observations_file)
            return observations

        try:
            with open(self.observations_file) as f:
                for line in f:
                    try:
                        obs = json.loads(line)
                        # obs has decision (engage/scroll/read_thread) and confidence
                        if obs.get("decision") in ["engage", "read_thread"]:
                            tweet_id = obs.get("tweet_id", "")
                            # Extract ID from "tweet_N" format
                            if tweet_id.startswith("tweet_"):
                                numeric_id = tweet_id.split("_")[1]
                                observations[numeric_id] = obs
                    except (json.JSONDecodeError, IndexError):
                        pass

            logger.info("✓ Loaded %d agent predictions", len(observations))
            return observations

        except Exception as e:
            logger.error("Failed to load observations: %s", e)
            return observations

    def correlate(self, engagements: Dict[str, dict], observations: Dict[str, dict]) -> Metrics:
        """Correlate engagement with predictions."""
        actual_ids: Set[str] = set(engagements.keys())
        predicted_ids: Set[str] = set(observations.keys())

        true_positives = actual_ids & predicted_ids  # Both
        false_positives = predicted_ids - actual_ids  # Agent predicted, T. didn't engage
        false_negatives = actual_ids - predicted_ids  # T. engaged, agent didn't predict

        logger.info("\nCorrelation breakdown:")
        logger.info("  Actual engagement: %d unique tweets", len(actual_ids))
        logger.info("  Agent predictions: %d unique tweets", len(predicted_ids))
        logger.info("  Overlap (TP):      %d", len(true_positives))
        logger.info("  Agent only (FP):   %d", len(false_positives))
        logger.info("  T. only (FN):      %d", len(false_negatives))

        return Metrics(
            total_engaged=len(actual_ids),
            total_predicted=len(predicted_ids),
            true_positives=len(true_positives),
            false_positives=len(false_positives),
            false_negatives=len(false_negatives)
        )

    def analyze_false_positives(self, engagements: Dict, observations: Dict):
        """Analyze tweets agent predicted but T. didn't engage."""
        actual_ids = set(engagements.keys())
        predicted_ids = set(observations.keys())
        false_positives = predicted_ids - actual_ids

        if not false_positives:
            logger.info("\n✓ No false positives (perfect specificity)")
            return

        logger.info("\n=== False Positives (Agent predicted, T. didn't engage) ===")
        logger.info("Sample FP tweets:")

        for i, tweet_id in enumerate(sorted(false_positives)[:5]):
            obs = observations[tweet_id]
            logger.info("  [%d] ID: %s", i + 1, tweet_id)
            logger.info("      Decision: %s (conf: %.2f)", obs.get("decision"), obs.get("confidence", 0))
            logger.info("      Signals: %s", obs.get("reasoning", []))

    def analyze_false_negatives(self, engagements: Dict, observations: Dict):
        """Analyze tweets T. engaged with but agent didn't predict."""
        actual_ids = set(engagements.keys())
        predicted_ids = set(observations.keys())
        false_negatives = actual_ids - predicted_ids

        if not false_negatives:
            logger.info("\n✓ No false negatives (perfect sensitivity)")
            return

        logger.info("\n=== False Negatives (T. engaged, agent didn't predict) ===")
        logger.info("Sample FN tweets:")

        for i, tweet_id in enumerate(sorted(false_negatives)[:5]):
            eng = engagements[tweet_id][0]  # First engagement
            logger.info("  [%d] ID: %s", i + 1, tweet_id)
            logger.info("      Action: %s (@%s)", eng.get("action"), eng.get("author"))
            logger.info("      Text: %s", eng.get("text_preview", "")[:60])

    def run(self) -> Metrics:
        """Run full correlation analysis."""
        logger.info("Engagement Correlator v%s", __version__)

        # Load data
        engagements = self.load_engagement()
        observations = self.load_observations()

        if not engagements and not observations:
            logger.error("No data to correlate. Run browser extension and agent first.")
            return None

        # Correlate
        metrics = self.correlate(engagements, observations)
        logger.info(metrics)

        # Detailed analysis
        if observations:
            self.analyze_false_positives(engagements, observations)
            self.analyze_false_negatives(engagements, observations)

        # Success interpretation
        logger.info("\n=== Interpretation ===")
        if metrics.precision > 0.60:
            logger.info("✓ HIGH PRECISION: Agent's predictions are reliable (> 60%)")
        elif metrics.precision > 0.50:
            logger.info("~ MEDIUM PRECISION: Agent shows learning signal (50-60%)")
        else:
            logger.info("✗ LOW PRECISION: Agent needs refinement (< 50%)")

        if metrics.recall > 0.50:
            logger.info("✓ HIGH RECALL: Agent caught most of T.'s interests (> 50%)")
        elif metrics.recall > 0.30:
            logger.info("~ MEDIUM RECALL: Agent caught some interests (30-50%)")
        else:
            logger.info("✗ LOW RECALL: Agent missed many interests (< 30%)")

        return metrics


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes Engagement Correlator")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("Organ directory not found: %s", organ_dir)
        return 1

    correlator = EngagementCorrelator(organ_dir)
    metrics = correlator.run()

    return 0 if metrics else 1


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
