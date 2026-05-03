#!/usr/bin/env python3
"""
Extract existing Hermes data and map to unified behavioral schema.

Reads:
  - behavior_log.jsonl (741K events: mouse_move, key, scroll, click)
  - observations/ (Hermes' processed findings)

Outputs:
  - behavioral_interactions.jsonl (unified format for all agents)

This is the bridge: existing data → new infrastructure.
Not transformation, not cleanup. Just mapping observed structure to unified log.
"""

import json
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

# Import our schema
from unified_schema import (
    HumanInteraction, InteractionType, OrganCapture, OrganObservation,
    ObservationType, Agent, BehavioralInteraction, BehavioralLog
)


def load_behavior_log(max_events: int = 0) -> List[Dict]:
    """Load behavior_log.jsonl (raw events from user interaction)."""
    behavior_path = Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl"
    events = []
    count = 0

    if not behavior_path.exists():
        print(f"✗ Not found: {behavior_path}")
        return events

    with open(behavior_path) as f:
        for line in f:
            if max_events > 0 and count >= max_events:
                break
            try:
                event = json.loads(line)
                events.append(event)
                count += 1
            except:
                pass

    print(f"✓ Loaded {count} behavior events")
    return events


def load_observations() -> Dict[str, List[Dict]]:
    """Load observations from Hermes' analysis."""
    obs_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations"
    observations = defaultdict(list)

    if not obs_dir.exists():
        print(f"✗ Not found: {obs_dir}")
        return observations

    for fname in os.listdir(obs_dir):
        if fname.endswith('.json'):
            fpath = os.path.join(obs_dir, fname)
            try:
                with open(fpath) as f:
                    obs = json.load(f)
                    domain = obs.get('domain', 'unknown')
                    observations[domain].append(obs)
            except:
                pass

    total = sum(len(v) for v in observations.values())
    print(f"✓ Loaded {total} observations across {len(observations)} domains")
    return observations


def extract_clicks(behavior_events: List[Dict]) -> List[Tuple[Dict, HumanInteraction]]:
    """Extract click events (intentional actions) with timestamps."""
    clicks = []

    for event in behavior_events:
        if event.get('type') == 'click':
            try:
                interaction = HumanInteraction(
                    timestamp=event.get('ts'),
                    type=InteractionType.CLICK,
                    x=event.get('x', 0),
                    y=event.get('y', 0),
                    window_id=event.get('window_id', ''),
                    window_name=event.get('window_name', ''),
                    button=event.get('button', 'left')
                )
                clicks.append((event, interaction))
            except:
                pass

    print(f"✓ Extracted {len(clicks)} clicks from behavior log")
    return clicks


def map_observation_to_unified(obs: Dict) -> Optional[OrganObservation]:
    """Map Hermes observation (JSON file) to unified OrganObservation."""
    try:
        timestamp = obs.get('timestamp', datetime.utcnow().isoformat())
        domain = obs.get('domain', 'unknown')
        signal_score = obs.get('signal_score', 0.0)
        finding = obs.get('finding', '')

        # Heuristic: infer observation type from presence of fields
        obs_type = ObservationType.SIGNAL
        if 'narratives' in obs:
            obs_type = ObservationType.NARRATIVE
        if 'prediction_verified' in obs:
            obs_type = ObservationType.DOMAIN_PREDICTION

        # Extract evidence (tweet IDs, references)
        evidence = []
        if 'tweet_ids' in obs:
            evidence.extend([f"tweet:{tid}" for tid in obs['tweet_ids']])

        return OrganObservation(
            timestamp=timestamp,
            agent=Agent.HERMES,
            domain=domain,
            observation_type=obs_type,
            signal_score=signal_score,
            finding=finding,
            evidence=evidence,
            metadata={
                'narratives': obs.get('narratives', []),
                'author_tiers': obs.get('author_tiers', []),
                'details': obs.get('details', {}),
                'type': obs.get('type'),
            }
        )
    except Exception as e:
        print(f"  ✗ Failed to map observation: {e}")
        return None


def extract_unified_interactions(
    clicks: List[Tuple[Dict, HumanInteraction]],
    observations: Dict[str, List[Dict]]
) -> List[BehavioralInteraction]:
    """
    Build unified interactions by linking clicks → observations.

    Heuristic: within 30 seconds of a click, if Hermes produces an observation,
    it's likely from content accessed in that click.
    """
    interactions = []

    for click_event, click_interaction in clicks:
        click_ts = datetime.fromisoformat(click_interaction.timestamp.replace('Z', '+00:00'))

        # Find observations within ±30s of this click
        nearby_obs = []
        for domain_obs_list in observations.values():
            for obs in domain_obs_list:
                obs_ts_str = obs.get('timestamp', '')
                try:
                    obs_ts = datetime.fromisoformat(obs_ts_str.replace('Z', '+00:00'))
                    delta = abs((obs_ts - click_ts).total_seconds())
                    if delta < 30:
                        nearby_obs.append(obs)
                except:
                    pass

        # If we found observations, link them
        if nearby_obs:
            for obs in nearby_obs:
                obs_unified = map_observation_to_unified(obs)
                if obs_unified:
                    interaction = BehavioralInteraction(
                        timestamp=click_interaction.timestamp,
                        agent=Agent.HERMES,
                        human_interaction=click_interaction,
                        observation=obs_unified,
                        domain_predicted=obs_unified.domain,
                        signal_measured=obs_unified.signal_score,
                        human_engagement=1.0,  # User clicked, so engaged
                        outcome=f"Hermes produced {obs_unified.observation_type.value} observation"
                    )
                    interactions.append(interaction)
        else:
            # Click with no nearby observation (organ didn't act)
            interaction = BehavioralInteraction(
                timestamp=click_interaction.timestamp,
                agent=Agent.HERMES,
                human_interaction=click_interaction,
                domain_predicted=None,
                signal_measured=None,
                human_engagement=1.0,  # User clicked
                outcome="No observation from Hermes"
            )
            interactions.append(interaction)

    print(f"✓ Built {len(interactions)} unified interactions")
    return interactions


def main():
    print("\n=== Extract Hermes Data → Unified Schema ===\n")

    # 1. Load existing data
    behavior_events = load_behavior_log(max_events=100000)  # First 100K for speed
    observations = load_observations()

    # 2. Extract semantically rich events (clicks)
    clicks = extract_clicks(behavior_events)

    # 3. Build unified interactions
    interactions = extract_unified_interactions(clicks, observations)

    # 4. Write to unified log
    log = BehavioralLog()
    for interaction in interactions:
        log.append(interaction)

    print(f"\n✓ Wrote {len(interactions)} interactions to {log.log_path}\n")

    # 5. Summary by domain
    domain_counts = defaultdict(int)
    for i in interactions:
        if i.get('domain_predicted'):
            domain_counts[i['domain_predicted']] += 1

    print("=== Unified Interactions by Domain ===")
    for domain in sorted(domain_counts.keys(), key=lambda d: domain_counts[d], reverse=True):
        print(f"  {domain}: {domain_counts[domain]}")

    print(f"\n✓ Infrastructure ready for multi-agent learning\n")


if __name__ == "__main__":
    main()
