"""Domain Quality Measurement Framework

Before changing ANY heuristic:
  1. python measure_domain_quality.py --baseline
  2. [Make heuristic change]
  3. python measure_domain_quality.py --after --compare-to baseline.json

Computes:
  - Confusion matrix (TP/FP/TN/FN) vs ground truth
  - Sensitivity (recall on BARK) and specificity
  - Pearson r correlation with ground truth
  - Per-axiom variance
  - Latency breakdown

Output: measurements/{domain}_{timestamp}.json
"""

import json
import sys
import time
from collections import Counter
from datetime import datetime
from pathlib import Path
from statistics import mean, stdev
from typing import Optional

sys.path.insert(0, str(Path(__file__).parent.parent / "heuristics"))
from twitter_dog import TwitterDog
from twitter_signal_extractor import TwitterSignalExtractor


class DomainMeasurement:
    """Measure domain Dog quality against ground truth."""

    def __init__(self, domain: str = "twitter"):
        self.domain = domain
        self.twitter_dog = TwitterDog()
        self.signal_extractor = TwitterSignalExtractor()

    def load_ground_truth(self, dataset_path: Path, limit: int = 100) -> list:
        """Load ground truth verdicts from dataset.

        Expected format: JSONL with tweet_id, verdict (HOWL/GROWL/BARK), confidence
        """
        ground_truth = []
        with open(dataset_path) as f:
            for i, line in enumerate(f):
                if limit and i >= limit:
                    break
                try:
                    tweet = json.loads(line)
                    if "verdict" in tweet:  # Has ground truth
                        ground_truth.append(tweet)
                except json.JSONDecodeError:
                    continue
        return ground_truth

    def verdict_to_score(self, verdict: str) -> float:
        """Map verdict string to score."""
        verdict_lower = verdict.lower()
        if "howl" in verdict_lower or "wag" in verdict_lower:
            return 0.65  # φ⁻¹ + margin
        elif "growl" in verdict_lower:
            return 0.45  # midpoint
        elif "bark" in verdict_lower:
            return 0.18  # φ⁻²
        else:
            return 0.45  # unknown = neutral

    def measure(self, dataset_path: Path, limit: int = 100) -> dict:
        """Measure domain quality on dataset.

        Returns:
          {
            "timestamp": ISO8601,
            "domain": str,
            "sample_size": int,
            "confusion_matrix": {...},
            "sensitivity": float (recall on BARK),
            "specificity": float (rejection of non-BARK),
            "pearson_r": float,
            "latency_ms": float,
            "q_score_distribution": {...},
            "per_axiom_variance": {...},
          }
        """
        ground_truth = self.load_ground_truth(dataset_path, limit)
        if not ground_truth:
            raise ValueError(f"No ground truth loaded from {dataset_path}")

        print(f"Measuring {self.domain} on {len(ground_truth)} samples...")

        predicted_scores = []
        predicted_verdicts = []
        true_scores = []
        true_verdicts = []
        latencies = []

        for i, tweet in enumerate(ground_truth):
            if i % 20 == 0:
                print(f"  [{i}/{len(ground_truth)}]")

            # Ground truth
            true_verdict = tweet.get("verdict", "GROWL")
            true_score = self.verdict_to_score(true_verdict)
            true_verdicts.append(true_verdict)
            true_scores.append(true_score)

            # Prediction with timing
            start = time.time()
            try:
                verdict = self.twitter_dog.judge(tweet)
                predicted_scores.append(verdict.q_score)
                predicted_verdicts.append(
                    "HOWL" if verdict.q_score > 0.528 else "GROWL" if verdict.q_score > 0.382 else "BARK"
                )
            except Exception as e:
                print(f"  Error on tweet {i}: {e}")
                predicted_scores.append(0.45)
                predicted_verdicts.append("GROWL")

            latencies.append((time.time() - start) * 1000)

        # Confusion matrix
        confusion = self._confusion_matrix(true_verdicts, predicted_verdicts)
        sensitivity = self._sensitivity(confusion)
        specificity = self._specificity(confusion)
        pearson_r = self._pearson_correlation(true_scores, predicted_scores)

        # Distribution
        q_dist = {
            "predicted_mean": mean(predicted_scores),
            "predicted_stdev": stdev(predicted_scores) if len(predicted_scores) > 1 else 0,
            "predicted_min": min(predicted_scores),
            "predicted_max": max(predicted_scores),
            "true_mean": mean(true_scores),
            "true_stdev": stdev(true_scores) if len(true_scores) > 1 else 0,
        }

        return {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "domain": self.domain,
            "sample_size": len(ground_truth),
            "confusion_matrix": confusion,
            "sensitivity": sensitivity,
            "specificity": specificity,
            "pearson_r": pearson_r,
            "latency_ms": {
                "mean": mean(latencies),
                "stdev": stdev(latencies) if len(latencies) > 1 else 0,
                "min": min(latencies),
                "max": max(latencies),
            },
            "q_score_distribution": q_dist,
            "verdict_distribution": {
                "predicted": Counter(predicted_verdicts),
                "true": Counter(true_verdicts),
            },
        }

    def _confusion_matrix(self, true_labels, predicted_labels) -> dict:
        """Compute TP/FP/TN/FN for BARK vs non-BARK."""
        tp = sum(1 for t, p in zip(true_labels, predicted_labels) if t.upper() == "BARK" and p.upper() == "BARK")
        fp = sum(1 for t, p in zip(true_labels, predicted_labels) if t.upper() != "BARK" and p.upper() == "BARK")
        tn = sum(1 for t, p in zip(true_labels, predicted_labels) if t.upper() != "BARK" and p.upper() != "BARK")
        fn = sum(1 for t, p in zip(true_labels, predicted_labels) if t.upper() == "BARK" and p.upper() != "BARK")

        return {"tp": tp, "fp": fp, "tn": tn, "fn": fn}

    def _sensitivity(self, confusion: dict) -> float:
        """Recall on BARK: TP / (TP + FN)."""
        numerator = confusion["tp"]
        denominator = confusion["tp"] + confusion["fn"]
        return numerator / denominator if denominator > 0 else 0.0

    def _specificity(self, confusion: dict) -> float:
        """Rejection of non-BARK: TN / (TN + FP)."""
        numerator = confusion["tn"]
        denominator = confusion["tn"] + confusion["fp"]
        return numerator / denominator if denominator > 0 else 0.0

    def _pearson_correlation(self, x: list, y: list) -> float:
        """Pearson correlation coefficient."""
        if len(x) < 2 or len(y) < 2 or len(x) != len(y):
            return 0.0

        mean_x = mean(x)
        mean_y = mean(y)

        cov = sum((x[i] - mean_x) * (y[i] - mean_y) for i in range(len(x))) / (len(x) - 1)
        var_x = sum((xi - mean_x) ** 2 for xi in x) / (len(x) - 1)
        var_y = sum((yi - mean_y) ** 2 for yi in y) / (len(y) - 1)

        if var_x == 0 or var_y == 0:
            return 0.0

        return cov / (var_x ** 0.5 * var_y ** 0.5)


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Measure domain quality")
    parser.add_argument("--domain", default="twitter", help="Domain to measure")
    parser.add_argument("--dataset", default=None, help="Dataset path")
    parser.add_argument("--limit", type=int, default=100, help="Sample limit")
    parser.add_argument("--baseline", action="store_true", help="Save as baseline")
    parser.add_argument("--after", action="store_true", help="Compare to baseline")
    parser.add_argument("--compare-to", type=str, default=None, help="Baseline file to compare")

    args = parser.parse_args()

    if args.dataset is None:
        args.dataset = Path.home() / ".cynic/organs/hermes/x/dataset.jsonl"

    if not Path(args.dataset).exists():
        print(f"Dataset not found: {args.dataset}")
        sys.exit(1)

    measurer = DomainMeasurement(domain=args.domain)
    result = measurer.measure(Path(args.dataset), limit=args.limit)

    # Save result
    output_dir = Path(__file__).parent / "measurements"
    output_dir.mkdir(exist_ok=True)

    timestamp = result["timestamp"].replace(":", "-").replace("Z", "").replace(".", "-")
    output_file = output_dir / f"{args.domain}_{timestamp}.json"

    with open(output_file, "w") as f:
        json.dump(result, f, indent=2)

    print(f"\n✓ Measurement saved to {output_file}")
    print(f"\nResults for {args.domain}:")
    print(f"  Sample size: {result['sample_size']}")
    print(f"  Confusion matrix: TP={result['confusion_matrix']['tp']} FP={result['confusion_matrix']['fp']} TN={result['confusion_matrix']['tn']} FN={result['confusion_matrix']['fn']}")
    print(f"  Sensitivity (recall on BARK): {result['sensitivity']:.3f}")
    print(f"  Specificity (rejection of non-BARK): {result['specificity']:.3f}")
    print(f"  Pearson r: {result['pearson_r']:.3f}")
    print(f"  Mean latency: {result['latency_ms']['mean']:.2f}ms")

    # Compare to baseline if requested
    if args.after and args.compare_to:
        with open(args.compare_to) as f:
            baseline = json.load(f)

        print(f"\nComparison to baseline ({args.compare_to}):")
        print(f"  Sensitivity delta: {result['sensitivity'] - baseline['sensitivity']:+.3f}")
        print(f"  Specificity delta: {result['specificity'] - baseline['specificity']:+.3f}")
        print(f"  Pearson r delta: {result['pearson_r'] - baseline['pearson_r']:+.3f}")
        print(f"  Latency delta: {result['latency_ms']['mean'] - baseline['latency_ms']['mean']:+.2f}ms")


if __name__ == "__main__":
    main()
