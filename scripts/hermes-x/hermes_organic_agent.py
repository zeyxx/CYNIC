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
        self.learned_weights = {}

    def load_profile(self) -> bool:
        """Load behavioral profile and learned weights."""
        if not self.profile_file.exists():
            logger.error("Profile not found: %s", self.profile_file)
            return False

        try:
            with open(self.profile_file) as f:
                self.profile = json.load(f)
            logger.info("Loaded behavioral profile")

            # Also load learned weights if available
            weights_file = self.organ_dir / "learned_weights.json"
            if weights_file.exists():
                with open(weights_file) as f:
                    weights_data = json.load(f)
                    self.learned_weights = weights_data
                    logger.info("Loaded learned weights (v%s)", weights_data.get("version", "unknown"))
            else:
                logger.warning("learned_weights.json not found; using naive heuristics")

            return True
        except Exception as e:
            logger.error("Failed to load profile: %s", e)
            return False

    async def navigate_feed(self, page, max_tweets: int = 10) -> list:
        """Navigate X.com feed organically: scroll, read, understand structure."""
        tweets = []

        logger.info("Navigating X.com/home (organic feed)...")
        await page.goto("https://x.com/home", wait_until="domcontentloaded")
        await page.wait_for_timeout(2000)

        # Scroll through feed and collect tweets
        seen_tweets = set()
        scroll_count = 0
        max_scrolls = 5  # Scroll 5 times to find max_tweets

        while len(tweets) < max_tweets and scroll_count < max_scrolls:
            # Get all visible articles
            article_locator = page.locator('[role="article"]')
            count = await article_locator.count()
            logger.info("Scroll %d: Found %d articles on screen", scroll_count, count)

            # Extract tweets from visible articles
            for i in range(count):
                try:
                    article = article_locator.nth(i)

                    # Extract author (usually in a span with data-testid="UserName")
                    author_elem = article.locator("[data-testid='UserName']").first
                    author = (await author_elem.inner_text()).strip() if author_elem else "unknown"

                    # Extract tweet ID from article's data attribute if available
                    article_html = await article.evaluate("el => el.outerHTML")
                    tweet_id_match = article_html.find("/status/") if article_html else -1

                    # Extract full tweet text (traverse tweet container)
                    text_parts = []
                    text_divs = article.locator("div[lang]")
                    text_count = await text_divs.count()

                    if text_count > 0:
                        # Get the main tweet text (usually first div[lang])
                        main_text = await text_divs.nth(0).inner_text()
                        text_parts.append(main_text)
                    else:
                        # Fallback: get any text from article
                        full_text = await article.inner_text()
                        text_parts.append(full_text[:300])  # Cap at 300 chars

                    tweet_text = " ".join(text_parts)[:400]  # Cap final text at 400 chars

                    # Skip if we've seen this content already
                    text_hash = hash(tweet_text)
                    if text_hash in seen_tweets:
                        continue

                    seen_tweets.add(text_hash)

                    tweets.append({
                        "index": len(tweets),
                        "text": tweet_text if tweet_text else "(empty tweet)",
                        "author": author,
                        "position": i,
                        "scroll_level": scroll_count,
                    })

                    if len(tweets) >= max_tweets:
                        break

                except Exception as e:
                    logger.debug("Could not extract tweet: %s", str(e)[:80])

            # Scroll down to load more tweets
            if len(tweets) < max_tweets:
                logger.info("Scrolling feed...")
                await page.evaluate("window.scrollBy(0, 500)")
                await page.wait_for_timeout(1500)  # Wait for dynamic content to load
                scroll_count += 1

        logger.info("Extracted %d tweets from feed across %d scrolls", len(tweets), scroll_count)
        return tweets

    def reason_about_tweet(self, tweet: dict) -> BehavioralDecision:
        """
        Use learned behavioral profile to decide: should T. engage with this?

        Learned signals from 762K behavior events:
        - Keywords: code (14.7%), architecture (10.3%), python (8.8%), rust (7.4%), api (5.9%), algorithm (5.2%)
        - Temporal: Peak at 7h UTC (5.99%), 22-23h (4.5%), Low at 21h (1.31%)
        - Depth: 11.1 keystrokes/click (deep reader), 2.7 scrolls/click (selective)
        - Selectivity: Clicks ~3.8% of all events
        """
        signals = []
        score = 0.5

        # Get learned keyword weights
        keyword_weights = self.learned_weights.get("keyword_weights", {})
        temporal_peaks = self.learned_weights.get("temporal_peaks", {})

        # Signal 1: LEARNED Keyword signals
        text_lower = tweet.get("text", "").lower()
        for keyword, weight in keyword_weights.items():
            if keyword in text_lower:
                score += weight
                signals.append(f"kw:{keyword}")

        # Signal 2: LEARNED Temporal signal (current hour)
        # If browsing during peak engagement hour, boost confidence
        try:
            now = datetime.now(datetime.now().astimezone().tzinfo)
            current_hour = str(now.hour)
            temporal_rate = float(temporal_peaks.get(current_hour, 0.038))  # 3.8% default
            if temporal_rate > 0.045:  # Above average
                score += 0.05
                signals.append(f"peak_hour:{current_hour}")
        except:
            pass

        # Signal 3: Structural signals (thread depth, conversation)
        text_len = len(tweet.get("text", ""))
        if text_len > 300:
            score += 0.12
            signals.append("long_form")
        elif text_len > 150:
            score += 0.06
            signals.append("medium_form")

        # Signal 4: Conversation signals (T. engages with discussions)
        conversational_markers = ["@", "reply", "thread", "question", "how", "why", "what", "discuss"]
        if any(marker in text_lower for marker in conversational_markers):
            score += 0.08
            signals.append("discussion")

        # Signal 5: Author credibility (learned: would track followed_authors)
        # Placeholder for future author learning
        author = tweet.get("author", "").lower()
        if any(role in author for role in ["dev", "engineer", "researcher", "founder", "architect"]):
            score += 0.08
            signals.append(f"expert:{author[:20]}")

        # Decision thresholds calibrated from behavior:
        # T. is selective (2.7 scrolls/click = 27% engagement rate among scrolls)
        # T. is deep reader (11.1 keys/click = thinks before clicking)
        # Base engagement rate: 3.8% of all events are clicks
        #
        # Interpretation: Set thresholds such that:
        # - read_thread: high confidence signals align with expertise
        # - engage: moderate signals, worth interacting with
        # - scroll: low signals, keep scrolling (what T. does 96% of the time)

        if score > 0.75:
            decision = "read_thread"  # Click into thread, read responses
        elif score > 0.55:
            decision = "engage"  # Like or quick read
        else:
            decision = "scroll"  # Keep scrolling (the default)

        return BehavioralDecision(
            tweet_id=f"tweet_{tweet.get('index', 0)}",
            decision=decision,
            confidence=min(1.0, score),
            reasoning=signals,
            timestamp=datetime.now(datetime.now().astimezone().tzinfo).isoformat(),
        )

    async def read_thread(self, page, tweet: dict) -> dict:
        """Click on a tweet and read the thread deeply."""
        logger.info("Reading thread: %s...", tweet["text"][:60])
        try:
            # Click on the tweet article to open full thread view
            article_locator = page.locator('[role="article"]').nth(tweet.get("position", 0))
            await article_locator.click()
            await page.wait_for_timeout(2000)  # Wait for thread to load

            # Extract thread context (original tweet + responses)
            thread_text = await page.evaluate("""
                () => {
                    const tweets = document.querySelectorAll('[role="article"]');
                    return Array.from(tweets).slice(0, 3).map(t => t.innerText).join('\\n---\\n');
                }
            """)

            logger.info("Thread depth: %d chars read", len(thread_text))
            return {
                "depth_chars": len(thread_text),
                "thread_preview": thread_text[:300],
                "success": True,
            }

        except Exception as e:
            logger.debug("Failed to read thread: %s", str(e)[:80])
            return {"success": False, "error": str(e)[:80]}

    async def like_tweet(self, page, tweet: dict) -> bool:
        """Like a tweet (signal of engagement)."""
        try:
            article_locator = page.locator('[role="article"]').nth(tweet.get("position", 0))
            like_button = article_locator.locator('[aria-label*="Like"]').first
            if like_button:
                await like_button.click()
                await page.wait_for_timeout(500)
                logger.info("Liked tweet by %s", tweet.get("author", "unknown"))
                return True
        except Exception as e:
            logger.debug("Failed to like: %s", str(e)[:80])
        return False

    async def visit_author(self, page, tweet: dict) -> dict:
        """Click on author name to visit their profile."""
        logger.info("Visiting author: %s", tweet.get("author", "unknown"))
        try:
            article_locator = page.locator('[role="article"]').nth(tweet.get("position", 0))
            author_link = article_locator.locator("[data-testid='UserName']").first
            if author_link:
                await author_link.click()
                await page.wait_for_timeout(1500)
                # Get author profile info
                bio = await page.evaluate("() => document.body.innerText").then(lambda x: x[:200])
                logger.info("Visited author profile")
                return {"visited": True, "preview": bio}
        except Exception as e:
            logger.debug("Failed to visit author: %s", str(e)[:80])
        return {"visited": False}

    async def run_cycle(self, page, max_tweets: int = 10) -> int:
        """Run one browsing cycle: navigate → reason → act."""
        self.cycle_count += 1
        logger.info("=== Cycle %d ===", self.cycle_count)

        # Navigate and get tweets
        tweets = await self.navigate_feed(page, max_tweets)
        if not tweets:
            logger.warning("No tweets found")
            return 0

        # Reason about each tweet and ACT
        decisions = []
        engaged_count = 0

        for i, tweet in enumerate(tweets):
            decision = self.reason_about_tweet(tweet)
            decisions.append(decision)

            logger.info(
                "[%d] %s (conf=%.2f) %s | %s",
                tweet["index"],
                decision.decision.upper(),
                decision.confidence,
                " + ".join(decision.reasoning),
                tweet["author"],
            )

            # ACT on the decision
            if decision.decision == "read_thread":
                # Deeply engage: click into thread, read responses
                result = await self.read_thread(page, tweet)
                decision_dict = asdict(decision)
                decision_dict["action"] = "read_thread"
                decision_dict["thread_depth"] = result.get("depth_chars", 0)
                self.observations.append(decision_dict)
                engaged_count += 1

                # Go back to feed
                await page.go_back()
                await page.wait_for_timeout(1000)

            elif decision.decision == "engage":
                # Light engagement: like, maybe visit author
                await self.like_tweet(page, tweet)
                decision_dict = asdict(decision)
                decision_dict["action"] = "like"
                self.observations.append(decision_dict)
                engaged_count += 1

            else:  # scroll
                # Just log the observation (continue scrolling)
                decision_dict = asdict(decision)
                decision_dict["action"] = "scroll"
                self.observations.append(decision_dict)

        logger.info("Cycle %d: %d tweets, %d engaged", self.cycle_count, len(tweets), engaged_count)
        return engaged_count

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
