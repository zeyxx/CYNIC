#!/usr/bin/env python3
"""
Tier 1 Falsification Test — Agent Adaptation to SKILL.md

Measures whether agent decisions correlate with learned domain confidence.
The feedback loop closes when agent chooses high-confidence domains more often.

Hypothesis: After agent reads SKILL.md, Pearson r(agent_decision_freq, skill_confidence) > 0.6

Scientific Protocol:
1. OBSERVE: Baseline agent decisions before SKILL injection (3 decisions)
2. HYPOTHESIZE: Agent will weight high-confidence domains more heavily
3. EXPERIMENT: Run agent with SKILL.md context for 3 cycles
4. ANALYZE: Measure correlation between domain choice frequency and confidence
5. CONCLUDE: If r > 0.6, adaptation observed. If r < 0.3, hypothesis falsified.

Usage:
  python3 tier1_falsification.py --organ-dir ~/.cynic/organs/hermes/x --baseline
  python3 tier1_falsification.py --organ-dir ~/.cynic/organs/hermes/x --measure
"""

__version__ = "0.1.0"

import json
import statistics
from pathlib import Path
from typing import Dict, List, Tuple
import argparse
import sys


class Tier1Falsification:
    """Measure agent adaptation to SKILL.md learning."""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.feedback_log = self.organ_dir / "feedback_decision_log.jsonl"
        self.skill_path = self.organ_dir / "SKILL.md"
        self.baseline_file = self.organ_dir / "tier1_baseline.json"
        self.results_file = self.organ_dir / "tier1_results.json"

    def read_agent_decisions(self) -> List[dict]:
        """Read all agent decisions from feedback log."""
        decisions = []
        if not self.feedback_log.exists():
            return decisions

        try:
            with open(self.feedback_log, 'r') as f:
                for line in f:
                    try:
                        decisions.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except Exception:
            pass

        return decisions

    def compute_decision_frequency(self, decisions: List[dict]) -> Dict[str, float]:
        """Compute how often agent chose each domain."""
        domain_counts = {}
        for decision in decisions:
            domain = decision.get("decision", "unknown")
            domain_counts[domain] = domain_counts.get(domain, 0) + 1

        total = sum(domain_counts.values())
        if total == 0:
            return {}

        # Normalize to frequencies
        return {d: count / total for d, count in domain_counts.items()}

    def extract_skill_confidence(self) -> Dict[str, float]:
        """Extract domain confidence scores from SKILL.md."""
        confidences = {}
        if not self.skill_path.exists():
            return confidences

        try:
            skill_text = self.skill_path.read_text()

            # Parse lines like: "- **D1 Domain:** 530 verdicts, avg confidence 0.273"
            for line in skill_text.split('\n'):
                if "avg confidence" in line and "Domain:" in line:
                    try:
                        parts = line.split("Domain:")
                        if len(parts) > 1:
                            domain_part = parts[0].split("**")[-1].strip()
                            confidence_parts = parts[1].split("avg confidence ")
                            if len(confidence_parts) > 1:
                                conf_str = confidence_parts[1].split(',')[0].strip()
                                confidences[domain_part] = float(conf_str)
                    except (ValueError, IndexError):
                        pass

        except Exception:
            pass

        return confidences

    def compute_pearson_r(
        self, agent_freq: Dict[str, float], skill_conf: Dict[str, float]
    ) -> Tuple[float, str]:
        """Compute Pearson correlation between agent frequency and skill confidence.

        Returns (r_value, interpretation).
        """
        # Align domains present in both
        common_domains = set(agent_freq.keys()) & set(skill_conf.keys())
        if len(common_domains) < 2:
            return 0.0, "insufficient data (< 2 common domains)"

        x = [agent_freq[d] for d in common_domains]
        y = [skill_conf[d] for d in common_domains]

        # Pearson correlation
        n = len(x)
        mean_x = statistics.mean(x)
        mean_y = statistics.mean(y)

        numerator = sum((x[i] - mean_x) * (y[i] - mean_y) for i in range(n))
        denom_x = sum((x[i] - mean_x) ** 2 for i in range(n))
        denom_y = sum((y[i] - mean_y) ** 2 for i in range(n))

        if denom_x == 0 or denom_y == 0:
            return 0.0, "zero variance (agent or SKILL)"

        r = numerator / (denom_x ** 0.5 * denom_y ** 0.5)

        # Interpret
        if r > 0.6:
            interpretation = "PASS: Agent adapted to SKILL.md (strong correlation)"
        elif r > 0.3:
            interpretation = "PARTIAL: Weak adaptation visible"
        else:
            interpretation = "FAIL: Agent ignoring SKILL.md (or random)"

        return r, interpretation

    def establish_baseline(self) -> Dict:
        """Establish baseline before SKILL injection."""
        decisions = self.read_agent_decisions()
        if not decisions:
            return {"error": "No agent decisions found"}

        freq = self.compute_decision_frequency(decisions)
        skill_conf = self.extract_skill_confidence()

        baseline = {
            "timestamp": decisions[-1].get("timestamp") if decisions else None,
            "agent_decisions_count": len(decisions),
            "agent_frequency": freq,
            "skill_confidence": skill_conf,
            "hypothesis": "After agent reads SKILL.md, decision frequency will correlate with domain confidence (r > 0.6)",
        }

        self.baseline_file.write_text(json.dumps(baseline, indent=2))
        print(f"✓ Tier 1 baseline established")
        print(f"  Agent decisions: {len(decisions)}")
        print(f"  Decision frequency: {freq}")
        print(f"  SKILL confidence: {skill_conf}")
        return baseline

    def measure_adaptation(self) -> Dict:
        """Measure correlation after SKILL.md injection."""
        # Load baseline
        if not self.baseline_file.exists():
            return {"error": "No baseline established. Run --baseline first."}

        baseline = json.loads(self.baseline_file.read_text())

        # Get current state
        decisions = self.read_agent_decisions()
        freq = self.compute_decision_frequency(decisions)
        skill_conf = self.extract_skill_confidence()

        # Compute correlation
        r, interpretation = self.compute_pearson_r(freq, skill_conf)

        results = {
            "baseline_decisions": baseline.get("agent_decisions_count"),
            "current_decisions": len(decisions),
            "new_decisions": len(decisions) - baseline.get("agent_decisions_count", 0),
            "agent_frequency": freq,
            "skill_confidence": skill_conf,
            "pearson_r": r,
            "interpretation": interpretation,
            "verdict": "PASS" if r > 0.6 else ("PARTIAL" if r > 0.3 else "FAIL"),
        }

        self.results_file.write_text(json.dumps(results, indent=2))

        print(f"✓ Tier 1 measurement complete")
        print(f"  Pearson r: {r:.3f}")
        print(f"  Verdict: {results['verdict']}")
        print(f"  Interpretation: {interpretation}")
        print(f"  Agent frequency: {freq}")
        print(f"  SKILL confidence: {skill_conf}")

        return results


def main():
    parser = argparse.ArgumentParser(
        description="Tier 1 Falsification Test — Agent Adaptation to SKILL.md"
    )
    parser.add_argument("--organ-dir", default=str(Path.home() / ".cynic" / "organs" / "hermes" / "x"))
    parser.add_argument("--baseline", action="store_true", help="Establish baseline")
    parser.add_argument("--measure", action="store_true", help="Measure correlation")

    args = parser.parse_args()
    organ_dir = Path(args.organ_dir).expanduser()

    if not organ_dir.exists():
        print(f"ERROR: Organ directory not found: {organ_dir}")
        return 1

    tier1 = Tier1Falsification(organ_dir)

    if args.baseline:
        tier1.establish_baseline()
        return 0

    if args.measure:
        results = tier1.measure_adaptation()
        if "error" in results:
            print(f"ERROR: {results['error']}")
            return 1
        return 0

    print("Use --baseline or --measure")
    return 1


if __name__ == "__main__":
    sys.exit(main())
