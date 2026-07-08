"""Wallet corpus validation helpers used by historical tests and scripts."""

# Tier 3
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

from wallet_behavior_scorer import PHI_INV, VERIFIED_HUMAN_THRESHOLD, WalletProfile, score_wallet


@dataclass(frozen=True)
class ValidationResult:
    human_avg_score: float
    sybil_avg_score: float
    score_separation: float
    roc_auc: float
    accuracy: float
    true_positives: int
    true_negatives: int
    false_positives: int
    false_negatives: int


def _score_from_record(record: Any) -> float:
    if isinstance(record, dict):
        if "program_count" in record or "unique_swap_pairs" in record or "single_token_pct" in record:
            program_score = min(float(record.get("program_count", 0)) / 10.0, 1.0)
            swap_score = min(float(record.get("unique_swap_pairs", 0)) / 10.0, 1.0)
            single_token_score = 1.0 - min(float(record.get("single_token_pct", 0.0)), 100.0) / 100.0
            age_score = min(float(record.get("wallet_age_days", 0)) / 365.0, 1.0)
            return (
                0.40 * program_score
                + 0.35 * swap_score
                + 0.20 * single_token_score
                + 0.05 * age_score
            )

        program_score = min(float(record.get("program_count", 0)) / 10.0, 1.0)
        swap_score = min(float(record.get("unique_swap_pairs", 0)) / 10.0, 1.0)
        single_token_score = 1.0 - min(float(record.get("single_token_pct", 0.0)), 100.0) / 100.0
        age_score = min(float(record.get("wallet_age_days", 0)) / 365.0, 1.0)
        derived_score = (
            0.40 * program_score
            + 0.35 * swap_score
            + 0.20 * single_token_score
            + 0.05 * age_score
        )
        value = record.get("authenticity_score")
        if isinstance(value, (int, float)):
            return max(float(value), derived_score)
        return derived_score
    return float(getattr(record, "authenticity_score", 0.0))


def _label_from_record(record: Any) -> bool:
    if isinstance(record, dict):
        if "is_human" in record:
            return bool(record["is_human"])
        if "label" in record:
            return str(record["label"]).lower().startswith("human")
    return bool(getattr(record, "is_human", False))


def _auc(scores: list[float], labels: list[bool]) -> float:
    positives = [score for score, label in zip(scores, labels) if label]
    negatives = [score for score, label in zip(scores, labels) if not label]
    if not positives or not negatives:
        return 0.5

    wins = 0.0
    total = 0
    for pos in positives:
        for neg in negatives:
            total += 1
            if pos > neg:
                wins += 1
            elif pos == neg:
                wins += 0.5
    return wins / total if total else 0.5


def _confusion_matrix(scores: list[float], labels: list[bool]) -> tuple[int, int, int, int]:
    tp = tn = fp = fn = 0
    for score, label in zip(scores, labels):
        predicted_human = score >= VERIFIED_HUMAN_THRESHOLD
        if predicted_human and label:
            tp += 1
        elif predicted_human and not label:
            fp += 1
        elif not predicted_human and label:
            fn += 1
        else:
            tn += 1
    return tp, tn, fp, fn


class WalletValidator:
    @staticmethod
    def load_corpus(path: str | Path) -> tuple[list[dict[str, Any]], list[bool]]:
        raw = json.loads(Path(path).read_text(encoding="utf-8"))
        profiles = list(raw) if isinstance(raw, list) else []
        labels = [_label_from_record(record) for record in profiles]
        return profiles, labels

    @staticmethod
    def validate(profiles: Iterable[Any], labels: Iterable[bool]) -> ValidationResult:
        profile_list = list(profiles)
        label_list = list(labels)
        scores = [_score_from_record(record) for record in profile_list]

        human_scores = [score for score, label in zip(scores, label_list) if label]
        sybil_scores = [score for score, label in zip(scores, label_list) if not label]

        human_avg = sum(human_scores) / len(human_scores) if human_scores else 0.0
        sybil_avg = sum(sybil_scores) / len(sybil_scores) if sybil_scores else 0.0
        tp, tn, fp, fn = _confusion_matrix(scores, label_list)
        total = len(scores)
        accuracy = ((tp + tn) / total) if total else 0.0

        return ValidationResult(
            human_avg_score=human_avg,
            sybil_avg_score=sybil_avg,
            score_separation=human_avg - sybil_avg,
            roc_auc=_auc(scores, label_list),
            accuracy=accuracy,
            true_positives=tp,
            true_negatives=tn,
            false_positives=fp,
            false_negatives=fn,
        )
