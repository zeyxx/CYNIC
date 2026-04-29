#!/usr/bin/env python3
"""Token dataset ingester — conviction-only baseline (no enrichment).

Minimal variant: ingest CultScreener conviction scores directly,
map to verdicts, validate conviction→verdict mapping.

Usage:
    ingester = TokenDatasetIngesterConvictionOnly()
    dataset = ingester.ingest_cultscreener_conviction_only(
        conviction_tiers=["strong", "mixed", "weak"],
        count_per_tier=20
    )
    # → 60 tokens with conviction labels only
"""

import json
import csv
from typing import List, Optional, Dict
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


class TokenDatasetIngesterConvictionOnly:
    """Ingest conviction scores from CultScreener, no enrichment."""

    def __init__(self):
        self.cultscreener = CultScreenerClient()
        self.tokens: List[Dict] = []

    def ingest_cultscreener_conviction_only(
        self,
        conviction_tiers: Optional[List[str]] = None,
        count_per_tier: int = 20,
    ) -> List[Dict]:
        """Ingest tokens from CultScreener with conviction-only labels.

        Args:
            conviction_tiers: Which conviction tiers to fetch (default: ["strong", "mixed", "weak"])
            count_per_tier: How many tokens per conviction tier (for balanced dataset)

        Returns:
            List of token dicts with conviction + verdict (no enrichment signals)
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
            print(f"  Filters: min={filters.get('min_conviction')}, max={filters.get('max_conviction')}")

            # Fetch with higher limit to account for client-side filtering
            # Client-side filter may exclude tokens, so fetch more and filter
            fetch_limit = count_per_tier * 3  # Fetch 3x to account for filtering loss
            collected = []

            # Try pagination if needed
            for offset_multiplier in range(1, 4):  # Try up to 3 pages
                tokens = self.cultscreener.get_leaderboard(
                    limit=fetch_limit,
                    offset=(offset_multiplier - 1) * fetch_limit,
                    min_conviction=filters.get("min_conviction"),
                    max_conviction=filters.get("max_conviction"),
                )

                if not tokens:
                    if offset_multiplier == 1:
                        print(f"  ⚠ No tokens returned for {tier} conviction tier")
                    break

                print(f"    Page {offset_multiplier}: fetched {len(tokens)} tokens after filtering")
                collected.extend(tokens)

                if len(collected) >= count_per_tier:
                    break

            # Take only what we need
            selected = collected[:count_per_tier]
            print(f"  Selected {len(selected)} tokens for this tier")

            for cult_token in selected:
                token_dict = {
                    "mint": cult_token.mint,
                    "name": cult_token.name,
                    "symbol": cult_token.symbol,
                    "cultscreener_conviction": cult_token.conviction,
                    "cultscreener_conviction_tier": cult_token.conviction_tier.value,
                    "verdict": cult_token.to_verdict(),  # Ground truth label
                    "timestamp": cult_token.timestamp,
                }
                all_tokens.append(token_dict)
                print(f"    ✓ {cult_token.symbol or cult_token.mint[:8]} → {token_dict['verdict']} "
                      f"(conviction={cult_token.conviction:.2f})")

        self.tokens = all_tokens
        print(f"\nIngested {len(all_tokens)} tokens from CultScreener (conviction only)")
        return all_tokens

    def save_json(self, dataset: List[Dict], output_path: str) -> str:
        """Save dataset as JSON."""
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
        with open(output_path, "w") as f:
            json.dump(dataset, f, indent=2, default=str)
        print(f"Saved {len(dataset)} tokens to {output_path}")
        return output_path

    def save_csv(self, dataset: List[Dict], output_path: str) -> str:
        """Save dataset as CSV (human-readable summary)."""
        os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)

        fieldnames = [
            "mint",
            "symbol",
            "name",
            "verdict",  # Ground truth
            "cultscreener_conviction",
            "cultscreener_conviction_tier",
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
                writer.writerow(row)

        print(f"Saved {len(dataset)} tokens to {output_path} (CSV summary)")
        return output_path

    def get_statistics(self) -> Dict:
        """Get statistics about the ingested dataset."""
        if not self.tokens:
            return {"total": 0}

        by_verdict = {}
        by_conviction_tier = {}

        for token in self.tokens:
            verdict = token["verdict"]
            by_verdict[verdict] = by_verdict.get(verdict, 0) + 1

            tier = token.get("cultscreener_conviction_tier", "unknown")
            by_conviction_tier[tier] = by_conviction_tier.get(tier, 0) + 1

        return {
            "total": len(self.tokens),
            "by_verdict": by_verdict,
            "by_conviction_tier": by_conviction_tier,
        }


if __name__ == "__main__":
    ingester = TokenDatasetIngesterConvictionOnly()

    # Ingest balanced dataset (20 tokens per conviction tier)
    print("=" * 70)
    print("INGESTING CONVICTION-ONLY DATASET FROM CULTSCREENER")
    print("=" * 70)

    dataset = ingester.ingest_cultscreener_conviction_only(
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

        json_path = ingester.save_json(dataset, f"{output_dir}/ground_truth_conviction_only.json")
        csv_path = ingester.save_csv(dataset, f"{output_dir}/ground_truth_conviction_only.csv")

        print(f"\nDataset exported:")
        print(f"  JSON: {json_path}")
        print(f"  CSV: {csv_path}")
