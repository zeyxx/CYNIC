#!/usr/bin/env python3
"""
CYNIC Hermes Organic Agent — behavioral-driven real-time navigation.

NOT a search loop. NOT mechanical. Real agent that:
  1. Reads T.'s behavioral profile (peak hours, engagement depth, typing patterns)
  2. Navigates X.com organically (scrolls feed, encounters tweets)
  3. Reasons about each tweet (would T. care about this?)
  4. Acts authentically (likes, reads threads, learns)
  5. Stores observations for learning

Falsifiable hypothesis:
  "An agent that learns from T.'s behavior will find higher-signal content
   than one that just searches keywords."

Architecture:
  behavioral_profile.json (T.'s patterns)
         ↓
  Hermes Agent (live in Chrome via CDP)
         ↓
  X.com/home feed (organic, algorithm-driven)
         ↓
  For each tweet: reason(behavioral_profile, tweet_metadata) → engage/scroll
         ↓
  observations.jsonl (signal + context)

Usage:
    python3 hermes_organic_agent.py --organ-dir ~/.cynic/organs/hermes/x
                                    --max-cycles 5
                                    --reasoning-model=qwen

Training: 761K behavior events, 29K clicks, 11.1 keystrokes/click (deep reader)
Temporal: Peak at 21h, 16h, 17h UTC
"""

__version__ = "0.1.0"

import json
import logging
import asyncio
import os
from datetime import datetime
from pathlib import Path
from typing import Optional
from dataclasses import dataclass, asdict

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("hermes-organic")

try:
    from playwright.async_api import async_playwright
    PLAYWRIGHT_AVAILABLE = True
except ImportError:
    PLAYWRIGHT_AVAILABLE = False


@dataclass
class BehavioralDecision:
    """Decision to engage with a tweet based on behavioral reasoning."""

    tweet_id: str
    decision: str  # "engage" | "scroll" | "read_thread" | "follow"
    confidence: float  # 0.0-1.0
    reasoning: list  # why?
    timestamp: str


class HermesOrganicAgent:
    """Agent that navigates X.com guided by T.'s behavior patterns."""

    def __init__(self, organ_dir: Path, profile_file: Path):
        self.organ_dir = Path(organ_dir)
        self.profile_file = profile_file
        self.profile = {}
        self.observations = []
        self.cycle_count = 0

    def load_profile(self) -> bool:
        """Load behavioral profile."""
        if not self.profile_file.exists():
            logger.error("Profile not found: %s", self.profile_file)
            return False

        try:
            with open(self.profile_file) as f:
                self.profile = json.load(f)
            logger.info("Loaded behavioral profile")
            return True
        except Exception as e:
            logger.error("Failed to load profile: %s", e)
            return False

    async def navigate_feed(self, page, max_tweets: int = 10) -> list:
        """Navigate X.com feed organically, return tweets encountered."""
        tweets = []

        logger.info("Navigating X.com/home (organic feed)...")
        await page.goto("https://x.com/home", wait_until="domcontentloaded")
        await page.wait_for_timeout(2000)

        # Get visible tweets
        article_locator = page.locator('[role="article"]')
        count = await article_locator.count()
        logger.info("Found %d tweets on screen", count)

        # Extract tweet metadata
        for i in range(min(count, max_tweets)):
            try:
                article = article_locator.nth(i)
                # Tweet text: usually in a div with text content
                text_element = article.locator("div:has-text('')").first
                text = await text_element.inner_text() if text_element else ""

                # Fallback: get any text from article
                if not text:
                    text = await article.inner_text()

                tweets.append({
                    "index": i,
                    "text": (text[:140] if text else "")[:140],
                    "author": "x.com",  # Would need deeper parsing to extract real author
                    "position": i,
                })
            except Exception as e:
                logger.debug("Could not extract tweet %d: %s", i, str(e)[:80])
                # Still count it, just with no text
                tweets.append({
                    "index": i,
                    "text": "(tweet content unavailable)",
                    "author": "x.com",
                    "position": i,
                })

        logger.info("Extracted %d tweets from feed", len(tweets))
        return tweets

    def reason_about_tweet(self, tweet: dict) -> BehavioralDecision:
        """
        Use behavioral profile to decide: should T. engage with this?

        Simple heuristics for now (will learn from observations):
        - T. types a lot (11.1 keystrokes/click) → deep reader, probably reads threads
        - T. clicks moderately (2.7 scrolls/click) → selective, not all tweets catch attention
        - T. browses during breaks (peak 21h UTC, 16-17h) → not always available
        - T. works on CYNIC heavily (70% of clicks) → probably interested in tech/dev content
        """
        signals = []
        score = 0.5

        # Heuristic 1: Does tweet mention tech/development keywords?
        tech_keywords = ["code", "python", "rust", "api", "algorithm", "data", "inference", "model", "llm"]
        text_lower = tweet.get("text", "").lower()
        if any(kw in text_lower for kw in tech_keywords):
            score += 0.15
            signals.append("tech_keyword_match")

        # Heuristic 2: Is it a thread (longer text suggests it)?
        if len(tweet.get("text", "")) > 200:
            score += 0.1
            signals.append("thread_candidate")

        # Heuristic 3: Is it conversational (replies, quotes)?
        # (Would need to check tweet structure, skipping for now)

        # Decision threshold
        if score > 0.65:
            decision = "read_thread"
        elif score > 0.55:
            decision = "engage"
        else:
            decision = "scroll"

        return BehavioralDecision(
            tweet_id=f"tweet_{tweet.get('index', 0)}",
            decision=decision,
            confidence=min(1.0, score),
            reasoning=signals,
            timestamp=datetime.now(datetime.now().astimezone().tzinfo).isoformat(),
        )

    async def run_cycle(self, page, max_tweets: int = 10) -> int:
        """Run one browsing cycle: navigate → reason → act."""
        self.cycle_count += 1
        logger.info("=== Cycle %d ===", self.cycle_count)

        # Navigate and get tweets
        tweets = await self.navigate_feed(page, max_tweets)
        if not tweets:
            logger.warning("No tweets found")
            return 0

        # Reason about each tweet
        decisions = []
        for tweet in tweets:
            decision = self.reason_about_tweet(tweet)
            decisions.append(decision)
            logger.info(
                "[%d] %s (conf=%.2f) %s",
                tweet["index"],
                decision.decision.upper(),
                decision.confidence,
                " + ".join(decision.reasoning),
            )

        # Store observations
        self.observations.extend([asdict(d) for d in decisions])

        # Act on decisions (for now, just log)
        engage_count = sum(1 for d in decisions if d.decision != "scroll")
        logger.info("Cycle %d: %d tweets, %d engaged", self.cycle_count, len(tweets), engage_count)

        return engage_count

    def save_observations(self) -> None:
        """Write observations to disk."""
        obs_file = self.organ_dir / "hermes_observations.jsonl"
        try:
            with open(obs_file, "a") as f:
                for obs in self.observations:
                    f.write(json.dumps(obs) + "\n")
            logger.info("✓ Saved %d observations", len(self.observations))
        except Exception as e:
            logger.error("Failed to save observations: %s", e)

    async def run(self, max_cycles: int = 5) -> int:
        """Run agent for max_cycles browsing sessions."""
        if not self.load_profile():
            return 1

        if not PLAYWRIGHT_AVAILABLE:
            logger.error("Playwright not available")
            return 1

        try:
            async with async_playwright() as p:
                # Connect to hermes-browser
                state_file = self.organ_dir.parent / "browser-state.json"
                if state_file.exists():
                    try:
                        with open(state_file) as f:
                            state = json.load(f)
                        cdp_url = state.get("cdp_url", "").replace("ws://", "http://")
                        browser = await p.chromium.connect_over_cdp(cdp_url)
                        logger.info("✓ Connected to hermes-browser")
                    except Exception as e:
                        logger.warning("CDP connect failed: %s", str(e)[:80])
                        browser = None
                else:
                    browser = None

                if not browser:
                    logger.info("Launching headless browser")
                    browser = await p.chromium.launch(headless=True)

                # Get context and create page
                contexts = browser.contexts if hasattr(browser, "contexts") else []
                context = contexts[0] if contexts else await browser.new_context()
                page = await context.new_page()

                # Run cycles
                for i in range(max_cycles):
                    engage_count = await self.run_cycle(page, max_tweets=10)
                    if i < max_cycles - 1:
                        logger.info("Waiting 5s before next cycle...")
                        await asyncio.sleep(5)

                await page.close()
                await browser.close()

        except Exception as e:
            logger.error("Run failed: %s", str(e)[:150])
            return 1

        # Save observations
        self.save_observations()
        logger.info("Agent complete. %d total observations.", len(self.observations))
        return 0


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes Organic Agent")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    parser.add_argument("--max-cycles", type=int, default=5, help="Number of browsing cycles")
    parser.add_argument("--reasoning-model", default="qwen", help="Reasoning model (stub for now)")
    args = parser.parse_args()

    profile_file = args.organ_dir / "behavioral_profile.json"

    logger.info("Hermes Organic Agent v%s", __version__)
    agent = HermesOrganicAgent(args.organ_dir, profile_file)

    return await agent.run(max_cycles=args.max_cycles)


if __name__ == "__main__":
    exit(asyncio.run(main()))
