#!/usr/bin/env python3
"""Create a mock ground-truth dataset for testing the measurement pipeline.

Uses realistic token profiles based on known tokens (BONK, JUP, etc.)
and known rugs, to test the calibration framework without external APIs.

This is a substitute for CultScreener when the API is unavailable.
"""

import json
import os
from dataclasses import asdict

from dataset_builder import TwitterSignals, WalletSignals


# Known rugs (BARK)
BARK_TOKENS = [
    {
        "mint": "RUG001aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "RUG_CLASSIC",
        "name": "Classic Rug Pull",
        "cultscreener_risk": "high",
        "cultscreener_confidence": 0.99,
        "cultscreener_reasons": ["active_mint_authority", "high_concentration", "dev_dump"],
        "verdict": "Bark",
        "wallet_signals": {
            "whale_count": 3,
            "top_10_hold_pct": 85.0,
            "bot_score": 0.80,
            "daily_active_traders": 5,
            "avg_hold_duration_days": 2,
            "accumulation_trend": 0.05,
            "exchange_held_pct": 50.0,
            "retail_held_pct": 5.0,
            "nft_holder_overlap": 0.0,
        },
        "twitter_signals": {
            "follower_count": 100,
            "tweet_count": 20,
            "engagement_rate": 0.02,
            "positive_pct": 0.20,
            "negative_pct": 0.70,
            "neutral_pct": 0.10,
            "has_rug_allegations": True,
            "has_creator_criticism": True,
            "has_buy_pressure_spam": True,
            "liquidity_discussion_active": False,
            "tweets_last_7d": 0,
            "tweet_velocity": 0.0,
        },
    },
    {
        "mint": "RUG002aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "HONEYPOT",
        "name": "Honeypot Token",
        "cultscreener_risk": "high",
        "cultscreener_confidence": 0.98,
        "cultscreener_reasons": ["freeze_authority_active", "mint_authority_active"],
        "verdict": "Bark",
        "wallet_signals": {
            "whale_count": 2,
            "top_10_hold_pct": 82.0,
            "bot_score": 0.75,
            "daily_active_traders": 8,
            "avg_hold_duration_days": 3,
            "accumulation_trend": 0.08,
            "exchange_held_pct": 45.0,
            "retail_held_pct": 8.0,
            "nft_holder_overlap": 0.0,
        },
        "twitter_signals": {
            "follower_count": 150,
            "tweet_count": 35,
            "engagement_rate": 0.015,
            "positive_pct": 0.15,
            "negative_pct": 0.75,
            "neutral_pct": 0.10,
            "has_rug_allegations": True,
            "has_creator_criticism": True,
            "has_buy_pressure_spam": True,
            "liquidity_discussion_active": False,
            "tweets_last_7d": 1,
            "tweet_velocity": 0.1,
        },
    },
    {
        "mint": "RUG003aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "SCAM_PUMP",
        "name": "Scam Pump Token",
        "cultscreener_risk": "high",
        "cultscreener_confidence": 0.96,
        "cultscreener_reasons": ["extreme_concentration", "bot_wallets", "zero_utility"],
        "verdict": "Bark",
        "wallet_signals": {
            "whale_count": 4,
            "top_10_hold_pct": 92.0,
            "bot_score": 0.85,
            "daily_active_traders": 3,
            "avg_hold_duration_days": 1,
            "accumulation_trend": 0.02,
            "exchange_held_pct": 55.0,
            "retail_held_pct": 2.0,
            "nft_holder_overlap": 0.0,
        },
        "twitter_signals": {
            "follower_count": 50,
            "tweet_count": 10,
            "engagement_rate": 0.01,
            "positive_pct": 0.10,
            "negative_pct": 0.80,
            "neutral_pct": 0.10,
            "has_rug_allegations": True,
            "has_creator_criticism": True,
            "has_buy_pressure_spam": True,
            "liquidity_discussion_active": False,
            "tweets_last_7d": 0,
            "tweet_velocity": 0.0,
        },
    },
]

# Known legitimate tokens (HOWL)
HOWL_TOKENS = [
    {
        "mint": "DezXAZ8z7PnrnRJjz3wXBoRgixVqXaSo1S1zceA85q",
        "symbol": "BONK",
        "name": "Bonk, Solana's Dogecoin",
        "cultscreener_risk": "low",
        "cultscreener_confidence": 0.97,
        "cultscreener_reasons": ["distributed_holders", "active_trading", "major_exchange_listed"],
        "verdict": "Howl",
        "token_metrics": {
            "holders": 1200000,
            "top1_pct": 0.8,
            "top10_pct": 3.5,
            "herfindahl": 0.03,
            "age_hours": 10000,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 50.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "wallet_signals": {
            "whale_count": 0,
            "top_10_hold_pct": 3.5,
            "bot_score": 0.02,
            "daily_active_traders": 1000,
            "avg_hold_duration_days": 120,
            "accumulation_trend": 0.65,
            "exchange_held_pct": 15.0,
            "retail_held_pct": 80.0,
            "nft_holder_overlap": 0.30,
        },
        "twitter_signals": {
            "follower_count": 100000,
            "tweet_count": 5000,
            "engagement_rate": 0.08,
            "positive_pct": 0.75,
            "negative_pct": 0.05,
            "neutral_pct": 0.20,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 100,
            "tweet_velocity": 10.0,
        },
    },
    {
        "mint": "JUPyiwrYJFskE4Zr5beJWXo8tRF4ZvLn8z5chd6tb1j",
        "symbol": "JUP",
        "name": "Jupiter Aggregator",
        "cultscreener_risk": "low",
        "cultscreener_confidence": 0.96,
        "cultscreener_reasons": ["decentralized_governance", "active_development", "major_exchange_listed"],
        "verdict": "Howl",
        "token_metrics": {
            "holders": 580000,
            "top1_pct": 1.2,
            "top10_pct": 5.0,
            "herfindahl": 0.05,
            "age_hours": 8760,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 30.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "wallet_signals": {
            "whale_count": 0,
            "top_10_hold_pct": 5.0,
            "bot_score": 0.03,
            "daily_active_traders": 800,
            "avg_hold_duration_days": 100,
            "accumulation_trend": 0.60,
            "exchange_held_pct": 12.0,
            "retail_held_pct": 82.0,
            "nft_holder_overlap": 0.25,
        },
        "twitter_signals": {
            "follower_count": 80000,
            "tweet_count": 3500,
            "engagement_rate": 0.07,
            "positive_pct": 0.72,
            "negative_pct": 0.08,
            "neutral_pct": 0.20,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 80,
            "tweet_velocity": 9.0,
        },
    },
    {
        "mint": "MarBmsSgKXdrQEffljWfQSnrYm64aYs2MNvGeZZoJo",
        "symbol": "MARINADE",
        "name": "Marinade Liquid Staking",
        "cultscreener_risk": "low",
        "cultscreener_confidence": 0.95,
        "cultscreener_reasons": ["institutional_backed", "audited_code", "active_development"],
        "verdict": "Howl",
        "token_metrics": {
            "holders": 125000,
            "top1_pct": 2.5,
            "top10_pct": 8.0,
            "herfindahl": 0.12,
            "age_hours": 7200,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 25.0,
            "origin_pump_fun": False,
            "exchange_listed": True,
        },
        "wallet_signals": {
            "whale_count": 1,
            "top_10_hold_pct": 8.0,
            "bot_score": 0.01,
            "daily_active_traders": 500,
            "avg_hold_duration_days": 180,
            "accumulation_trend": 0.70,
            "exchange_held_pct": 10.0,
            "retail_held_pct": 85.0,
            "nft_holder_overlap": 0.20,
        },
        "twitter_signals": {
            "follower_count": 50000,
            "tweet_count": 2000,
            "engagement_rate": 0.06,
            "positive_pct": 0.70,
            "negative_pct": 0.10,
            "neutral_pct": 0.20,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 50,
            "tweet_velocity": 7.0,
        },
    },
]

# Ambiguous tokens (GROWL - pump.fun legit but young)
GROWL_TOKENS = [
    {
        "mint": "GROW001aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "PUMP_LEGIT",
        "name": "Pump.Fun Legitimate Token",
        "cultscreener_risk": "medium",
        "cultscreener_confidence": 0.65,
        "cultscreener_reasons": ["young_token", "real_community", "no_rug_indicators"],
        "verdict": "Growl",
        "token_metrics": {
            "holders": 500,
            "top1_pct": 8.0,
            "top10_pct": 35.0,
            "herfindahl": 0.20,
            "age_hours": 2000,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 15.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
        "wallet_signals": {
            "whale_count": 1,
            "top_10_hold_pct": 35.0,
            "bot_score": 0.15,
            "daily_active_traders": 50,
            "avg_hold_duration_days": 45,
            "accumulation_trend": 0.50,
            "exchange_held_pct": 20.0,
            "retail_held_pct": 75.0,
            "nft_holder_overlap": 0.10,
        },
        "twitter_signals": {
            "follower_count": 8000,
            "tweet_count": 400,
            "engagement_rate": 0.05,
            "positive_pct": 0.60,
            "negative_pct": 0.15,
            "neutral_pct": 0.25,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 15,
            "tweet_velocity": 1.8,
        },
    },
    {
        "mint": "GROW002aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "NEW_PROTO",
        "name": "New DeFi Protocol",
        "cultscreener_risk": "medium",
        "cultscreener_confidence": 0.62,
        "cultscreener_reasons": ["new_protocol", "real_utility", "active_development"],
        "verdict": "Growl",
        "token_metrics": {
            "holders": 200,
            "top1_pct": 5.0,
            "top10_pct": 25.0,
            "herfindahl": 0.15,
            "age_hours": 120,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 10.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
        "wallet_signals": {
            "whale_count": 1,
            "top_10_hold_pct": 40.0,
            "bot_score": 0.20,
            "daily_active_traders": 40,
            "avg_hold_duration_days": 35,
            "accumulation_trend": 0.48,
            "exchange_held_pct": 22.0,
            "retail_held_pct": 72.0,
            "nft_holder_overlap": 0.12,
        },
        "twitter_signals": {
            "follower_count": 6000,
            "tweet_count": 300,
            "engagement_rate": 0.045,
            "positive_pct": 0.58,
            "negative_pct": 0.17,
            "neutral_pct": 0.25,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 12,
            "tweet_velocity": 1.5,
        },
    },
    {
        "mint": "GROW003aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "symbol": "NICHE_COM",
        "name": "Niche Community Token",
        "cultscreener_risk": "medium",
        "cultscreener_confidence": 0.60,
        "cultscreener_reasons": ["niche_community", "real_use_case", "low_liquidity"],
        "verdict": "Growl",
        "token_metrics": {
            "holders": 300,
            "top1_pct": 25.0,
            "top10_pct": 65.0,
            "herfindahl": 0.35,
            "age_hours": 600,
            "mint_authority_active": False,
            "freeze_authority_active": False,
            "lp_burned": True,
            "lp_locked": False,
            "supply_burned_pct": 20.0,
            "origin_pump_fun": True,
            "exchange_listed": False,
        },
        "wallet_signals": {
            "whale_count": 1,
            "top_10_hold_pct": 38.0,
            "bot_score": 0.18,
            "daily_active_traders": 45,
            "avg_hold_duration_days": 55,
            "accumulation_trend": 0.52,
            "exchange_held_pct": 25.0,
            "retail_held_pct": 70.0,
            "nft_holder_overlap": 0.08,
        },
        "twitter_signals": {
            "follower_count": 7000,
            "tweet_count": 350,
            "engagement_rate": 0.048,
            "positive_pct": 0.62,
            "negative_pct": 0.12,
            "neutral_pct": 0.26,
            "has_rug_allegations": False,
            "has_creator_criticism": False,
            "has_buy_pressure_spam": False,
            "liquidity_discussion_active": True,
            "tweets_last_7d": 18,
            "tweet_velocity": 2.0,
        },
    },
]


def create_mock_dataset():
    """Create mock ground-truth dataset."""
    dataset = []

    # Add all token categories
    for token in BARK_TOKENS:
        dataset.append(token)
    for token in HOWL_TOKENS:
        dataset.append(token)
    for token in GROWL_TOKENS:
        dataset.append(token)

    return dataset


if __name__ == "__main__":
    dataset = create_mock_dataset()

    # Save JSON
    output_dir = os.path.expanduser("~/.cynic/datasets/tokens/")
    os.makedirs(output_dir, exist_ok=True)

    json_path = os.path.join(output_dir, "ground_truth.json")
    with open(json_path, "w") as f:
        json.dump(dataset, f, indent=2)

    print(f"Created mock dataset: {json_path}")
    print(f"Tokens: {len(dataset)}")
    print(f"  BARK (rugs): {len(BARK_TOKENS)}")
    print(f"  HOWL (legit): {len(HOWL_TOKENS)}")
    print(f"  GROWL (ambiguous): {len(GROWL_TOKENS)}")
