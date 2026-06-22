#!/usr/bin/env python3
"""
Convergence analyzer: measure impact of parameter tuning on quality metrics.
Compares before/after observations to validate recommendations.

Version: 0.1.0
"""

import json
import glob
import logging
from dataclasses import dataclass
from typing import Dict, List, Optional
from datetime import datetime

logger = logging.getLogger(__name__)


@dataclass
class ConvergenceMetric:
    """Single before/after measurement."""
    metric_name: str  # e.g., "mean_latency_ms", "success_rate", "mean_q_score"
    before_value: float
    after_value: float
    delta_percent: float
    direction: str  # "up" or "down"
    target: str  # e.g., "down" for latency, "up" for success rate

    @property
    def improved(self) -> bool:
        """True if change is in the target direction."""
        if self.target == "down":
            return self.delta_percent < 0
        else:
            return self.delta_percent > 0


class ConvergenceAnalyzer:
    """Measure impact of parameter changes."""

    def __init__(self, dog_name: str = "gemma-4-e4b", observations_dir: str = "observations"):
        self.dog_name = dog_name
        self.observations_dir = observations_dir

    def load_run(self, run_id: str) -> Dict:
        """Load observations from a specific run."""
        pattern = f"{self.observations_dir}/{self.dog_name}-{run_id}.json"
        try:
            with open(f"{self.observations_dir}/{self.dog_name}-{run_id}.json") as f:
                return json.load(f)
        except FileNotFoundError:
            logger.warning(f"Run not found: {run_id}")
            return {}

    def get_latest_runs(self, count: int = 2) -> List[str]:
        """Get the N latest run IDs for this dog."""
        import glob
        from pathlib import Path
        pattern = f"{self.observations_dir}/{self.dog_name}-*.json"
        files = sorted(glob.glob(pattern))
        # Extract run_id from filename: gemma-4-e4b-20260427_063127.json → 20260427_063127
        run_ids = [Path(f).stem.replace(f"{self.dog_name}-", "") for f in files]
        return run_ids[-count:]

    def extract_summary(self, run: Dict) -> Dict:
        """Extract summary metrics from a run."""
        summary = run.get("summary", {})
        latency_stats = summary.get("latency_stats", {})
        q_score_stats = summary.get("q_score_stats", {})

        return {
            "mean_latency_ms": latency_stats.get("mean_ms"),
            "median_latency_ms": latency_stats.get("median_ms"),
            "p95_latency_ms": latency_stats.get("p95_ms"),
            "success_rate": summary.get("successful", 0) / summary.get("total_samples", 1),
            "mean_q_score": q_score_stats.get("mean"),
            "consistency": summary.get("consistency", 0),
        }

    def measure_impact(self, before_run_id: str, after_run_id: str) -> List[ConvergenceMetric]:
        """Measure impact of a parameter change."""
        before = self.load_run(before_run_id)
        after = self.load_run(after_run_id)

        if not before or not after:
            logger.warning("Could not load before/after runs")
            return []

        before_metrics = self.extract_summary(before)
        after_metrics = self.extract_summary(after)

        results = []

        # Metrics and their improvement directions
        metrics_config = {
            "mean_latency_ms": "down",
            "median_latency_ms": "down",
            "p95_latency_ms": "down",
            "success_rate": "up",
            "mean_q_score": "up",
            "consistency": "up",
        }

        for metric, direction in metrics_config.items():
            before_val = before_metrics.get(metric)
            after_val = after_metrics.get(metric)

            if before_val is None or after_val is None:
                continue

            # Avoid division by zero
            if before_val == 0:
                delta_pct = 0
            else:
                delta_pct = ((after_val - before_val) / before_val) * 100

            results.append(ConvergenceMetric(
                metric_name=metric,
                before_value=before_val,
                after_value=after_val,
                delta_percent=delta_pct,
                direction="up" if after_val > before_val else "down",
                target=direction,
            ))

        return results

    def report(self, metrics: List[ConvergenceMetric]) -> Dict:
        """Generate structured convergence report."""
        if not metrics:
            return {
                "generated_at": datetime.now().isoformat(),
                "metrics": [],
                "summary": {
                    "improved": 0,
                    "total": 0,
                    "improvement_rate": 0,
                    "target": "improve latency while maintaining quality",
                },
            }

        # Separate into improved and degraded
        improved = [m for m in metrics if m.improved]
        degraded = [m for m in metrics if not m.improved]

        metric_rows = [
            {
                "metric_name": m.metric_name,
                "before_value": m.before_value,
                "after_value": m.after_value,
                "delta_percent": m.delta_percent,
                "direction": m.direction,
                "target": m.target,
                "improved": m.improved,
            }
            for m in metrics
        ]

        return {
            "generated_at": datetime.now().isoformat(),
            "metrics": metric_rows,
            "improved_metrics": [m["metric_name"] for m in metric_rows if m["improved"]],
            "degraded_metrics": [m["metric_name"] for m in metric_rows if not m["improved"]],
            "summary": {
                "improved": len(improved),
                "total": len(metrics),
                "improvement_rate": len(improved) / len(metrics) if metrics else 0,
                "target": "improve latency while maintaining quality",
                "before_run_id": None,
                "after_run_id": None,
            },
            "data_points": [
                {
                    "metric_name": m.metric_name,
                    "value": m.before_value,
                    "kind": "before",
                }
                for m in metrics
            ] + [
                {
                    "metric_name": m.metric_name,
                    "value": m.after_value,
                    "kind": "after",
                }
                for m in metrics
            ],
        }


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    analyzer = ConvergenceAnalyzer()

    # Get latest two runs
    runs = analyzer.get_latest_runs(2)
    if len(runs) < 2:
        logger.warning("Not enough runs to compare. Run benchmarks multiple times.")
        exit(1)

    before_id, after_id = runs

    logger.info(f"Comparing {before_id} (before) vs {after_id} (after)")

    metrics = analyzer.measure_impact(before_id, after_id)

    report = analyzer.report(metrics)
    logger.info(f"\n{json.dumps(report, indent=2)}")

    # Save report
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report["summary"]["before_run_id"] = before_id
    report["summary"]["after_run_id"] = after_id

    report_file = f"observations/convergence-{timestamp}.json"
    with open(report_file, "w") as f:
        json.dump(report, f, indent=2)
    logger.info(f"✅ Report saved to {report_file}")
