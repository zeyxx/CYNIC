#!/usr/bin/env python3
"""
Analyze benchmark observations: compare runs, detect regressions.
Version: 0.1.0
"""

import json
import os
import glob
from datetime import datetime
from typing import Dict, List
import logging

logger = logging.getLogger(__name__)


def load_observation_file(path: str) -> Dict:
    """Load a single observation JSON."""
    with open(path) as f:
        return json.load(f)


def compare_runs(dog_name: str, observations_dir: str = "observations") -> Dict:
    """
    Compare all runs for a dog.

    Returns: {dog_name, runs: [metadata, metrics], trend: "stable|degrading|improving"}
    """
    pattern = f"{observations_dir}/{dog_name}-*.json"
    files = sorted(glob.glob(pattern))

    if not files:
        logger.warning(f"No observations found for {dog_name}")
        return {}

    runs = [load_observation_file(f) for f in files]

    # Compute trend
    if len(runs) >= 2:
        latest_latency = runs[-1]["summary"]["latency_stats"]["mean_ms"]
        previous_latency = runs[-2]["summary"]["latency_stats"]["mean_ms"]

        if latest_latency is None or previous_latency is None:
            trend = "unknown"
        elif latest_latency > previous_latency * 1.1:
            trend = "degrading"
        elif latest_latency < previous_latency * 0.9:
            trend = "improving"
        else:
            trend = "stable"
    else:
        trend = "first_run"

    return {
        "dog_name": dog_name,
        "num_runs": len(runs),
        "runs": [r["summary"] for r in runs],
        "trend": trend,
    }


def generate_report(observations_dir: str = "observations") -> str:
    """Generate markdown report."""
    dogs = set()
    for f in glob.glob(f"{observations_dir}/*.json"):
        dog_name = os.path.basename(f).split("-")[0]
        dogs.add(dog_name)

    report = f"# Benchmark Report\n\nGenerated: {datetime.now().isoformat()}\n\n"

    for dog in sorted(dogs):
        analysis = compare_runs(dog, observations_dir)
        if not analysis:
            continue

        report += f"## {dog}\n\n"
        report += f"**Trend:** {analysis['trend']}\n"
        report += f"**Runs:** {analysis['num_runs']}\n\n"

        if analysis["runs"]:
            latest = analysis["runs"][-1]
            report += f"### Latest Run\n"
            report += f"- Mean latency: {latest['latency_stats']['mean_ms']:.1f}ms\n"
            report += f"- Success rate: {latest['successful']}/{latest['total_samples']}\n"
            report += f"- Consistency: {latest['consistency']:.2%}\n\n"

    return report


if __name__ == "__main__":
    report = generate_report()
    print(report)

    # Save report
    with open("observations/analysis.md", "w") as f:
        f.write(report)
    print("\n✅ Report saved to observations/analysis.md")
