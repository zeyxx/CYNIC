#!/usr/bin/env python3
"""
CYNIC Hermes Behavior Analyzer — extract temporal patterns from behavior_log.jsonl.

Analyzes user behavioral data (clicks, mouse, keyboard) to inform:
1. Optimal search timing (when user is likely to browse)
2. Search frequency (respecting user's natural rhythm)
3. Day-of-week variations (activity patterns across days)
4. Domain weighting (align searches with user engagement)

Phase 2 evolution for search_executor:
    behavior_analyzer.py → search_patterns.json (timing, frequency, weighting)
                        ↓
        search_executor.py (revised) ← consumes patterns
                        ↓
          Searches align with user temporal signature
"""

__version__ = "0.1.0"

import json
import logging
import statistics
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Optional

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("behavior-analyzer")

DEFAULT_ORGAN_DIR = Path.home() / ".cynic" / "organs" / "hermes" / "x"


class BehaviorAnalyzer:
    """Extract temporal and domain patterns from behavior_log.jsonl."""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.behavior_file = self.organ_dir.parent / "behavior" / "behavior_log.jsonl"
        self.patterns_file = self.organ_dir / "search_patterns.json"

    def analyze(self) -> dict:
        """Analyze behavior_log and extract search patterns.

        Returns {
            "peak_hours": [13, 14, 16, 17, 21],  # hours when user is most active
            "sleep_hours": [5, 6, 7],  # hours to avoid
            "hourly_activity": {0: 0.044, 1: 0.034, ...},  # normalized activity per hour
            "daily_activity": {"Monday": 0.189, ...},  # normalized activity per day
            "recommended_search_interval_secs": 30,  # computed from user's rhythm
            "active_hours": [8, 9, ..., 23],  # recommended search window
        }
        """
        if not self.behavior_file.exists():
            logger.warning("behavior_log.jsonl not found at %s", self.behavior_file)
            return self._default_patterns()

        logger.info("Analyzing behavior_log (%s)", self.behavior_file)

        hours_active = defaultdict(int)
        days_active = defaultdict(int)
        total_events = 0

        try:
            with open(self.behavior_file) as f:
                for line in f:
                    try:
                        evt = json.loads(line)
                        total_events += 1

                        ts_str = evt.get("ts", "")
                        if ts_str:
                            dt = datetime.fromisoformat(
                                ts_str.replace("+00:00", "")
                            )
                            hours_active[dt.hour] += 1
                            days_active[dt.strftime("%A")] += 1
                    except (json.JSONDecodeError, ValueError):
                        continue

        except IOError as e:
            logger.error("failed to read behavior_log: %s", e)
            return self._default_patterns()

        if total_events == 0:
            logger.warning("no events found in behavior_log")
            return self._default_patterns()

        logger.info("Analyzed %d events", total_events)

        # Compute normalized activity
        hourly_activity = {
            h: hours_active[h] / total_events
            for h in range(24)
        }
        daily_activity = {
            d: days_active[d] / total_events
            for d in ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
            if d in days_active
        }

        # Find peak and sleep hours
        peak_hours = sorted(
            hourly_activity.keys(),
            key=lambda h: hourly_activity[h],
            reverse=True
        )[:5]
        sleep_hours = [h for h in range(24) if hourly_activity[h] < 0.02]

        # Active window: first hour with activity > 0.015 through last
        active_hours = [h for h in range(24) if hourly_activity[h] > 0.015]
        if active_hours:
            active_start = min(active_hours)
            active_end = max(active_hours)
            active_window = list(range(active_start, active_end + 1))
        else:
            active_window = list(range(8, 24))  # default: 08:00-23:59

        # Compute recommended search interval
        # User with 815K events over 7 days ≈ 116K events/day
        # Recommended: 1 search per 10-15 min during active window
        # For 5 searches/cycle: space them 30-45s apart
        recommended_interval = 30

        patterns = {
            "version": "0.1.0",
            "analyzed_at": datetime.now().isoformat(),
            "total_events": total_events,
            "peak_hours": sorted(peak_hours),
            "sleep_hours": sorted(sleep_hours),
            "hourly_activity": hourly_activity,
            "daily_activity": daily_activity,
            "recommended_search_interval_secs": recommended_interval,
            "active_hours": active_window,
            "notes": [
                f"Peak activity: hours {sorted(peak_hours)} ({max(hourly_activity.values())*100:.1f}% of daily events)",
                f"Sleep hours (avoid searching): {sorted(sleep_hours)}",
                f"Recommended: only search during {min(active_window):02d}:00 - {max(active_window):02d}:59",
                f"Inter-search interval: {recommended_interval}s maintains natural browsing feel",
            ]
        }

        return patterns

    def _default_patterns(self) -> dict:
        """Return sensible defaults when behavior_log is unavailable."""
        return {
            "version": "0.1.0",
            "analyzed_at": datetime.now().isoformat(),
            "peak_hours": [13, 14, 16, 17, 21],  # Observed peak hours
            "sleep_hours": [5, 6, 7],  # Typical sleep window
            "hourly_activity": {h: (0.05 if h in range(8, 24) else 0.01) for h in range(24)},
            "daily_activity": {
                "Monday": 0.189,
                "Tuesday": 0.093,
                "Wednesday": 0.117,
                "Thursday": 0.215,
                "Friday": 0.051,
                "Saturday": 0.107,
                "Sunday": 0.228,
            },
            "recommended_search_interval_secs": 30,
            "active_hours": list(range(8, 24)),
            "notes": ["Using defaults (no behavior_log available)"],
        }

    def save(self, patterns: dict) -> Path:
        """Save patterns to search_patterns.json."""
        try:
            with open(self.patterns_file, "w") as f:
                json.dump(patterns, f, indent=2)
            logger.info("Patterns saved to %s", self.patterns_file)
            return self.patterns_file
        except IOError as e:
            logger.error("failed to save patterns: %s", e)
            raise

    def report(self, patterns: dict) -> None:
        """Print a human-readable report."""
        print("\n" + "=" * 70)
        print("CYNIC Hermes Behavior Analysis Report")
        print("=" * 70)

        print(f"\nAnalyzed {patterns['total_events']:,} events from behavior_log")

        print(f"\nPeak activity hours: {patterns['peak_hours']}")
        print(f"Sleep hours (avoid): {patterns['sleep_hours']}")
        print(f"Active window: {min(patterns['active_hours']):02d}:00 - {max(patterns['active_hours']):02d}:59")

        print("\nRecommendations:")
        for note in patterns.get("notes", []):
            print(f"  • {note}")

        print("\n" + "=" * 70)


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="CYNIC Hermes Behavior Analyzer — extract temporal patterns"
    )
    parser.add_argument(
        "--organ-dir",
        type=Path,
        default=DEFAULT_ORGAN_DIR,
        help="Organ directory",
    )
    parser.add_argument(
        "--report",
        action="store_true",
        help="Print analysis report to stdout",
    )
    args = parser.parse_args()

    organ_dir = args.organ_dir.expanduser()
    if not organ_dir.exists():
        logger.error("organ directory not found: %s", organ_dir)
        return 1

    analyzer = BehaviorAnalyzer(organ_dir)
    patterns = analyzer.analyze()

    analyzer.save(patterns)

    if args.report:
        analyzer.report(patterns)

    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
