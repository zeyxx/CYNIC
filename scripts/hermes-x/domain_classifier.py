#!/usr/bin/env python3
"""
Domain Classifier — Route enriched tweets to domain-aware Dogs

Reads killchain.jsonl, classifies tweets into 7 clusters (C0-C6),
maps clusters to domain labels, and routes to kernel via /observe.

Falsifiable: If routing preserves >80% of raw signal yield vs baseline,
domain classification is working.

Usage:
    python3 domain_classifier.py --killchain ~/.cynic/organs/hermes/x/killchain.jsonl
                                 --output ~/.cynic/organs/hermes/x/domain_routed.jsonl
"""

__version__ = "0.1.0"

import json
import logging
import re
import os
from pathlib import Path
from typing import Dict, List, Tuple
from dataclasses import dataclass
import requests

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("domain-classifier")

# Cluster → domain label mapping (from emergent_clusters_tfidf.json top_words analysis)
CLUSTER_DOMAINS = {
    0: "solana",        # C0: sol, lan, solana
    1: "general",       # C1: multilingual, general
    2: "tokens",        # C2: coin, meme, sol
    3: "general",       # C3: noise, general
    4: "general",       # C4: Elon, general AI content
    5: "infrastructure", # C5: tps, http, vllm, infrastructure
    6: "general",       # C6: general, mixed
}

# Keyword-based cluster heuristics (for P0)
CLUSTER_KEYWORDS = {
    0: {"sol", "solana", "lan", "blockchain", "network"},
    1: {"le", "est", "les", "français", "bilingual"},
    2: {"coin", "meme", "token", "memecoin", "pump"},
    4: {"elon", "twitter", "ai", "gpt", "reasoning"},
    5: {"tps", "http", "performance", "throughput", "vllm", "llama", "inference"},
}

@dataclass
class ClassifiedTweet:
    tweet_id: str
    cluster: int
    domain: str
    confidence: float
    text: str

class DomainClassifier:
    def __init__(self, killchain_path: Path, output_path: Path, kernel_url: str = None):
        self.killchain_path = Path(killchain_path)
        self.output_path = Path(output_path)
        # Kernel URL from env var (CYNIC_REST_ADDR, e.g., host:port)
        # or via --kernel-url flag. Format: "host:port" (no http://)
        default_addr = os.environ.get("CYNIC_REST_ADDR", "localhost:3030")
        self.kernel_url = kernel_url or default_addr
        self.api_key = os.environ.get("CYNIC_API_KEY", "")
        self.stats = {"total": 0, "routed": 0, "failed": 0}

    def _keyword_score(self, text: str, keywords: set) -> float:
        """Score text against keyword set (0-1)."""
        if not text or not keywords:
            return 0.0
        text_lower = text.lower()
        matches = sum(1 for kw in keywords if kw in text_lower)
        return min(1.0, matches / max(1, len(keywords)))

    def classify_tweet(self, tweet: Dict) -> Tuple[int, float]:
        """Classify a tweet into cluster (0-6) and return confidence (0-1)."""
        text = tweet.get("text", "").lower()

        # Score each cluster
        scores = {}
        for cluster_id, keywords in CLUSTER_KEYWORDS.items():
            scores[cluster_id] = self._keyword_score(text, keywords)

        # Default: C4 (general) gets baseline
        for cluster_id in CLUSTER_DOMAINS.keys():
            if cluster_id not in scores:
                scores[cluster_id] = 0.1  # baseline

        # Highest score wins
        cluster = max(scores.keys(), key=lambda k: scores[k])
        confidence = scores[cluster]

        return cluster, confidence

    def route_to_kernel(self, classified: ClassifiedTweet) -> bool:
        """POST classified tweet to kernel /observe endpoint."""
        if not self.api_key:
            logger.warning("CYNIC_API_KEY not set; skipping kernel routing")
            return False

        try:
            payload = {
                "tool": "domain_classify",
                "domain": classified.domain,
                "content": classified.text[:500],
                "metadata": {
                    "tweet_id": classified.tweet_id,
                    "cluster": classified.cluster,
                    "confidence": round(classified.confidence, 3),
                }
            }

            # Add http:// if not present
            url = self.kernel_url if self.kernel_url.startswith("http") else f"http://{self.kernel_url}"
            response = requests.post(
                f"{url}/observe",
                json=payload,
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=5
            )

            if response.status_code == 200:
                self.stats["routed"] += 1
                return True
            else:
                logger.debug("Kernel routing failed (status %d): %s", response.status_code, response.text[:100])
                self.stats["failed"] += 1
                return False
        except Exception as e:
            logger.debug("Kernel routing error: %s", str(e)[:100])
            self.stats["failed"] += 1
            return False

    def classify_killchain(self) -> List[ClassifiedTweet]:
        """Classify all tweets in killchain.jsonl."""
        classified = []

        if not self.killchain_path.exists():
            logger.error("killchain.jsonl not found: %s", self.killchain_path)
            return classified

        try:
            with open(self.killchain_path) as f:
                for line_no, line in enumerate(f, 1):
                    if (line_no % 1000) == 0:
                        logger.info("  Processing %d...", line_no)

                    try:
                        link = json.loads(line)

                        # Extract top tweet from this click's analysis
                        top_tweets = link.get("top_tweets", [])
                        if not top_tweets:
                            continue

                        for tweet_data in top_tweets:
                            tweet_id = tweet_data.get("tweet_id", "")
                            text = tweet_data.get("text", "")

                            if not tweet_id or not text:
                                continue

                            self.stats["total"] += 1

                            # Classify
                            cluster, confidence = self.classify_tweet(tweet_data)
                            domain = CLUSTER_DOMAINS.get(cluster, "general")

                            classified_tweet = ClassifiedTweet(
                                tweet_id=tweet_id,
                                cluster=cluster,
                                domain=domain,
                                confidence=confidence,
                                text=text
                            )
                            classified.append(classified_tweet)

                            # Route to kernel
                            self.route_to_kernel(classified_tweet)

                    except json.JSONDecodeError:
                        pass
                    except Exception as e:
                        logger.debug("Error processing line %d: %s", line_no, str(e)[:80])

        except Exception as e:
            logger.error("Failed to read killchain: %s", e)

        logger.info("✓ Classified %d tweets", len(classified))
        return classified

    def save_results(self, classified: List[ClassifiedTweet]) -> None:
        """Save classified tweets to output file."""
        try:
            self.output_path.parent.mkdir(parents=True, exist_ok=True)
            with open(self.output_path, "w") as f:
                for c in classified:
                    f.write(json.dumps({
                        "tweet_id": c.tweet_id,
                        "cluster": c.cluster,
                        "domain": c.domain,
                        "confidence": c.confidence,
                    }) + "\n")

            logger.info("✓ Saved %d classified tweets to %s", len(classified), self.output_path)
        except Exception as e:
            logger.error("Failed to save results: %s", e)

    def print_stats(self) -> None:
        """Print classification statistics."""
        logger.info("\n=== Domain Classification Results ===")
        logger.info("Total tweets classified: %d", self.stats["total"])
        logger.info("Routed to kernel: %d", self.stats["routed"])
        logger.info("Routing failures: %d", self.stats["failed"])

        if self.stats["total"] > 0:
            routing_rate = self.stats["routed"] / self.stats["total"] * 100
            logger.info("Routing success rate: %.1f%%", routing_rate)


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Domain Classifier")
    parser.add_argument("--killchain", type=Path,
                       default=Path.home() / ".cynic/organs/hermes/x/killchain.jsonl")
    parser.add_argument("--output", type=Path,
                       default=Path.home() / ".cynic/organs/hermes/x/domain_routed.jsonl")
    parser.add_argument("--kernel-url", type=str, default=None)
    args = parser.parse_args()

    logger.info("Domain Classifier v%s", __version__)
    classifier = DomainClassifier(args.killchain, args.output, args.kernel_url)

    # Classify
    classified = classifier.classify_killchain()

    # Save
    classifier.save_results(classified)

    # Stats
    classifier.print_stats()

    return 0 if classifier.stats["total"] > 0 else 1


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
