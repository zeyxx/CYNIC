"""Falsification Test 2: Real corpus collection and ROC-AUC measurement."""

from __future__ import annotations

from pathlib import Path

from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_validator import WalletValidator
from wallet_corpus_builder import KNOWN_HUMANS, KNOWN_SYBILS, build_corpus


def test_wallet_corpus_measurement(tmp_path: Path) -> None:
    """Validate the wallet corpus pipeline and its ROC-AUC threshold."""

    collector = HeliusWalletCollector()
    corpus_path = build_corpus(
        collector,
        output_file=tmp_path / "validation_corpus_live.json",
        include_humans=True,
        include_sybils=True,
    )

    assert Path(corpus_path).exists()
    profiles, labels = WalletValidator.load_corpus(corpus_path)
    assert len(profiles) == len(labels)
    assert len(KNOWN_HUMANS) > 0
    assert len(KNOWN_SYBILS) > 0

    result = WalletValidator.validate(profiles, labels)
    assert result.roc_auc > 0.7
