#!/usr/bin/env python3
"""
Wallet Validation Corpus Builder — Data-Centric Collection

CYNIC collects its own validation data. No external dependencies.
Builds corpus from public, verifiable sources:
  - Humans: long-term holders, known devs, governance participants
  - Sybils: pump.fun rugs, MEV bots, documented exploits

Usage:
  python3 wallet_corpus_builder.py --collect-humans --collect-sybils
"""

import json
import logging
import os
from typing import List, Dict, Optional
from datetime import datetime

logger = logging.getLogger(__name__)
logger.setLevel(logging.INFO)
if not logger.handlers:
    handler = logging.StreamHandler()
    formatter = logging.Formatter(
        fmt="%(asctime)s [%(levelname)s] corpus_builder: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S"
    )
    handler.setFormatter(formatter)
    logger.addHandler(handler)


# ============================================================================
# Known Good Wallets (Humans)
# ============================================================================

KNOWN_HUMANS = [
    # Solana Foundation & OGs
    {
        "address": "TokenkegQfeZyiNwAJsyFbPVwwQnmZKeyHeVUTX1159",  # Token Program
        "label": "Solana Token Program (verified on-chain)",
        "reason": "Active since mainnet genesis, institutional",
    },
    {
        "address": "MarinadeMintLp6hTpf3yAKfQzLyp7UWnv3x2hZHG",  # Marinade
        "label": "Marinade Finance (governance)",
        "reason": "Multi-year protocol, active governance",
    },
    # Long-term ecosystem participants (governance, staking)
    {
        "address": "orcaEKTdK7LKz57chYcUdjik6bk5FqDNUucA2B6Hb8Q",  # Orca
        "label": "Orca AMM (verified protocol)",
        "reason": "Established DeFi, institutional activity",
    },
    # Chess/Game players (B&C reference)
    # Note: Populate with real B&C verified player addresses once available
    {
        "address": "game_verified_player_1",  # Placeholder
        "label": "B&C Verified Chess Player (50+ games)",
        "reason": "Game history + wallet age > 30d",
    },
    {
        "address": "game_verified_player_2",
        "label": "B&C Verified Chess Player (100+ games)",
        "reason": "Game history + wallet age > 60d",
    },
    # Synthetic: T.'s own wallet (if willing to share)
    # {
    #     "address": "T_primary_wallet",
    #     "label": "T. (CYNIC architect)",
    #     "reason": "Observable: 93 WPM typing, deliberation pauses, multi-domain interaction",
    # },
]

# ============================================================================
# Known Bad Wallets (Sybils)
# ============================================================================

KNOWN_SYBILS = [
    # Pump.fun documented liquidations
    {
        "address": "pump_rug_1_known",  # Replace with real address from CultScreener
        "label": "Pump.fun Liquidation (Rug #1)",
        "reason": "Identified by CultScreener as rug: single token concentration, rapid selloff",
    },
    {
        "address": "pump_rug_2_known",
        "label": "Pump.fun Liquidation (Rug #2)",
        "reason": "Identified by CultScreener as rug",
    },
    # MEV bots
    {
        "address": "mev_jito_bot_1",  # MEV bot identified in Jito bundles
        "label": "Jito MEV Bot",
        "reason": "100+ txs/hour, high-frequency atomic swaps, fresh age",
    },
    {
        "address": "mev_sandwicher_1",
        "label": "Sandwich Attack Bot",
        "reason": "Transaction timing anomalies, MEV extraction pattern",
    },
    # Documented exploits
    {
        "address": "exploit_sybil_1",  # From security incident post-mortems
        "label": "Known Exploit Account",
        "reason": "Documented in Discord archives as sybil farm",
    },
    # Synthetic: intentional pump pattern
    {
        "address": "synthetic_pump_bot",  # Wallet we control
        "label": "Synthetic Pump Bot (created for test)",
        "reason": "50 txs on single token within 2-hour window",
    },
]


def fetch_wallet_profile(address: str, collector) -> Optional[Dict]:
    """Fetch profile from Helius for a wallet address.

    Args:
        address: Solana wallet address
        collector: HeliusWalletCollector instance

    Returns:
        Dict with all WalletProfile fields, or None on error
    """
    try:
        profile = collector.collect_wallet_profile(address)
        if profile:
            return {
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
            }
    except Exception as e:
        logger.error(f"Failed to fetch {address}: {e}")
    return None


def build_corpus(
    collector,
    output_file: str = "validation_corpus.json",
    include_humans: bool = True,
    include_sybils: bool = True,
) -> str:
    """Build validation corpus from known wallets.

    Args:
        collector: HeliusWalletCollector instance
        output_file: Path to save JSON corpus
        include_humans: Fetch human wallets
        include_sybils: Fetch sybil wallets

    Returns:
        Path to generated corpus file
    """
    corpus = []

    if include_humans:
        logger.info("Collecting verified human wallets...")
        for entry in KNOWN_HUMANS:
            logger.info(f"  Fetching: {entry['label']}")
            profile = fetch_wallet_profile(entry["address"], collector)
            if profile:
                profile["is_human"] = True
                profile["source"] = entry["label"]
                profile["reason"] = entry["reason"]
                corpus.append(profile)
                logger.info(f"    ✓ Score: {profile.get('authenticity_score', 'N/A')}")

    if include_sybils:
        logger.info("Collecting known sybil wallets...")
        for entry in KNOWN_SYBILS:
            logger.info(f"  Fetching: {entry['label']}")
            profile = fetch_wallet_profile(entry["address"], collector)
            if profile:
                profile["is_human"] = False
                profile["source"] = entry["label"]
                profile["reason"] = entry["reason"]
                corpus.append(profile)
                logger.info(f"    ✓ Score: {profile.get('authenticity_score', 'N/A')}")

    # Save corpus
    with open(output_file, "w") as f:
        json.dump(corpus, f, indent=2)

    logger.info(f"\n✓ Corpus saved: {output_file}")
    logger.info(f"  Total wallets: {len(corpus)} ({sum(1 for c in corpus if c['is_human'])}H + {sum(1 for c in corpus if not c['is_human'])}S)")

    return output_file


# ============================================================================
# Template for Manual Addition
# ============================================================================

CORPUS_TEMPLATE = """
# To add wallets to the corpus manually:

## Known Humans

1. **Marinade Finance (MarinadeMintLp6hTpf3yAKfQzLyp7UWnv3x2hZHG)**
   - Multi-year protocol
   - Active governance participation
   - Institutional activity pattern

2. **Orca Protocol**
   - Established DeFi
   - Consistent, diverse interactions
   - Public reputation

3. **B&C Verified Chess Players**
   - Fetch from B&C game API
   - Filter by: games_completed >= 20, wallet_age_days >= 14
   - Reason: gameplay history + temporal proof

4. **Your Own Wallet (T.)**
   - Known behavior patterns
   - Observable activity
   - Multi-domain interactions

## Known Sybils

1. **Pump.fun Liquidations (from CultScreener)**
   - Visit cultscreener-api.onrender.com/api/tokens/leaderboard/conviction
   - Identify rugs (conviction = 0)
   - Extract liquidated token creator wallets
   - Check: age < 2 weeks, single token > 95%

2. **MEV Bots (from Jito)**
   - Query Helius transaction history
   - Filter: 100+ txs/hour, atomic swaps
   - Common addresses in bundle frontrunning

3. **Discord Exploit Archives**
   - Search Solana/crypto security discords
   - Known sybil farm attacks documented
   - Extractable wallet addresses

4. **Synthetic Test Wallet**
   - Create fresh wallet
   - Execute pump pattern: 50 txs on single token within 2 hours
   - Verify all_txs_same_hour = true
   - Expected score: < 0.20
"""


def print_template():
    """Print template for manual corpus building."""
    print(CORPUS_TEMPLATE)


if __name__ == "__main__":
    import sys

    print("\n=== Wallet Corpus Builder ===\n")
    print("This module builds validation corpus for wallet behavior scoring.")
    print("\nUsage:")
    print("  from wallet_corpus_builder import build_corpus, print_template")
    print("  from wallet_behavior_helius import HeliusWalletCollector")
    print("")
    print("  collector = HeliusWalletCollector()")
    print("  corpus_path = build_corpus(collector)")
    print("\n  # Or print template for manual additions:")
    print("  print_template()")
    print("\n---")
    print("\nKNOWN_HUMANS (placeholders, need real addresses):")
    for h in KNOWN_HUMANS[:3]:
        print(f"  - {h['label']}")
    print(f"  ... ({len(KNOWN_HUMANS)} total)")

    print("\nKNOWN_SYBILS (placeholders, need real addresses):")
    for s in KNOWN_SYBILS[:3]:
        print(f"  - {s['label']}")
    print(f"  ... ({len(KNOWN_SYBILS)} total)")

    print("\nTo generate corpus with Helius, run:")
    print("  Load API key from secure location")
    print("  python3 -c \"from wallet_corpus_builder import build_corpus; from wallet_behavior_helius import HeliusWalletCollector; build_corpus(HeliusWalletCollector())\"")
