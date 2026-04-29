#!/usr/bin/env python3
"""Measure multi-modal calibration: token + twitter + wallet fusion.

Usage:
    python measure_multi_modal.py [--json]

Outputs:
    - Console: human-readable accuracy by domain and combined
    - JSON (with --json): detailed scores for analysis
"""

import sys
import json
from typing import Optional
from dataclasses import dataclass

from dataset_builder import TokenDatasetBuilder
from token_heuristics import TokenScorer, TokenMetrics
from twitter_heuristics import TwitterScorer
from wallet_heuristics import WalletScorer


@dataclass
class MultiModalScores:
    """Scores from all three domains plus combined."""
    token_q_score: float
    twitter_q_score: float
    wallet_q_score: float
    combined_q_score: float  # Trimmed mean of the three
    expected_verdict: str
    predicted_verdict: str
    correct: bool


class MultiModalCalibrationMeasure:
    """Measure multi-modal token judgment on expanded corpus."""

    VERDICT_THRESHOLDS = {
        "Bark": (0.0, 0.236),
        "Growl": (0.236, 0.382),
        "Wag": (0.382, 0.528),
        "Howl": (0.528, 1.0),
    }

    def __init__(self):
        self.token_scorer = TokenScorer()
        self.twitter_scorer = TwitterScorer()
        self.wallet_scorer = WalletScorer()
        self.results = {
            "total": 0,
            "correct": 0,
            "accuracy": 0.0,
            "by_category": {},
            "by_domain": {
                "token_only": {"correct": 0, "total": 0, "accuracy": 0.0},
                "twitter_only": {"correct": 0, "total": 0, "accuracy": 0.0},
                "wallet_only": {"correct": 0, "total": 0, "accuracy": 0.0},
                "multi_modal": {"correct": 0, "total": 0, "accuracy": 0.0},
            },
            "scores": {},
        }

    def measure(self) -> dict:
        """Run measurement on expanded baseline corpus."""
        builder = TokenDatasetBuilder()
        corpus = builder.expand_baseline(target_size=39)  # 13 per category

        for token_data in corpus:
            expected = token_data["expected_verdict"]
            category = expected.capitalize()

            # Initialize category counters
            if category not in self.results["by_category"]:
                self.results["by_category"][category] = {
                    "total": 0,
                    "correct": 0,
                    "accuracy": 0.0,
                    "token_only_acc": 0.0,
                    "twitter_only_acc": 0.0,
                    "wallet_only_acc": 0.0,
                    "multi_modal_acc": 0.0,
                }

            self.results["total"] += 1
            self.results["by_category"][category]["total"] += 1
            self.results["by_domain"]["multi_modal"]["total"] += 1

            # Score on all three axes
            token_metrics = TokenMetrics(**token_data["token_metrics"])
            token_scores = self.token_scorer.score(token_metrics)

            twitter_signals = token_data["twitter_signals"]
            twitter_scores = self.twitter_scorer.score(twitter_signals)

            wallet_signals = token_data["wallet_signals"]
            wallet_scores = self.wallet_scorer.score(wallet_signals)

            # Combine with weighted fusion (token-heavy)
            # Token domain is most reliable (100% single-domain accuracy)
            # Twitter/wallet need real data for proper calibration
            combined_score = (
                0.6 * token_scores.q_score +
                0.2 * twitter_scores.q_score +
                0.2 * wallet_scores.q_score
            )

            # Predict verdicts
            token_verdict = self._score_to_verdict(token_scores.q_score)
            twitter_verdict = self._score_to_verdict(twitter_scores.q_score)
            wallet_verdict = self._score_to_verdict(wallet_scores.q_score)
            combined_verdict = self._score_to_verdict(combined_score)

            # Check correctness
            token_correct = token_verdict == expected
            twitter_correct = twitter_verdict == expected
            wallet_correct = wallet_verdict == expected
            combined_correct = combined_verdict == expected

            if token_correct:
                self.results["by_domain"]["token_only"]["correct"] += 1
            if twitter_correct:
                self.results["by_domain"]["twitter_only"]["correct"] += 1
            if wallet_correct:
                self.results["by_domain"]["wallet_only"]["correct"] += 1
            if combined_correct:
                self.results["correct"] += 1
                self.results["by_category"][category]["correct"] += 1
                self.results["by_domain"]["multi_modal"]["correct"] += 1

            # Store detailed result
            token_name = token_data["name"]
            self.results["scores"][token_name] = {
                "expected": expected,
                "token_verdict": token_verdict,
                "twitter_verdict": twitter_verdict,
                "wallet_verdict": wallet_verdict,
                "combined_verdict": combined_verdict,
                "token_score": token_scores.q_score,
                "twitter_score": twitter_scores.q_score,
                "wallet_score": wallet_scores.q_score,
                "combined_score": combined_score,
                "token_correct": token_correct,
                "twitter_correct": twitter_correct,
                "wallet_correct": wallet_correct,
                "combined_correct": combined_correct,
            }

        # Compute accuracies
        self.results["accuracy"] = (
            self.results["correct"] / self.results["total"]
            if self.results["total"] > 0
            else 0.0
        )

        # Compute per-domain accuracy
        for domain_key in self.results["by_domain"]:
            domain = self.results["by_domain"][domain_key]
            domain["accuracy"] = (
                domain["correct"] / domain["total"] if domain["total"] > 0 else 0.0
            )

        # Compute per-category accuracy for each domain
        for token_name, score_data in self.results["scores"].items():
            expected = score_data["expected"]
            category = expected.capitalize()

            if score_data["token_correct"]:
                # Count towards category token_only accuracy
                pass
            if score_data["twitter_correct"]:
                pass
            if score_data["wallet_correct"]:
                pass

        # Better: recompute per-category per-domain
        for category in self.results["by_category"]:
            category_data = self.results["by_category"][category]
            token_correct = sum(
                1
                for s in self.results["scores"].values()
                if s["expected"] == category and s["token_correct"]
            )
            twitter_correct = sum(
                1
                for s in self.results["scores"].values()
                if s["expected"] == category and s["twitter_correct"]
            )
            wallet_correct = sum(
                1
                for s in self.results["scores"].values()
                if s["expected"] == category and s["wallet_correct"]
            )

            category_data["token_only_acc"] = (
                token_correct / category_data["total"]
                if category_data["total"] > 0
                else 0.0
            )
            category_data["twitter_only_acc"] = (
                twitter_correct / category_data["total"]
                if category_data["total"] > 0
                else 0.0
            )
            category_data["wallet_only_acc"] = (
                wallet_correct / category_data["total"]
                if category_data["total"] > 0
                else 0.0
            )
            category_data["multi_modal_acc"] = (
                category_data["correct"] / category_data["total"]
                if category_data["total"] > 0
                else 0.0
            )

        return self.results

    def _score_to_verdict(self, score: float) -> str:
        """Map score to verdict."""
        for verdict, (low, high) in self.VERDICT_THRESHOLDS.items():
            if low <= score < high:
                return verdict
        return "Howl"

    def print_results(self) -> None:
        """Print human-readable results."""
        print("\n" + "=" * 100)
        print("MULTI-MODAL CALIBRATION MEASUREMENT RESULTS")
        print("=" * 100)

        # Overall accuracy by domain
        print("\nAccuracy by Domain:")
        for domain_key in ["token_only", "twitter_only", "wallet_only", "multi_modal"]:
            domain = self.results["by_domain"][domain_key]
            print(
                f"  {domain_key:15s}: {domain['accuracy']:6.1%} "
                f"({domain['correct']}/{domain['total']})"
            )

        # Per-category accuracy
        print("\nPer-Category Accuracy:")
        print(
            f"{'Category':10s} {'Total':6s} "
            f"{'Token':8s} {'Twitter':8s} {'Wallet':8s} {'Combined':8s}"
        )
        print("-" * 65)
        for category in sorted(self.results["by_category"].keys()):
            cat = self.results["by_category"][category]
            print(
                f"{category:10s} {cat['total']:6d} "
                f"{cat['token_only_acc']:7.1%}  {cat['twitter_only_acc']:7.1%}  "
                f"{cat['wallet_only_acc']:7.1%}  {cat['multi_modal_acc']:7.1%}"
            )

        # Improvement analysis
        print("\nMulti-Modal Advantage (combined vs best single domain):")
        for category in sorted(self.results["by_category"].keys()):
            cat = self.results["by_category"][category]
            best_single = max(
                cat["token_only_acc"],
                cat["twitter_only_acc"],
                cat["wallet_only_acc"],
            )
            improvement = cat["multi_modal_acc"] - best_single
            print(
                f"  {category:10s}: {improvement:+6.1%} "
                f"(combined={cat['multi_modal_acc']:.1%}, best_single={best_single:.1%})"
            )

        # Detailed score table (sampling for readability)
        print("\nDetailed Scores (sample):")
        print(
            f"{'Token':20s} {'Expected':8s} {'Combined':8s} "
            f"{'Token':8s} {'Twitter':8s} {'Wallet':8s} {'✓':2s}"
        )
        print("-" * 85)

        # Print first 5 of each category
        for category in sorted(self.results["by_category"].keys()):
            count = 0
            for token_name in sorted(self.results["scores"].keys()):
                score_data = self.results["scores"][token_name]
                if score_data["expected"] == category and count < 5:
                    check = "✓" if score_data["combined_correct"] else "✗"
                    print(
                        f"{token_name:20s} {score_data['expected']:8s} "
                        f"{score_data['combined_verdict']:8s} "
                        f"{score_data['token_score']:8.3f} {score_data['twitter_score']:8.3f} "
                        f"{score_data['wallet_score']:8.3f} {check:2s}"
                    )
                    count += 1

        print("=" * 100 + "\n")

    def save_json(self, output_path: Optional[str] = None) -> str:
        """Save results as JSON."""
        if output_path is None:
            output_path = "/tmp/multi_modal_calibration_results.json"

        with open(output_path, "w") as f:
            json.dump(self.results, f, indent=2)

        return output_path


def main():
    measure = MultiModalCalibrationMeasure()
    measure.measure()
    measure.print_results()

    if "--json" in sys.argv:
        json_path = measure.save_json()
        print(f"Results saved to: {json_path}")


if __name__ == "__main__":
    main()
