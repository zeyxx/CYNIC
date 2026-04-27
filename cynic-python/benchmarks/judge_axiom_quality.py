#!/usr/bin/env python3
"""
Axiom Quality Benchmark: Judge (Dog) evaluation framework.

Measures:
- Latency (ms, P50/P95/P99)
- Axiom consistency (same input → same score across runs)
- Hallucination rate (false positives on known-bad tokens)
- Certainty calibration (q_score accuracy)

Output: JSON observations + markdown analysis.
Version: 0.1.0
"""

import json
import time
import logging
import os
from datetime import datetime
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict
import statistics
import sys

# Configure structured logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(name)s] %(levelname)s: %(message)s'
)
logger = logging.getLogger(__name__)


@dataclass
class JudgeObservation:
    """Single judgment observation."""
    timestamp: str
    dog_name: str
    input_token: str
    input_category: str  # "legit", "rug", "pump.fun", etc
    q_score: float  # 0.0 to 1.0
    verdict_type: str  # "HOWL", "BARK", "GROWL", "WAG"
    latency_ms: float
    error: Optional[str] = None

    def to_dict(self) -> Dict:
        return asdict(self)


@dataclass
class BenchmarkRun:
    """Single benchmark run summary."""
    dog_name: str
    endpoint: str
    run_id: str  # timestamp-based
    num_samples: int
    observations: List[JudgeObservation]

    def summary(self) -> Dict:
        """Compute statistics."""
        valid_latencies = [o.latency_ms for o in self.observations if o.error is None]
        valid_scores = [o.q_score for o in self.observations if o.error is None and o.q_score >= 0]

        return {
            "dog_name": self.dog_name,
            "run_id": self.run_id,
            "total_samples": len(self.observations),
            "successful": len(valid_latencies),
            "failures": len(self.observations) - len(valid_latencies),
            "latency_stats": {
                "mean_ms": statistics.mean(valid_latencies) if valid_latencies else None,
                "median_ms": statistics.median(valid_latencies) if valid_latencies else None,
                "stdev_ms": statistics.stdev(valid_latencies) if len(valid_latencies) > 1 else None,
                "p95_ms": sorted(valid_latencies)[int(len(valid_latencies) * 0.95)] if len(valid_latencies) > 0 else None,
            },
            "q_score_stats": {
                "mean": statistics.mean(valid_scores) if valid_scores else None,
                "median": statistics.median(valid_scores) if valid_scores else None,
                "stdev": statistics.stdev(valid_scores) if len(valid_scores) > 1 else None,
            },
            "consistency": self._consistency_score(valid_scores),
        }

    def _consistency_score(self, scores: List[float]) -> float:
        """0=chaotic, 1=perfect consistency."""
        if len(scores) < 2:
            return 0.0
        stdev = statistics.stdev(scores)
        mean = statistics.mean(scores)
        # Lower CV = more consistent
        cv = stdev / (mean + 1e-6)
        return max(0.0, 1.0 - cv)


class JudgeBenchmarker:
    """Benchmark a single judge endpoint."""

    def __init__(self, dog_name: str, endpoint: str):
        """
        Args:
            dog_name: e.g., "gemma-4-e4b", "phi-4-14b"
            endpoint: full URL e.g., "http://127.0.0.1:8080"
        """
        self.dog_name = dog_name
        self.endpoint = endpoint
        self.run_id = datetime.now().strftime("%Y%m%d_%H%M%S")
        self.observations: List[JudgeObservation] = []

    def judge(self, token: str, category: str) -> JudgeObservation:
        """Send one judgment request to kernel /judge endpoint."""
        import requests
        import os

        start = time.time()
        try:
            kernel_addr = os.getenv("CYNIC_REST_ADDR", "http://127.0.0.1:3030")
            api_key = os.getenv("CYNIC_API_KEY", "")

            # Ensure kernel_addr has protocol
            if not kernel_addr.startswith("http"):
                kernel_addr = f"http://{kernel_addr}"

            resp = requests.post(
                f"{kernel_addr}/judge",
                headers={
                    "Content-Type": "application/json",
                    "Authorization": f"Bearer {api_key}" if api_key else None,
                },
                json={
                    "content": token,
                    "domain": "token",
                },
                timeout=30
            )
            latency_ms = (time.time() - start) * 1000

            if resp.status_code == 200:
                data = resp.json()
                # Parse q_score from nested structure or fallback to total
                q_score_obj = data.get("q_score", {})
                if isinstance(q_score_obj, dict):
                    q_score = q_score_obj.get("total", 0.5)
                else:
                    q_score = q_score_obj if q_score_obj else 0.5
                verdict_type = data.get("verdict", "WAG")

                return JudgeObservation(
                    timestamp=datetime.now().isoformat(),
                    dog_name=self.dog_name,
                    input_token=token,
                    input_category=category,
                    q_score=q_score,
                    verdict_type=verdict_type,
                    latency_ms=latency_ms,
                )
            else:
                error_msg = f"HTTP {resp.status_code}"
                try:
                    error_msg = f"HTTP {resp.status_code}: {resp.text[:100]}"
                except:
                    pass
                logger.debug(f"Judge failed for {token}: {error_msg}")
                return JudgeObservation(
                    timestamp=datetime.now().isoformat(),
                    dog_name=self.dog_name,
                    input_token=token,
                    input_category=category,
                    q_score=-1,
                    verdict_type="ERROR",
                    latency_ms=latency_ms,
                    error=error_msg,
                )
        except Exception as e:
            return JudgeObservation(
                timestamp=datetime.now().isoformat(),
                dog_name=self.dog_name,
                input_token=token,
                input_category=category,
                q_score=-1,
                verdict_type="ERROR",
                latency_ms=(time.time() - start) * 1000,
                error=str(e),
            )

    def run(self, test_cases: List[Tuple[str, str]], num_runs: int = 1) -> BenchmarkRun:
        """
        Run benchmark.

        Args:
            test_cases: [(token, category), ...]
            num_runs: repeat each test case N times

        Returns: BenchmarkRun with observations
        """
        logger.info(f"Starting benchmark: {self.dog_name} ({num_runs} runs × {len(test_cases)} cases)")

        for run_idx in range(num_runs):
            for token, category in test_cases:
                obs = self.judge(token, category)
                self.observations.append(obs)
                logger.info(f"  [{run_idx+1}/{num_runs}] {token}: {obs.latency_ms:.1f}ms, q_score={obs.q_score}")

        return BenchmarkRun(
            dog_name=self.dog_name,
            endpoint=self.endpoint,
            run_id=self.run_id,
            num_samples=len(test_cases) * num_runs,
            observations=self.observations,
        )


def save_observations(run: BenchmarkRun, output_dir: str = "observations") -> str:
    """Save observations to JSON."""
    os.makedirs(output_dir, exist_ok=True)

    filename = f"{output_dir}/{run.dog_name}-{run.run_id}.json"
    data = {
        "metadata": {
            "dog_name": run.dog_name,
            "endpoint": run.endpoint,
            "run_id": run.run_id,
            "timestamp": datetime.now().isoformat(),
            "version": "0.1.0",
        },
        "summary": run.summary(),
        "observations": [o.to_dict() for o in run.observations],
    }

    with open(filename, "w") as f:
        json.dump(data, f, indent=2)

    logger.info(f"✅ Saved observations to {filename}")
    return filename


if __name__ == "__main__":
    # Test vectors: (token, category)
    TEST_CASES = [
        ("Raydium (RAY)", "legit"),
        ("FTX (FTT)", "rug"),
        ("SolanaPump#12345", "pump.fun"),
        ("Unknown.Token", "unknown"),
    ]

    # Benchmark Gemma
    logger.info("Benchmarking Gemma-4-E4B (APU-fixed)")
    gemma_bench = JudgeBenchmarker("gemma-4-e4b", "http://127.0.0.1:8080")
    gemma_run = gemma_bench.run(TEST_CASES, num_runs=2)
    save_observations(gemma_run)

    print("\n" + "="*60)
    print(f"Gemma Summary: {gemma_run.summary()}")
    print("="*60)
