#!/usr/bin/env python3
"""
Wallet Behavior Validator — Falsification Test Framework

Measures ROC-AUC and other metrics on validation corpus.
"""

import json
import logging
from dataclasses import dataclass
from typing import List, Tuple
import os

from wallet_behavior_scorer import WalletProfile, score_wallet

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] validator: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


@dataclass
class ValidationResult:
    """Results from validation run."""
    corpus_size: int
    human_count: int
    sybil_count: int
    human_avg_score: float
    sybil_avg_score: float
    score_separation: float  # human_avg - sybil_avg
    roc_auc: float
    accuracy: float
    threshold: float
    true_positives: int
    true_negatives: int
    false_positives: int
    false_negatives: int

    def __str__(self) -> str:
        return (
            f"Validation Results\n"
            f"==================\n"
            f"Corpus: {self.corpus_size} wallets ({self.human_count}H + {self.sybil_count}S)\n"
            f"\n"
            f"Score Distribution:\n"
            f"  Human avg:  {self.human_avg_score:.3f}\n"
            f"  Sybil avg:  {self.sybil_avg_score:.3f}\n"
            f"  Separation: {self.score_separation:.3f}\n"
            f"\n"
            f"Classification (gate={self.threshold:.3f}):\n"
            f"  TP (verified human):  {self.true_positives}\n"
            f"  TN (detected bot):    {self.true_negatives}\n"
            f"  FP (false human):     {self.false_positives}\n"
            f"  FN (missed human):    {self.false_negatives}\n"
            f"  Accuracy:             {self.accuracy:.1%}\n"
            f"\n"
            f"ROC-AUC: {self.roc_auc:.3f}\n"
            f"{'✓ PASS' if self.roc_auc > 0.7 else '✗ FAIL'} (target > 0.7)"
        )


class WalletValidator:
    """Validate wallet behavior scoring on labeled corpus."""

    @staticmethod
    def load_corpus(corpus_file: str) -> Tuple[List[WalletProfile], List[bool]]:
        """Load validation corpus from JSON.

        Format:
        [
            {"wallet_address": "...", "is_human": true, "token_count": ..., ...},
            ...
        ]

        Returns:
            (profiles, labels) where labels[i] = True if profiles[i] is human
        """
        with open(corpus_file, "r") as f:
            data = json.load(f)

        profiles = []
        labels = []

        for item in data:
            profile = WalletProfile(
                wallet_address=item["wallet_address"],
                wallet_age_days=item.get("wallet_age_days", 1),
                token_count=item.get("token_count", 0),
                program_count=item.get("program_count", 0),
                unique_swap_pairs=item.get("unique_swap_pairs", 0),
                activity_span_days=item.get("activity_span_days", 0),
                total_transactions=item.get("total_transactions", 0),
                transaction_density=item.get("transaction_density", 0.0),
                gap_max_days=item.get("gap_max_days", 0),
                all_txs_same_hour=item.get("all_txs_same_hour", False),
                single_token_pct=item.get("single_token_pct", 0.0),
                recent_whale_flag=item.get("recent_whale_flag", False),
                transaction_frequency_anomaly=item.get("transaction_frequency_anomaly", False),
            )
            profiles.append(profile)
            labels.append(item["is_human"])

        return profiles, labels

    @staticmethod
    def validate(
        profiles: List[WalletProfile],
        labels: List[bool],
        threshold: float = 0.618,
    ) -> ValidationResult:
        """Validate corpus.

        Args:
            profiles: List of WalletProfile objects
            labels: List of ground truth labels (True = human, False = bot)
            threshold: Classification threshold (default φ⁻¹)

        Returns:
            ValidationResult with metrics
        """
        # Score all profiles
        scores = []
        for profile in profiles:
            scored = score_wallet(profile)
            scores.append(scored.authenticity_score)

        # Split by label
        human_scores = [scores[i] for i in range(len(scores)) if labels[i]]
        sybil_scores = [scores[i] for i in range(len(scores)) if not labels[i]]

        human_avg = sum(human_scores) / len(human_scores) if human_scores else 0.0
        sybil_avg = sum(sybil_scores) / len(sybil_scores) if sybil_scores else 0.0

        # Compute confusion matrix
        tp = sum(
            1 for i in range(len(scores))
            if labels[i] and scores[i] >= threshold
        )
        tn = sum(
            1 for i in range(len(scores))
            if not labels[i] and scores[i] < threshold
        )
        fp = sum(
            1 for i in range(len(scores))
            if not labels[i] and scores[i] >= threshold
        )
        fn = sum(
            1 for i in range(len(scores))
            if labels[i] and scores[i] < threshold
        )

        accuracy = (tp + tn) / len(scores) if scores else 0.0

        # Compute ROC-AUC
        roc_auc = _compute_roc_auc(scores, labels)

        return ValidationResult(
            corpus_size=len(profiles),
            human_count=len(human_scores),
            sybil_count=len(sybil_scores),
            human_avg_score=human_avg,
            sybil_avg_score=sybil_avg,
            score_separation=human_avg - sybil_avg,
            roc_auc=roc_auc,
            accuracy=accuracy,
            threshold=threshold,
            true_positives=tp,
            true_negatives=tn,
            false_positives=fp,
            false_negatives=fn,
        )


def _compute_roc_auc(scores: List[float], labels: List[bool]) -> float:
    """Compute ROC-AUC score.

    Simple implementation: uses trapezoidal rule on ROC curve.
    """
    if len(set(labels)) < 2:
        return 0.5  # Degenerate case

    # Create (score, label) pairs and sort by score descending
    pairs = list(zip(scores, labels))
    pairs.sort(key=lambda x: x[0], reverse=True)

    # Compute ROC curve
    n_positive = sum(labels)
    n_negative = len(labels) - n_positive

    if n_positive == 0 or n_negative == 0:
        return 0.5

    tpr_list = [0.0]  # True positive rate at threshold = ∞
    fpr_list = [0.0]  # False positive rate at threshold = ∞

    tp_count = 0
    fp_count = 0

    for score, label in pairs:
        if label:
            tp_count += 1
        else:
            fp_count += 1

        tpr = tp_count / n_positive
        fpr = fp_count / n_negative

        tpr_list.append(tpr)
        fpr_list.append(fpr)

    # Trapezoidal rule
    auc = 0.0
    for i in range(len(fpr_list) - 1):
        auc += (fpr_list[i + 1] - fpr_list[i]) * (tpr_list[i] + tpr_list[i + 1]) / 2.0

    return auc


def generate_test_corpus() -> str:
    """Generate synthetic test corpus (4 wallets: 2H + 2S).

    Returns: path to JSON corpus file
    """
    corpus = [
        # Authentic human
        {
            "wallet_address": "test_human_1",
            "is_human": True,
            "wallet_age_days": 45,
            "token_count": 22,
            "program_count": 8,
            "unique_swap_pairs": 15,
            "activity_span_days": 40,
            "total_transactions": 87,
            "transaction_density": 2.17,
            "gap_max_days": 3,
            "all_txs_same_hour": False,
            "single_token_pct": 8.5,
            "recent_whale_flag": False,
            "transaction_frequency_anomaly": False,
        },
        # Another authentic human
        {
            "wallet_address": "test_human_2",
            "is_human": True,
            "wallet_age_days": 90,
            "token_count": 34,
            "program_count": 11,
            "unique_swap_pairs": 28,
            "activity_span_days": 85,
            "total_transactions": 156,
            "transaction_density": 1.84,
            "gap_max_days": 4,
            "all_txs_same_hour": False,
            "single_token_pct": 5.1,
            "recent_whale_flag": False,
            "transaction_frequency_anomaly": False,
        },
        # Pump & dump bot
        {
            "wallet_address": "test_sybil_1",
            "is_human": False,
            "wallet_age_days": 3,
            "token_count": 1,
            "program_count": 2,
            "unique_swap_pairs": 1,
            "activity_span_days": 2,
            "total_transactions": 47,
            "transaction_density": 23.5,
            "gap_max_days": 1,
            "all_txs_same_hour": True,
            "single_token_pct": 98.0,
            "recent_whale_flag": False,
            "transaction_frequency_anomaly": False,
        },
        # MEV bot
        {
            "wallet_address": "test_sybil_2",
            "is_human": False,
            "wallet_age_days": 1,
            "token_count": 2,
            "program_count": 3,
            "unique_swap_pairs": 1,
            "activity_span_days": 1,
            "total_transactions": 150,
            "transaction_density": 150.0,
            "gap_max_days": 0,
            "all_txs_same_hour": False,
            "single_token_pct": 45.0,
            "recent_whale_flag": True,
            "transaction_frequency_anomaly": True,
        },
    ]

    corpus_path = "/tmp/wallet_validation_corpus.json"
    with open(corpus_path, "w") as f:
        json.dump(corpus, f, indent=2)

    logger.info(f"Generated test corpus: {corpus_path}")
    return corpus_path


if __name__ == "__main__":
    import sys

    # Generate test corpus
    corpus_path = generate_test_corpus()

    # Load and validate
    logger.info("Loading corpus...")
    profiles, labels = WalletValidator.load_corpus(corpus_path)

    logger.info(f"Validating {len(profiles)} wallets...")
    result = WalletValidator.validate(profiles, labels)

    print(f"\n{result}")

    # Exit with appropriate code
    sys.exit(0 if result.roc_auc > 0.7 else 1)
