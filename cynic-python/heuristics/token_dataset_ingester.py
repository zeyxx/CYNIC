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

from cultscreener_client import CultScreenerClient, TokenRiskAssessment, RiskLevel
from real_data_loader import RealDataLoader
from dataset_builder import TokenMetrics, TwitterSignals, WalletSignals


class TokenDatasetIngester:
    """Ingest token data from CultScreener, enrich from Helius, persist with labels."""

    def __init__(self):
        self.cultscreener = CultScreenerClient()
        self.helius_loader = RealDataLoader()
        self.enriched_tokens: List[Dict] = []

    def ingest_cultscreener(
        self,
        risk_levels: Optional[List[str]] = None,
        count_per_level: int = 20,
    ) -> List[Dict]:
        """Ingest tokens from CultScreener with ground-truth risk labels.

        Args:
            risk_levels: Which risk levels to fetch (default: ["high", "low", "medium"])
            count_per_level: How many tokens per risk level (for balanced dataset)

        Returns:
            List of enriched token dicts with labels
        """
        if risk_levels is None:
            risk_levels = ["high", "low", "medium"]

        all_tokens = []

        for risk_level in risk_levels:
            print(f"Fetching {count_per_level} {risk_level}-risk tokens from CultScreener...")
            tokens = self.cultscreener.search_tokens(
                risk_level=risk_level,
                limit=count_per_level,
            )

            for cult_token in tokens:
                enriched = self._enrich_token(cult_token)
                if enriched:
                    all_tokens.append(enriched)
                    print(f"  ✓ {cult_token.symbol or cult_token.mint[:8]} → {enriched['verdict']} "
                          f"(cultscreener_risk={cult_token.risk_level.value})")
                else:
                    print(f"  ✗ {cult_token.symbol or cult_token.mint[:8]} (enrichment failed)")

        self.enriched_tokens = all_tokens
        print(f"\nIngested {len(all_tokens)} tokens from CultScreener")
        return all_tokens

    def _enrich_token(self, cult_token: TokenRiskAssessment) -> Optional[Dict]:
        """Enrich a CultScreener token with on-chain and twitter data.

        Args:
            cult_token: Token from CultScreener

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
            "cultscreener_risk": cult_token.risk_level.value,
            "cultscreener_confidence": cult_token.confidence,
            "cultscreener_reasons": cult_token.reasons,
            "verdict": cult_token.to_verdict(),  # Ground truth label
            "wallet_signals": asdict(wallet_signals) if wallet_signals else None,
            "twitter_signals": asdict(twitter_signals) if twitter_signals else None,
            "timestamp": cult_token.last_updated,
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
            "cultscreener_risk",
            "cultscreener_confidence",
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
                    "cultscreener_risk": token["cultscreener_risk"],
                    "cultscreener_confidence": token["cultscreener_confidence"],
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
        by_cultscreener_risk = {}

        for token in self.enriched_tokens:
            verdict = token["verdict"]
            by_verdict[verdict] = by_verdict.get(verdict, 0) + 1

            risk = token["cultscreener_risk"]
            by_cultscreener_risk[risk] = by_cultscreener_risk.get(risk, 0) + 1

        return {
            "total": len(self.enriched_tokens),
            "by_verdict": by_verdict,
            "by_cultscreener_risk": by_cultscreener_risk,
        }


if __name__ == "__main__":
    ingester = TokenDatasetIngester()

    # Ingest balanced dataset (10 tokens per risk level)
    print("=" * 70)
    print("INGESTING GROUND TRUTH DATASET FROM CULTSCREENER")
    print("=" * 70)

    dataset = ingester.ingest_cultscreener(
        risk_levels=["high", "low", "medium"],
        count_per_level=10,
    )

    # Show statistics
    stats = ingester.get_statistics()
    print("\nDataset Statistics:")
    print(f"  Total tokens: {stats['total']}")
    print(f"  By verdict: {stats['by_verdict']}")
    print(f"  By CultScreener risk: {stats['by_cultscreener_risk']}")

    # Export
    if dataset:
        output_dir = os.path.expanduser("~/.cynic/datasets/tokens/")
        os.makedirs(output_dir, exist_ok=True)

        json_path = ingester.save_json(dataset, f"{output_dir}/ground_truth.json")
        csv_path = ingester.save_csv(dataset, f"{output_dir}/ground_truth.csv")

        print(f"\nDataset exported:")
        print(f"  JSON (full signals): {json_path}")
        print(f"  CSV (summary): {csv_path}")
