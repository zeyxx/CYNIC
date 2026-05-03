#!/usr/bin/env python3
"""
A/B Test Routing — Compare hardcoded vs data-driven weights

7-day measurement window:
  Days 1-3 (2026-05-05 to 2026-05-07): Hardcoded (D1=25%)
  Days 4-7 (2026-05-08 to 2026-05-12): Data-driven (D1=69.6%)

Metrics tracked per period:
  - Signal yield per domain (mean, stdev, count)
  - Engagement per domain (clicks on observations)
  - Observation count
  - Farming efficiency (observations per cycle)

Verdict (2026-05-12):
  If data-driven > hardcoded by φ⁻¹ (0.618): CHAOS→MATRIX works
  If equal: patterns exist but don't improve routing
  If less: hardcoded is better (unexpected)

This is a 7-day temporary harness. Will be deleted after verdict.
"""

__version__ = "0.1.0"
__lifespan__ = "2026-05-05 to 2026-05-12"
__sunset__ = "2026-05-12"  # Delete after verdict

import json
import logging
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Optional
from collections import defaultdict
import statistics

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
logger = logging.getLogger("ab-test-routing")

# Hardcoded weights (Phase A baseline)
HARDCODED_WEIGHTS = {
    "D1": 0.25,
    "D2": 0.15,
    "D3": 0.15,
    "D4": 0.15,
    "D5": 0.15,
    "D6": 0.15,
}

# Data-driven weights (CHAOS→MATRIX result)
DATA_DRIVEN_WEIGHTS = {
    "D1": 0.696,
    "D2": 0.061,
    "D3": 0.061,
    "D4": 0.061,
    "D5": 0.061,
    "D6": 0.061,
}


class ABTestMeasurement:
    """Measure performance of routing weights."""

    def __init__(self):
        self.observations: List[Dict] = []
        self.test_period_start = datetime.fromisoformat("2026-05-05T00:00:00")  # Day 1
        self.test_period_mid = datetime.fromisoformat("2026-05-08T00:00:00")     # Day 4 (switch)
        self.test_period_end = datetime.fromisoformat("2026-05-12T23:59:59")     # Day 7
        # Domain mapping: observation domain strings (twitter, token-analysis, etc) → D1-D6
        self.domain_map = {
            "token-analysis": "D1",
            "token": "D1",
            "security": "D2",
            "general": "D3",
            "social": "D4",
            "llm": "D5",
            "twitter": "D6",  # fallback for unclassified twitter observations
        }

    def load_observations(self) -> int:
        """Load observations from test period."""
        obs_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "observations"
        count = 0

        if not obs_dir.exists():
            logger.warning(f"Observations directory not found: {obs_dir}")
            return 0

        for fpath in obs_dir.glob("*.json"):
            try:
                with open(fpath) as f:
                    obs = json.load(f)
                    if obs.get('signal_score'):
                        self.observations.append(obs)
                        count += 1
            except:
                pass

        logger.info(f"✓ Loaded {count} observations")
        return count

    def parse_timestamp(self, ts_str: str) -> Optional[datetime]:
        """Parse ISO 8601 timestamp."""
        if not ts_str:
            return None
        try:
            dt = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
            if dt.tzinfo:
                dt = dt.replace(tzinfo=None)
            return dt
        except:
            return None

    def _map_domain_to_letter(self, domain: str) -> str:
        """Map observation domain string to D1-D6 label. Default to D1."""
        return self.domain_map.get(domain.lower(), "D1")

    def segment_observations(self) -> Dict[str, List[Dict]]:
        """Segment observations into test periods."""
        segments = {"hardcoded": [], "data_driven": []}

        for obs in self.observations:
            ts_str = obs.get('timestamp')
            ts = self.parse_timestamp(ts_str)

            if not ts:
                continue

            if self.test_period_start <= ts < self.test_period_mid:
                segments["hardcoded"].append(obs)
            elif self.test_period_mid <= ts <= self.test_period_end:
                segments["data_driven"].append(obs)

        logger.info(f"✓ Segmented observations:")
        logger.info(f"  Hardcoded period (Days 1-3): {len(segments['hardcoded'])} observations")
        logger.info(f"  Data-driven period (Days 4-7): {len(segments['data_driven'])} observations")

        return segments

    def analyze_segment(self, segment: List[Dict], period_name: str) -> Dict:
        """Analyze signal yield for a test segment."""
        domain_signals = defaultdict(list)
        domain_engagement = defaultdict(list)

        for obs in segment:
            # Map domain field (twitter, token-analysis, etc) to D1-D6 if available, else default D1.
            # (inferred_domain field added by domain router; observations may not have it yet)
            domain_str = obs.get('inferred_domain') or obs.get('domain', 'D1')
            domain = self._map_domain_to_letter(domain_str) if isinstance(domain_str, str) else 'D1'
            signal = obs.get('signal_score', 0)
            engagement = obs.get('source', 'unknown')  # Source as proxy for engagement

            if signal:
                domain_signals[domain].append(signal)

            if engagement == 'human_interaction':
                domain_engagement[domain].append(1)
            else:
                domain_engagement[domain].append(0)

        # Compute stats
        stats = {
            "period": period_name,
            "observation_count": len(segment),
            "domains": {}
        }

        for domain in ['D1', 'D2', 'D3', 'D4', 'D5', 'D6']:
            signals = domain_signals.get(domain, [])
            engagements = domain_engagement.get(domain, [])

            if signals:
                stats["domains"][domain] = {
                    "mean_signal": statistics.mean(signals),
                    "stdev_signal": statistics.stdev(signals) if len(signals) > 1 else 0,
                    "count": len(signals),
                    "engagement_rate": statistics.mean(engagements) if engagements else 0,
                }
            else:
                stats["domains"][domain] = {
                    "mean_signal": 0,
                    "stdev_signal": 0,
                    "count": 0,
                    "engagement_rate": 0,
                }

        return stats

    def compute_verdict(self, hardcoded_stats: Dict, datadriven_stats: Dict) -> Dict:
        """Compute A/B test verdict."""
        verdict = {
            "timestamp": datetime.now().isoformat(),
            "version": __version__,
            "test_start": "2026-05-05",
            "test_end": "2026-05-12",
            "hypothesis": "Data-driven weights improve signal yield by φ⁻¹ (0.618)",
            "threshold": 0.618,
        }

        # Aggregate domain signal across all domains
        hardcoded_signals = []
        datadriven_signals = []

        for domain in ['D1', 'D2', 'D3', 'D4', 'D5', 'D6']:
            hc_domain = hardcoded_stats["domains"].get(domain, {})
            dd_domain = datadriven_stats["domains"].get(domain, {})

            if hc_domain.get("count", 0) > 0:
                hardcoded_signals.append(hc_domain.get("mean_signal", 0))

            if dd_domain.get("count", 0) > 0:
                datadriven_signals.append(dd_domain.get("mean_signal", 0))

        hardcoded_mean = statistics.mean(hardcoded_signals) if hardcoded_signals else 0
        datadriven_mean = statistics.mean(datadriven_signals) if datadriven_signals else 0

        improvement = (datadriven_mean - hardcoded_mean) / hardcoded_mean if hardcoded_mean > 0 else 0

        verdict["hardcoded_mean_signal"] = hardcoded_mean
        verdict["datadriven_mean_signal"] = datadriven_mean
        verdict["improvement_ratio"] = improvement
        verdict["passes_threshold"] = improvement >= verdict["threshold"]

        if improvement >= verdict["threshold"]:
            verdict["result"] = "PASS: Data-driven routing works"
            verdict["recommendation"] = "Deploy data-driven weights, sunset hardcoded routing"
        elif improvement > 0:
            verdict["result"] = "PARTIAL: Data-driven improves but below threshold"
            verdict["recommendation"] = "Extend measurement 3 more days, tune weights"
        else:
            verdict["result"] = "FAIL: Hardcoded outperforms data-driven"
            verdict["recommendation"] = "Debug CHAOS→MATRIX discovery, revert to hardcoded"

        return verdict

    def write_test_report(self, hardcoded_stats: Dict, datadriven_stats: Dict, verdict: Dict) -> None:
        """Write comprehensive A/B test report."""
        report = {
            "timestamp": datetime.now().isoformat(),
            "version": __version__,
            "test_design": {
                "period_1_dates": "2026-05-05 to 2026-05-07 (Days 1-3)",
                "period_1_weight_set": "hardcoded",
                "period_1_weights": HARDCODED_WEIGHTS,
                "period_2_dates": "2026-05-08 to 2026-05-12 (Days 4-7)",
                "period_2_weight_set": "data_driven",
                "period_2_weights": DATA_DRIVEN_WEIGHTS,
            },
            "hardcoded_period": hardcoded_stats,
            "datadriven_period": datadriven_stats,
            "verdict": verdict,
        }

        report_path = Path.home() / ".cynic" / "organisms" / f"ab_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        report_path.parent.mkdir(parents=True, exist_ok=True)

        with open(report_path, 'w') as f:
            json.dump(report, f, indent=2)

        logger.info(f"✓ Wrote A/B test report to {report_path}")

    def run(self) -> Dict:
        """Execute A/B test measurement."""
        logger.info(f"\n=== A/B Test Routing v{__version__} ===")
        logger.info(f"Lifespan: {self.__lifespan__}")
        logger.info(f"Sunset: {self.__sunset__}\n")

        # Load and segment observations
        self.load_observations()
        segments = self.segment_observations()

        if not segments["hardcoded"] and not segments["data_driven"]:
            logger.error("✗ No observations in test period")
            return {}

        # Analyze each period
        hardcoded_stats = self.analyze_segment(segments["hardcoded"], "hardcoded (Days 1-3)")
        datadriven_stats = self.analyze_segment(segments["data_driven"], "data_driven (Days 4-7)")

        # Compute verdict
        verdict = self.compute_verdict(hardcoded_stats, datadriven_stats)

        # Report
        self.write_test_report(hardcoded_stats, datadriven_stats, verdict)

        # Log verdict
        logger.info(f"\n=== VERDICT ===")
        logger.info(f"Hardcoded mean signal: {verdict['hardcoded_mean_signal']:.2f}")
        logger.info(f"Data-driven mean signal: {verdict['datadriven_mean_signal']:.2f}")
        logger.info(f"Improvement: {verdict['improvement_ratio']:.1%}")
        logger.info(f"Threshold: {verdict['threshold']:.1%}")
        logger.info(f"Result: {verdict['result']}")
        logger.info(f"Recommendation: {verdict['recommendation']}\n")

        return verdict


def main():
    measurement = ABTestMeasurement()
    verdict = measurement.run()
    return 0 if verdict else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
