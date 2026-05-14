#!/usr/bin/env python3
"""
Hermes Data Organism — Continuous perception, analysis, reflection, synthesis.

Runs on systemd timer (hourly ingestion, daily reflection).
Compounds on datasets: each cycle builds on the last.
Looks for: self-diagnosis, anomalies, opportunities, causality.

LAYER 1: PERCEPTION (ingestion of deltas)
LAYER 2: ANALYSIS (pattern extraction from deltas)
LAYER 3: REFLECTION (compound patterns into wisdom)
LAYER 4: SYNTHESIS (generate meta-guidance via Gemini)
LAYER 5: FEEDBACK (emit signals for human/system action)
"""

import json
import sys
import os
import statistics
from pathlib import Path
from collections import defaultdict, Counter
from datetime import datetime, timedelta
from dataclasses import dataclass, asdict
from typing import Dict, List, Optional
import hashlib
try:
    import requests
except ImportError:
    requests = None

sys.path.insert(0, str(Path(__file__).resolve().parent / "hermes-x" / "core"))
from hermes_paths import HERMES_X_DIR, STATE_DIR


@dataclass
class DatasetState:
    """Track what we've already processed"""
    last_processed_timestamp: str
    last_reflection_date: str
    session_count: int
    observation_count: int
    fingerprint: str  # hash of last line processed


class HermesDataOrganism:
    """Continuous data processing organism"""

    def __init__(self, data_dir: Path = None, state_dir: Path = None):
        self.data_dir = data_dir or HERMES_X_DIR
        # FIX: Output path aligned with X organ directory structure
        self.datasets_dir = self.data_dir / "datasets"
        # State persisted separately for cycle tracking
        self.state_dir = state_dir or STATE_DIR
        self.state_file = self.state_dir / "organism_state.json"

        self.datasets_dir.mkdir(parents=True, exist_ok=True)
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.state: Dict = self._load_state()

    def _load_state(self) -> Dict:
        """Load previous cycle state"""
        if self.state_file.exists():
            with open(self.state_file, 'r') as f:
                return json.load(f)
        return {
            "last_ingestion": None,
            "last_reflection": None,
            "cycle_count": 0,
        }

    def _save_state(self) -> None:
        """Persist state for next cycle"""
        with open(self.state_file, 'w') as f:
            json.dump(self.state, f, indent=2, default=str)

    def perceive_sessions(self) -> Dict:
        """LAYER 1: Ingest session deltas since last cycle"""
        project_dir = Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC"
        session_files = list(project_dir.glob("*.jsonl"))

        sessions_new = 0
        turns_new = 0
        intents = Counter()

        for session_file in session_files[:10]:  # Sample: first 10 sessions
            session_id = session_file.stem

            with open(session_file, 'r') as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        if obj.get('type') in ['user', 'assistant']:
                            turns_new += 1
                            # Infer intent from message
                            msg = str(obj.get('message', ''))[:50].lower()
                            if 'refactor' in msg:
                                intents['refactor'] += 1
                            elif 'debug' in msg or 'error' in msg:
                                intents['debug'] += 1
                            elif 'feature' in msg or 'add' in msg:
                                intents['feature'] += 1
                    except:
                        pass

        return {
            "sessions_scanned": len(session_files),
            "turns_ingested": turns_new,
            "intents": dict(intents),
        }

    def perceive_observations(self) -> Dict:
        """LAYER 1: Query kernel health for Dog metrics (fixed: uses /health not /observations)"""
        if not requests:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": "requests library not installed"
            }

        cynic_addr = os.environ.get("CYNIC_REST_ADDR")
        cynic_key = os.environ.get("CYNIC_API_KEY")

        if not cynic_addr or not cynic_key:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": "CYNIC_REST_ADDR or CYNIC_API_KEY not set"
            }

        # Ensure URL has scheme
        if not cynic_addr.startswith(("http://", "https://")):
            cynic_addr = f"http://{cynic_addr}"

        try:
            resp = requests.get(
                f"{cynic_addr}/health",
                headers={"Authorization": f"Bearer {cynic_key}"},
                timeout=10,
            )
            # 200 = sovereign, 503 = degraded — both return valid Dog metrics
            if resp.status_code not in (200, 503):
                resp.raise_for_status()
            health = resp.json()

            # Extract Dog metrics from health endpoint
            dogs = health.get("dogs", [])
            total_failures = 0
            total_dogs = len(dogs)

            # Compute dog_agreement from circuit states and failure counts
            # High agreement = all circuits closed + low failure counts
            # dog_agreement = 1 - (total_failures / max(total_dogs, 1))
            for dog in dogs:
                total_failures += dog.get("failures", 0)

            # Normalize: if no failures, agreement = 1.0; if all failed, agreement = 0.0
            dog_agreement = 1.0 - (total_failures / max(total_dogs, 1)) if total_dogs > 0 else None
            dog_agreement_samples = [dog_agreement] if dog_agreement is not None else []

            return {
                "observations_queried": total_dogs,  # number of dogs evaluated
                "failure_count": total_failures,
                "dog_agreement_samples": dog_agreement_samples,
            }
        except Exception as e:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": str(e)
            }


    def analyze_patterns(self, perception: Dict) -> Dict:
        """LAYER 2: Extract patterns from perceived data (FIXED: computed, not hardcoded)"""
        obs_summary = perception.get("observations", {})

        # Compute dog_agreement from real samples
        dog_agreement_samples = obs_summary.get("dog_agreement_samples", [])
        if dog_agreement_samples:
            dog_agreement = statistics.mean(dog_agreement_samples)
        else:
            dog_agreement = None

        # Compute failure rate from real observations
        observations_queried = obs_summary.get("observations_queried", 0)
        failure_count = obs_summary.get("failure_count", 0)
        failure_rate = (failure_count / observations_queried) if observations_queried > 0 else None

        analysis = {
            "timestamp": datetime.now().isoformat(),
            "perception_summary": perception,
            "patterns": {
                "dog_agreement": dog_agreement,
                "failure_rate": failure_rate,
                "observations_sampled": observations_queried,
                "failure_modes": {},
                "anomalies": [],
                "kernel_status": "unreachable" if dog_agreement is None else "responding",
            },
        }

        # Detect anomalies from REAL data
        if dog_agreement is not None:
            if dog_agreement < 0.5:
                analysis["patterns"]["anomalies"].append("dog_agreement_low")
        if failure_rate is not None:
            if failure_rate > 0.1:
                analysis["patterns"]["anomalies"].append("failure_rate_high")

        return analysis

    def reflect(self, analysis: Dict) -> Dict:
        """LAYER 3: Compound analysis into wisdom"""
        dog_agreement = analysis["patterns"].get("dog_agreement")
        kernel_status = analysis["patterns"].get("kernel_status", "unknown")

        # Diagnosis: honest about what we measured
        is_healthy = kernel_status == "responding" and len(analysis["patterns"]["anomalies"]) == 0
        diagnosis = {
            "is_healthy": is_healthy,
            "anomalies_detected": analysis["patterns"]["anomalies"],
            "kernel_status": kernel_status,
        }

        # Opportunities: data-driven, not hardcoded
        opportunities = []
        if kernel_status == "unreachable":
            opportunities.append("⚠ Kernel unreachable — cannot measure system health")
        elif dog_agreement is not None:
            if dog_agreement < 0.5:
                opportunities.append(f"Dogs disagreeing heavily: {dog_agreement:.3f} < 0.5 threshold")
            elif dog_agreement < 0.7:
                opportunities.append(f"Dogs agreement suboptimal: {dog_agreement:.3f}, target 0.70")
            else:
                opportunities.append(f"Dogs agreement healthy: {dog_agreement:.3f}")

        reflection = {
            "timestamp": datetime.now().isoformat(),
            "cycle": self.state.get("cycle_count", 0),
            "data_summary": {
                "turns_ingested": analysis["perception_summary"].get("sessions", {}).get("turns_ingested", 0),
                "intents": analysis["perception_summary"].get("sessions", {}).get("intents", {}),
                "observations_sampled": analysis["patterns"].get("observations_sampled", 0),
            },
            "patterns": analysis["patterns"],
            "diagnosis": diagnosis,
            "opportunities": opportunities,
            "recommendation": "Monitor dog_agreement daily. If kernel unreachable, check /health. If agreement < 0.5, investigate Dog divergence.",
        }

        return reflection

    def _synthesize_guidance(self) -> None:
        """LAYER 4: Generate meta-guidance via Gemini meta-advisor"""
        try:
            import subprocess

            # Find the project root
            script_dir = Path(__file__).parent
            project_root = script_dir.parent
            synthesis_script = project_root / "cynic-python" / "organs" / "hermes_x" / "gemini_meta_advisor.py"

            if not synthesis_script.exists():
                print(f"  Synthesis: Script not found at {synthesis_script}", file=sys.stderr)
                return

            organ_dir = str(self.data_dir)
            result = subprocess.run(
                ["python3", str(synthesis_script), "--organ-dir", organ_dir],
                capture_output=True,
                text=True,
                timeout=90,
            )

            if result.returncode == 0:
                print(f"  Synthesis: Guidance generated successfully", file=sys.stderr)
            else:
                print(f"  Synthesis: Failed (code {result.returncode}): {result.stderr[:200]}", file=sys.stderr)
        except subprocess.TimeoutExpired:
            print(f"  Synthesis: Timeout (90s)", file=sys.stderr)
        except Exception as e:
            print(f"  Synthesis: Error — {str(e)[:100]}", file=sys.stderr)

    def compound(self, reflection: Dict) -> None:
        """LAYER 5: Output signals and persist for next cycle"""
        # Write reflection
        with open(self.datasets_dir / "reflection_latest.json", 'w') as f:
            json.dump(reflection, f, indent=2, default=str)

        # Append to history (compound)
        with open(self.datasets_dir / "reflections.jsonl", 'a') as f:
            f.write(json.dumps(reflection, default=str) + '\n')

        # Update state
        self.state["last_ingestion"] = datetime.now().isoformat()
        self.state["cycle_count"] = self.state.get("cycle_count", 0) + 1
        self._save_state()

    def run_cycle(self) -> Dict:
        """Execute one full cycle: perceive → analyze → reflect → synthesize → compound"""
        print(f"[Hermes Data Organism] Cycle {self.state.get('cycle_count', 0) + 1}", file=sys.stderr)

        # LAYER 1: Perceive (2 real data sources: sessions, kernel observations)
        perception = {
            "sessions": self.perceive_sessions(),
            "observations": self.perceive_observations(),
        }
        obs_error = perception["observations"].get("error")
        if obs_error:
            print(f"  Perception: {perception['sessions']['turns_ingested']} turns, observations ERROR: {obs_error}", file=sys.stderr)
        else:
            print(f"  Perception: {perception['sessions']['turns_ingested']} turns, {perception['observations']['observations_queried']} observations sampled", file=sys.stderr)

        # LAYER 2: Analyze
        analysis = self.analyze_patterns(perception)
        kernel_status = analysis["patterns"].get("kernel_status", "unknown")
        dog_agreement = analysis["patterns"].get("dog_agreement")
        print(f"  Analysis: kernel_status={kernel_status}, dog_agreement={dog_agreement}, anomalies={analysis['patterns']['anomalies']}", file=sys.stderr)

        # LAYER 3: Reflect
        reflection = self.reflect(analysis)
        print(f"  Diagnosis: {reflection['diagnosis']}", file=sys.stderr)

        # LAYER 4: Synthesize (generate meta-guidance via Gemini)
        self._synthesize_guidance()

        # LAYER 5: Compound
        self.compound(reflection)
        print(f"  Compounded to {self.datasets_dir}", file=sys.stderr)

        return reflection


def main():
    organism = HermesDataOrganism()

    # Run one cycle
    reflection = organism.run_cycle()

    # Output for Hermes agent to consume
    print("\n=== HERMES DATA ORGANISM OUTPUT ===")
    print(json.dumps(reflection, indent=2, default=str))

    # Also emit to stdout for systemd logging
    print(f"\nReflection written to {organism.datasets_dir / 'reflection_latest.json'}", file=sys.stderr)


if __name__ == '__main__':
    main()
