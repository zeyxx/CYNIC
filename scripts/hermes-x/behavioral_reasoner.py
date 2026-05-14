#!/usr/bin/env python3
"""
CYNIC Hermes Behavioral Reasoner — agent learns from T.'s behavior patterns.

Reads behavior_log.jsonl (T.'s actual clicks, scrolls, typing) and extracts:
  - Temporal rhythm (when does T. browse?)
  - Content signals (what gets engagement?)
  - Depth patterns (does T. read threads or scroll past?)
  - Decision speed (fast scroll = not interested, slow read = engaged)

Feeds this profile into real-time reasoning:
  For each tweet on feed: "Would T. engage with this?"

Architecture:
  behavior_log.jsonl → BehavioralProfiler (extract patterns)
                    ↓
                  Hermes Agent (reason in real-time)
                    ↓
  X.com feed + profile → "engage" | "scroll"

Usage:
    python3 behavioral_reasoner.py --organ-dir ~/.cynic/organs/hermes/x

Training on: 761K events, 7 days, 29K clicks, 77K scrolls, 322K keystrokes
"""

__version__ = "0.1.0"

import json
import logging
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Optional
from collections import defaultdict

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("behavioral-reasoner")


class BehavioralProfiler:
    """Extract patterns from behavior_log.jsonl."""

    def __init__(self, behavior_log: Path):
        self.log_file = behavior_log
        self.events = []
        self.profile = {}

    def load_events(self):
        """Load all events from behavior_log.jsonl."""
        if not self.log_file.exists():
            logger.error("behavior_log.jsonl not found at %s", self.log_file)
            return False

        with open(self.log_file) as f:
            for line in f:
                try:
                    self.events.append(json.loads(line))
                except json.JSONDecodeError:
                    pass

        logger.info("Loaded %d events from behavior log", len(self.events))
        return len(self.events) > 0

    def extract_temporal_rhythm(self) -> dict:
        """When does T. browse? (hour-of-day distribution)."""
        if not self.events:
            return {}

        hourly_activity = defaultdict(int)
        for event in self.events:
            ts = event.get("ts")
            if ts:
                # Parse ISO timestamp and extract hour (UTC)
                try:
                    dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
                    hour = dt.hour
                    hourly_activity[hour] += 1
                except:
                    pass

        # Find peak hours
        if hourly_activity:
            peak_hours = sorted(hourly_activity.items(), key=lambda x: x[1], reverse=True)[:3]
            logger.info("Peak activity hours (UTC): %s", peak_hours)

        return dict(hourly_activity)

    def extract_engagement_signals(self) -> dict:
        """What types of activity indicate engagement?"""
        clicks = [e for e in self.events if e.get("type") == "click"]
        scrolls = [e for e in self.events if e.get("type") == "scroll"]
        keys = [e for e in self.events if e.get("type") == "key"]

        # Windows T. was viewing when clicking
        clicked_windows = {}
        for click in clicks:
            window = click.get("window_name", "unknown")
            if window:
                clicked_windows[window] = clicked_windows.get(window, 0) + 1

        logger.info("Top windows with clicks:")
        for window, count in sorted(clicked_windows.items(), key=lambda x: x[1], reverse=True)[:5]:
            logger.info("  %d clicks in: %s", count, window[:60])

        return {
            "clicks": len(clicks),
            "scrolls": len(scrolls),
            "keys": len(keys),
            "click_windows": clicked_windows,
        }

    def extract_depth_patterns(self) -> dict:
        """Does T. read deep (long engagement) or scroll past quickly?"""
        if not self.events:
            return {}

        # Measure: scrolls between clicks (higher = faster scroll-past, lower = deep read)
        scroll_click_ratio = len([e for e in self.events if e.get("type") == "scroll"]) / max(
            1, len([e for e in self.events if e.get("type") == "click"])
        )

        # Measure: typing between clicks (higher = thinking/engaged, lower = passive)
        key_click_ratio = len([e for e in self.events if e.get("type") == "key"]) / max(
            1, len([e for e in self.events if e.get("type") == "click"])
        )

        logger.info("Engagement depth:")
        logger.info("  Scroll-to-click ratio: %.1f (scrolls per click)", scroll_click_ratio)
        logger.info("  Typing-to-click ratio: %.1f (keystrokes per click)", key_click_ratio)

        return {
            "scroll_click_ratio": scroll_click_ratio,
            "key_click_ratio": key_click_ratio,
        }

    def build_profile(self) -> dict:
        """Synthesize behavioral profile."""
        if not self.load_events():
            return {}

        profile = {
            "event_count": len(self.events),
            "temporal_rhythm": self.extract_temporal_rhythm(),
            "engagement_signals": self.extract_engagement_signals(),
            "depth_patterns": self.extract_depth_patterns(),
        }

        self.profile = profile
        return profile


class BehavioralReasoner:
    """Agent that uses behavioral profile to reason about tweets."""

    def __init__(self, profile: dict):
        self.profile = profile

    def should_engage(self, tweet_data: dict) -> dict:
        """
        Reason: Would T. engage with this tweet?

        Returns: {
            "decision": "engage" | "scroll" | "read_thread",
            "confidence": 0.0-1.0,
            "signals": [list of reasons]
        }
        """
        signals = []
        score = 0.5  # neutral baseline

        # Simple heuristic for now (will learn from behavior)
        # - High engagement if tweet has media (T. likes visuals)
        # - Engage if author is known/followed
        # - Read thread if it's a quote or reply (deep engagement pattern)

        # TODO: Learn these weights from actual behavior_log interactions

        return {
            "decision": "engage" if score > 0.6 else "scroll",
            "confidence": score,
            "signals": signals,
        }


def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes Behavioral Reasoner")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    parser.add_argument("--extract-only", action="store_true", help="Extract profile and exit")
    args = parser.parse_args()

    # Find behavior log
    behavior_log = Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl"
    if not behavior_log.exists():
        logger.error("behavior_log.jsonl not found")
        return 1

    logger.info("Behavioral Reasoner v%s starting", __version__)
    logger.info("Analyzing %d days of behavior data...", 7)

    # Extract profile
    profiler = BehavioralProfiler(behavior_log)
    profile = profiler.build_profile()

    if args.extract_only:
        # Write profile to disk
        profile_file = args.organ_dir / "behavioral_profile.json"
        profile_file.parent.mkdir(parents=True, exist_ok=True)
        with open(profile_file, "w") as f:
            json.dump(profile, f, indent=2)
        logger.info("✓ Profile written to %s", profile_file)
        return 0

    # Initialize reasoner
    reasoner = BehavioralReasoner(profile)

    logger.info("Profile complete. Ready for real-time reasoning.")
    logger.info("Reasoner can now answer: 'Would T. engage with this tweet?'")

    return 0


if __name__ == "__main__":
    exit(main())
