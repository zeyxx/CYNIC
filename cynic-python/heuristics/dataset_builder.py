#!/usr/bin/env python3
"""Multi-modal token dataset builder.

Combines three signal channels:
1. Token metrics (on-chain: distribution, authorities, LP)
2. Twitter signals (community: sentiment, engagement, red flags)
3. Wallet signals (holder behavior: accumulation, distribution, bot patterns)

Each token gets scored on all three axes before final verdict.

Usage:
    builder = TokenDatasetBuilder()

    # Add token with all three signals
    builder.add_token(
        name="BONK",
        token_metrics={...},
        twitter_signals={...},
        wallet_signals={...},
        expected_verdict="Howl",  # Ground truth
    )

    # Expand baseline corpus
    corpus = builder.expand_baseline(target_size=40)

    # Measure multi-modal scoring
    results = builder.measure_multi_modal(scorer)
"""

from dataclasses import dataclass
from typing import Optional, Dict, List
import json
import random


@dataclass
class TwitterSignals:
    """Social signal metrics from Hermes twitter analysis."""
    # Community health
    follower_count: int  # Token account followers
    tweet_count: int  # Total tweets about token
    engagement_rate: float  # Avg likes/retweets per tweet (0.0-1.0)

    # Sentiment
    positive_pct: float  # % of positive sentiment tweets (0.0-1.0)
    negative_pct: float  # % of negative sentiment tweets (0.0-1.0)
    neutral_pct: float  # % of neutral/mixed sentiment (0.0-1.0)

    # Red flags
    has_rug_allegations: bool  # Community mentions of rug/scam
    has_creator_criticism: bool  # Criticism of creator/team
    has_buy_pressure_spam: bool  # Spam "buy now" posts (bot-like)
    liquidity_discussion_active: bool  # LP/trading discussion (healthy sign)

    # Recent activity
    tweets_last_7d: int  # Activity in last week
    tweet_velocity: float  # Tweets per day (healthy: 2-10, spam: >20)


@dataclass
class WalletSignals:
    """Holder behavior metrics from Helius enrichment."""
    # Distribution quality
    whale_count: int  # Wallets holding >10% supply (should be 0-1)
    top_10_hold_pct: float  # % held by top 10 wallets (should be <40%)
    bot_score: float  # Estimated % of bot wallets (0.0-1.0)

    # Activity patterns
    daily_active_traders: int  # Wallets trading in last 24h
    avg_hold_duration_days: int  # Average holder time (longer = better)
    accumulation_trend: float  # % holders increasing balance (0.0-1.0, >0.5 = healthy)

    # Exchange vs retail
    exchange_held_pct: float  # % on exchange addresses (should be <20%)
    retail_held_pct: float  # % on retail wallets (should be >70%)
    nft_holder_overlap: Optional[float]  # % of holders also hold SOL NFTs (can indicate real users)


class TokenDatasetBuilder:
    """Build expanded multi-modal token judgment corpus."""

    # Known good tokens (HOWL) - use for bootstrap
    KNOWN_HOWL = {
        "BONK": {
            "description": "Bonk, Solana's Dogecoin. 1.2M holders, major DEX trading.",
            "age_days": 400,
        },
        "JUP": {
            "description": "Jupiter Aggregator. 580K holders, most liquid DEX.",
            "age_days": 600,
        },
        "MARINADE": {
            "description": "mSOL liquid staking token. 50K holders, institutional backed.",
            "age_days": 1200,
        },
        "ORCA": {
            "description": "Orca DEX governance token. Active trading, low concentration.",
            "age_days": 900,
        },
    }

    # Known bad tokens (BARK) - rugs, honeypots, scams
    KNOWN_BARK = {
        "RUG_CLASSIC": {
            "description": "Textbook rug pull. High concentration, dev dumped.",
            "age_days": 2,
        },
        "FREEZE_TRAP": {
            "description": "Freeze authority active. Prevents selling.",
            "age_days": 5,
        },
        "MINT_EXPLOIT": {
            "description": "Active mint authority. Supply inflated 100x.",
            "age_days": 10,
        },
    }

    # Ambiguous (GROWL) - pump.fun legit, real utility but young, etc
    KNOWN_GROWL = {
        "PUMP_FUN_LEGIT": {
            "description": "pump.fun token with 6+ months history and real community.",
            "age_days": 200,
        },
        "NEW_PROTOCOL": {
            "description": "New DeFi protocol, audited, real utility, <30 days.",
            "age_days": 14,
        },
        "NICHE_COMMUNITY": {
            "description": "Real community use case but low liquidity.",
            "age_days": 60,
        },
    }

    def __init__(self):
        self.tokens: Dict[str, dict] = {}

    def add_token(
        self,
        name: str,
        token_metrics: Dict,
        twitter_signals: TwitterSignals,
        wallet_signals: WalletSignals,
        expected_verdict: str,
        notes: str = "",
    ) -> None:
        """Add a token to the corpus."""
        self.tokens[name] = {
            "name": name,
            "token_metrics": token_metrics,
            "twitter_signals": twitter_signals,
            "wallet_signals": wallet_signals,
            "expected_verdict": expected_verdict,
            "notes": notes,
        }

    def generate_synthetic_howl(self) -> dict:
        """Generate synthetic HOWL token (exchange-listed, healthy community)."""
        return {
            "name": "SYNTHETIC_HOWL",
            "token_metrics": {
                "holders": 500_000,
                "top1_pct": 1.5,
                "top10_pct": 5.0,
                "herfindahl": 0.08,
                "age_hours": 8760,  # 1 year
                "mint_authority_active": False,
                "freeze_authority_active": False,
                "lp_burned": True,
                "lp_locked": False,
                "supply_burned_pct": 25.0,
                "origin_pump_fun": False,
                "exchange_listed": True,
            },
            "twitter_signals": TwitterSignals(
                follower_count=50_000,
                tweet_count=2000,
                engagement_rate=0.08,  # 8% engagement
                positive_pct=0.75,  # 75% positive sentiment
                negative_pct=0.05,
                neutral_pct=0.20,
                has_rug_allegations=False,
                has_creator_criticism=False,
                has_buy_pressure_spam=False,
                liquidity_discussion_active=True,
                tweets_last_7d=50,
                tweet_velocity=7.1,  # Healthy activity
            ),
            "wallet_signals": WalletSignals(
                whale_count=0,
                top_10_hold_pct=8.0,
                bot_score=0.05,  # Very low bot %
                daily_active_traders=1000,
                avg_hold_duration_days=120,
                accumulation_trend=0.65,  # 65% accumulating
                exchange_held_pct=15.0,
                retail_held_pct=80.0,
                nft_holder_overlap=0.30,
            ),
            "expected_verdict": "Howl",
            "notes": "Synthetic: established exchange token with healthy community",
        }

    def generate_synthetic_bark(self) -> dict:
        """Generate synthetic BARK token (rug, honeypot, scam)."""
        return {
            "name": "SYNTHETIC_BARK",
            "token_metrics": {
                "holders": 100,
                "top1_pct": 60.0,
                "top10_pct": 95.0,
                "herfindahl": 0.65,
                "age_hours": 24,
                "mint_authority_active": True,
                "freeze_authority_active": True,
                "lp_burned": False,
                "lp_locked": False,
                "supply_burned_pct": None,
                "origin_pump_fun": True,
                "exchange_listed": False,
            },
            "twitter_signals": TwitterSignals(
                follower_count=100,  # Very few followers
                tweet_count=20,
                engagement_rate=0.02,  # Low engagement
                positive_pct=0.20,  # Mostly negative
                negative_pct=0.70,
                neutral_pct=0.10,
                has_rug_allegations=True,
                has_creator_criticism=True,
                has_buy_pressure_spam=True,
                liquidity_discussion_active=False,
                tweets_last_7d=0,
                tweet_velocity=0.0,  # Dead account
            ),
            "wallet_signals": WalletSignals(
                whale_count=3,
                top_10_hold_pct=98.0,
                bot_score=0.80,  # Mostly bots
                daily_active_traders=5,
                avg_hold_duration_days=2,
                accumulation_trend=0.05,  # 95% dumping
                exchange_held_pct=50.0,
                retail_held_pct=5.0,
                nft_holder_overlap=0.0,
            ),
            "expected_verdict": "Bark",
            "notes": "Synthetic: rug pull with active authorities and negative community",
        }

    def generate_synthetic_growl(self) -> dict:
        """Generate synthetic GROWL token (ambiguous - pump.fun legit or real utility but young)."""
        return {
            "name": "SYNTHETIC_GROWL",
            "token_metrics": {
                "holders": 400,
                "top1_pct": 8.0,
                "top10_pct": 30.0,
                "herfindahl": 0.18,
                "age_hours": 1800,  # 75 days
                "mint_authority_active": False,
                "freeze_authority_active": False,
                "lp_burned": True,
                "lp_locked": False,
                "supply_burned_pct": 20.0,
                "origin_pump_fun": True,
                "exchange_listed": False,
            },
            "twitter_signals": TwitterSignals(
                follower_count=5000,
                tweet_count=300,
                engagement_rate=0.05,
                positive_pct=0.55,  # Mixed sentiment
                negative_pct=0.20,
                neutral_pct=0.25,
                has_rug_allegations=False,  # No rug allegations
                has_creator_criticism=False,
                has_buy_pressure_spam=False,  # No spam
                liquidity_discussion_active=True,
                tweets_last_7d=10,
                tweet_velocity=1.4,  # Modest activity
            ),
            "wallet_signals": WalletSignals(
                whale_count=1,
                top_10_hold_pct=40.0,  # Moderate concentration
                bot_score=0.20,  # Some bot activity
                daily_active_traders=30,
                avg_hold_duration_days=40,
                accumulation_trend=0.45,  # Mixed
                exchange_held_pct=25.0,
                retail_held_pct=70.0,
                nft_holder_overlap=0.15,
            ),
            "expected_verdict": "Growl",
            "notes": "Synthetic: pump.fun token with real community, survived 75 days, but still uncertain",
        }

    def _randomize_profile(self, base_profile: dict, variance: float = 0.15) -> dict:
        """Add randomization to synthetic profile to increase diversity.

        variance: 0.0-1.0, how much to vary numeric fields (as fraction of base value)
        """
        profile = base_profile.copy()

        # Randomize token metrics
        metrics = profile["token_metrics"].copy()
        for key in ["holders", "top1_pct", "top10_pct", "herfindahl"]:
            if key in metrics and isinstance(metrics[key], (int, float)):
                base = metrics[key]
                factor = 1.0 + random.uniform(-variance, variance)
                if key == "holders":
                    metrics[key] = max(int(base * factor), 10)
                else:
                    metrics[key] = max(base * factor, 0)
        profile["token_metrics"] = metrics

        # Randomize twitter signals
        twitter = profile["twitter_signals"].__dict__.copy()
        for key in ["follower_count", "tweet_count", "engagement_rate", "positive_pct",
                    "negative_pct", "neutral_pct", "tweets_last_7d", "tweet_velocity"]:
            if key in twitter and isinstance(twitter[key], (int, float)):
                base = twitter[key]
                factor = 1.0 + random.uniform(-variance, variance)
                if key in ["follower_count", "tweet_count", "tweets_last_7d"]:
                    twitter[key] = max(int(base * factor), 0)
                else:
                    twitter[key] = max(base * factor, 0.0)
        profile["twitter_signals"] = TwitterSignals(**twitter)

        # Randomize wallet signals
        wallet = profile["wallet_signals"].__dict__.copy()
        for key in ["whale_count", "top_10_hold_pct", "bot_score", "daily_active_traders",
                    "avg_hold_duration_days", "accumulation_trend", "exchange_held_pct",
                    "retail_held_pct", "nft_holder_overlap"]:
            if key in wallet and isinstance(wallet[key], (int, float)) and wallet[key] is not None:
                base = wallet[key]
                factor = 1.0 + random.uniform(-variance, variance)
                if key in ["whale_count", "daily_active_traders", "avg_hold_duration_days"]:
                    wallet[key] = max(int(base * factor), 0)
                else:
                    wallet[key] = max(base * factor, 0.0)
        profile["wallet_signals"] = WalletSignals(**wallet)

        return profile

    def expand_baseline(self, target_size: int = 40) -> List[dict]:
        """Expand baseline corpus to target size with synthetic tokens.

        Generates synthetic HOWL/BARK/GROWL tokens to reach target_size.
        Maintains 1:1:1 ratio across categories.
        Adds 15% randomization to each profile for diversity.
        """
        tokens_per_category = target_size // 3

        corpus = []

        # Add synthetic HOWL tokens with randomization
        for i in range(tokens_per_category):
            token = self.generate_synthetic_howl()
            token = self._randomize_profile(token, variance=0.15)
            token["name"] = f"HOWL_SYNTHETIC_{i+1}"
            corpus.append(token)

        # Add synthetic BARK tokens with randomization
        for i in range(tokens_per_category):
            token = self.generate_synthetic_bark()
            token = self._randomize_profile(token, variance=0.15)
            token["name"] = f"BARK_SYNTHETIC_{i+1}"
            corpus.append(token)

        # Add synthetic GROWL tokens with randomization
        for i in range(tokens_per_category):
            token = self.generate_synthetic_growl()
            token = self._randomize_profile(token, variance=0.15)
            token["name"] = f"GROWL_SYNTHETIC_{i+1}"
            corpus.append(token)

        return corpus

    def to_json(self, output_path: str = "/tmp/token_corpus_expanded.json") -> str:
        """Export corpus to JSON."""
        corpus = []
        for token_data in self.tokens.values():
            # Convert dataclasses to dicts
            token_dict = token_data.copy()
            if isinstance(token_data["twitter_signals"], TwitterSignals):
                token_dict["twitter_signals"] = token_data["twitter_signals"].__dict__
            if isinstance(token_data["wallet_signals"], WalletSignals):
                token_dict["wallet_signals"] = token_data["wallet_signals"].__dict__

            corpus.append(token_dict)

        with open(output_path, "w") as f:
            json.dump(corpus, f, indent=2, default=str)

        return output_path


if __name__ == "__main__":
    builder = TokenDatasetBuilder()

    # Generate expanded corpus
    print("Generating expanded corpus (40 tokens: 1:1:1 ratio)...")
    corpus = builder.expand_baseline(target_size=40)

    print(f"Generated {len(corpus)} tokens:")
    for category in ["HOWL", "BARK", "GROWL"]:
        count = sum(1 for t in corpus if category in t["name"])
        print(f"  {category}: {count}")

    # Export
    output_path = builder.to_json()
    print(f"\nCorpus saved to: {output_path}")
    print(f"Token count: {len(corpus)}")
