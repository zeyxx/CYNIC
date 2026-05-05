#!/usr/bin/env python3
"""
Extract Organism Causality — Discover blockchain/consensus topology from data

Traces real causality chains through organism data:
  click → capture → enrichment → verdict → crystal → decision

Outputs:
  - causality_dag.json: click → capture → enrichment → verdict → crystal flow
  - consensus_gaps.json: where Dogs disagree, routing fails, proofs needed
  - blockchain_primitives.json: immutability requirements, proof points, validator sets
  - hypergraph_edges.json: observation → crystal → decision relationships

Usage:
    python3 extract_organism_causality.py --kernel-addr <host:port> --api-key <key>

Philosophy:
    Don't design blockchain topology. Extract what's already flowing.
    CHAOS→MATRIX: Let emergent structure reveal itself from real data.
"""

__version__ = "0.1.0"

import json
import logging
import os
from pathlib import Path
from typing import Dict, List, Any, Set, Tuple
from collections import defaultdict
from datetime import datetime, timedelta
import requests
from dataclasses import dataclass, asdict

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("organism-causality")


@dataclass
class CausalityChain:
    """Single click→verdict→crystal chain."""
    click_id: str
    click_ts: str
    click_domain: str
    capture_id: str
    capture_ts: str
    enriched: bool
    signal_score: float
    verdict_id: str
    verdict_ts: str
    verdict_dogs: List[str]
    verdict_scores: Dict[str, float]
    verdict_consensus: float
    crystal_id: str
    crystal_ts: str
    crystal_quality: float
    chain_valid: bool
    missing_steps: List[str]


class OrganismCausality:
    """Extract causality chains from behavior_log, killchain, verdicts, crystals."""

    def __init__(self, kernel_addr: str, api_key: str, data_root: str):
        self.kernel_addr = kernel_addr if kernel_addr.startswith("http") else f"http://{kernel_addr}"
        self.api_key = api_key
        self.data_root = Path(data_root)

        self.behavior_log = []
        self.killchain = []
        self.verdicts = []
        self.crystals = []

        self.chains = []
        self.consensus_gaps = []
        self.immutability_points = []
        self.hypergraph_edges = []

    def load_behavior_log(self) -> List[Dict[str, Any]]:
        """Load click events from behavior_log.jsonl."""
        log_path = self.data_root / "behavior" / "behavior_log.jsonl"
        clicks = []

        if not log_path.exists():
            logger.warning("behavior_log.jsonl not found at %s", log_path)
            return clicks

        try:
            with open(log_path) as f:
                for i, line in enumerate(f):
                    if i % 5000 == 0:
                        logger.info("  Loaded %d behavior events", i)

                    try:
                        event = json.loads(line)
                        # Filter to clicks only (not mouse_move, scroll, etc)
                        if event.get("type") == "click":
                            clicks.append(event)
                    except json.JSONDecodeError:
                        continue

            logger.info("✓ Loaded %d clicks from behavior_log", len(clicks))
        except Exception as e:
            logger.error("Failed to load behavior_log: %s", str(e)[:100])

        return clicks

    def load_killchain(self) -> List[Dict[str, Any]]:
        """Load enriched tweets from killchain.jsonl."""
        kc_path = self.data_root / "x" / "killchain.jsonl"
        tweets = []

        if not kc_path.exists():
            logger.warning("killchain.jsonl not found at %s", kc_path)
            return tweets

        try:
            with open(kc_path) as f:
                for i, line in enumerate(f):
                    if i % 5000 == 0:
                        logger.info("  Loaded %d killchain entries", i)

                    try:
                        tweet = json.loads(line)
                        tweets.append(tweet)
                    except json.JSONDecodeError:
                        continue

            logger.info("✓ Loaded %d enriched tweets from killchain", len(tweets))
        except Exception as e:
            logger.error("Failed to load killchain: %s", str(e)[:100])

        return tweets

    def load_verdicts(self) -> List[Dict[str, Any]]:
        """Load verdicts from local storage and kernel."""
        verdicts = []

        # Load from local verdicts directory
        verdicts_dir = self.data_root / "x" / "verdicts"
        if verdicts_dir.exists():
            for file in verdicts_dir.glob("*.json"):
                try:
                    with open(file) as f:
                        data = json.load(f)
                        if isinstance(data, list):
                            verdicts.extend(data)
                        else:
                            verdicts.append(data)
                except Exception as e:
                    logger.debug("Failed to load %s: %s", file.name, str(e)[:50])

        # Try kernel endpoint
        try:
            response = requests.get(
                f"{self.kernel_addr}/verdicts",
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=10,
                params={"limit": 1000}
            )
            if response.status_code == 200:
                kernel_verdicts = response.json()
                if isinstance(kernel_verdicts, list):
                    verdicts.extend(kernel_verdicts)
                logger.info("  Loaded %d verdicts from kernel", len(kernel_verdicts))
        except Exception as e:
            logger.debug("Kernel verdicts fetch failed: %s", str(e)[:50])

        logger.info("✓ Loaded %d verdicts total", len(verdicts))
        return verdicts

    def load_crystals(self) -> List[Dict[str, Any]]:
        """Load crystals from kernel."""
        crystals = []

        try:
            response = requests.get(
                f"{self.kernel_addr}/crystals",
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=10,
                params={"limit": 1000}
            )
            if response.status_code == 200:
                batch = response.json()
                if isinstance(batch, list):
                    crystals.extend(batch)
                    logger.info("✓ Loaded %d crystals from kernel", len(crystals))
        except Exception as e:
            logger.warning("Failed to load crystals: %s", str(e)[:100])

        return crystals

    def match_click_to_capture(self, click: Dict, tweets: List[Dict]) -> Tuple[Dict, bool]:
        """Match click event to enriched tweet capture by timestamp proximity."""
        click_ts = datetime.fromisoformat(click.get("ts", "").replace("Z", "+00:00"))
        click_domain = click.get("domain", "unknown")

        # Find captures within 5 seconds of click
        best_match = None
        best_distance = timedelta(seconds=5)

        for tweet in tweets:
            try:
                capture_ts = datetime.fromisoformat(tweet.get("captured_at", "").replace("Z", "+00:00"))
                distance = abs((capture_ts - click_ts).total_seconds())

                if distance < best_distance.total_seconds():
                    best_distance = timedelta(seconds=distance)
                    best_match = tweet
            except (ValueError, TypeError):
                continue

        return (best_match or {}, best_match is not None)

    def extract_chains(self) -> List[CausalityChain]:
        """Reconstruct causality chains: click → capture → verdict → crystal."""
        chains = []

        logger.info("\n=== Reconstructing Causality Chains ===")

        # Match clicks to captures
        click_to_capture = {}
        for i, click in enumerate(self.behavior_log[:5000]):  # Sample 5K clicks
            if i % 500 == 0:
                logger.info("  Matching click %d / %d", i, min(5000, len(self.behavior_log)))

            capture, matched = self.match_click_to_capture(click, self.killchain)
            if matched:
                click_to_capture[i] = {
                    "click": click,
                    "capture": capture,
                    "matched": True
                }

        logger.info("✓ Matched %d clicks to captures", len(click_to_capture))

        # Match captures to verdicts by tweet_id
        for click_idx, chain_data in click_to_capture.items():
            capture = chain_data["capture"]
            tweet_id = capture.get("id") or capture.get("tweet_id")

            matching_verdicts = [v for v in self.verdicts
                                if v.get("target") == str(tweet_id) or v.get("tweet_id") == str(tweet_id)]

            if matching_verdicts:
                for verdict in matching_verdicts:
                    # Extract dog scores
                    dog_scores = {}
                    for dog_name in ["deterministic-dog", "qwen-7b", "qwen35-9b", "gemini"]:
                        if dog_name in verdict:
                            dog_scores[dog_name] = verdict[dog_name].get("score", 0) if isinstance(verdict[dog_name], dict) else verdict[dog_name]

                    consensus = sum(dog_scores.values()) / len(dog_scores) if dog_scores else 0

                    # Try to find matching crystal
                    crystal = None
                    for c in self.crystals:
                        if c.get("verdict_id") == verdict.get("id") or c.get("source") == str(tweet_id):
                            crystal = c
                            break

                    chain = CausalityChain(
                        click_id=str(click_idx),
                        click_ts=chain_data["click"].get("ts", ""),
                        click_domain=chain_data["click"].get("domain", "unknown"),
                        capture_id=capture.get("id", ""),
                        capture_ts=capture.get("captured_at", ""),
                        enriched=bool(capture.get("signal_score")),
                        signal_score=capture.get("signal_score", 0.0),
                        verdict_id=verdict.get("id", ""),
                        verdict_ts=verdict.get("created_at", ""),
                        verdict_dogs=list(dog_scores.keys()),
                        verdict_scores=dog_scores,
                        verdict_consensus=consensus,
                        crystal_id=crystal.get("id", "") if crystal else "",
                        crystal_ts=crystal.get("created_at", "") if crystal else "",
                        crystal_quality=crystal.get("quality_score", 0.0) if crystal else 0.0,
                        chain_valid=crystal is not None,
                        missing_steps=[] if crystal else ["crystal"]
                    )
                    chains.append(chain)

        logger.info("✓ Extracted %d complete causality chains", len(chains))
        return chains

    def analyze_consensus_gaps(self) -> List[Dict[str, Any]]:
        """Identify where Dogs disagree, routing fails, proofs needed."""
        gaps = []

        logger.info("\n=== Analyzing Consensus Gaps ===")

        # Count verdict disagreements
        disagreements = 0
        for chain in self.chains:
            scores = list(chain.verdict_scores.values())
            if len(scores) > 1:
                score_range = max(scores) - min(scores)
                if score_range > 0.3:  # Significant disagreement
                    disagreements += 1
                    gaps.append({
                        "type": "dog_disagreement",
                        "verdict_id": chain.verdict_id,
                        "dogs": chain.verdict_dogs,
                        "scores": chain.verdict_scores,
                        "range": score_range,
                        "consensus": chain.verdict_consensus
                    })

        # Count routing failures (verdict without crystal)
        routing_failures = sum(1 for c in self.chains if not c.chain_valid)
        logger.info("  Found %d dog disagreements (range > 0.3)", disagreements)
        logger.info("  Found %d routing failures (verdict without crystal)", routing_failures)

        for chain in self.chains:
            if not chain.chain_valid:
                gaps.append({
                    "type": "routing_failure",
                    "verdict_id": chain.verdict_id,
                    "capture_id": chain.capture_id,
                    "missing": chain.missing_steps,
                    "consensus": chain.verdict_consensus
                })

        logger.info("✓ Identified %d consensus gaps", len(gaps))
        return gaps

    def extract_blockchain_primitives(self) -> Dict[str, Any]:
        """Identify immutability requirements and proof points."""
        primitives = {
            "immutability_points": [],
            "proof_points": [],
            "validator_sets": [],
            "causality_constraints": []
        }

        logger.info("\n=== Extracting Blockchain Primitives ===")

        # Immutability points: where forging breaks causality
        immutability = [
            {
                "point": "verdict_hash",
                "breaks_if_forged": "crystal derivation becomes invalid",
                "requires": ["verdict_content_hash", "dog_signatures", "timestamp"]
            },
            {
                "point": "crystal_hash",
                "breaks_if_forged": "decision routing becomes invalid",
                "requires": ["verdict_reference", "quality_proof", "chain_of_custody"]
            },
            {
                "point": "click_origin",
                "breaks_if_forged": "causality chain unmaps",
                "requires": ["timestamp_monotonicity", "source_signature", "observer_witness"]
            }
        ]
        primitives["immutability_points"] = immutability

        # Proof points: where Dogs agree becomes consensus
        proof_points = []
        for chain in self.chains:
            if chain.verdict_consensus > 0.618:  # φ⁻¹ threshold
                proof_points.append({
                    "verdict_id": chain.verdict_id,
                    "dogs": chain.verdict_dogs,
                    "consensus": round(chain.verdict_consensus, 3),
                    "proof_type": "multi_dog_agreement",
                    "weight": len(chain.verdict_dogs)
                })

        primitives["proof_points"] = proof_points
        logger.info("  Identified %d proof points (consensus > φ⁻¹)", len(proof_points))

        # Validator sets: natural groupings of Dogs
        dog_agreement = defaultdict(int)
        for chain in self.chains:
            if len(chain.verdict_dogs) >= 2:
                pairs = [(chain.verdict_dogs[i], chain.verdict_dogs[j])
                        for i in range(len(chain.verdict_dogs))
                        for j in range(i+1, len(chain.verdict_dogs))]
                for pair in pairs:
                    dog_agreement[pair] += 1

        validators = []
        for (dog_a, dog_b), count in sorted(dog_agreement.items(), key=lambda x: -x[1])[:10]:
            agreement_rate = count / len(self.chains) if self.chains else 0
            validators.append({
                "dogs": [dog_a, dog_b],
                "agreement_count": count,
                "agreement_rate": round(agreement_rate, 3),
                "validator_type": "pairwise"
            })

        primitives["validator_sets"] = validators
        logger.info("  Found %d pairwise validator agreements", len(validators))

        # Causality constraints: what must be true
        constraints = [
            "click_ts < capture_ts < verdict_ts < crystal_ts",
            "verdict_dogs >= 1 (single witness)",
            "crystal_quality >= 0 (always exists)",
            "consensus = mean(dog_scores)",
            "chain_valid ⟺ crystal exists for verdict"
        ]
        primitives["causality_constraints"] = constraints

        logger.info("✓ Extracted blockchain primitives")
        return primitives

    def extract_hypergraph_edges(self) -> List[Dict[str, Any]]:
        """Extract observation → crystal → decision relationships."""
        edges = []

        logger.info("\n=== Extracting Hypergraph Edges ===")

        for chain in self.chains:
            if chain.chain_valid:
                # Edge: click → capture → verdict → crystal
                edge = {
                    "type": "complete_chain",
                    "vertices": {
                        "click": chain.click_id,
                        "capture": chain.capture_id,
                        "verdict": chain.verdict_id,
                        "crystal": chain.crystal_id
                    },
                    "weights": {
                        "signal": chain.signal_score,
                        "consensus": round(chain.verdict_consensus, 3),
                        "quality": chain.crystal_quality
                    },
                    "metadata": {
                        "duration_seconds": 0,  # TODO: compute from timestamps
                        "dog_count": len(chain.verdict_dogs),
                        "domain": chain.click_domain
                    }
                }
                edges.append(edge)

        # Summary edges: domain → verdict consensus
        domain_consensus = defaultdict(list)
        for chain in self.chains:
            domain_consensus[chain.click_domain].append(chain.verdict_consensus)

        for domain, scores in domain_consensus.items():
            avg_consensus = sum(scores) / len(scores) if scores else 0
            edges.append({
                "type": "domain_aggregate",
                "domain": domain,
                "chain_count": len(scores),
                "avg_consensus": round(avg_consensus, 3),
                "min_consensus": round(min(scores), 3) if scores else 0,
                "max_consensus": round(max(scores), 3) if scores else 0
            })

        logger.info("✓ Extracted %d hypergraph edges", len(edges))
        return edges

    def run(self) -> Dict[str, Any]:
        """Run full causality extraction."""
        logger.info("Organism Causality Extraction v%s", __version__)
        logger.info("Data root: %s", self.data_root)

        # Load all data
        logger.info("\n=== Loading Data ===")
        self.behavior_log = self.load_behavior_log()
        self.killchain = self.load_killchain()
        self.verdicts = self.load_verdicts()
        self.crystals = self.load_crystals()

        if not self.behavior_log or not self.killchain:
            logger.error("Missing critical data sources (behavior_log or killchain)")
            return None

        # Extract chains
        self.chains = self.extract_chains()

        if not self.chains:
            logger.error("No causality chains reconstructed")
            return None

        # Analyze
        self.consensus_gaps = self.analyze_consensus_gaps()
        blockchain_primitives = self.extract_blockchain_primitives()
        self.hypergraph_edges = self.extract_hypergraph_edges()

        # Report
        logger.info("\n=== Causality Extraction Report ===")
        logger.info("Total behavior events: %d", len(self.behavior_log))
        logger.info("Total enriched tweets: %d", len(self.killchain))
        logger.info("Total verdicts: %d", len(self.verdicts))
        logger.info("Total crystals: %d", len(self.crystals))
        logger.info("Complete causality chains: %d", len(self.chains))
        logger.info("Consensus gaps identified: %d", len(self.consensus_gaps))
        logger.info("Hypergraph edges: %d", len(self.hypergraph_edges))

        # Compute chain validity stats
        valid_chains = sum(1 for c in self.chains if c.chain_valid)
        logger.info("\n=== Chain Validity ===")
        logger.info("  Complete (click→capture→verdict→crystal): %d (%.1f%%)",
                   valid_chains, 100 * valid_chains / len(self.chains) if self.chains else 0)
        logger.info("  Incomplete (verdict without crystal): %d", len(self.chains) - valid_chains)

        return {
            "causality_dag": [asdict(c) for c in self.chains],
            "consensus_gaps": self.consensus_gaps,
            "blockchain_primitives": blockchain_primitives,
            "hypergraph_edges": self.hypergraph_edges,
            "statistics": {
                "total_chains": len(self.chains),
                "valid_chains": valid_chains,
                "consensus_gaps": len(self.consensus_gaps),
                "hypergraph_edges": len(self.hypergraph_edges)
            }
        }


async def main():
    import argparse

    parser = argparse.ArgumentParser(description="Extract Organism Causality")
    parser.add_argument("--kernel-addr", type=str, default=os.environ.get("CYNIC_REST_ADDR", "localhost:3030"))
    parser.add_argument("--api-key", type=str, default=os.environ.get("CYNIC_API_KEY", ""))
    parser.add_argument("--data-root", type=str, default=os.path.expanduser("~/.cynic/organs/hermes"))
    args = parser.parse_args()

    if not args.api_key:
        logger.error("CYNIC_API_KEY required (env or --api-key)")
        return 1

    extractor = OrganismCausality(args.kernel_addr, args.api_key, args.data_root)
    result = extractor.run()

    if result:
        # Save outputs
        outputs = [
            ("causality_dag.json", result["causality_dag"]),
            ("consensus_gaps.json", result["consensus_gaps"]),
            ("blockchain_primitives.json", result["blockchain_primitives"]),
            ("hypergraph_edges.json", result["hypergraph_edges"]),
            ("causality_statistics.json", result["statistics"])
        ]

        for filename, data in outputs:
            with open(filename, "w") as f:
                json.dump(data, f, indent=2, default=str)
            logger.info("✓ Saved to %s", filename)

        return 0

    return 1


if __name__ == "__main__":
    import asyncio
    exit(asyncio.run(main()))
