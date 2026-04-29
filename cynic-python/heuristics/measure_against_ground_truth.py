#!/usr/bin/env python3
"""Measure calibration against ground-truth labels (CultScreener).

Loads a ground-truth dataset and measures how well our heuristics align
with independent expert labels. This answers:
1. How well does token domain predict expert risk assessments?
2. How much does twitter/wallet domain add?
3. What are our blind spots (confusion matrix)?

Usage:
    python measure_against_ground_truth.py \
        --dataset ~/.cynic/datasets/tokens/ground_truth.json
"""

import json
import sys
import argparse
from pathlib import Path
from typing import Dict, List, Optional
from dataclasses import dataclass

from token_heuristics import TokenScorer, TokenMetrics
from twitter_heuristics import TwitterScorer
from wallet_heuristics import WalletScorer
from dataset_builder import TwitterSignals, WalletSignals


@dataclass
class MeasurementResult:
    """Result for a single token."""
    mint: str
    symbol: str
    ground_truth: str  # CultScreener verdict
    token_verdict: str
    twitter_verdict: str
    wallet_verdict: str
    fused_verdict: str
    token_score: float
    fused_score: float
    correct: bool


class GroundTruthMeasure:
    """Measure calibration against CultScreener ground truth."""

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
        self.results: List[MeasurementResult] = []

    def load_dataset(self, json_path: str) -> List[Dict]:
        """Load ground-truth dataset from JSON."""
        with open(json_path, "r") as f:
            return json.load(f)

    def measure_dataset(self, dataset: List[Dict]) -> Dict:
        """Measure calibration across all tokens in dataset."""
        if not dataset:
            print("ERROR: Dataset is empty")
            return {}

        for token_data in dataset:
            result = self.measure_token(token_data)
            if result:
                self.results.append(result)

        return self.summarize()

    def measure_token(self, token_data: Dict) -> Optional[MeasurementResult]:
        """Measure a single token against ground truth."""
        mint = token_data.get("mint")
        symbol = token_data.get("symbol", "?")
        ground_truth = token_data.get("verdict")  # CultScreener label

        # Reconstruct signals (may be None if enrichment failed)
        wallet_signals = token_data.get("wallet_signals")
        twitter_signals = token_data.get("twitter_signals")

        if not wallet_signals:
            return None  # Can't score without on-chain data

        try:
            # Convert dicts back to dataclasses
            wallet_obj = WalletSignals(**wallet_signals) if wallet_signals else None
            twitter_obj = TwitterSignals(**twitter_signals) if twitter_signals else None

            # Score on token domain (on-chain only)
            # Reconstruct TokenMetrics from wallet_signals (hack: we don't have original token metrics)
            # For now, estimate based on wallet signals
            token_metrics = self._estimate_token_metrics(wallet_signals)
            token_scores = self.token_scorer.score(token_metrics)
            token_verdict = self._score_to_verdict(token_scores.q_score)

            # Score on twitter domain
            twitter_verdict = "?"
            twitter_score = 0.0
            if twitter_obj:
                twitter_scores = self.twitter_scorer.score(twitter_obj)
                twitter_verdict = self._score_to_verdict(twitter_scores.q_score)
                twitter_score = twitter_scores.q_score
            else:
                twitter_verdict = "?"

            # Score on wallet domain
            wallet_verdict = "?"
            wallet_score = 0.0
            if wallet_obj:
                wallet_scores = self.wallet_scorer.score(wallet_obj)
                wallet_verdict = self._score_to_verdict(wallet_scores.q_score)
                wallet_score = wallet_scores.q_score
            else:
                wallet_verdict = "?"

            # Fused score (weighted)
            fused_score = (
                0.6 * token_scores.q_score +
                0.2 * (twitter_score if twitter_obj else token_scores.q_score) +
                0.2 * (wallet_score if wallet_obj else token_scores.q_score)
            )
            fused_verdict = self._score_to_verdict(fused_score)

            correct = fused_verdict == ground_truth

            return MeasurementResult(
                mint=mint,
                symbol=symbol,
                ground_truth=ground_truth,
                token_verdict=token_verdict,
                twitter_verdict=twitter_verdict,
                wallet_verdict=wallet_verdict,
                fused_verdict=fused_verdict,
                token_score=token_scores.q_score,
                fused_score=fused_score,
                correct=correct,
            )
        except Exception as e:
            print(f"Error scoring {symbol} ({mint[:8]}): {e}")
            return None

    def _estimate_token_metrics(self, wallet_signals: Dict) -> TokenMetrics:
        """Estimate TokenMetrics from wallet signals (hack for ground truth measurement).

        In real usage, we'd have actual token metrics. Here we infer from holder behavior.
        """
        ws = wallet_signals
        return TokenMetrics(
            holders=max(10, int(1000 / (1.0 + ws.get("top_10_hold_pct", 0) / 10))),
            top1_pct=ws.get("top_10_hold_pct", 0) / 3,  # Rough estimate
            top10_pct=ws.get("top_10_hold_pct", 0),
            herfindahl=min(1.0, ws.get("top_10_hold_pct", 0) / 100),
            age_hours=max(24, int(ws.get("avg_hold_duration_days", 7) * 24)),
            mint_authority_active=ws.get("bot_score", 0) > 0.3,
            freeze_authority_active=ws.get("bot_score", 0) > 0.3,
            lp_burned=ws.get("retail_held_pct", 0) > 50,
            lp_locked=False,
            supply_burned_pct=None,
            origin_pump_fun=False,  # Unknown from wallet data
            exchange_listed=ws.get("exchange_held_pct", 0) < 15,
        )

    def _score_to_verdict(self, score: float) -> str:
        """Map score to verdict."""
        for verdict, (low, high) in self.VERDICT_THRESHOLDS.items():
            if low <= score < high:
                return verdict
        return "Howl"

    def summarize(self) -> Dict:
        """Generate summary statistics."""
        if not self.results:
            return {"total": 0, "correct": 0, "accuracy": 0.0}

        correct = sum(1 for r in self.results if r.correct)
        total = len(self.results)

        # Accuracy by ground truth verdict
        by_verdict = {}
        for verdict in ["Bark", "Growl", "Wag", "Howl"]:
            matching = [r for r in self.results if r.ground_truth == verdict]
            if matching:
                correct_matching = sum(1 for r in matching if r.correct)
                by_verdict[verdict] = {
                    "total": len(matching),
                    "correct": correct_matching,
                    "accuracy": correct_matching / len(matching),
                }

        # Confusion matrix
        confusion = {}
        for r in self.results:
            key = f"{r.ground_truth}→{r.fused_verdict}"
            confusion[key] = confusion.get(key, 0) + 1

        return {
            "total": total,
            "correct": correct,
            "accuracy": correct / total if total > 0 else 0.0,
            "by_verdict": by_verdict,
            "confusion_matrix": confusion,
        }

    def print_summary(self) -> None:
        """Print human-readable summary."""
        summary = self.summarize()

        print("\n" + "=" * 90)
        print("GROUND TRUTH CALIBRATION MEASUREMENT")
        print("=" * 90)

        print(f"\nOverall Accuracy: {summary['accuracy']:.1%} ({summary['correct']}/{summary['total']})")

        print("\nAccuracy by Ground Truth Verdict:")
        for verdict in sorted(summary.get("by_verdict", {}).keys()):
            v_stats = summary["by_verdict"][verdict]
            print(
                f"  {verdict:6s}: {v_stats['accuracy']:6.1%} "
                f"({v_stats['correct']}/{v_stats['total']})"
            )

        print("\nConfusion Matrix (ground_truth→predicted):")
        for key in sorted(summary.get("confusion_matrix", {}).keys()):
            count = summary["confusion_matrix"][key]
            print(f"  {key}: {count}")

        print("\nDetailed Results (first 10):")
        print(
            f"{'Symbol':8s} {'Truth':6s} {'Pred':6s} "
            f"{'Token':8s} {'Fused':8s} {'✓':2s}"
        )
        print("-" * 60)
        for result in self.results[:10]:
            check = "✓" if result.correct else "✗"
            print(
                f"{result.symbol:8s} {result.ground_truth:6s} {result.fused_verdict:6s} "
                f"{result.token_verdict:8s} {result.fused_score:8.3f} {check:2s}"
            )

        print("=" * 90 + "\n")


def main():
    parser = argparse.ArgumentParser(
        description="Measure calibration against ground-truth labels"
    )
    parser.add_argument(
        "--dataset",
        type=str,
        default=str(Path.home() / ".cynic/datasets/tokens/ground_truth.json"),
        help="Path to ground-truth dataset JSON",
    )
    args = parser.parse_args()

    # Load dataset
    dataset_path = Path(args.dataset)
    if not dataset_path.exists():
        print(f"ERROR: Dataset not found: {args.dataset}")
        print("Run: python token_dataset_ingester.py")
        return

    print(f"Loading ground-truth dataset: {args.dataset}")
    measure = GroundTruthMeasure()
    dataset = measure.load_dataset(str(dataset_path))

    print(f"Loaded {len(dataset)} tokens")
    print("Measuring calibration...\n")

    measure.measure_dataset(dataset)
    measure.print_summary()


if __name__ == "__main__":
    main()
