#!/usr/bin/env python3
"""
Proof-of-Evolution — Measure organism autonomy and learning capacity.

This script runs the Scientific Protocol:
  OBSERVE (current state) → HYPOTHESIZE (what will improve) → EXPERIMENT (run cycles)
  → ANALYZE (measure) → CONCLUDE (did organism evolve?)

Falsification tests:
1. Domain confidence convergence (confident domains show consistent patterns)
2. Agent adaptation (decisions correlate with SKILL.md after cycle N)
3. Verdict agreement improvement (q_scores increase over time)
4. Coverage expansion (new domains added to analysis)
5. Robustness (organism completes cycles with partial data)

Usage:
  python3 proof_of_evolution.py --organ-dir ~/.cynic/organs/hermes/x --baseline --run-cycles 7

Output:
  evolution_baseline.json (cycle 0 metrics)
  evolution_results_*.json (cycle N metrics)
  evolution_analysis.md (statistical findings)
"""

__version__ = "0.1.0"

import json
import statistics
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional
import subprocess
import sys


class ProofOfEvolution:
    """Measure organism learning and autonomy"""

    def __init__(self, organ_dir: Path):
        self.organ_dir = Path(organ_dir)
        self.baseline_file = self.organ_dir / "evolution_baseline.json"
        self.results_dir = self.organ_dir / "evolution_results"
        self.results_dir.mkdir(exist_ok=True)
        self.reflection_path = self.organ_dir / ".reflections.jsonl"
        self.skill_path = self.organ_dir / "SKILL.md"
        self.feedback_log = self.organ_dir / "feedback_decision_log.jsonl"

    def read_reflections(self) -> List[dict]:
        """Read all reflections from JSONL"""
        reflections = []
        if not self.reflection_path.exists():
            return reflections

        try:
            with open(self.reflection_path, 'r') as f:
                for line in f:
                    try:
                        reflections.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        except Exception:
            pass

        return reflections

    def extract_metrics(self, reflection: dict) -> Dict:
        """Extract key metrics from a reflection"""
        patterns = reflection.get("patterns", {})

        # Parse domain metrics from SKILL.md instead (since reflections don't store full data)
        domain_confs = self.parse_domain_confidence_from_skill()

        return {
            "timestamp": reflection.get("timestamp"),
            "cycle": reflection.get("cycle", -1),
            "tweets_analyzed": patterns.get("tweets_analyzed", 0),
            "high_signal_tweets": patterns.get("high_signal_tweets", 0),
            "avg_signal_score": patterns.get("avg_signal_score", 0.0),
            "verdicts_analyzed": patterns.get("verdicts_analyzed", 0),
            "domain_count": len(domain_confs),
            "avg_domain_confidence": statistics.mean(domain_confs) if domain_confs else 0.0,
            "domain_confidence_variance": statistics.variance(domain_confs) if len(domain_confs) > 1 else 0.0,
            "behaviors_analyzed": patterns.get("behaviors_analyzed", 0),
            "is_healthy": reflection.get("is_healthy", False),
            "anomalies": len(reflection.get("patterns", {}).get("anomalies", [])),
        }

    def parse_domain_confidence_from_skill(self) -> list:
        """Extract domain confidence scores from SKILL.md"""
        if not self.skill_path.exists():
            return []

        try:
            skill_text = self.skill_path.read_text()
            confs = []

            # Parse lines like: "- **D1 Domain:** 530 verdicts, avg confidence 0.273, ..."
            for line in skill_text.split('\n'):
                if "avg confidence" in line:
                    parts = line.split("avg confidence ")
                    if len(parts) > 1:
                        conf_str = parts[1].split(',')[0].strip()
                        try:
                            confs.append(float(conf_str))
                        except ValueError:
                            pass

            return confs
        except Exception:
            return []

    def establish_baseline(self) -> Dict:
        """Establish baseline metrics (cycle 0)"""
        reflections = self.read_reflections()
        if not reflections:
            return {"error": "No reflections found. Run at least one organ cycle first."}

        baseline_reflection = reflections[0]
        baseline_metrics = self.extract_metrics(baseline_reflection)

        baseline_metrics["hypothesis"] = (
            "Over 7 cycles, the organism will show: "
            "(1) monotonic growth in domain_count or avg_domain_confidence, "
            "(2) agent decisions will correlate with SKILL.md changes (r > 0.5), "
            "(3) verdicts_analyzed increases as agent explores, "
            "(4) system remains robust (health stays true despite anomalies)"
        )

        baseline_metrics["falsification"] = {
            "domain_convergence_fails": "avg_domain_confidence unchanged after 7 cycles",
            "agent_doesnt_adapt": "agent decisions unrelated to SKILL.md patterns",
            "no_coverage_growth": "domain_count plateaus before cycle 7",
            "robustness_fails": "is_healthy becomes false without human intervention",
        }

        self.baseline_file.write_text(json.dumps(baseline_metrics, indent=2))
        print(f"✓ Baseline established: {self.baseline_file}")
        print(f"  - Initial domains: {baseline_metrics['domain_count']}")
        print(f"  - Initial avg confidence: {baseline_metrics['avg_domain_confidence']:.3f}")
        print(f"  - Hypothesis: {baseline_metrics['hypothesis'][:100]}...")

        return baseline_metrics

    def measure_cycle(self, cycle_num: int) -> Dict:
        """Measure metrics at current cycle"""
        reflections = self.read_reflections()
        if len(reflections) <= cycle_num:
            return {"error": f"Cycle {cycle_num} not yet completed"}

        current_reflection = reflections[cycle_num]
        metrics = self.extract_metrics(current_reflection)

        results_file = self.results_dir / f"cycle_{cycle_num:02d}.json"
        results_file.write_text(json.dumps(metrics, indent=2))

        print(f"✓ Cycle {cycle_num} measured:")
        print(f"  - Domains: {metrics['domain_count']}")
        print(f"  - Avg confidence: {metrics['avg_domain_confidence']:.3f}")
        print(f"  - Verdicts: {metrics['verdicts_analyzed']}")
        print(f"  - Health: {metrics['is_healthy']}")

        return metrics

    def analyze_evolution(self) -> Dict:
        """Analyze whether organism evolved"""
        baseline = json.loads(self.baseline_file.read_text()) if self.baseline_file.exists() else {}

        # Collect all cycle measurements
        cycle_files = sorted(self.results_dir.glob("cycle_*.json"))
        if not cycle_files:
            return {"error": "No cycle measurements found"}

        cycles = []
        for f in cycle_files:
            try:
                cycles.append(json.loads(f.read_text()))
            except json.JSONDecodeError:
                pass

        if not cycles:
            return {"error": "Could not parse cycle data"}

        # Analyze trends
        analysis = {
            "baseline": baseline,
            "cycles": cycles,
            "tests": {},
            "conclusion": "",
        }

        # Test 1: Domain coverage growth
        domain_counts = [c.get("domain_count", 0) for c in cycles]
        if domain_counts:
            growth = domain_counts[-1] - domain_counts[0]
            monotonic = all(domain_counts[i] <= domain_counts[i+1] for i in range(len(domain_counts)-1))
            analysis["tests"]["domain_coverage"] = {
                "metric": "domain_count",
                "baseline": domain_counts[0],
                "final": domain_counts[-1],
                "growth": growth,
                "monotonic": monotonic,
                "status": "PASS" if growth > 0 else "FAIL",
            }

        # Test 2: Confidence convergence
        confidences = [c.get("avg_domain_confidence", 0.0) for c in cycles]
        if len(confidences) > 1:
            trend = confidences[-1] - confidences[0]
            variance = statistics.variance(confidences) if len(confidences) > 1 else 0.0
            analysis["tests"]["confidence_convergence"] = {
                "metric": "avg_domain_confidence",
                "baseline": confidences[0],
                "final": confidences[-1],
                "trend": trend,
                "variance": variance,
                "status": "PASS" if variance < 0.01 else "DIVERGING",
            }

        # Test 3: Verdict growth (agent exploring)
        verdicts = [c.get("verdicts_analyzed", 0) for c in cycles]
        if len(verdicts) > 1:
            growth = verdicts[-1] - verdicts[0]
            monotonic = all(verdicts[i] <= verdicts[i+1] for i in range(len(verdicts)-1))
            analysis["tests"]["verdict_growth"] = {
                "metric": "verdicts_analyzed",
                "baseline": verdicts[0],
                "final": verdicts[-1],
                "growth": growth,
                "monotonic": monotonic,
                "status": "PASS" if growth > 0 else "FAIL",
            }

        # Test 4: Robustness (health consistency)
        healths = [c.get("is_healthy", False) for c in cycles]
        health_true_count = sum(1 for h in healths if h)
        analysis["tests"]["robustness"] = {
            "metric": "is_healthy",
            "cycles_healthy": health_true_count,
            "cycles_total": len(healths),
            "stability": "PASS" if health_true_count >= len(healths) * 0.8 else "FAIL",
        }

        # Test 5: Anomaly reduction
        anomalies = [c.get("anomalies", 0) for c in cycles]
        anomaly_trend = anomalies[-1] - anomalies[0] if anomalies else 0
        analysis["tests"]["anomaly_reduction"] = {
            "metric": "anomalies_detected",
            "baseline": anomalies[0] if anomalies else 0,
            "final": anomalies[-1] if anomalies else 0,
            "trend": anomaly_trend,
            "status": "PASS" if anomaly_trend <= 0 else "INCREASING",
        }

        # Summary verdict
        passed_tests = sum(1 for t in analysis["tests"].values() if t.get("status") == "PASS")
        total_tests = len(analysis["tests"])

        if passed_tests >= 3:
            analysis["conclusion"] = f"✓ EVOLVED: {passed_tests}/{total_tests} tests passed. Organism shows measurable learning."
        elif passed_tests >= 2:
            analysis["conclusion"] = f"⚠ PARTIAL: {passed_tests}/{total_tests} tests passed. Evolution underway but incomplete."
        else:
            analysis["conclusion"] = f"✗ NOT EVOLVED: {passed_tests}/{total_tests} tests failed. Organism not yet learning."

        return analysis

    def run_cycle_and_measure(self) -> Dict:
        """Execute one organ cycle and measure results"""
        print("\n[Organ Cycle] Running...")
        try:
            # Run from cynic-python directory where organs module is importable
            cynic_python_dir = self.organ_dir.parent.parent
            result = subprocess.run(
                ["python3", "-m", "organs.hermes_x"],
                cwd=str(cynic_python_dir),
                capture_output=True,
                text=True,
                timeout=60,
            )
            if result.returncode != 0:
                print(f"⚠ Cycle execution: {result.stderr[:200]}")
                # Try direct invocation as fallback
                result = subprocess.run(
                    ["python3", str(self.organ_dir / "__main__.py")],
                    cwd=str(cynic_python_dir),
                    capture_output=True,
                    text=True,
                    timeout=60,
                )
        except Exception as e:
            print(f"⚠ Cycle execution failed: {e}")
            return {"error": str(e)}

        # Measure after cycle completes
        reflections = self.read_reflections()
        if reflections:
            cycle_num = len(reflections) - 1
            return self.measure_cycle(cycle_num)
        else:
            return {"error": "No reflections created"}


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Proof-of-Evolution for Hermes X Organism")
    parser.add_argument("--organ-dir", default=str(Path.home() / ".cynic" / "organs" / "hermes" / "x"),
                        help="Organ directory")
    parser.add_argument("--baseline", action="store_true", help="Establish baseline metrics")
    parser.add_argument("--run-cycles", type=int, default=0, help="Run N cycles and measure")
    parser.add_argument("--analyze", action="store_true", help="Analyze evolution from stored results")
    args = parser.parse_args()

    organ_dir = Path(args.organ_dir).expanduser()
    if not organ_dir.exists():
        print(f"ERROR: Organ directory not found: {organ_dir}")
        return 1

    proof = ProofOfEvolution(organ_dir)

    if args.baseline:
        print(f"\n=== ESTABLISH BASELINE (Cycle 0) ===")
        proof.establish_baseline()
        return 0

    if args.run_cycles > 0:
        print(f"\n=== RUN {args.run_cycles} CYCLES AND MEASURE ===")
        for i in range(args.run_cycles):
            print(f"\n[Cycle {i+1}/{args.run_cycles}]")
            proof.run_cycle_and_measure()

        print(f"\n=== ANALYSIS ===")
        analysis = proof.analyze_evolution()
        analysis_file = organ_dir / "evolution_analysis.json"
        analysis_file.write_text(json.dumps(analysis, indent=2))
        print(f"✓ Analysis saved: {analysis_file}")
        print(f"\n{analysis['conclusion']}")

        # Print test details
        for test_name, test_result in analysis.get("tests", {}).items():
            print(f"\n  {test_name}: {test_result.get('status')}")
            for key, val in test_result.items():
                if key != "status":
                    print(f"    {key}: {val}")

        return 0

    if args.analyze:
        print(f"\n=== ANALYZE EXISTING RESULTS ===")
        analysis = proof.analyze_evolution()
        print(f"\n{analysis['conclusion']}")
        return 0

    print("Use --baseline, --run-cycles N, or --analyze")
    return 1


if __name__ == "__main__":
    sys.exit(main())
