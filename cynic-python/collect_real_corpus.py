#!/usr/bin/env python3
"""
Real Wallet Corpus Collection — Direct On-Chain Source

Strategy: Query CultScreener for tokens by conviction, extract creator/authority
via Helius, collect wallet profiles.

Sybil candidates (human-labeled): pump.fun rugs (low conviction 0-30)
Human candidates (human-labeled): established protocols (high conviction 80-100)

Usage:
  python3 collect_real_corpus.py
"""

import json
import logging
import time
import requests
from typing import List, Dict, Optional
from pathlib import Path

from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_scorer import WalletProfile

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] corpus_collector: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


class CorpusCollector:
    """Collect real wallet profiles from on-chain data sources."""

    CULTSCREENER_URL = "https://cultscreener-api.onrender.com/api/tokens/leaderboard/conviction"
    CONVICTION_THRESHOLDS = {
        "sybil_candidates": (0, 30),      # Low conviction = likely rugs
        "human_candidates": (80, 100),    # High conviction = established
    }

    def __init__(self, helius_api_key: Optional[str] = None, timeout: int = 30):
        self.timeout = timeout
        self.collector = HeliusWalletCollector(api_key=helius_api_key, timeout=timeout)
        self.corpus = []

    def fetch_tokens_by_conviction(
        self, min_conviction: int, max_conviction: int, limit: int = 10
    ) -> List[Dict]:
        """Fetch tokens from CultScreener by conviction range."""
        try:
            params = {
                "conviction_min": min_conviction,
                "conviction_max": max_conviction,
                "limit": limit,
            }
            response = requests.get(self.CULTSCREENER_URL, params=params, timeout=self.timeout)
            response.raise_for_status()
            data = response.json()
            tokens = data.get("tokens", [])
            logger.info(
                f"Fetched {len(tokens)} tokens (conviction {min_conviction}-{max_conviction})"
            )
            return tokens
        except requests.RequestException as e:
            logger.error(f"CultScreener fetch failed: {e}")
            return []

    def collect_wallet_profiles(
        self, tokens: List[Dict], label: str, is_sybil: bool = False
    ) -> List[Dict]:
        """Collect wallet profiles for token creators."""
        profiles = []

        for token in tokens:
            mint = token.get("mintAddress") or token.get("address")
            symbol = token.get("symbol", "UNKNOWN")
            conviction = token.get("conviction1m", 0)

            if not mint or len(mint) < 40:
                logger.warning(f"Skipping {symbol}: invalid mint address")
                continue

            logger.info(f"Collecting {label}: {symbol} (conviction={conviction})")

            try:
                # Extract the actual creator wallet from token mint authority
                logger.debug(f"  Extracting creator from mint {mint}...")
                wallet = self.collector.get_token_authority(mint)

                if not wallet or len(wallet) < 40:
                    logger.warning(f"  ✗ Could not extract creator wallet from {symbol} (mint={mint[:8]}...)")
                    time.sleep(0.5)
                    continue

                logger.debug(f"  Found creator wallet: {wallet}")

                # Helius collection with rate limit handling
                profile = self.collector.collect_wallet_profile(wallet)

                if profile:
                    profiles.append(
                        {
                            "wallet_address": profile.wallet_address,
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
                            "is_human": not is_sybil,
                            "source": f"CultScreener ({symbol})",
                            "conviction": conviction,
                            "label": label,
                        }
                    )
                    logger.info(
                        f"  ✓ Score: {profile.authenticity_score:.3f}, "
                        f"verified={profile.is_verified_human}"
                    )
                else:
                    logger.warning(f"  ✗ Failed to collect profile for {wallet}")

            except Exception as e:
                logger.error(f"  ✗ Error: {e}")
                # Continue on individual failures, rate-limit delays will be handled by HeliusWalletCollector
                time.sleep(1.5)

        return profiles

    def collect(self) -> List[Dict]:
        """Collect full corpus from CultScreener."""
        logger.info("=== Real Wallet Corpus Collection ===\n")

        # Collect sybil candidates (low conviction = rugs)
        logger.info("Phase 1: Collecting sybil candidates (low conviction tokens)...")
        rug_tokens = self.fetch_tokens_by_conviction(0, 30, limit=10)
        sybil_profiles = self.collect_wallet_profiles(rug_tokens, "rug_pump_fun", is_sybil=True)

        logger.info(f"  → Collected {len(sybil_profiles)} sybil profiles\n")

        # Collect human candidates (high conviction = established)
        logger.info("Phase 2: Collecting human candidates (high conviction tokens)...")
        legit_tokens = self.fetch_tokens_by_conviction(80, 100, limit=10)
        human_profiles = self.collect_wallet_profiles(legit_tokens, "established_token", is_sybil=False)

        logger.info(f"  → Collected {len(human_profiles)} human profiles\n")

        # Combine
        self.corpus = human_profiles + sybil_profiles

        logger.info("=== Corpus Summary ===")
        logger.info(f"Total: {len(self.corpus)} wallets")
        logger.info(f"Humans: {sum(1 for w in self.corpus if w['is_human'])}")
        logger.info(f"Sybils: {sum(1 for w in self.corpus if not w['is_human'])}")

        return self.corpus

    def save(self, output_file: str = "validation_corpus_real.json") -> str:
        """Save corpus to file."""
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, "w") as f:
            json.dump(self.corpus, f, indent=2)

        logger.info(f"\n✓ Corpus saved: {output_path}")
        return str(output_path)


if __name__ == "__main__":
    import os

    api_key = os.getenv("HELIUS_API_KEY")
    if not api_key:
        logger.warning("HELIUS_API_KEY not set. Set env var to enable collection.")
        logger.warning("  export HELIUS_API_KEY=...")
        logger.warning("Proceeding with demo mode (0 profiles).\n")

    collector = CorpusCollector(helius_api_key=api_key)
    corpus = collector.collect()
    output_file = collector.save()

    logger.info(f"\nReady for validation: python3 wallet_behavior_validator.py {output_file}")
