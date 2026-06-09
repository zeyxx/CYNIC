#!/usr/bin/env python3
"""Measure token calibration accuracy on baseline corpus.

Usage:
    python measure_token_calibration.py [--json]

Outputs:
    - Console: human-readable results
    - JSON (with --json): machine-readable for further analysis
"""

import sys
import json
from typing import Optional
from pathlib import Path

from token_heuristics import TokenScorer, TokenMetrics, clamp


class TokenCalibrationMeasure:
    """Measure token-judgment calibration against baseline."""

    # Map label patterns to expected verdicts
    LABEL_VERDICT_MAP = {
        "BARK": "Bark",      # 3 rugs: classic_rug, freeze_trap, copycat
        "HOWL": "Howl",      # 3 legitimate: jup, bonk, defi
        "GROWL": "Growl",    # 3 ambiguous: pumpfun_legit, new_clean, concentrated
    }

    # Baseline token metadata (extracted from baseline corpus)
    BASELINE_TOKENS = {
        "BARK_classic_rug": {
            "expected": "Bark",
            "holders": 50,
            "top1_pct": 45.0,
            "top10_pct": 85.0,
            "herfindahl": 0.45,
            "age_hours": 12,
            "mint_authority_active": True,
            "freeze_authority_active": True,
            "lp_burned": False,
            "lp_locked": False,
            "supply_burned_pct": None,
            "origin_pump_fun": True,
        },
        "BARK_freeze_trap": {
            "expected": "Bark",
            "holders": 45,
            "top1_pct": 50.0,
            "top10_pct": 82.0,
            "herfindahl": 0.50,
            "age_hours": 6,
            "mint_authority_active": True,
            "freeze_authority_active": True,
            "lp_burned": False,
            "lp_locked": False,
            "supply_burned_pct": None,
            "origin_pump_fun": True,
        },
        "BARK_copycat": {
            "expected": "Bark",
            "holders": 55,
            "top1_pct": 42.0,
            "top10_pct": 80.0,
            "herfindahl": 0.42,
            "age_hours": 18,
            "mint_authority_active": True,
            "freeze_authority_active": True,
            "lp_burned": False,
            "lp_locked": False,
            "supply_burned_pct": None,
            "origin_pump_fun": True,
        },
        "HOWL_jup": {
            "expected": "Howl",
            "holders": 580_000,
            "top1_pct": 1.2,
            "top10_pct": 5.0,
            "herfindahl": 0.05,
            "age_hours": 8760,  # ~1 year
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 30.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "HOWL_bonk": {
            "expected": "Howl",
            "holders": 1_200_000,
            "top1_pct": 0.8,
            "top10_pct": 3.5,
            "herfindahl": 0.03,
            "age_hours": 10_000,  # ~1.1 years
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 50.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "HOWL_defi": {
            "expected": "Howl",
            "holders": 125_000,
            "top1_pct": 2.5,
            "top10_pct": 8.0,
            "herfindahl": 0.12,
            "age_hours": 7_200,  # ~300 days
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 25.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "GROWL_pumpfun_legit": {
            "expected": "Growl",
            "holders": 500,
            "top1_pct": 8.0,
            "top10_pct": 35.0,
            "herfindahl": 0.20,
            "age_hours": 2_000,  # ~83 days
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 15.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
        "GROWL_new_clean": {
            "expected": "Growl",
            "holders": 200,
            "top1_pct": 5.0,
            "top10_pct": 25.0,
            "herfindahl": 0.15,
            "age_hours": 120,  # ~5 days
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 10.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
        "GROWL_concentrated": {
            "expected": "Growl",
            "holders": 300,
            "top1_pct": 25.0,
            "top10_pct": 65.0,
            "herfindahl": 0.35,
            "age_hours": 600,  # ~25 days
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 20.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
    }

    VERDICT_THRESHOLDS = {
        "Bark": (0.0, 0.236),
        "Growl": (0.236, 0.382),
        "Wag": (0.382, 0.528),
        "Howl": (0.528, 1.0),
    }

    def __init__(self):
        self.scorer = TokenScorer()
        self.results = {
            "total": 0,
            "correct": 0,
            "accuracy": 0.0,
            "by_category": {},
            "misclassifications": [],
            "scores": {},
        }

    def measure(self) -> dict:
        """Run measurement on all baseline tokens."""
        for label, token_data in self.BASELINE_TOKENS.items():
            expected = token_data["expected"]
            category = expected.capitalize()

            # Initialize category counters
            if category not in self.results["by_category"]:
                self.results["by_category"][category] = {
                    "total": 0,
                    "correct": 0,
                    "accuracy": 0.0,
                }

            self.results["total"] += 1
            self.results["by_category"][category]["total"] += 1

            # Create metrics object
            metrics = TokenMetrics(
                holders=token_data["holders"],
                top1_pct=token_data["top1_pct"],
                top10_pct=token_data["top10_pct"],
                herfindahl=token_data.get("herfindahl"),
                age_hours=token_data["age_hours"],
                mint_authority_active=token_data["mint_authority_active"],
                freeze_authority_active=token_data["freeze_authority_active"],
                lp_burned=token_data["lp_burned"],
                lp_locked=token_data["lp_locked"],
                supply_burned_pct=token_data.get("supply_burned_pct"),
                origin_pump_fun=token_data["origin_pump_fun"],
                exchange_listed=token_data.get("exchange_listed", False),
            )

            # Score the token
            scores = self.scorer.score(metrics)

            # Determine predicted verdict
            predicted = self._score_to_verdict(scores.q_score)

            # Check correctness
            is_correct = predicted == expected
            if is_correct:
                self.results["correct"] += 1
                self.results["by_category"][category]["correct"] += 1

            # Store result
            self.results["scores"][label] = {
                "expected": expected,
                "predicted": predicted,
                "q_score": scores.q_score,
                "fidelity": scores.fidelity,
                "phi": scores.phi,
                "verify": scores.verify,
                "culture": scores.culture,
                "burn": scores.burn,
                "sovereignty": scores.sovereignty,
                "correct": is_correct,
            }

            if not is_correct:
                self.results["misclassifications"].append({
                    "token": label,
                    "expected": expected,
                    "predicted": predicted,
                    "q_score": scores.q_score,
                })

        # Compute accuracy
        self.results["accuracy"] = (
            self.results["correct"] / self.results["total"]
            if self.results["total"] > 0
            else 0.0
        )

        # Compute per-category accuracy
        for category in self.results["by_category"]:
            cat = self.results["by_category"][category]
            cat["accuracy"] = cat["correct"] / cat["total"] if cat["total"] > 0 else 0.0

        return self.results

    def _score_to_verdict(self, score: float) -> str:
        """Map score to verdict."""
        for verdict, (low, high) in self.VERDICT_THRESHOLDS.items():
            if low <= score < high:
                return verdict
        return "Howl"  # Default if above all thresholds

    def print_results(self) -> None:
        """Print human-readable results."""
        print("\n" + "=" * 70)
        print("TOKEN CALIBRATION MEASUREMENT RESULTS")
        print("=" * 70)

        # Overall accuracy
        print(f"\nOverall Accuracy: {self.results['accuracy']:.1%}")
        print(f"Correct: {self.results['correct']}/{self.results['total']}")

        # Per-category accuracy
        print("\nPer-Category Accuracy:")
        for category in sorted(self.results["by_category"].keys()):
            cat = self.results["by_category"][category]
            print(
                f"  {category:6s}: {cat['accuracy']:6.1%} "
                f"({cat['correct']}/{cat['total']})"
            )

        # Misclassifications
        if self.results["misclassifications"]:
            print(f"\nMisclassifications ({len(self.results['misclassifications'])}):")
            for mis in self.results["misclassifications"]:
                print(
                    f"  {mis['token']:25s} "
                    f"expected={mis['expected']:6s} "
                    f"predicted={mis['predicted']:6s} "
                    f"score={mis['q_score']:6.3f}"
                )

        # Per-token scores (for debugging)
        print("\nDetailed Scores:")
        print(
            f"{'Token':25s} {'Expected':8s} {'Predicted':8s} "
            f"{'q_score':8s} {'FID':5s} {'PHI':5s} {'VER':5s} "
            f"{'CUL':5s} {'BRN':5s} {'SOV':5s} {'✓':2s}"
        )
        print("-" * 115)

        for label, score_data in sorted(self.results["scores"].items()):
            check = "✓" if score_data["correct"] else "✗"
            print(
                f"{label:25s} {score_data['expected']:8s} {score_data['predicted']:8s} "
                f"{score_data['q_score']:8.3f} "
                f"{score_data['fidelity']:5.3f} {score_data['phi']:5.3f} {score_data['verify']:5.3f} "
                f"{score_data['culture']:5.3f} {score_data['burn']:5.3f} {score_data['sovereignty']:5.3f} "
                f"{check:2s}"
            )

        print("=" * 70 + "\n")

    def save_json(self, output_path: Optional[str] = None) -> str:
        """Save results as JSON."""
        if output_path is None:
            output_path = "/tmp/token_calibration_results.json"

        with open(output_path, "w") as f:
            json.dump(self.results, f, indent=2)

        return output_path


def main():
    measure = TokenCalibrationMeasure()
    measure.measure()
    measure.print_results()

    if "--json" in sys.argv:
        json_path = measure.save_json()
        print(f"Results saved to: {json_path}")


if __name__ == "__main__":
    main()
