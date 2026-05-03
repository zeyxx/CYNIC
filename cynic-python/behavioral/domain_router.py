#!/usr/bin/env python3
"""
Domain Router — K15 Consumer for organ-x observations.

Closes the feedback loop:
  farming_log (searches by domain)
    ↓
  x_ingest (captures content)
    ↓
  observations (signal_score per observation)
    ↓
  domain_router (you are here) — routes by domain, adjusts farming weights
    ↓
  behavioral_interactions.jsonl (unified learning)
    ↓
  next farming cycle (weighted by observed signal)

Architecture:
  1. Load observations with timestamps and signal scores
  2. Link observations to farming cycles (temporal proximity)
  3. Aggregate signal by domain
  4. Compute domain weights for next cycle
  5. Write adjusted weights to farming_config.json
  6. Log routing decisions to K15 consumer domain
"""

__version__ = "0.1.0"

import json
import logging
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
from collections import defaultdict
import statistics

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("domain-router")

# Domain definitions (match organ-x domains)
DOMAINS = {
    "D1": "Token/Rug Detection",
    "D2": "Inference/LLM Performance",
    "D3": "Sovereignty/Consensus",
    "D4": "Security/Exploits",
    "D5": "Macro/Market Trends",
    "D6": "Epistemology/Truth",
}

# Heuristic keywords for domain inference
DOMAIN_KEYWORDS = {
    "D1": ["token", "rug", "mint", "pump", "dex", "trading", "scammer", "recovery", "bedrock", "sol", "solana", "honeypot", "exploit"],
    "D2": ["llm", "claude", "reasoning", "latency", "fine-tune", "inference", "model", "kv cache", "serving"],
    "D3": ["custody", "validator", "node", "consensus", "p2p", "sovereignty"],
    "D4": ["security", "vulnerability", "audit", "exploit", "hack", "cross-chain"],
    "D5": ["btc", "bitcoin", "leverage", "regulation", "sec", "macro", "market"],
    "D6": ["confidence", "certainty", "calibration", "probability", "truth", "measurement error"],
}


class DomainRouter:
    """Routes observations by domain and learns domain productivity."""

    def __init__(self, obs_dir: Path, farming_log_path: Path, config_output_path: Optional[Path] = None):
        self.obs_dir = obs_dir
        self.farming_log_path = farming_log_path
        self.config_output_path = config_output_path or Path.home() / ".cynic" / "organs" / "hermes" / "x" / "farming_config.json"

        self.observations: List[Dict] = []
        self.farming_cycles: List[Dict] = []
        self.domain_signals: Dict[str, List[float]] = defaultdict(list)
        self.routing_decisions: List[Dict] = []

    def load_observations(self) -> int:
        """Load all observations with timestamps and signal scores."""
        if not self.obs_dir.exists():
            logger.warning(f"Observations directory not found: {self.obs_dir}")
            return 0

        count = 0
        for fpath in self.obs_dir.glob("*.json"):
            try:
                with open(fpath) as f:
                    obs = json.load(f)
                    # Standardize timestamp format
                    if 'timestamp' not in obs:
                        now = datetime.now().isoformat()
                        obs['timestamp'] = obs.get('created_at', now)

                    self.observations.append(obs)
                    count += 1
            except Exception as e:
                logger.debug(f"Failed to load {fpath.name}: {e}")

        logger.info(f"✓ Loaded {count} observations")
        return count

    def load_farming_log(self) -> int:
        """Load farming log to understand search cycles."""
        if not self.farming_log_path.exists():
            logger.warning(f"Farming log not found: {self.farming_log_path}")
            return 0

        count = 0
        try:
            with open(self.farming_log_path) as f:
                for line in f:
                    try:
                        cycle = json.loads(line)
                        self.farming_cycles.append(cycle)
                        count += 1
                    except:
                        pass
        except Exception as e:
            logger.error(f"Failed to load farming log: {e}")

        logger.info(f"✓ Loaded {count} farming cycles")
        return count

    def link_observations_to_cycles(self) -> Dict[str, List[Dict]]:
        """
        Link observations to farming cycles by timestamp proximity.

        Heuristic: If observation timestamp is within 30 minutes of cycle,
        assume it resulted from that cycle's searches. Infer D1-D6 domain
        from observation content.
        """
        cycle_observations: Dict[str, List[Dict]] = defaultdict(list)

        for obs in self.observations:
            # Skip non-production observations (heartbeat, test, etc.)
            if obs.get('domain') in [None, 'organ-health', 'general', 'test']:
                continue

            if 'signal_score' not in obs or obs['signal_score'] is None:
                continue

            obs_ts = self._parse_timestamp(obs.get('timestamp'))
            if not obs_ts:
                continue

            # Infer D1-D6 domain from content
            inferred_domain = self.infer_domain(obs)
            obs['inferred_domain'] = inferred_domain

            # Find nearest farming cycle (30 min window)
            nearest_cycle = None
            min_delta = timedelta(minutes=30)

            for cycle in self.farming_cycles:
                cycle_ts = self._parse_timestamp(cycle.get('timestamp'))
                if not cycle_ts:
                    continue

                delta = abs((obs_ts - cycle_ts).total_seconds())
                if delta < min_delta.total_seconds():
                    min_delta = timedelta(seconds=delta)
                    nearest_cycle = cycle

            if nearest_cycle:
                cycle_key = nearest_cycle.get('timestamp', 'unknown')
                cycle_observations[cycle_key].append(obs)

        logger.info(f"✓ Linked observations to {len(cycle_observations)} farming cycles")
        return cycle_observations

    def infer_domain(self, observation: Dict) -> str:
        """
        Infer D1-D6 domain from observation content.

        Uses heuristic keyword matching on finding, narratives, and other fields.
        Returns the domain with highest keyword match score.
        """
        text_parts = [
            observation.get('finding', ''),
            ' '.join(observation.get('narratives', [])) if isinstance(observation.get('narratives'), list) else '',
        ]

        # Handle details field (could be dict or string)
        details = observation.get('details', {})
        if isinstance(details, dict) and 'keywords' in details:
            text_parts.append(' '.join(details['keywords']))
        elif isinstance(details, str):
            text_parts.append(details)

        text = " ".join(text_parts).lower()

        domain_scores: Dict[str, int] = {d: 0 for d in DOMAINS.keys()}

        for domain, keywords in DOMAIN_KEYWORDS.items():
            for keyword in keywords:
                if keyword in text:
                    domain_scores[domain] += 1

        # Return highest scoring domain, or D1 as default
        best_domain = max(domain_scores.items(), key=lambda x: x[1])
        return best_domain[0] if best_domain[1] > 0 else "D1"

    def _parse_timestamp(self, ts_str: str) -> Optional[datetime]:
        """Parse ISO 8601 timestamp to naive UTC."""
        if not ts_str:
            return None
        try:
            dt = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
            # Convert to naive UTC for comparison
            if dt.tzinfo:
                dt = dt.replace(tzinfo=None)
            return dt
        except:
            return None

    def compute_domain_signals(self, cycle_observations: Dict[str, List[Dict]]) -> Dict[str, Dict[str, float]]:
        """
        Aggregate signal scores by domain across all observations.

        Uses inferred domain (D1-D6) from observation content.
        Returns: {domain: {mean_signal, count, observations}}
        """
        domain_stats: Dict[str, List[float]] = defaultdict(list)

        for cycle_ts, obs_list in cycle_observations.items():
            for obs in obs_list:
                signal = obs.get('signal_score', 0)
                domain = obs.get('inferred_domain', 'D1')  # Use inferred domain

                if signal is not None and signal > 0:
                    domain_stats[domain].append(signal)

        result = {}
        for domain, scores in domain_stats.items():
            if scores:
                result[domain] = {
                    'mean_signal': statistics.mean(scores),
                    'count': len(scores),
                    'observations': scores,
                    'stdev': statistics.stdev(scores) if len(scores) > 1 else 0.0,
                }

        logger.info(f"✓ Computed signal stats for {len(result)} domains")
        for domain, stats in result.items():
            logger.info(f"  {domain}: mean={stats['mean_signal']:.2f}, count={stats['count']}, stdev={stats['stdev']:.2f}")

        return result

    def compute_domain_weights(self, domain_stats: Dict[str, Dict]) -> Dict[str, float]:
        """
        Compute farming weights for next cycle based on observed signal.

        Strategy:
          - High signal (5-7) → increase weight for that domain
          - Medium signal (3-5) → maintain weight
          - Low signal (1-3) → decrease weight
          - No observations → baseline weight

        Normalization: weights sum to 1.0 (proportional allocation)
        """
        weights: Dict[str, float] = {}
        baseline = 1.0 / len(DOMAINS)  # Equal if no data

        # Start with baseline for all domains
        for domain in DOMAINS.keys():
            if domain in domain_stats:
                # Weight by signal quality
                mean_signal = domain_stats[domain]['mean_signal']
                count = domain_stats[domain]['count']

                # Signal multiplier: 1.0 at signal=4, 1.5 at signal=7, 0.5 at signal=1
                multiplier = (mean_signal - 1) / 6.0 + 0.5  # Linear: [0.5, 1.5]
                multiplier = max(0.3, min(1.5, multiplier))  # Clamp to [0.3, 1.5]

                # Boost by observation count (more data = more confident)
                count_boost = min(1.5, 1.0 + (count - 1) * 0.1)

                weights[domain] = baseline * multiplier * count_boost
            else:
                weights[domain] = baseline * 0.8  # Slight decay for unexplored

        # Normalize to sum to 1.0
        total = sum(weights.values())
        if total > 0:
            weights = {k: v / total for k, v in weights.items()}

        logger.info("✓ Computed domain weights for next cycle:")
        for domain in sorted(weights.keys()):
            pct = weights[domain] * 100
            logger.info(f"  {domain}: {pct:.1f}%")

        return weights

    def write_routing_config(self, domain_weights: Dict[str, float]) -> None:
        """Write domain weights to config file for next farming cycle."""
        config = {
            'timestamp': datetime.now().isoformat(),
            'version': __version__,
            'source': 'domain_router',
            'domain_weights': domain_weights,
            'note': 'Weights computed from observed signal scores (K15 consumer)'
        }

        self.config_output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(self.config_output_path, 'w') as f:
            json.dump(config, f, indent=2)

        logger.info(f"✓ Wrote routing config to {self.config_output_path}")

    def route_observations(self) -> None:
        """
        Route observations to next action.

        For now: log domain and signal for each observation.
        Later: integrate with K15 consumers (e.g., broadcast to human, store learning).
        """
        for obs in self.observations:
            if obs.get('domain') in [None, 'organ-health', 'general', 'test']:
                continue

            signal = obs.get('signal_score', 0)
            if not signal or signal < 1:
                continue

            self.routing_decisions.append({
                'observation': obs.get('finding', ''),
                'domain': obs.get('domain'),
                'signal': signal,
                'timestamp': obs.get('timestamp'),
                'action': 'route_to_human' if signal >= 5 else 'queue_for_learning'
            })

        logger.info(f"✓ Routed {len(self.routing_decisions)} observations")

    def write_k15_observations(self) -> None:
        """Write routing decisions to K15 consumer domain (observability)."""
        k15_log_path = Path.home() / ".cynic" / "organisms" / "k15_consumer.jsonl"
        k15_log_path.parent.mkdir(parents=True, exist_ok=True)

        with open(k15_log_path, 'a') as f:
            for decision in self.routing_decisions:
                f.write(json.dumps({
                    'timestamp': datetime.now().isoformat(),
                    'consumer': 'domain_router',
                    'action': decision['action'],
                    'observation_domain': decision['domain'],
                    'signal_score': decision['signal'],
                }) + '\n')

        logger.info(f"✓ Logged {len(self.routing_decisions)} K15 decisions")

    def run(self) -> Dict[str, float]:
        """Execute full routing pipeline."""
        logger.info(f"\n=== Domain Router (K15 Consumer) v{__version__} ===\n")

        # Load data
        self.load_observations()
        self.load_farming_log()

        if not self.observations or not self.farming_cycles:
            logger.error("✗ Missing observations or farming log")
            return {}

        # Link and analyze
        cycle_observations = self.link_observations_to_cycles()
        domain_stats = self.compute_domain_signals(cycle_observations)

        if not domain_stats:
            logger.warning("✗ No observations linked to farming cycles")
            return {}

        # Compute and write weights
        domain_weights = self.compute_domain_weights(domain_stats)
        self.write_routing_config(domain_weights)

        # Route observations
        self.route_observations()
        self.write_k15_observations()

        logger.info(f"\n✓ Domain routing complete\n")
        return domain_weights


def main():
    from argparse import ArgumentParser

    parser = ArgumentParser(description="Route organ-x observations by domain and compute farming weights")
    parser.add_argument(
        '--obs-dir',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations",
        help="Observations directory"
    )
    parser.add_argument(
        '--farming-log',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "farming_log.jsonl",
        help="Farming log path"
    )
    parser.add_argument(
        '--output',
        type=Path,
        default=Path.home() / ".cynic" / "organs" / "hermes" / "x" / "farming_config.json",
        help="Output config path"
    )
    args = parser.parse_args()

    router = DomainRouter(args.obs_dir, args.farming_log, args.output)
    weights = router.run()

    if weights:
        print("\nDomain weights (next cycle):")
        for domain in sorted(weights.keys()):
            print(f"  {domain}: {weights[domain]:.1%}")


if __name__ == "__main__":
    main()
