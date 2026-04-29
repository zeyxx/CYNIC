#!/usr/bin/env python3
"""Measure conviction→verdict mapping accuracy.

Simple baseline: does CultScreener conviction score map correctly to CYNIC verdicts?

Target: 75%+ accuracy (validation that conviction is a strong signal).
"""

import json
import sys
from pathlib import Path

def load_dataset(path: str):
    """Load conviction-only dataset."""
    with open(path) as f:
        return json.load(f)

def measure_accuracy(dataset):
    """Measure conviction→verdict mapping accuracy."""
    if not dataset:
        return {
            "total": 0,
            "correct": 0,
            "accuracy": 0.0,
            "by_verdict": {},
            "confusion": {},
        }

    correct = 0
    total = len(dataset)
    by_verdict = {}
    confusion = {}

    for token in dataset:
        verdict = token["verdict"]
        conviction = token["cultscreener_conviction"]

        # Count by verdict
        if verdict not in by_verdict:
            by_verdict[verdict] = {"total": 0, "correct": 0}
        by_verdict[verdict]["total"] += 1

        # Validate conviction→verdict mapping
        # Using ConvictionTier thresholds: 0.7→HOWL, 0.4-0.7→GROWL, <0.4→BARK
        predicted_verdict = None
        if conviction >= 0.7:
            predicted_verdict = "Howl"
        elif conviction >= 0.4:
            predicted_verdict = "Growl"
        else:
            predicted_verdict = "Bark"

        # Check if correct
        is_correct = (predicted_verdict == verdict)
        if is_correct:
            correct += 1
            by_verdict[verdict]["correct"] += 1

        # Track confusion
        key = f"{verdict}→{predicted_verdict}"
        confusion[key] = confusion.get(key, 0) + 1

    accuracy = (correct / total * 100) if total > 0 else 0.0

    return {
        "total": total,
        "correct": correct,
        "accuracy": accuracy,
        "by_verdict": by_verdict,
        "confusion": confusion,
    }

def main():
    dataset_path = Path.home() / ".cynic/datasets/tokens/ground_truth_conviction_only.json"

    if len(sys.argv) > 1:
        dataset_path = sys.argv[1]

    if not dataset_path.exists():
        print(f"❌ Dataset not found: {dataset_path}")
        sys.exit(1)

    print(f"Loading dataset: {dataset_path}")
    dataset = load_dataset(str(dataset_path))
    print(f"Loaded {len(dataset)} tokens\n")

    result = measure_accuracy(dataset)

    print("=" * 70)
    print("CONVICTION→VERDICT MAPPING ACCURACY")
    print("=" * 70)
    print(f"Overall Accuracy: {result['accuracy']:.1f}% ({result['correct']}/{result['total']})")
    print(f"Target: 75%+ (validation threshold)\n")

    print("Accuracy by Ground Truth Verdict:")
    for verdict in ["Bark", "Growl", "Howl"]:
        if verdict in result["by_verdict"]:
            stats = result["by_verdict"][verdict]
            acc = (
                stats["correct"] / stats["total"] * 100
                if stats["total"] > 0
                else 0.0
            )
            print(f"  {verdict:6s}: {acc:5.1f}% ({stats['correct']}/{stats['total']})")

    print("\nConfusion Matrix (truth→predicted):")
    for key, count in sorted(result["confusion"].items()):
        print(f"  {key}: {count}")

    print("\n" + "=" * 70)
    if result["accuracy"] >= 75:
        print(f"✅ PASSED: {result['accuracy']:.1f}% ≥ 75% target")
        print("Conviction score is a strong signal for verdict classification.")
    elif result["accuracy"] >= 50:
        print(f"⚠ PARTIAL: {result['accuracy']:.1f}% (below 75% target)")
        print("Conviction is useful but needs enrichment for better calibration.")
    else:
        print(f"❌ FAILED: {result['accuracy']:.1f}% (below 50% baseline)")
        print("Conviction mapping needs review.")

    return 0 if result["accuracy"] >= 75 else 1

if __name__ == "__main__":
    sys.exit(main())
