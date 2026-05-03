#!/usr/bin/env python3
"""
Tag Observations by Source — Foundational for CHAOS→MATRIX

Links each observation to its origin:
  - human_interaction: T. clicked content, Hermes captured & analyzed
  - farming_cycle: Hermes autonomous search (D1-D6 weighted sampling)
  - unknown: can't determine source (rare, should be < 5%)

Uses temporal proximity heuristics:
  - Human interaction: ±30 seconds (tight coupling, direct click result)
  - Farming cycle: ±30 minutes (loose coupling, farmer found it)

Outputs: observations/ with added source, source_id, source_confidence fields.
"""

__version__ = "0.1.0"

import json
import logging
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("tag-observations")


def load_clicks() -> List[Dict]:
    """Load human clicks from behavior_log.jsonl."""
    behavior_path = Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl"
    clicks = []

    if not behavior_path.exists():
        logger.warning(f"Behavior log not found: {behavior_path}")
        return clicks

    with open(behavior_path) as f:
        for line in f:
            try:
                event = json.loads(line)
                if event.get('type') == 'click':
                    clicks.append(event)
            except:
                pass

    logger.info(f"✓ Loaded {len(clicks)} clicks")
    return clicks


def load_farming_cycles() -> List[Dict]:
    """Load farming cycles from farming_log.jsonl."""
    log_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "farming_log.jsonl"
    cycles = []

    if not log_path.exists():
        logger.warning(f"Farming log not found: {log_path}")
        return cycles

    with open(log_path) as f:
        for line in f:
            try:
                cycle = json.loads(line)
                cycles.append(cycle)
            except:
                pass

    logger.info(f"✓ Loaded {len(cycles)} farming cycles")
    return cycles


def load_observations() -> List[Tuple[Path, Dict]]:
    """Load all observations from observations/ directory."""
    obs_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations"
    observations = []

    if not obs_dir.exists():
        logger.warning(f"Observations directory not found: {obs_dir}")
        return observations

    for fpath in obs_dir.glob("*.json"):
        try:
            with open(fpath) as f:
                obs = json.load(f)
                observations.append((fpath, obs))
        except Exception as e:
            logger.debug(f"Failed to load {fpath.name}: {e}")

    logger.info(f"✓ Loaded {len(observations)} observations")
    return observations


def parse_timestamp(ts_str: str) -> Optional[datetime]:
    """Parse ISO 8601 timestamp to naive UTC."""
    if not ts_str:
        return None
    try:
        dt = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
        if dt.tzinfo:
            dt = dt.replace(tzinfo=None)
        return dt
    except:
        return None


def find_nearest_click(obs_ts: datetime, clicks: List[Dict], window_sec: int = 30) -> Optional[Tuple[Dict, float]]:
    """Find nearest click within ±window_sec. Returns (click, delta_seconds)."""
    nearest = None
    min_delta = float('inf')

    for click in clicks:
        click_ts_str = click.get('ts') or click.get('timestamp')
        click_ts = parse_timestamp(click_ts_str)
        if not click_ts:
            continue

        delta = abs((obs_ts - click_ts).total_seconds())
        if delta < min_delta and delta <= window_sec:
            min_delta = delta
            nearest = click

    if nearest:
        return nearest, min_delta
    return None


def find_nearest_cycle(obs_ts: datetime, cycles: List[Dict], window_min: int = 30) -> Optional[Tuple[Dict, float]]:
    """Find nearest farming cycle within ±window_min. Returns (cycle, delta_seconds)."""
    nearest = None
    min_delta = float('inf')
    window_sec = window_min * 60

    for cycle in cycles:
        cycle_ts_str = cycle.get('timestamp')
        cycle_ts = parse_timestamp(cycle_ts_str)
        if not cycle_ts:
            continue

        delta = abs((obs_ts - cycle_ts).total_seconds())
        if delta < min_delta and delta <= window_sec:
            min_delta = delta
            nearest = cycle

    if nearest:
        return nearest, min_delta
    return None


def tag_observation(obs: Dict, obs_ts: datetime, clicks: List[Dict], cycles: List[Dict]) -> Dict:
    """Tag observation with source, source_id, and confidence."""
    # Try to link to human click (tighter window, higher confidence if found)
    click_result = find_nearest_click(obs_ts, clicks, window_sec=30)
    if click_result:
        click, delta = click_result
        obs['source'] = 'human_interaction'
        obs['source_id'] = f"click_{hash(str(click)) % 1000000}"  # Simple click ID
        obs['source_confidence'] = 0.7  # Imperfect heuristic
        obs['source_delta_seconds'] = delta
        return obs

    # Otherwise link to farming cycle
    cycle_result = find_nearest_cycle(obs_ts, cycles, window_min=30)
    if cycle_result:
        cycle, delta = cycle_result
        obs['source'] = 'farming_cycle'
        obs['source_id'] = f"cycle_{cycle.get('cycle_index', 'unknown')}"
        obs['source_confidence'] = 0.9  # More reliable (fewer confounds)
        obs['source_delta_seconds'] = delta
        return obs

    # Unknown
    obs['source'] = 'unknown'
    obs['source_id'] = None
    obs['source_confidence'] = 0.0
    obs['source_delta_seconds'] = None
    return obs


def write_tagged_observation(fpath: Path, obs: Dict) -> None:
    """Write tagged observation back to file."""
    with open(fpath, 'w') as f:
        json.dump(obs, f, indent=2)


def run():
    """Execute tagging pipeline."""
    logger.info(f"\n=== Tag Observations by Source v{__version__} ===\n")

    # Load data
    clicks = load_clicks()
    cycles = load_farming_cycles()
    observations = load_observations()

    if not observations:
        logger.error("✗ No observations to tag")
        return

    # Tag each observation
    tagged = 0
    sources = defaultdict(int)

    for fpath, obs in observations:
        obs_ts_str = obs.get('timestamp', obs.get('created_at'))
        obs_ts = parse_timestamp(obs_ts_str)

        if not obs_ts:
            logger.debug(f"Skipping {fpath.name}: no timestamp")
            continue

        # Tag it
        tagged_obs = tag_observation(obs, obs_ts, clicks, cycles)
        sources[tagged_obs['source']] += 1

        # Write back
        write_tagged_observation(fpath, tagged_obs)
        tagged += 1

    # Summary
    logger.info(f"✓ Tagged {tagged} observations")
    for source, count in sorted(sources.items()):
        logger.info(f"  {source}: {count}")

    logger.info(f"\n✓ Observation tagging complete\n")


if __name__ == "__main__":
    run()
