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


def generate_report(observations_dir: str = "observations") -> Dict:
    """Generate structured report payload."""
    dogs = set()
    for f in glob.glob(f"{observations_dir}/*.json"):
        dog_name = os.path.basename(f).split("-")[0]
        dogs.add(dog_name)

    dogs_report = []

    for dog in sorted(dogs):
        analysis = compare_runs(dog, observations_dir)
        if not analysis:
            continue

        dog_payload = {
            "dog_name": dog,
            "trend": analysis["trend"],
            "num_runs": analysis["num_runs"],
            "runs": analysis["runs"],
        }
        if analysis["runs"]:
            latest = analysis["runs"][-1]
            dog_payload["latest"] = {
                "mean_latency_ms": latest["latency_stats"]["mean_ms"],
                "success_rate": latest["successful"] / latest["total_samples"] if latest["total_samples"] else 0,
                "consistency": latest["consistency"],
            }
        dogs_report.append(dog_payload)

    return {
        "generated_at": datetime.now().isoformat(),
        "dogs": dogs_report,
    }


if __name__ == "__main__":
    report = generate_report()
    print(json.dumps(report, indent=2))

    # Save report
    report_path = "observations/analysis.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"\n✅ Report saved to {report_path}")
