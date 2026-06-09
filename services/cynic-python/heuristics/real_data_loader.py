#!/usr/bin/env python3
"""Load real token data from Helius (wallet) and Hermes (twitter).

This module bridges external APIs into the heuristics framework.
Real data replaces synthetic profiles for accurate calibration.

Usage:
    loader = RealDataLoader()

    # Fetch wallet signals for a token (via Helius)
    wallet_signals = loader.fetch_wallet_signals(mint_address)

    # Fetch twitter signals for a token (via Hermes)
    twitter_signals = loader.fetch_twitter_signals(token_name)

Note: Requires environment variables:
    - CYNIC_REST_ADDR: Kernel REST endpoint
    - CYNIC_API_KEY: Auth token
    - HELIUS_API_KEY: Helius API key (optional, uses CYNIC kernel as proxy)
"""

from dataclasses import dataclass
from typing import Optional, Dict
import os
import requests
from dataset_builder import WalletSignals, TwitterSignals


class RealDataLoader:
    """Load real signal data from production APIs."""

    def __init__(self):
        self.kernel_addr = os.getenv("CYNIC_REST_ADDR", "http://localhost:3030")
        self.api_key = os.getenv("CYNIC_API_KEY", "")
        self.hermes_addr = os.getenv("HERMES_ADDR", "http://localhost:9999")

    def fetch_wallet_signals(self, mint_address: str) -> Optional[WalletSignals]:
        """Fetch wallet holder behavior signals via Helius API (proxied through kernel).

        Args:
            mint_address: Token mint address (base58)

        Returns:
            WalletSignals with holder distribution, bot score, trading activity
        """
        try:
            # Ask kernel to enrich the token with wallet signals
            response = requests.post(
                f"{self.kernel_addr}/enrich/wallet",
                json={"mint": mint_address},
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=30,
            )
            if response.status_code != 200:
                print(f"Wallet enrichment failed: {response.status_code} {response.text}")
                return None

            data = response.json()

            # Map Helius response to WalletSignals
            return WalletSignals(
                whale_count=data.get("whale_count", 0),
                top_10_hold_pct=data.get("top_10_hold_pct", 0.0),
                bot_score=data.get("estimated_bot_pct", 0.0),
                daily_active_traders=data.get("daily_active_traders", 0),
                avg_hold_duration_days=data.get("avg_hold_duration_days", 0),
                accumulation_trend=data.get("accumulation_trend", 0.0),
                exchange_held_pct=data.get("exchange_held_pct", 0.0),
                retail_held_pct=data.get("retail_held_pct", 0.0),
                nft_holder_overlap=data.get("nft_holder_overlap", None),
            )
        except Exception as e:
            print(f"Error fetching wallet signals: {e}")
            return None

    def fetch_twitter_signals(self, token_name: str) -> Optional[TwitterSignals]:
        """Fetch twitter community signals via Hermes API.

        Args:
            token_name: Token name or symbol (e.g., "BONK", "JUP")

        Returns:
            TwitterSignals with sentiment, engagement, red flags
        """
        try:
            # Ask Hermes for twitter analysis
            response = requests.post(
                f"{self.hermes_addr}/analyze/twitter",
                json={"token": token_name},
                timeout=30,
            )
            if response.status_code != 200:
                print(f"Twitter analysis failed: {response.status_code} {response.text}")
                return None

            data = response.json()

            # Map Hermes response to TwitterSignals
            return TwitterSignals(
                follower_count=data.get("follower_count", 0),
                tweet_count=data.get("total_tweets", 0),
                engagement_rate=data.get("engagement_rate", 0.0),
                positive_pct=data.get("positive_pct", 0.0),
                negative_pct=data.get("negative_pct", 0.0),
                neutral_pct=data.get("neutral_pct", 0.0),
                has_rug_allegations=data.get("has_rug_allegations", False),
                has_creator_criticism=data.get("has_creator_criticism", False),
                has_buy_pressure_spam=data.get("has_buy_pressure_spam", False),
                liquidity_discussion_active=data.get("liquidity_discussion_active", False),
                tweets_last_7d=data.get("tweets_last_7d", 0),
                tweet_velocity=data.get("tweet_velocity", 0.0),
            )
        except Exception as e:
            print(f"Error fetching twitter signals: {e}")
            return None

    def load_dataset_from_tokens(self, token_list: list) -> list:
        """Load real signals for a list of tokens.

        Args:
            token_list: List of dicts with "mint" (wallet) and "name" (twitter) keys

        Returns:
            List of enriched token dicts with real wallet and twitter signals
        """
        dataset = []

        for token in token_list:
            mint = token.get("mint")
            name = token.get("name")

            # Fetch real signals
            wallet_signals = self.fetch_wallet_signals(mint) if mint else None
            twitter_signals = self.fetch_twitter_signals(name) if name else None

            if wallet_signals and twitter_signals:
                dataset.append({
                    "name": name,
                    "mint": mint,
                    "wallet_signals": wallet_signals,
                    "twitter_signals": twitter_signals,
                })

        return dataset


if __name__ == "__main__":
    loader = RealDataLoader()

    # Example: fetch signals for BONK
    print("Fetching real signals for BONK...")
    wallet = loader.fetch_wallet_signals("DezXAZ8z7PnrnRJjz3wXBoRgixVqXaSo1S1zceA85q")
    if wallet:
        print(f"Wallet: whale_count={wallet.whale_count}, bot_score={wallet.bot_score:.2%}")
    else:
        print("(Wallet fetch requires kernel running)")

    twitter = loader.fetch_twitter_signals("BONK")
    if twitter:
        print(f"Twitter: followers={twitter.follower_count}, engagement={twitter.engagement_rate:.2%}")
    else:
        print("(Twitter fetch requires Hermes running)")
