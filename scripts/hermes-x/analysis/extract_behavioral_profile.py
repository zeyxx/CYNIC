#!/usr/bin/env python3
"""
Extract behavioral profile from behavior_log.jsonl.

Analyzes typing, mouse, scroll patterns to build a statistical signature
of user interaction style. Output: behavioral_profile.json with metrics
suitable for behavioral mimicry in search_executor.

Usage:
    python3 extract_behavioral_profile.py --input behavior_log.jsonl --output behavioral_profile.json
"""

__version__ = "0.1.0"

import json
import logging
import statistics
from argparse import ArgumentParser
from datetime import datetime, time
from pathlib import Path
from typing import Dict, List, Optional
from collections import defaultdict

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("behavioral-profile")


def parse_events(log_file: Path) -> List[Dict]:
    """Load events from behavior_log.jsonl."""
    events = []
    try:
        with open(log_file) as f:
            for i, line in enumerate(f):
                try:
                    event = json.loads(line)
                    events.append(event)
                except json.JSONDecodeError:
                    if i < 5:
                        logger.warning("skipped malformed line %d", i)
                if i > 0 and (i + 1) % 50000 == 0:
                    logger.info("loaded %d events...", i + 1)
    except IOError as e:
        logger.error("failed to load behavior log: %s", e)
    return events


def extract_typing_metrics(events: List[Dict]) -> Dict:
    """Extract typing fingerprint: WPM, keystroke pauses, timing distribution."""
    keystroke_times = []
    key_events = [e for e in events if e.get("type") == "key"]

    if not key_events:
        return {"keystroke_count": 0, "error": "no keystroke data"}

    logger.info("analyzing %d keystroke events...", len(key_events))

    # Parse timestamps
    for ke in key_events:
        try:
            ts_str = ke.get("ts", "")
            # Handle ISO format with timezone
            if "+" in ts_str:
                ts_str = ts_str.split("+")[0]
            dt = datetime.fromisoformat(ts_str)
            keystroke_times.append(dt.timestamp())
        except (ValueError, AttributeError):
            pass

    if len(keystroke_times) < 2:
        return {"keystroke_count": len(keystroke_times), "error": "insufficient keystroke data"}

    # Calculate inter-keystroke intervals (in milliseconds)
    intervals = []
    for i in range(1, len(keystroke_times)):
        interval_ms = (keystroke_times[i] - keystroke_times[i - 1]) * 1000
        if 0 < interval_ms < 5000:  # Filter: ignore pauses >5s and same-ms
            intervals.append(interval_ms)

    if not intervals:
        return {"keystroke_count": len(keystroke_times), "error": "no valid intervals"}

    # Estimate WPM (avg chars per second, 5 chars = 1 word)
    total_time_sec = (keystroke_times[-1] - keystroke_times[0])
    chars_per_sec = len(keystroke_times) / max(total_time_sec, 1)
    wpm = (chars_per_sec / 5) * 60

    return {
        "keystroke_count": len(keystroke_times),
        "wpm": round(wpm, 1),
        "interval_mean_ms": round(statistics.mean(intervals), 1),
        "interval_stdev_ms": round(statistics.stdev(intervals), 1) if len(intervals) > 1 else 0,
        "interval_median_ms": round(statistics.median(intervals), 1),
        "interval_p10_ms": round(sorted(intervals)[len(intervals) // 10], 1),
        "interval_p90_ms": round(sorted(intervals)[9 * len(intervals) // 10], 1),
    }


def extract_mouse_metrics(events: List[Dict]) -> Dict:
    """Extract mouse patterns: click locations, movement velocity."""
    clicks = []
    moves = []

    for e in events:
        if e.get("type") == "click":
            clicks.append((e.get("x", 0), e.get("y", 0)))
        elif e.get("type") == "mouse_move":
            moves.append((e.get("x", 0), e.get("y", 0), e.get("ts")))

    logger.info("analyzing %d clicks, %d moves...", len(clicks), len(moves))

    click_profile = {
        "click_count": len(clicks),
        "mean_x": round(statistics.mean([c[0] for c in clicks]), 0) if clicks else 0,
        "mean_y": round(statistics.mean([c[1] for c in clicks]), 0) if clicks else 0,
        "stdev_x": round(statistics.stdev([c[0] for c in clicks]), 0) if len(clicks) > 1 else 0,
        "stdev_y": round(statistics.stdev([c[1] for c in clicks]), 0) if len(clicks) > 1 else 0,
    }

    # Movement velocity (distance per unit time)
    velocities = []
    for i in range(1, min(len(moves), 1000)):
        try:
            x1, y1, ts1_str = moves[i - 1]
            x2, y2, ts2_str = moves[i]

            if "+" in ts1_str:
                ts1_str = ts1_str.split("+")[0]
            if "+" in ts2_str:
                ts2_str = ts2_str.split("+")[0]

            t1 = datetime.fromisoformat(ts1_str).timestamp()
            t2 = datetime.fromisoformat(ts2_str).timestamp()

            dt_sec = t2 - t1
            if 0 < dt_sec < 1:  # Only consecutive moves within 1 second
                dist = ((x2 - x1) ** 2 + (y2 - y1) ** 2) ** 0.5
                vel = dist / dt_sec
                if 0 < vel < 10000:  # Reasonable velocities
                    velocities.append(vel)
        except (ValueError, AttributeError):
            pass

    if velocities:
        click_profile.update({
            "velocity_mean": round(statistics.mean(velocities), 1),
            "velocity_median": round(statistics.median(velocities), 1),
        })

    return click_profile


def extract_temporal_metrics(events: List[Dict]) -> Dict:
    """Extract temporal patterns: time of day, inter-event timing."""
    times_of_day = []

    for e in events:
        try:
            ts_str = e.get("ts", "")
            if "+" in ts_str:
                ts_str = ts_str.split("+")[0]
            dt = datetime.fromisoformat(ts_str)
            times_of_day.append(dt.hour)
        except (ValueError, AttributeError):
            pass

    if not times_of_day:
        return {"error": "no timestamp data"}

    logger.info("analyzing temporal patterns (%d events)...", len(times_of_day))

    # Peak hours
    hour_counts = defaultdict(int)
    for h in times_of_day:
        hour_counts[h] += 1

    peak_hours = sorted(hour_counts.items(), key=lambda x: x[1], reverse=True)[:3]

    return {
        "peak_hours": [h for h, _ in peak_hours],
        "hour_distribution": dict(hour_counts),
        "activity_start_hour": min(times_of_day),
        "activity_end_hour": max(times_of_day),
    }


def extract_scroll_metrics(events: List[Dict]) -> Dict:
    """Extract scroll patterns: direction, speed, burst frequency."""
    scrolls = [e for e in events if e.get("type") == "scroll"]

    if not scrolls:
        return {"scroll_count": 0}

    logger.info("analyzing %d scroll events...", len(scrolls))

    scroll_down = 0
    scroll_up = 0
    scroll_distances = []

    for scroll in scrolls:
        dy = scroll.get("dy", 0)
        if dy > 0:
            scroll_down += 1
        elif dy < 0:
            scroll_up += 1
        scroll_distances.append(abs(dy))

    return {
        "scroll_count": len(scrolls),
        "scroll_down_percent": round(100 * scroll_down / len(scrolls), 1) if scrolls else 0,
        "scroll_up_percent": round(100 * scroll_up / len(scrolls), 1) if scrolls else 0,
        "scroll_distance_mean": round(statistics.mean(scroll_distances), 1) if scroll_distances else 0,
        "scroll_distance_median": round(statistics.median(scroll_distances), 1) if scroll_distances else 0,
    }


def main():
    parser = ArgumentParser(description="Extract behavioral profile from behavior_log.jsonl")
    parser.add_argument("--input", type=Path, default=Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl")
    parser.add_argument("--output", type=Path, default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "behavioral_profile.json")
    args = parser.parse_args()

    if not args.input.exists():
        logger.error("behavior log not found: %s", args.input)
        return 1

    logger.info("extracting behavioral profile from %s", args.input)
    logger.info("total size: %.1f MB", args.input.stat().st_size / 1e6)

    # Parse events
    events = parse_events(args.input)
    logger.info("loaded %d events", len(events))

    # Extract metrics
    profile = {
        "version": __version__,
        "timestamp": datetime.utcnow().isoformat(),
        "total_events": len(events),
        "typing": extract_typing_metrics(events),
        "mouse": extract_mouse_metrics(events),
        "temporal": extract_temporal_metrics(events),
        "scroll": extract_scroll_metrics(events),
    }

    # Write output
    args.output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(args.output, "w") as f:
            json.dump(profile, f, indent=2)
        logger.info("✓ behavioral profile written: %s", args.output)
    except IOError as e:
        logger.error("failed to write profile: %s", e)
        return 1

    # Summary
    logger.info("\nBEHAVIORAL PROFILE SUMMARY:")
    logger.info("  Typing: %.1f WPM, %.0f ms inter-keystroke (σ=%.0f)",
                profile["typing"].get("wpm", 0),
                profile["typing"].get("interval_mean_ms", 0),
                profile["typing"].get("interval_stdev_ms", 0))
    logger.info("  Mouse: %d clicks at (%.0f, %.0f), velocity %.1f px/s",
                profile["mouse"].get("click_count", 0),
                profile["mouse"].get("mean_x", 0),
                profile["mouse"].get("mean_y", 0),
                profile["mouse"].get("velocity_mean", 0))
    logger.info("  Temporal: peak hours %s", profile["temporal"].get("peak_hours", []))
    logger.info("  Scroll: %d events, %.1f%% down, %.1f%% up",
                profile["scroll"].get("scroll_count", 0),
                profile["scroll"].get("scroll_down_percent", 0),
                profile["scroll"].get("scroll_up_percent", 0))

    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
