#!/usr/bin/env python3
"""
Manual Wallet Corpus Collection — Direct Address Input

Strategy: Accept pre-curated wallet addresses, collect profiles, validate.

Usage:
  python3 collect_manual_corpus.py wallets.json

  wallets.json format:
  [
    {
      "wallet_address": "...",
      "label": "rug_deployer" or "active_trader",
      "is_human": false or true,
      "source": "documentation" or "organ_x" or "manual"
    },
    ...
  ]
"""

import json
import logging
import time
from typing import List, Dict, Optional
from pathlib import Path
import sys

from wallet_behavior_helius import HeliusWalletCollector
from wallet_behavior_scorer import WalletProfile

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] manual_collector: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


class ManualCorpusCollector:
    """Collect wallet profiles from manually-curated addresses."""

    def __init__(self, endpoint_token: Optional[str] = None, timeout: int = 30):
        self.timeout = timeout
        self.collector = HeliusWalletCollector(endpoint_credentials=endpoint_token, timeout=timeout)
        self.corpus = []

    def collect_wallet_profiles(self, wallets: List[Dict]) -> List[Dict]:
        """Collect wallet profiles for manually-curated addresses."""
        profiles = []
        backoff_delay = 1.0
        max_backoff = 60.0

        for wallet_info in wallets:
            wallet = wallet_info.get("wallet_address")
            label = wallet_info.get("label", "unknown")
            is_human = wallet_info.get("is_human", False)
            source = wallet_info.get("source", "manual")

            if not wallet or len(wallet) < 40:
                logger.warning(f"Skipping {label}: invalid wallet address")
                continue

            logger.info(f"Collecting {label}: {wallet} (human={is_human})")

            retries = 0
            max_retries = 3
            while retries < max_retries:
                try:
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
                                "is_human": is_human,
                                "source": source,
                                "label": label,
                            }
                        )
                        logger.info(
                            f"  ✓ Score: {profile.authenticity_score:.3f}, "
                            f"verified={profile.is_verified_human}"
                        )
                        backoff_delay = 1.0  # Reset backoff on success
                        break
                    else:
                        logger.warning(f"  ✗ Failed to collect profile for {wallet}")
                        break

                except Exception as e:
                    if "429" in str(e) or "rate" in str(e).lower():
                        retries += 1
                        if retries < max_retries:
                            logger.warning(
                                f"  ⚠ Rate limited (attempt {retries}/{max_retries}). "
                                f"Backoff: {backoff_delay:.1f}s"
                            )
                            time.sleep(backoff_delay)
                            backoff_delay = min(backoff_delay * 2, max_backoff)
                        else:
                            logger.error(
                                f"  ✗ Gave up after {max_retries} retries (rate limited): {e}"
                            )
                    else:
                        logger.error(f"  ✗ Error: {e}")
                        break

        return profiles

    def collect_from_file(self, input_file: str) -> List[Dict]:
        """Load wallets from JSON file and collect profiles."""
        try:
            with open(input_file, "r") as f:
                wallets = json.load(f)

            if not isinstance(wallets, list):
                raise ValueError("Input file must contain a JSON array")

            logger.info(f"=== Manual Wallet Corpus Collection ===\n")
            logger.info(f"Loaded {len(wallets)} wallets from {input_file}")

            profiles = self.collect_wallet_profiles(wallets)
            self.corpus = profiles

            logger.info(f"\n=== Corpus Summary ===")
            logger.info(f"Total: {len(self.corpus)} wallets")
            logger.info(f"Humans: {sum(1 for w in self.corpus if w['is_human'])}")
            logger.info(f"Sybils: {sum(1 for w in self.corpus if not w['is_human'])}")

            return self.corpus

        except FileNotFoundError:
            logger.error(f"File not found: {input_file}")
            return []
        except json.JSONDecodeError as e:
            logger.error(f"Invalid JSON in {input_file}: {e}")
            return []

    def save(self, output_file: str = "validation_corpus_manual.json") -> str:
        """Save corpus to file."""
        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, "w") as f:
            json.dump(self.corpus, f, indent=2)

        logger.info(f"\n✓ Corpus saved: {output_path}")
        return str(output_path)


if __name__ == "__main__":
    import os

    if len(sys.argv) < 2:
        print("Usage: python3 collect_manual_corpus.py <input_wallets.json>")
        print("\nExample input_wallets.json:")
        print(json.dumps([
            {
                "wallet_address": "11111111111111111111111111111111",
                "label": "rug_deployer",
                "is_human": False,
                "source": "cultscreener"
            },
            {
                "wallet_address": "TokenkegQfeZyiNwAJsyFbPVwwQnmZKeyHeVUTX1159",
                "label": "active_trader",
                "is_human": True,
                "source": "manual"
            }
        ], indent=2))
        sys.exit(1)

    input_file = sys.argv[1]
    credentials = os.getenv("HELIUS_API_KEY")
    if not credentials:
        logger.warning("HELIUS_API_KEY not set. Set env var to enable collection.")

    collector = ManualCorpusCollector(endpoint_token=credentials)
    corpus = collector.collect_from_file(input_file)
    output_file = collector.save()

    logger.info(f"\nReady for validation: python3 wallet_behavior_validator.py {output_file}")
