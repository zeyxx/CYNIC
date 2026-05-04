#!/usr/bin/env python3
"""
CYNIC Hermes Phase 1 — Extract Learning Weights from Behavioral Log

Analyzes 761K behavior events to extract:
  1. Keyword engagement rates (which keywords → clicks vs scrolls)
  2. Temporal rhythm (when does T. engage per domain)
  3. Author preferences (which authors get engagement)
  4. Depth patterns (how long does T. read before clicking)

Input: behavior_log.jsonl (761K events from pynput logger)
Output: learned_weights.json (weights for reason_about_tweet reasoning)

Usage:
    python3 learn_behavioral_weights.py --organ-dir ~/.cynic/organs/hermes/x
"""

__version__ = "0.1.0"

import json
import logging
from datetime import datetime
from pathlib import Path
from typing import Dict, List
from collections import defaultdict
from dataclasses import dataclass

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("learn-weights")


@dataclass
class EngagementSignal:
    """Result of analyzing behavioral patterns."""
    keyword_weights: Dict[str, float]
    temporal_peaks: Dict[int, float]  # hour -> engagement rate
    author_preferences: List[str]
    depth_threshold: float  # seconds before click
    overall_selectivity: float  # click_count / scroll_count


class BehavioralLearner:
    """Extract learning weights from behavior_log.jsonl."""

    def __init__(self, behavior_log: Path):
        self.log_file = behavior_log
        self.events = []
        self.clicks = []
        self.scrolls = []
        self.keystrokes = []

    def load_events(self) -> bool:
        """Load all events from behavior_log.jsonl."""
        if not self.log_file.exists():
            logger.error("behavior_log.jsonl not found at %s", self.log_file)
            return False

        with open(self.log_file) as f:
            for line in f:
                try:
                    event = json.loads(line)
                    self.events.append(event)

                    # Categorize by type
                    event_type = event.get("type")
                    if event_type == "click":
                        self.clicks.append(event)
                    elif event_type == "scroll":
                        self.scrolls.append(event)
                    elif event_type == "key":
                        self.keystrokes.append(event)
                except json.JSONDecodeError:
                    pass

        logger.info("Loaded %d events (clicks=%d, scrolls=%d, keys=%d)",
                    len(self.events), len(self.clicks), len(self.scrolls), len(self.keystrokes))
        return len(self.events) > 0

    def extract_keyword_weights(self, observations_file: Path) -> Dict[str, float]:
        """
        Extract which keywords correlate with engagement.

        Strategy: Load hermes_observations.jsonl (tweets agent decided to engage with),
        extract keywords, measure frequency in engaged vs all tweets.

        For now: scan behavior_log for window context (CYNIC = code, Planning = other, etc)
        and build a keyword profile from that.
        """
        weights = {}

        # Analyze click contexts to infer keyword interests
        cynic_clicks = sum(1 for c in self.clicks if "CYNIC" in c.get("window_name", ""))
        planning_clicks = sum(1 for c in self.clicks if "planning" in c.get("window_name", "").lower())

        # Base weights from activity
        total_clicks = len(self.clicks)
        if total_clicks > 0:
            weights["code"] = cynic_clicks / total_clicks
            weights["architecture"] = cynic_clicks / total_clicks * 0.7
            weights["python"] = cynic_clicks / total_clicks * 0.6
            weights["rust"] = cynic_clicks / total_clicks * 0.5
            weights["api"] = cynic_clicks / total_clicks * 0.4
            weights["algorithm"] = cynic_clicks / total_clicks * 0.35

        logger.info("Extracted keyword weights from %d clicks", total_clicks)
        logger.info("CYNIC context: %d clicks (%.1f%% of total)", cynic_clicks, cynic_clicks/total_clicks*100 if total_clicks else 0)

        return weights

    def extract_temporal_peaks(self) -> Dict[int, float]:
        """Extract which hours show peak engagement."""
        hourly_clicks = defaultdict(int)
        hourly_total = defaultdict(int)

        for click in self.clicks:
            ts = click.get("ts")
            if ts:
                try:
                    dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
                    hour = dt.hour
                    hourly_clicks[hour] += 1
                except:
                    pass

        for event in self.events:
            ts = event.get("ts")
            if ts:
                try:
                    dt = datetime.fromisoformat(ts.replace("Z", "+00:00"))
                    hour = dt.hour
                    hourly_total[hour] += 1
                except:
                    pass

        # Calculate engagement rate per hour
        engagement_rate = {}
        for hour in range(24):
            total = hourly_total.get(hour, 1)
            clicks = hourly_clicks.get(hour, 0)
            engagement_rate[hour] = clicks / total if total > 0 else 0.0

        # Find peak hours
        peak_hours = sorted(engagement_rate.items(), key=lambda x: x[1], reverse=True)[:3]
        logger.info("Peak engagement hours (UTC): %s", peak_hours)

        return engagement_rate

    def extract_author_preferences(self) -> List[str]:
        """
        Extract which authors T. engages with.

        For now: placeholder (would need to correlate clicks with tweet metadata).
        Strategy: When agent runs, store clicked-on tweets, extract author list.
        """
        # This requires the kill-chain correlation (clicks → captured URLs → tweet IDs → author)
        # For now, return empty; will be populated in Phase 2 with real data
        logger.info("Author extraction requires kill-chain correlation (Phase 1.5)")
        return []

    def extract_depth_patterns(self) -> float:
        """
        Measure: how long does T. read before deciding?

        Metric: keystrokes between clicks (high = reading/thinking, low = skimming)
        """
        keys_per_click = len(self.keystrokes) / max(1, len(self.clicks))
        selectivity = len(self.scrolls) / max(1, len(self.clicks))

        logger.info("Depth patterns:")
        logger.info("  Keys per click: %.1f (higher = deeper reader)", keys_per_click)
        logger.info("  Scrolls per click: %.1f (lower = more selective)", selectivity)

        # Return depth threshold: T. is a deep reader (11.1 keys/click observed)
        # Threshold = time spent thinking before engagement
        return keys_per_click

    def learn(self) -> EngagementSignal:
        """Analyze behavior_log and extract learning signals."""
        if not self.load_events():
            return None

        logger.info("Learning from behavioral patterns...")

        signal = EngagementSignal(
            keyword_weights=self.extract_keyword_weights(None),
            temporal_peaks=self.extract_temporal_peaks(),
            author_preferences=self.extract_author_preferences(),
            depth_threshold=self.extract_depth_patterns(),
            overall_selectivity=len(self.clicks) / max(1, len(self.scrolls)),
        )

        logger.info("Learning complete. Signals:")
        logger.info("  Keywords: %d weights extracted", len(signal.keyword_weights))
        logger.info("  Temporal: %.1f engagement cycles/day", sum(signal.temporal_peaks.values()) / 24)
        logger.info("  Authors: %d preferences tracked", len(signal.author_preferences))
        logger.info("  Depth: %.1f keystrokes per decision", signal.depth_threshold)

        return signal


def main():
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Hermes — Learn Behavioral Weights")
    parser.add_argument("--organ-dir", type=Path, default=Path.home() / ".cynic/organs/hermes/x")
    parser.add_argument("--output", type=Path, help="Output file for learned weights (default: behavioral_profile.json)")
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("organ directory not found: %s", organ_dir)
        return 1

    behavior_log = Path.home() / ".cynic/organs/hermes/behavior/behavior_log.jsonl"

    logger.info("Learning Behavioral Weights v%s", __version__)
    logger.info("Analyzing: %s", behavior_log)

    # Learn from behavior
    learner = BehavioralLearner(behavior_log)
    signal = learner.learn()

    if signal:
        # Save learned weights
        output_file = args.output or (organ_dir / "learned_weights.json")
        output_file.parent.mkdir(parents=True, exist_ok=True)

        learned_data = {
            "version": __version__,
            "timestamp": datetime.now().isoformat(),
            "keyword_weights": signal.keyword_weights,
            "temporal_peaks": {str(h): v for h, v in signal.temporal_peaks.items()},
            "author_preferences": signal.author_preferences,
            "depth_threshold": signal.depth_threshold,
            "selectivity": signal.overall_selectivity,
        }

        with open(output_file, "w") as f:
            json.dump(learned_data, f, indent=2)

        logger.info("✓ Weights saved to %s", output_file)
        return 0

    return 1


if __name__ == "__main__":
    exit(main())
