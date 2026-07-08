"""
Hermes Data Organism — continuous perception, analysis, reflection, synthesis.

Implementation owned by organ-reflection core. The entrypoint is a thin CLI
membrane.
"""

from __future__ import annotations

import json
import os
import statistics
import sys
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional

from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, normalize_kernel_base_url

from organs.contracts.src.surface_contracts import iter_json_objects
from organs.contracts.src.surface_contracts import read_json_file_or_default
try:
    from organs.reflection.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.twitter.src.adapters.hermes_paths import DATASETS_DIR, STATE_DIR


class HermesDataOrganism:
    """Continuous data processing organism."""

    def __init__(
        self,
        data_dir: Optional[Path] = None,
        state_dir: Optional[Path] = None,
        session_glob_dir: Optional[Path] = None,
        requests_timeout: int = 10,
    ):
        self.data_dir = Path(
            data_dir
            or os.environ.get(
                "HERMES_DATA_ORGANISM_DATA_DIR",
                str(DATASETS_DIR),
            )
        )
        self.datasets_dir = self.data_dir / "datasets" if (self.data_dir / "datasets").is_dir() else self.data_dir
        self.state_dir = Path(
            state_dir
            or os.environ.get(
                "HERMES_DATA_ORGANISM_STATE_DIR",
                str(STATE_DIR),
            )
        )
        self.state_file = self.state_dir / "organism_state.json"
        self.session_dir = Path(
            session_glob_dir
            or os.environ.get("HERMES_DATA_ORGANISM_SESSION_DIR", Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC")
        )
        self.requests_timeout = requests_timeout

        self.datasets_dir.mkdir(parents=True, exist_ok=True)
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.state: Dict = self._load_state()

    def _load_state(self) -> Dict:
        if self.state_file.exists():
            try:
                state = read_json_file_or_default(self.state_file, {})
                if isinstance(state, dict):
                    return state
            except Exception:
                pass

        return {
            "last_ingestion": None,
            "last_reflection": None,
            "cycle_count": 0,
        }

    def _save_state(self) -> None:
        self.state_file.parent.mkdir(parents=True, exist_ok=True)
        with open(self.state_file, "w") as f:
            json.dump(self.state, f, indent=2, default=str)

    def _safe_parse_jsonl(self, path: Path):
        with open(path, "r") as f:
            yield from iter_json_objects(f)

    def perceive_sessions(self) -> Dict:
        """Perceive session deltas from CLI session logs."""
        session_files = list(self.session_dir.glob("*.jsonl"))

        sessions_new = 0
        turns_new = 0
        intents = Counter()

        for session_file in session_files[:10]:
            for obj in self._safe_parse_jsonl(session_file):
                if obj.get("type") in ["user", "assistant"]:
                    turns_new += 1
                    msg = str(obj.get("message", ""))[:50].lower()
                    if "refactor" in msg:
                        intents["refactor"] += 1
                    elif "debug" in msg or "error" in msg:
                        intents["debug"] += 1
                    elif "feature" in msg or "add" in msg:
                        intents["feature"] += 1

        return {
            "sessions_scanned": len(session_files),
            "turns_ingested": turns_new,
            "intents": dict(intents),
        }

    def perceive_observations(self) -> Dict:
        """Query kernel health endpoint for dog health signal."""
        try:
            import requests
        except ImportError:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": "requests library not installed",
            }

        cynic_addr = os.environ.get("CYNIC_REST_ADDR")
        cynic_key = os.environ.get("CYNIC_API_KEY")

        if not cynic_addr or not cynic_key:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": "CYNIC_REST_ADDR or CYNIC_API_KEY not set",
            }

        try:
            cynic_addr = normalize_kernel_base_url(cynic_addr)
            resp = requests.get(
                f"{cynic_addr}/health",
                headers=kernel_auth_headers(cynic_key, content_type=False),
                timeout=self.requests_timeout,
            )
            if resp.status_code not in (200, 503):
                resp.raise_for_status()

            health = resp.json()
            dogs = health.get("dogs", [])
            total_failures = 0
            total_dogs = len(dogs)

            for dog in dogs:
                total_failures += int(dog.get("failures", 0))

            dog_agreement = 1.0 - (total_failures / max(total_dogs, 1)) if total_dogs > 0 else None
            dog_agreement_samples = [dog_agreement] if dog_agreement is not None else []

            return {
                "observations_queried": total_dogs,
                "failure_count": total_failures,
                "dog_agreement_samples": dog_agreement_samples,
            }
        except Exception as exc:
            return {
                "observations_queried": 0,
                "failure_count": 0,
                "dog_agreement_samples": [],
                "error": str(exc),
            }

    def analyze_patterns(self, perception: Dict) -> Dict:
        """Extract patterns from perceived data."""
        obs_summary = perception.get("observations", {})
        dog_agreement_samples = obs_summary.get("dog_agreement_samples", [])
        dog_agreement = statistics.mean(dog_agreement_samples) if dog_agreement_samples else None

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

        if dog_agreement is not None and dog_agreement < 0.5:
            analysis["patterns"]["anomalies"].append("dog_agreement_low")
        if failure_rate is not None and failure_rate > 0.1:
            analysis["patterns"]["anomalies"].append("failure_rate_high")

        return analysis

    def reflect(self, analysis: Dict) -> Dict:
        """Compose the reflection object emitted for this cycle."""
        dog_agreement = analysis["patterns"].get("dog_agreement")
        kernel_status = analysis["patterns"].get("kernel_status", "unknown")

        is_healthy = kernel_status == "responding" and len(analysis["patterns"]["anomalies"]) == 0
        diagnosis = {
            "is_healthy": is_healthy,
            "anomalies_detected": analysis["patterns"]["anomalies"],
            "kernel_status": kernel_status,
        }

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

        return {
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
            "recommendation": (
                "Monitor dog_agreement daily. If kernel unreachable, check /health. "
                "If agreement < 0.5, investigate Dog divergence."
            ),
        }

    def _synthesize_guidance(self) -> None:
        """Optional synthesis step.

        Kept optional and best-effort so the organism cycle degrades gracefully when
        local LLM tooling is unavailable.
        """
        if os.environ.get("HERMES_DATA_ORGANISM_SYNTHESIS", "0").lower() not in {"1", "true", "yes"}:
            return

        script_candidates = [
            PROJECT_ROOT / "cynic-python" / "organs" / "hermes_x" / "gemini_meta_advisor.py",
            PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "gemini_learn_from_verdicts.py",
            PROJECT_ROOT / "organs" / "twitter" / "src" / "entrypoints" / "gemini_briefing_consumer.py",
        ]

        synthesis_script = next((p for p in script_candidates if p.exists()), None)
        if synthesis_script is None:
            print("  Synthesis: no local advisor script available", file=sys.stderr)
            return

        try:
            import subprocess

            # Use project root as working directory to keep imports stable.
            result = subprocess.run(
                ["python3", str(synthesis_script), "--organ-dir", str(self.data_dir.parent)],
                capture_output=True,
                text=True,
                timeout=90,
                cwd=str(PROJECT_ROOT),
                check=False,
            )
            if result.returncode == 0:
                print("  Synthesis: guidance generated successfully", file=sys.stderr)
            else:
                print(
                    f"  Synthesis: exited with code {result.returncode}: {result.stderr[:200]}",
                    file=sys.stderr,
                )
        except subprocess.TimeoutExpired:
            print("  Synthesis: timeout (90s)", file=sys.stderr)
        except Exception as exc:
            print(f"  Synthesis: error — {str(exc)[:100]}", file=sys.stderr)

    def compound(self, reflection: Dict) -> None:
        """Persist cycle outputs for later consumers."""
        with open(self.datasets_dir / "reflection_latest.json", "w") as f:
            json.dump(reflection, f, indent=2, default=str)

        with open(self.datasets_dir / "reflections.jsonl", "a") as f:
            f.write(json.dumps(reflection, default=str) + "\n")

        self.state["last_ingestion"] = datetime.now().isoformat()
        self.state["cycle_count"] = self.state.get("cycle_count", 0) + 1
        self._save_state()

    def run_cycle(self) -> Dict:
        print(f"[Hermes Data Organism] Cycle {self.state.get('cycle_count', 0) + 1}", file=sys.stderr)

        perception = {
            "sessions": self.perceive_sessions(),
            "observations": self.perceive_observations(),
        }
        obs_error = perception["observations"].get("error")
        if obs_error:
            print(f"  Perception: {perception['sessions']['turns_ingested']} turns, observations ERROR: {obs_error}", file=sys.stderr)
        else:
            print(
                f"  Perception: {perception['sessions']['turns_ingested']} turns,"
                f" {perception['observations']['observations_queried']} observations sampled",
                file=sys.stderr,
            )

        analysis = self.analyze_patterns(perception)
        print(
            f"  Analysis: kernel_status={analysis['patterns']['kernel_status']},"
            f" dog_agreement={analysis['patterns']['dog_agreement']},"
            f" anomalies={analysis['patterns']['anomalies']}",
            file=sys.stderr,
        )

        reflection = self.reflect(analysis)
        print(f"  Diagnosis: {reflection['diagnosis']}", file=sys.stderr)

        self._synthesize_guidance()
        self.compound(reflection)
        print(f"  Compounded to {self.datasets_dir}", file=sys.stderr)

        return reflection


def main() -> int:
    organism = HermesDataOrganism()
    reflection = organism.run_cycle()

    print("\n=== HERMES DATA ORGANISM OUTPUT ===")
    print(json.dumps(reflection, indent=2, default=str))
    print(f"\nReflection written to {organism.datasets_dir / 'reflection_latest.json'}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
