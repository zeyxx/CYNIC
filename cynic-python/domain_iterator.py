#!/usr/bin/env python3
"""
Domain Iterator: Wisdom Synthesis Loop
Cycle: Identify Gap → Mine Signals → Curate Data → Measure Coverage → Repeat

Protocol (CYNIC Constitution K12):
1. Identify lowest-coverage domain (priority gap)
2. Mine raw signals from hermes-x captures
3. Curator (Claude) structures patterns into JSONL
4. SKILL.md evolves with new patterns
5. Measure coverage improvement
6. Next iteration on new gap
"""

import json
import sys
from pathlib import Path
from datetime import datetime

# Add domains to path
sys.path.insert(0, str(Path(__file__).parent))
from domains import DOMAINS, priority_gap, next_to_explore

class DomainIterator:
    def __init__(self, repo_root="/home/user/Bureau/CYNIC"):
        self.repo_root = Path(repo_root)
        self.dataset_path = self.repo_root / "captures" / "dataset.jsonl"
        self.curation_path = self.repo_root / "cynic-python" / "curation"
        self.curation_path.mkdir(parents=True, exist_ok=True)

    def identify_gap(self):
        """Step 1: Find lowest-coverage domain"""
        domain_id, domain_meta = next_to_explore()
        return domain_id, domain_meta

    def load_raw_dataset(self):
        """Load all captured tweets"""
        if not self.dataset_path.exists():
            print(f"ERROR: {self.dataset_path} not found")
            return []

        tweets = []
        with open(self.dataset_path) as f:
            for line in f:
                if line.strip():
                    tweets.append(json.loads(line))
        return tweets

    def mine_signals(self, domain_id, tweets):
        """Step 2: Mine signals for domain (heuristic + manual)

        This is where hermes-x scripts run:
        - scan_high_signal: identifies candidate tweets
        - process_high_signal: extracts patterns
        - curator reviews: structures as domain-specific JSON
        """
        domain = DOMAINS[domain_id]
        print(f"\n=== Mining Domain {domain_id}: {domain['name']} ===")
        print(f"Coverage gap: {domain['coverage']:.1%}")
        print(f"Expected signals: {domain['signals']}")
        print(f"Consumer: {domain['consumer']}")

        # TODO: Implement domain-specific mining heuristics
        # For now: return candidates + curator instructions
        return {
            "domain_id": domain_id,
            "domain_name": domain["name"],
            "consumer": domain["consumer"],
            "expected_signals": domain["signals"],
            "raw_candidates": [],  # Will be populated by hermes-x
            "curator_task": f"Structure D{domain_id} patterns into curated.jsonl"
        }

    def curate_data(self, domain_id, mined_signals):
        """Step 3: Curator structures patterns (human or Claude)

        Output: Domain curated.jsonl with schema:
        {
            "signal_id": "D1_rug_mechanics_001",
            "domain": "D1",
            "pattern": "Liquidity lock removal + contract mint disabled",
            "strength": 0.85,
            "sources": ["tweet1_id", "tweet2_id"],
            "falsifiable_claim": "If liquidity is locked, rug probability < 0.1"
        }
        """
        curated_file = self.curation_path / f"D{domain_id}_curated.jsonl"
        print(f"\nCurator task: {mined_signals['curator_task']}")
        print(f"Output file: {curated_file}")

        # This is where Claude (curator skill) would structure the data
        # For now: return instructions
        return {
            "output_file": str(curated_file),
            "status": "awaiting_curator",
            "instructions": f"""
Curator (Claude): Review D{domain_id} signals and structure as JSONL.
Each line: signal_id, domain, pattern, strength, sources, falsifiable_claim.

Example (D1):
{{"signal_id": "D1_rug_001", "domain": "D1", "pattern": "Liquidity burned, supply capped, multi-sig governance", "strength": 0.9, "sources": ["tweet_123", "tweet_456"], "falsifiable_claim": "If all 3 present: rug probability < 0.1"}}

Commit curated.jsonl to git when ready.
"""
        }

    def measure_coverage(self, domain_id):
        """Step 4: Measure coverage improvement"""
        curated_file = self.curation_path / f"D{domain_id}_curated.jsonl"

        if not curated_file.exists():
            count = 0
        else:
            count = sum(1 for line in open(curated_file) if line.strip())

        # Update DOMAINS (simulated; real version updates SKILL.md)
        print(f"\nCoverage measurement: D{domain_id} = {count} curated patterns")
        return count

    def iterate(self):
        """Main loop: Gap → Mine → Curate → Measure → Repeat"""
        print("\n" + "="*70)
        print("CYNIC Domain Iterator — Wisdom Synthesis Loop")
        print("="*70)

        # Step 1: Identify gap
        domain_id, domain_meta = self.identify_gap()
        print(f"\nStep 1 — Gap Identified:")
        print(f"  Domain: {domain_id} ({domain_meta['name']})")
        print(f"  Coverage: {domain_meta['coverage']:.1%} (PRIORITY)")
        print(f"  Consumer: {domain_meta['consumer']}")

        # Step 2: Mine signals
        raw_tweets = self.load_raw_dataset()
        print(f"\nStep 2 — Mining ({len(raw_tweets)} raw tweets available)")
        mined = self.mine_signals(domain_id, raw_tweets)

        # Step 3: Curator structures (awaits human/Claude)
        print(f"\nStep 3 — Curator Task:")
        curated = self.curate_data(domain_id, mined)
        print(curated["instructions"])

        # Step 4: Measure
        print(f"\nStep 4 — Measure Coverage:")
        count = self.measure_coverage(domain_id)

        # Next iteration
        print(f"\n{'='*70}")
        print(f"Cycle complete. Commit curated/{domain_id}.jsonl to git.")
        print(f"Next iteration will target: {self.next_gap_after(domain_id)}")
        print(f"{'='*70}")

    def next_gap_after(self, current_domain_id):
        """Predict next highest-priority gap"""
        next_id, next_meta = next_to_explore()
        if next_id == current_domain_id:
            # Find second-best
            sorted_domains = sorted(DOMAINS.items(), key=lambda x: x[1]["coverage"])
            return sorted_domains[1][0]  # Second lowest
        return next_id

if __name__ == "__main__":
    iterator = DomainIterator()
    iterator.iterate()
