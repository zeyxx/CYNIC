#!/usr/bin/env python3
"""
Falsification Test 2: Real Corpus Collection + ROC-AUC Measurement

Runs wallet behavior scoring on real corpus, measures ROC-AUC.
Target: ROC-AUC > 0.7 (human vs sybil classification accuracy)
"""

import json
import sys
from pathlib import Path

from wallet_corpus_builder import build_corpus, KNOWN_HUMANS, KNOWN_SYBILS
from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_validator import WalletValidator

print("=== Falsification Test 2: Wallet Corpus Measurement ===\n")

# Step 1: Build corpus (fetch real data from Helius)
print("Step 1: Building corpus from Helius...")
print(f"  Humans to fetch: {len(KNOWN_HUMANS)}")
print(f"  Sybils to fetch: {len(KNOWN_SYBILS)}")

collector = HeliusWalletCollector()
corpus_path = build_corpus(collector, output_file="validation_corpus_live.json", include_humans=True, include_sybils=True)

print(f"\n✓ Corpus saved to {corpus_path}")

# Step 2: Load corpus and validate
print("\nStep 2: Loading and validating corpus...")
try:
    profiles, labels = WalletValidator.load_corpus(corpus_path)
    human_count = sum(1 for label in labels if label)
    sybil_count = len(labels) - human_count
    print(f"✓ Loaded {len(profiles)} wallets ({human_count}H + {sybil_count}S)")
except FileNotFoundError:
    print(f"❌ Corpus file not found: {corpus_path}")
    sys.exit(1)
except Exception as e:
    print(f"❌ Error loading corpus: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)

# Step 3: Measure ROC-AUC
print("\nStep 3: Measuring ROC-AUC...")
try:
    result = WalletValidator.validate(profiles, labels)
    print(f"\n✓ Measurement complete:")
    print(f"  Human avg score: {result.human_avg_score:.3f}")
    print(f"  Sybil avg score: {result.sybil_avg_score:.3f}")
    print(f"  Score separation: {result.score_separation:.3f}")
    print(f"  ROC-AUC: {result.roc_auc:.3f}")
    print(f"  Accuracy: {result.accuracy:.1%}")

    if result.roc_auc > 0.7:
        print(f"\n✅ FALSIFICATION PASS: ROC-AUC={result.roc_auc:.3f} > 0.7")
        print(f"  {result.true_positives} TP, {result.true_negatives} TN, {result.false_positives} FP, {result.false_negatives} FN")
        sys.exit(0)
    else:
        print(f"\n⚠️  FALSIFICATION FAIL: ROC-AUC={result.roc_auc:.3f} <= 0.7")
        print("Next: Expand corpus with more diverse wallets, recalibrate heuristics")
        sys.exit(1)

except Exception as e:
    print(f"Error during measurement: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)
