#!/usr/bin/env python3
"""Compare v1 (raw) vs v2 (stopword-filtered) clustering quality."""

import json
import sys
from pathlib import Path


def load_metrics(version: str) -> dict:
    """Load metrics for a version."""
    if version == "v1":
        path = "results_v1/metrics_v1.json"
    else:
        path = f"../domain-discovery/v2/results_v2/metrics_v2.json"

    try:
        with open(path) as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"✗ Metrics not found: {path}")
        return None


def compare_silhouette():
    """Compare silhouette scores between v1 and v2."""
    print("=" * 80)
    print("DOMAIN DISCOVERY: v1 vs v2 IMPROVEMENT COMPARISON")
    print("=" * 80)

    m1 = load_metrics("v1")
    m2 = load_metrics("v2")

    if m1 is None or m2 is None:
        print("\n✗ Cannot compare: missing metrics")
        print(f"  v1 metrics: {'✓' if m1 else '✗'}")
        print(f"  v2 metrics: {'✓' if m2 else '✗'}")
        return

    print("\nSilhouette Score (cluster quality metric, target > 0.5):")
    print(f"  v1 (raw): {m1['silhouette']:.3f}")
    print(f"  v2 (filtered): {m2['silhouette']:.3f}")

    delta = m2["silhouette"] - m1["silhouette"]
    if delta > 0.05:
        print(f"  ✓ Improvement: +{delta:.3f} (stopword filtering helps)")
    elif delta > -0.05:
        print(f"  ~ Similar: {delta:+.3f} (no significant change)")
    else:
        print(f"  ✗ Regression: {delta:.3f} (stopword filtering hurts)")

    print(f"\nVocabulary Size (smaller is cleaner):")
    print(f"  v1: {m1['vocabulary_size']} words")
    print(f"  v2: {m2['vocabulary_size']} words")
    vocab_reduction = (1 - m2["vocabulary_size"] / m1["vocabulary_size"]) * 100
    print(f"  Reduction: {vocab_reduction:.1f}%")

    print(f"\nClusters (should remain stable):")
    print(f"  v1: {m1['n_clusters']} clusters")
    print(f"  v2: {m2['n_clusters']} clusters")

    # Decision rule
    print("\n" + "=" * 80)
    if delta > 0.05:
        print("✓ RECOMMENDATION: Accept v2 (improvement confirmed)")
        print("  → Use v2 for Phase 2 integration")
    elif delta > -0.05:
        print("~ RECOMMENDATION: Keep v1 (v2 similar, no clear winner)")
        print("  → Use v1 unless v2 runs faster")
    else:
        print("✗ RECOMMENDATION: Keep v1 (v2 regression)")
        print("  → Stopword filtering not beneficial for this domain")

    print("=" * 80 + "\n")


if __name__ == "__main__":
    compare_silhouette()
