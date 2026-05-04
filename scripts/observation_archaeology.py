#!/usr/bin/env python3
"""
Observation Archaeology — Discover emergent metadata structure

Queries live kernel /observations, extracts metadata patterns,
identifies what fields actually exist, what survives, and what categories emerge.

Usage:
    python3 observation_archaeology.py --kernel-addr <host:port> --api-key <key>

Output: observation_schema_emergent.json
  - field_frequency: which metadata keys appear, how often
  - signal_types: observed categories (verdict, session, behavioral, etc.)
  - metadata_required: which fields appear in >80% of observations
  - metadata_optional: which fields appear in <20% of observations
  - metadata_noise: which fields appear once/twice only
  - categories_discovered: emergent groupings
"""

__version__ = "0.1.0"

import json
import logging
import os
from pathlib import Path
from typing import Dict, List, Any, Set
from collections import defaultdict
import requests

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("observation-archaeology")


class ObservationArchaeology:
    """Discover observation metadata structure."""

    def __init__(self, kernel_addr: str, api_key: str):
        self.kernel_addr = kernel_addr if kernel_addr.startswith("http") else f"http://{kernel_addr}"
        self.api_key = api_key
        self.observations = []
        self.metadata_stats = defaultdict(lambda: {"count": 0, "types": set(), "samples": []})
        self.signal_types = defaultdict(int)
        self.categories = defaultdict(list)

    def fetch_observations(self, limit: int = 1000) -> List[Dict[str, Any]]:
        """Fetch all observations from kernel."""
        observations = []
        page = 0

        try:
            while len(observations) < limit:
                page += 1
                params = {"limit": min(100, limit - len(observations))}

                response = requests.get(
                    f"{self.kernel_addr}/observations",
                    params=params,
                    headers={"Authorization": f"Bearer {self.api_key}"},
                    timeout=10
                )

                if response.status_code != 200:
                    logger.error("Fetch failed (status %d): %s", response.status_code, response.text[:200])
                    break

                # Response is a direct array, not wrapped in object
                batch = response.json()
                if not isinstance(batch, list) or not batch:
                    break

                observations.extend(batch)
                logger.info("Fetched page %d: %d observations (total: %d)", page, len(batch), len(observations))

                if len(batch) < 100 or len(observations) >= limit:
                    break

        except Exception as e:
            logger.error("Fetch error: %s", str(e)[:100])

        logger.info("✓ Fetched %d observations", len(observations))
        return observations[:limit]

    def analyze_metadata(self, observations: List[Dict[str, Any]]) -> None:
        """Analyze metadata patterns across observations."""
        self.observations = observations
        skipped_empty = 0

        for i, obs in enumerate(observations):
            # Extract signal type (inferred from fields)
            signal_type = obs.get("signal_type") or self._infer_signal_type(obs)
            self.signal_types[signal_type] += 1

            # Extract all metadata fields, tracking which are null
            for key, value in obs.items():
                self.metadata_stats[key]["count"] += 1
                self.metadata_stats[key]["types"].add(type(value).__name__)
                # Track non-null samples
                if value is not None and len(self.metadata_stats[key]["samples"]) < 2:
                    self.metadata_stats[key]["samples"].append(value)

            # Categorize by signal type
            self.categories[signal_type].append(obs)

        logger.info("✓ Analyzed %d observations", len(observations))

    def _infer_signal_type(self, obs: Dict[str, Any]) -> str:
        """Infer signal type from observation fields."""
        tool = obs.get("tool", "").lower()
        domain = obs.get("domain", "").lower()

        # Tool-based inference
        if tool == "judge":
            return "verdict"
        if tool in ["write", "edit", "bash"]:
            return "operation"
        if tool == "mcp":
            return "mcp_call"

        # Domain-based inference
        if domain == "mempool":
            return "planning"
        if domain in ["token-analysis", "wallet-judgment", "chess", "trading"]:
            return "judgment"
        if domain == "twitter":
            return "observation"
        if domain == "session":
            return "session"

        return "unknown"

    def generate_report(self) -> Dict[str, Any]:
        """Generate archaeology report."""
        total_obs = len(self.observations)

        # Categorize metadata by frequency
        required_fields = {}  # >80%
        common_fields = {}    # 20-80%
        optional_fields = {}  # <20%
        noise_fields = {}     # 1-3 occurrences

        for key, stats in self.metadata_stats.items():
            freq = stats["count"] / total_obs if total_obs > 0 else 0
            entry = {
                "frequency": round(freq, 3),
                "count": stats["count"],
                "types": list(stats["types"]),
                "samples": stats["samples"][:2]
            }

            if stats["count"] <= 3:
                noise_fields[key] = entry
            elif freq > 0.80:
                required_fields[key] = entry
            elif freq >= 0.20:
                common_fields[key] = entry
            else:
                optional_fields[key] = entry

        report = {
            "metadata": {
                "required_fields": required_fields,
                "common_fields": common_fields,
                "optional_fields": optional_fields,
                "noise_fields": noise_fields,
            },
            "signal_types": dict(self.signal_types),
            "categories": {
                cat: {
                    "count": len(obs_list),
                    "fields": list(set().union(*(o.keys() for o in obs_list)))
                }
                for cat, obs_list in self.categories.items()
            },
            "statistics": {
                "total_observations": total_obs,
                "unique_fields": len(self.metadata_stats),
                "unique_signal_types": len(self.signal_types),
                "unique_categories": len(self.categories),
            }
        }

        return report

    def discover_hypergraph_structure(self) -> Dict[str, Any]:
        """Propose hypergraph edges based on discovered metadata."""
        hypergraph = {
            "vertices": {
                "observation_types": list(self.signal_types.keys()),
                "metadata_dimensions": list(self.metadata_stats.keys()),
            },
            "edges": {
                "signal_type": "What kind of signal (verdict, session, behavioral, metric)",
                "consumer_contract": "Which consumer reads this (if present)",
                "routing_hint": "Which Dogs/subsystems care (if present)",
                "source": "Which producer created it",
                "quality_score": "Metadata completeness / signal confidence",
                "timestamp": "When it was created",
            },
            "k15_contracts": {},
        }

        # Infer K15 contracts from categories
        for signal_type, obs_list in self.categories.items():
            hypergraph["k15_contracts"][signal_type] = {
                "producer": self._infer_producer(obs_list),
                "consumer": None,  # To be discovered
                "required_metadata": self._extract_common_fields(obs_list),
            }

        return hypergraph

    def _infer_producer(self, obs_list: List[Dict]) -> str:
        """Infer producer from observation fields."""
        if not obs_list:
            return "unknown"

        # Count tool types
        tools = defaultdict(int)
        for obs in obs_list:
            tool = obs.get("tool", "unknown")
            tools[tool] += 1

        if tools:
            most_common = max(tools.items(), key=lambda x: x[1])[0]
            return most_common
        return "unknown"

    def _extract_common_fields(self, obs_list: List[Dict]) -> List[str]:
        """Extract fields that appear in >50% of observations in category."""
        if not obs_list:
            return []
        field_counts = defaultdict(int)
        for obs in obs_list:
            for key in obs.keys():
                field_counts[key] += 1
        threshold = len(obs_list) * 0.5
        return [k for k, count in field_counts.items() if count >= threshold]

    def run(self) -> Dict[str, Any]:
        """Run full archaeology."""
        logger.info("Observation Archaeology v%s", __version__)

        observations = self.fetch_observations(limit=1000)
        if not observations:
            logger.error("No observations fetched")
            return None

        self.analyze_metadata(observations)

        report = self.generate_report()
        hypergraph = self.discover_hypergraph_structure()

        logger.info("\n=== Observation Archaeology Report ===")
        logger.info("Total observations: %d", report["statistics"]["total_observations"])
        logger.info("Unique fields: %d", report["statistics"]["unique_fields"])
        logger.info("Signal types discovered: %s", list(report["signal_types"].keys()))

        logger.info("\n=== Required Metadata (>80% frequency) ===")
        for field, stats in report["metadata"]["required_fields"].items():
            logger.info("  %s: %.1f%% (in %d obs)", field, stats["frequency"] * 100, stats["count"])

        logger.info("\n=== Common Metadata (20-80% frequency) ===")
        for field, stats in report["metadata"]["common_fields"].items():
            logger.info("  %s: %.1f%% (in %d obs)", field, stats["frequency"] * 100, stats["count"])

        logger.info("\n=== Optional Metadata (<20% frequency) ===")
        for field, stats in report["metadata"]["optional_fields"].items():
            logger.info("  %s: %.1f%% (in %d obs)", field, stats["frequency"] * 100, stats["count"])

        logger.info("\n=== Discovered Signal Types & Categories ===")
        for signal_type, count in sorted(report["signal_types"].items(), key=lambda x: -x[1]):
            logger.info("  %s: %d observations", signal_type, count)

        logger.info("\n=== Inferred K15 Contracts (Producer → Consumer) ===")
        for signal_type, contract in hypergraph["k15_contracts"].items():
            logger.info("  %s:", signal_type)
            logger.info("    Producer: %s", contract["producer"])
            logger.info("    Required metadata: %s", contract["required_metadata"])
            logger.info("    Consumer: (to be discovered)")

        return {"report": report, "hypergraph": hypergraph}


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="Observation Archaeology")
    parser.add_argument("--kernel-addr", type=str, default=os.environ.get("CYNIC_REST_ADDR", "localhost:3030"))
    parser.add_argument("--api-key", type=str, default=os.environ.get("CYNIC_API_KEY", ""))
    args = parser.parse_args()

    if not args.api_key:
        logger.error("CYNIC_API_KEY required (env or --api-key)")
        return 1

    arch = ObservationArchaeology(args.kernel_addr, args.api_key)
    result = arch.run()

    if result:
        # Save report
        with open("observation_schema_emergent.json", "w") as f:
            json.dump(result, f, indent=2, default=str)
        logger.info("✓ Saved to observation_schema_emergent.json")
        return 0

    return 1


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
