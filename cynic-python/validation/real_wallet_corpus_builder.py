#!/usr/bin/env python3
"""
Real Wallet Corpus Builder — Extract from Production Data Sources

Sources:
  - CultScreener API: rug/honeypot tokens → creator wallets (sybils)
  - DexScreener: recent trades → trader wallets (humans)
  - Organ X captures: Twitter mentions with wallet addresses (mixed humans/bots)

Usage:
  python3 real_wallet_corpus_builder.py [--source cultscreener|dexscreener|organ-x|all]
"""

import json
import logging
import re
import os
from typing import List, Dict, Optional, Tuple
from datetime import datetime
import requests
from pathlib import Path

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


class CultScreenerCollector:
    """Extract sybil wallets from CultScreener tokens."""

    BASE_URL = "https://cultscreener-api.onrender.com"
    CONVICTION_THRESHOLDS = {
        "rug": (0, 20),      # Very low conviction = likely rug
        "honeypot": (0, 25),  # Low conviction = suspicious
        "high_quality": (80, 100),  # High conviction = legitimate
    }

    def __init__(self, timeout: int = 30):
        self.timeout = timeout

    def fetch_tokens_by_conviction(
        self, conviction_min: int = 0, conviction_max: int = 100, limit: int = 10
    ) -> List[Dict]:
        """Fetch tokens from CultScreener sorted by conviction.

        Args:
            conviction_min: Minimum conviction threshold
            conviction_max: Maximum conviction threshold
            limit: Number of tokens to fetch

        Returns:
            List of token records with mint, symbol, conviction, creator
        """
        try:
            url = f"{self.BASE_URL}/api/tokens/leaderboard/conviction"
            params = {
                "conviction_min": conviction_min,
                "conviction_max": conviction_max,
                "limit": limit,
            }

            response = requests.get(url, params=params, timeout=self.timeout)
            response.raise_for_status()
            data = response.json()

            tokens = data.get("tokens", [])
            logger.info(
                f"Fetched {len(tokens)} tokens (conviction {conviction_min}-{conviction_max})"
            )
            return tokens

        except requests.RequestException as e:
            logger.error(f"CultScreener fetch failed: {e}")
            return []

    def extract_wallets(self, conviction_range: Tuple[int, int]) -> List[Dict]:
        """Extract wallet addresses from tokens in conviction range.

        Args:
            conviction_range: (min, max) tuple for conviction filtering

        Returns:
            List of dicts: {address, label, conviction, mint, reason}
        """
        wallets = []
        tokens = self.fetch_tokens_by_conviction(
            conviction_min=conviction_range[0],
            conviction_max=conviction_range[1],
            limit=10,
        )

        for token in tokens:
            # Try to extract creator/deployer wallet
            creator = token.get("creator") or token.get("deployer")
            if creator and self._is_valid_solana_address(creator):
                wallets.append(
                    {
                        "address": creator,
                        "label": f"CultScreener Token Creator ({token.get('symbol', 'UNKNOWN')})",
                        "source": "cultscreener",
                        "conviction": token.get("conviction", 0),
                        "mint": token.get("mint"),
                        "is_sybil_candidate": conviction_range[0] < 25,
                    }
                )

        logger.info(f"Extracted {len(wallets)} wallets from CultScreener")
        return wallets

    @staticmethod
    def _is_valid_solana_address(addr: str) -> bool:
        """Check if address looks like valid base58 Solana address."""
        return bool(addr) and len(addr) >= 40 and len(addr) <= 48


class DexScreenerCollector:
    """Extract trader wallets from recent DexScreener trades (placeholder)."""

    BASE_URL = "https://api.dexscreener.com/latest"

    def __init__(self, timeout: int = 30):
        self.timeout = timeout

    def fetch_recent_trades(self, limit: int = 10) -> List[Dict]:
        """Fetch recent trades from DexScreener.

        Note: DexScreener API structure varies. This is a placeholder.
        May need to use specific token searches instead.

        Returns:
            List of trade records with trader addresses
        """
        # DexScreener doesn't expose a direct "recent trades" endpoint
        # Would need to query specific pairs and parse transactions
        logger.warning(
            "DexScreener direct trade API not available. Use pair-specific queries."
        )
        return []

    def extract_wallets(self) -> List[Dict]:
        """Placeholder: DexScreener extraction would require token-specific queries."""
        return []


class OrganXCollector:
    """Extract wallet addresses from Organ X Twitter captures."""

    CAPTURES_DIR = Path.home() / ".cynic/organs/hermes/x/captures"
    WALLET_PATTERN = re.compile(r"\b[1-9A-HJ-NP-Z]{44}\b")  # Base58 Solana addresses

    def __init__(self):
        pass

    def extract_wallets_from_captures(self) -> List[Dict]:
        """Parse all captured tweets for wallet addresses.

        Returns:
            List of dicts: {address, source_tweet, mentions, context}
        """
        wallets = {}

        if not self.CAPTURES_DIR.exists():
            logger.warning(f"Organ X captures directory not found: {self.CAPTURES_DIR}")
            return []

        capture_files = sorted(self.CAPTURES_DIR.glob("*.json"))
        logger.info(f"Found {len(capture_files)} Organ X capture files")

        for capture_file in capture_files:
            try:
                with open(capture_file) as f:
                    data = json.load(f)

                # Extract tweets from the capture
                tweets = self._extract_tweets_from_capture(data)

                for tweet in tweets:
                    # Search tweet text for wallet addresses
                    text = tweet.get("text", "")
                    matches = self.WALLET_PATTERN.findall(text)

                    for addr in matches:
                        if addr not in wallets:
                            wallets[addr] = {
                                "address": addr,
                                "source": "organ_x_twitter",
                                "mentions": 0,
                                "contexts": [],
                            }

                        wallets[addr]["mentions"] += 1
                        wallets[addr]["contexts"].append(
                            {
                                "tweet_id": tweet.get("id"),
                                "author": tweet.get("author", "unknown"),
                                "timestamp": tweet.get("timestamp"),
                                "text_snippet": text[:200],
                            }
                        )

            except Exception as e:
                logger.debug(f"Failed to parse {capture_file.name}: {e}")
                continue

        logger.info(f"Extracted {len(wallets)} unique wallets from Organ X captures")
        return list(wallets.values())

    @staticmethod
    def _extract_tweets_from_capture(data: Dict) -> List[Dict]:
        """Parse GraphQL response structure to extract tweets."""
        tweets = []

        try:
            # Navigate nested GraphQL response
            instructions = (
                data.get("response", {})
                .get("data", {})
                .get("home", {})
                .get("home_timeline_urt", {})
                .get("instructions", [])
            )

            for instruction in instructions:
                entries = instruction.get("entries", [])
                for entry in entries:
                    item_content = (
                        entry.get("content", {})
                        .get("itemContent", {})
                        .get("tweet_results", {})
                        .get("result", {})
                    )

                    if item_content.get("__typename") == "Tweet":
                        tweet_text = item_content.get("legacy", {}).get("full_text", "")
                        tweet_id = item_content.get("rest_id", "")
                        author = (
                            item_content.get("core", {})
                            .get("user_results", {})
                            .get("result", {})
                            .get("legacy", {})
                            .get("screen_name", "")
                        )

                        if tweet_text:
                            tweets.append(
                                {
                                    "id": tweet_id,
                                    "text": tweet_text,
                                    "author": author,
                                    "timestamp": data.get("timestamp"),
                                }
                            )

        except Exception as e:
            logger.debug(f"GraphQL parse error: {e}")

        return tweets


def merge_and_label_wallets(
    cultscreener_wallets: List[Dict],
    organx_wallets: List[Dict],
) -> List[Dict]:
    """Merge wallets from multiple sources with conflict resolution.

    Returns:
        Unified list with source attribution and signal confidence
    """
    wallets_by_addr = {}

    # Add CultScreener wallets (high-confidence sybils for low conviction)
    for w in cultscreener_wallets:
        addr = w["address"]
        if addr not in wallets_by_addr:
            wallets_by_addr[addr] = {
                "address": addr,
                "sources": [],
                "is_sybil_candidate": w.get("is_sybil_candidate", False),
                "conviction": w.get("conviction", 0.5),
                "metadata": {},
            }
        wallets_by_addr[addr]["sources"].append(
            {"source": "cultscreener", "label": w["label"]}
        )
        wallets_by_addr[addr]["metadata"].update(w)

    # Add Organ X wallets (mixed signal, needs validation)
    for w in organx_wallets:
        addr = w["address"]
        if addr not in wallets_by_addr:
            wallets_by_addr[addr] = {
                "address": addr,
                "sources": [],
                "is_sybil_candidate": False,  # Organ X is mixed
                "conviction": 0.5,
                "metadata": {},
            }
        wallets_by_addr[addr]["sources"].append(
            {"source": "organ_x", "mentions": w["mentions"]}
        )
        wallets_by_addr[addr]["metadata"].update(w)

    return list(wallets_by_addr.values())


if __name__ == "__main__":
    import sys

    logger.info("=== Real Wallet Corpus Builder ===\n")

    # Collect from all sources
    cultscreener = CultScreenerCollector()
    organx = OrganXCollector()

    logger.info("Collecting from CultScreener...")
    rugs = cultscreener.extract_wallets(conviction_range=(0, 20))  # Low conviction = rugs
    legit = cultscreener.extract_wallets(conviction_range=(80, 100))  # High conviction

    logger.info("Collecting from Organ X...")
    organ_x = organx.extract_wallets_from_captures()

    # Merge
    corpus = merge_and_label_wallets(rugs + legit, organ_x)

    logger.info(f"\n=== Corpus Summary ===")
    logger.info(f"Total wallets: {len(corpus)}")
    logger.info(f"Sybil candidates (CultScreener): {sum(1 for w in corpus if w['is_sybil_candidate'])}")
    logger.info(f"Mixed sources: {sum(1 for w in corpus if len(w['sources']) > 1)}")

    # Save corpus
    output_file = "/tmp/real_wallet_corpus.json"
    with open(output_file, "w") as f:
        json.dump(corpus, f, indent=2)

    logger.info(f"\n✓ Corpus saved to {output_file}")
    logger.info(f"  Ready for wallet_behavior_helius.py collection")
