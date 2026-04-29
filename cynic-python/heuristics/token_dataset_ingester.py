#!/usr/bin/env python3
"""Token dataset ingester — CultScreener labels + Helius enrichment.

Data-centric architecture (like organ-x):
1. Source (CultScreener API)
2. Enrich (Helius for on-chain metrics)
3. Transform (map to our signal dataclasses)
4. Persist (JSON/CSV with labels)

Usage:
    ingester = TokenDatasetIngester()

    # Ingest from CultScreener (fetch public database)
    dataset = ingester.ingest_cultscreener(
        risk_levels=["high", "low"],
        count_per_level=20
    )
    # → 40 tokens with ground truth labels

    # Export as JSON (for measurement)
    ingester.save_json(dataset, "token_dataset_ground_truth.json")

    # Export as CSV (for analysis/manual review)
    ingester.save_csv(dataset, "token_dataset_ground_truth.csv")
"""

import json
import csv
from typing import List, Optional, Dict
from dataclasses import asdict
import os

# Load .env file manually (no external dependency required)
def _load_env_file():
    """Load .env file from heuristics/ directory."""
    env_path = os.path.join(os.path.dirname(__file__), '.env')
    if os.path.exists(env_path):
        try:
            with open(env_path, 'r') as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#') and '=' in line:
                        key, value = line.split('=', 1)
                        if key not in os.environ:  # Don't override existing env vars
                            os.environ[key] = value
        except Exception:
            pass  # Silently skip if .env file is malformed

_load_env_file()

from cultscreener_client import CultScreenerClient
from real_data_loader import RealDataLoader
from dataset_builder import TwitterSignals, WalletSignals


class TokenDatasetIngester:
    """Ingest token data from CultScreener, enrich from Helius, persist with labels."""

    def __init__(self):
        self.cultscreener = CultScreenerClient()
        self.helius_loader = RealDataLoader()
        self.enriched_tokens: List[Dict] = []

    def ingest_cultscreener(
        self,
        conviction_tiers: Optional[List[str]] = None,
        count_per_tier: int = 20,
    ) -> List[Dict]:
        """Ingest tokens from CultScreener with conviction-based verdicts.

        Args:
            conviction_tiers: Which conviction tiers to fetch (default: ["strong", "mixed", "weak"])
                - "strong": conviction ≥ 0.7 → HOWL (legitimate)
                - "mixed": conviction 0.4-0.7 → GROWL (ambiguous)
                - "weak": conviction < 0.4 → BARK (suspicious)
            count_per_tier: How many tokens per conviction tier (for balanced dataset)

        Returns:
            List of enriched token dicts with labels
        """
        if conviction_tiers is None:
            conviction_tiers = ["strong", "mixed", "weak"]

        all_tokens = []

        # Conviction thresholds for each tier
        tier_filters = {
            "strong": {"min_conviction": 0.7},
            "mixed": {"min_conviction": 0.4, "max_conviction": 0.7},
            "weak": {"max_conviction": 0.4},
        }

        for tier in conviction_tiers:
            if tier not in tier_filters:
                print(f"Unknown conviction tier: {tier}. Skipping.")
                continue

            filters = tier_filters[tier]
            print(f"Fetching {count_per_tier} {tier}-conviction tokens from CultScreener...")

            # Fetch from leaderboard with conviction filters
            tokens = self.cultscreener.get_leaderboard(
                limit=count_per_tier,
                min_conviction=filters.get("min_conviction"),
                max_conviction=filters.get("max_conviction"),
            )

            for cult_token in tokens:
                enriched = self._enrich_token(cult_token)
                if enriched:
                    all_tokens.append(enriched)
                    print(f"  ✓ {cult_token.symbol or cult_token.mint[:8]} → {enriched['verdict']} "
                          f"(conviction={cult_token.conviction:.2f})")
                else:
                    print(f"  ✗ {cult_token.symbol or cult_token.mint[:8]} (enrichment failed)")

        self.enriched_tokens = all_tokens
        print(f"\nIngested {len(all_tokens)} tokens from CultScreener")
        return all_tokens

    def _enrich_token(self, cult_token) -> Optional[Dict]:  # TokenConvictionData
        """Enrich a CultScreener token with on-chain and twitter data.

        Args:
            cult_token: Token from CultScreener (TokenConvictionData)

        Returns:
            Enriched dict with all signals, or None if enrichment fails
        """
        # Fetch on-chain metrics from Helius
        wallet_signals = self.helius_loader.fetch_wallet_signals(cult_token.mint)
        if not wallet_signals:
            return None

        # Fetch twitter metrics from Hermes (if available)
        twitter_signals = None
        if cult_token.symbol:
            twitter_signals = self.helius_loader.fetch_twitter_signals(cult_token.symbol)

        return {
            "mint": cult_token.mint,
            "name": cult_token.name,
            "symbol": cult_token.symbol,
            "cultscreener_conviction": cult_token.conviction,
            "cultscreener_conviction_tier": cult_token.conviction_tier.value,
            "verdict": cult_token.to_verdict(),  # Ground truth label
            "wallet_signals": asdict(wallet_signals) if wallet_signals else None,
            "twitter_signals": asdict(twitter_signals) if twitter_signals else None,
            "timestamp": cult_token.timestamp,
        }

    def save_json(self, dataset: List[Dict], output_path: str) -> str:
        """Save dataset as JSON (complete signal data).

        Args:
            dataset: List of enriched token dicts
            output_path: Where to write JSON

        Returns:
            Path written
        """
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        with open(output_path, "w") as f:
            json.dump(dataset, f, indent=2, default=str)

        print(f"Saved {len(dataset)} tokens to {output_path}")
        return output_path

    def save_csv(self, dataset: List[Dict], output_path: str) -> str:
        """Save dataset as CSV (human-readable summary).

        Args:
            dataset: List of enriched token dicts
            output_path: Where to write CSV

        Returns:
            Path written
        """
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        fieldnames = [
            "mint",
            "symbol",
            "name",
            "verdict",  # Ground truth
            "cultscreener_conviction",
            "cultscreener_conviction_tier",
            "wallet_whale_count",
            "wallet_top10_pct",
            "wallet_bot_score",
            "wallet_daily_traders",
            "wallet_accumulation",
            "twitter_followers",
            "twitter_engagement",
            "twitter_positive_pct",
        ]

        with open(output_path, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()

            for token in dataset:
                row = {
                    "mint": token["mint"],
                    "symbol": token["symbol"],
                    "name": token["name"],
                    "verdict": token["verdict"],
                    "cultscreener_conviction": token.get("cultscreener_conviction"),
                    "cultscreener_conviction_tier": token.get("cultscreener_conviction_tier"),
                }

                # Wallet signals (if available)
                if token["wallet_signals"]:
                    ws = token["wallet_signals"]
                    row.update({
                        "wallet_whale_count": ws.get("whale_count"),
                        "wallet_top10_pct": ws.get("top_10_hold_pct"),
                        "wallet_bot_score": ws.get("bot_score"),
                        "wallet_daily_traders": ws.get("daily_active_traders"),
                        "wallet_accumulation": ws.get("accumulation_trend"),
                    })

                # Twitter signals (if available)
                if token["twitter_signals"]:
                    ts = token["twitter_signals"]
                    row.update({
                        "twitter_followers": ts.get("follower_count"),
                        "twitter_engagement": ts.get("engagement_rate"),
                        "twitter_positive_pct": ts.get("positive_pct"),
                    })

                writer.writerow(row)

        print(f"Saved {len(dataset)} tokens to {output_path} (CSV summary)")
        return output_path

    def get_statistics(self) -> Dict:
        """Get statistics about the ingested dataset."""
        if not self.enriched_tokens:
            return {"total": 0}

        by_verdict = {}
        by_conviction_tier = {}

        for token in self.enriched_tokens:
            verdict = token["verdict"]
            by_verdict[verdict] = by_verdict.get(verdict, 0) + 1

            tier = token.get("cultscreener_conviction_tier", "unknown")
            by_conviction_tier[tier] = by_conviction_tier.get(tier, 0) + 1

        return {
            "total": len(self.enriched_tokens),
            "by_verdict": by_verdict,
            "by_conviction_tier": by_conviction_tier,
        }


if __name__ == "__main__":
    ingester = TokenDatasetIngester()

    # Ingest balanced dataset (20 tokens per conviction tier)
    print("=" * 70)
    print("INGESTING GROUND TRUTH DATASET FROM CULTSCREENER")
    print("=" * 70)

    dataset = ingester.ingest_cultscreener(
        conviction_tiers=["strong", "mixed", "weak"],
        count_per_tier=20,
    )

    # Show statistics
    stats = ingester.get_statistics()
    print("\nDataset Statistics:")
    print(f"  Total tokens: {stats['total']}")
    print(f"  By verdict: {stats['by_verdict']}")

    # Export
    if dataset:
        output_dir = os.path.expanduser("~/.cynic/datasets/tokens/")
        os.makedirs(output_dir, exist_ok=True)

        json_path = ingester.save_json(dataset, f"{output_dir}/ground_truth.json")
        csv_path = ingester.save_csv(dataset, f"{output_dir}/ground_truth.csv")

        print(f"\nDataset exported:")
        print(f"  JSON (full signals): {json_path}")
        print(f"  CSV (summary): {csv_path}")
