#!/usr/bin/env python3
"""
Behavioral Learning Integrator — Links human engagement to domain signal.

Completes the feedback loop:
  human clicks on observation (behavioral_interactions.jsonl)
    ↓
  observation signal is recorded
    ↓
  domain learned from clicks
    ↓
  next cycle weights high-engagement domains higher

Architecture:
  1. Load behavioral_interactions.jsonl (human clicks)
  2. Link to observations (signal_score)
  3. Calculate engagement per domain
  4. Write learning profile (which domains engage humans)
"""

__version__ = "0.1.0"

import json
import logging
from pathlib import Path
from datetime import datetime
from typing import Dict, List
from collections import defaultdict
import statistics

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("behavioral-learning")


class BehavioralLearningIntegrator:
    """Links human engagement patterns to domain learning."""

    def __init__(self, interactions_path: Path, obs_dir: Path):
        self.interactions_path = interactions_path
        self.obs_dir = obs_dir
        self.interactions: List[Dict] = []
        self.observations: Dict[str, Dict] = {}
        self.domain_engagement: Dict[str, List[float]] = defaultdict(list)

    def load_interactions(self) -> int:
        """Load behavioral interactions."""
        if not self.interactions_path.exists():
            logger.warning(f"Interactions file not found: {self.interactions_path}")
            return 0

        count = 0
        try:
            with open(self.interactions_path) as f:
                for line in f:
                    try:
                        interaction = json.loads(line)
                        self.interactions.append(interaction)
                        count += 1
                    except:
                        pass
        except Exception as e:
            logger.error(f"Failed to load interactions: {e}")

        logger.info(f"✓ Loaded {count} behavioral interactions")
        return count

    def load_observations(self) -> int:
        """Load observations for reference."""
        if not self.obs_dir.exists():
            logger.warning(f"Observations directory not found: {self.obs_dir}")
            return 0

        count = 0
        for fpath in self.obs_dir.glob("*.json"):
            try:
                with open(fpath) as f:
                    obs = json.load(f)
                    # Key by finding for linking
                    key = obs.get('finding', fpath.stem)
                    self.observations[key] = obs
                    count += 1
            except:
                pass

        logger.info(f"✓ Loaded {count} observations")
        return count

    def compute_engagement(self) -> Dict[str, Dict[str, float]]:
        """
        Compute engagement metrics per domain.

        Engagement = human interaction signal score weighted by click proximity.
        """
        for interaction in self.interactions:
            if not interaction.get('observation'):
                continue

            obs = interaction['observation']
            domain = obs.get('inferred_domain', 'D1')
            signal = obs.get('signal_score', 0)
            engagement = interaction.get('human_engagement', 0)

            # Weighted engagement: signal × human_engagement
            # (high signal + human click = strong learning signal)
            weighted_engagement = (signal / 7.0) * engagement  # normalize signal to [0,1]

            self.domain_engagement[domain].append(weighted_engagement)

        # Aggregate to means
        result = {}
        for domain, engagements in self.domain_engagement.items():
            if engagements:
                result[domain] = {
                    'mean_engagement': statistics.mean(engagements),
                    'count': len(engagements),
                    'stdev': statistics.stdev(engagements) if len(engagements) > 1 else 0.0,
                }

        logger.info(f"✓ Computed engagement metrics for {len(result)} domains")
        for domain, metrics in sorted(result.items()):
            logger.info(f"  {domain}: engagement={metrics['mean_engagement']:.2f}, count={metrics['count']}")

        return result

    def write_learning_profile(self, engagement: Dict[str, Dict]) -> None:
        """Write behavioral learning profile for next cycle."""
        profile = {
            'timestamp': datetime.now().isoformat(),
            'version': __version__,
            'source': 'behavioral_learning_integrator',
            'domain_engagement': engagement,
            'note': 'Engagement = (signal_score / 7.0) × human_engagement (click=1, ignore=0)'
        }

        output_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "behavioral_learning_profile.json"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, 'w') as f:
            json.dump(profile, f, indent=2)

        logger.info(f"✓ Wrote learning profile to {output_path}")

    def run(self) -> Dict[str, Dict]:
        """Execute full integration pipeline."""
        logger.info(f"\n=== Behavioral Learning Integrator v{__version__} ===\n")

        # Load data
        int_count = self.load_interactions()
        obs_count = self.load_observations()

        if int_count == 0:
            logger.warning("✗ No interactions to process")
            return {}

        # Compute engagement
        engagement = self.compute_engagement()

        if engagement:
            self.write_learning_profile(engagement)

        logger.info(f"\n✓ Behavioral learning integration complete\n")
        return engagement


def main():
    from argparse import ArgumentParser

    parser = ArgumentParser(description="Integrate behavioral interactions into domain learning")
    parser.add_argument(
        '--interactions',
        type=Path,
        default=Path.home() / ".cynic" / "organisms" / "behavioral_interactions.jsonl",
        help="Behavioral interactions file"
    )
    parser.add_argument(
        '--obs-dir',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations",
        help="Observations directory"
    )
    args = parser.parse_args()

    integrator = BehavioralLearningIntegrator(args.interactions, args.obs_dir)
    engagement = integrator.run()

    return 0 if engagement else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
