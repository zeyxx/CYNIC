"""Compatibility corpus builder for wallet behavior validation."""

# Tier 3
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from wallet_behavior_helius import HeliusWalletCollector


_ARCHIVE_PATH = Path(__file__).resolve().parent / "datasets" / "archive" / "CORPUS_ADDRESSES.json"
_OFFLINE_CORPUS_PATH = Path(__file__).resolve().parent / "datasets" / "archive" / "real_wallet_corpus_validated.json"


def _load_addresses() -> dict[str, list[dict[str, Any]]]:
    if not _ARCHIVE_PATH.is_file():
        return {"humans": [], "sybils": []}
    return json.loads(_ARCHIVE_PATH.read_text(encoding="utf-8"))


_CORPUS_ADDRESSES = _load_addresses()
KNOWN_HUMANS = _CORPUS_ADDRESSES.get("humans", [])
KNOWN_SYBILS = _CORPUS_ADDRESSES.get("sybils", [])


def _collect_group(
    collector: HeliusWalletCollector,
    group: list[dict[str, Any]],
    is_human: bool,
) -> list[dict[str, Any]]:
    corpus: list[dict[str, Any]] = []
    for item in group:
        address = str(item.get("address", "")).strip()
        if not address:
            continue
        if address.startswith("synthetic_") or address.startswith("BadActor"):
            continue

        profile = collector.collect_wallet_profile(address)
        if not profile:
            continue

        corpus.append(
            {
                "wallet_address": profile.wallet_address,
                "is_human": is_human,
                "source": item.get("source", ""),
                "label": item.get("label", ""),
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
                "unique_swap_pairs": profile.unique_swap_pairs,
                "activity_span_days": profile.activity_span_days,
                "total_transactions": profile.total_transactions,
                "transaction_density": profile.transaction_density,
                "gap_max_days": profile.gap_max_days,
                "all_txs_same_hour": profile.all_txs_same_hour,
                "single_token_pct": profile.single_token_pct,
                "recent_whale_flag": profile.recent_whale_flag,
                "transaction_frequency_anomaly": profile.transaction_frequency_anomaly,
                "authenticity_score": profile.authenticity_score,
                "is_verified_human": profile.is_verified_human,
            }
        )
    return corpus


def _load_offline_corpus(include_humans: bool, include_sybils: bool) -> list[dict[str, Any]]:
    if not _OFFLINE_CORPUS_PATH.is_file():
        return []

    raw = json.loads(_OFFLINE_CORPUS_PATH.read_text(encoding="utf-8"))
    if not isinstance(raw, list):
        return []

    corpus: list[dict[str, Any]] = []
    for record in raw:
        if not isinstance(record, dict):
            continue
        is_human = bool(record.get("is_human", False))
        if is_human and not include_humans:
            continue
        if not is_human and not include_sybils:
            continue
        corpus.append(record)
    return corpus


def build_corpus(
    collector: HeliusWalletCollector,
    output_file: str | Path = "validation_corpus_real.json",
    *,
    include_humans: bool = True,
    include_sybils: bool = True,
) -> Path:
    corpus: list[dict[str, Any]] = []
    if getattr(collector, "api_key", ""):
        if include_humans:
            corpus.extend(_collect_group(collector, KNOWN_HUMANS, True))
        if include_sybils:
            corpus.extend(_collect_group(collector, KNOWN_SYBILS, False))

    if not corpus:
        corpus = _load_offline_corpus(include_humans=include_humans, include_sybils=include_sybils)

    output_path = Path(output_file)
    output_path.write_text(json.dumps(corpus, indent=2), encoding="utf-8")
    return output_path
