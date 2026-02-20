#!/usr/bin/env python3
"""
CYNIC Stress Test — Phase 4: Break the Organism

Load test CYNIC kernel under increasing concurrent load.
Measure: latency, throughput, errors, resource usage.

Usage:
  python3.13 scripts/stress_test.py --duration 300 --rps 50 --workers 10
"""
from __future__ import annotations

import asyncio
import json
import logging
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from statistics import mean, median, stdev
from typing import Any

import httpx

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("stress_test")


@dataclass
class Metrics:
    """Collected metrics from a test run."""
    level: str  # "L1", "L2", etc.
    duration_sec: int
    target_rps: int
    actual_rps: float = 0.0

    # Latencies (ms)
    latencies: list[float] = field(default_factory=list)
    latency_mean: float = 0.0
    latency_median: float = 0.0
    latency_p95: float = 0.0
    latency_p99: float = 0.0
    latency_max: float = 0.0

    # Outcomes
    success_count: int = 0
    error_count: int = 0
    timeout_count: int = 0

    # Q-Scores (quality)
    q_scores: list[float] = field(default_factory=list)
    q_score_mean: float = 0.0
    q_score_min: float = 0.0
    q_score_max: float = 0.0

    # Resource estimates
    cpu_percent: float = 0.0
    memory_mb: float = 0.0

    def compute(self) -> None:
        """Finalize metrics."""
        if self.latencies:
            self.latencies.sort()
            self.latency_mean = mean(self.latencies)
            self.latency_median = median(self.latencies)
            self.latency_p95 = self.latencies[int(len(self.latencies) * 0.95)]
            self.latency_p99 = self.latencies[int(len(self.latencies) * 0.99)]
            self.latency_max = max(self.latencies)

        if self.q_scores:
            self.q_score_mean = mean(self.q_scores)
            self.q_score_min = min(self.q_scores)
            self.q_score_max = max(self.q_scores)

        total = self.success_count + self.error_count + self.timeout_count
        self.actual_rps = total / self.duration_sec if self.duration_sec > 0 else 0

    def to_dict(self) -> dict[str, Any]:
        """Export as JSON-serializable dict."""
        return {
            "level": self.level,
            "duration_sec": self.duration_sec,
            "target_rps": self.target_rps,
            "actual_rps": round(self.actual_rps, 2),
            "latency_ms": {
                "mean": round(self.latency_mean, 1),
                "median": round(self.latency_median, 1),
                "p95": round(self.latency_p95, 1),
                "p99": round(self.latency_p99, 1),
                "max": round(self.latency_max, 1),
            },
            "outcomes": {
                "success": self.success_count,
                "error": self.error_count,
                "timeout": self.timeout_count,
                "total": self.success_count + self.error_count + self.timeout_count,
            },
            "q_score": {
                "mean": round(self.q_score_mean, 1),
                "min": round(self.q_score_min, 1),
                "max": round(self.q_score_max, 1),
            },
            "resources": {
                "cpu_percent": round(self.cpu_percent, 1),
                "memory_mb": round(self.memory_mb, 1),
            },
        }


async def make_judgment(client: httpx.AsyncClient, judgment_id: int, timeout_sec: float = 10.0) -> tuple[float, bool, float]:
    """
    Make a single judgment request.

    Returns: (latency_ms, success, q_score)
    """
    try:
        payload = {
            "content": f"def calculate(x, y):\n    return x + y\n# judgment {judgment_id}",
            "reality": "CODE",
            "analysis": "JUDGE",
            "context": f"Stress test payload {judgment_id}",
            "budget_usd": 0.001,
            "level": "REFLEX",  # Fast path
        }

        t0 = time.perf_counter()
        resp = await client.post("/judge", json=payload, timeout=timeout_sec)
        latency_ms = (time.perf_counter() - t0) * 1000

        if resp.status_code == 200:
            data = resp.json()
            q_score = data.get("q_score", 0.0)
            return latency_ms, True, q_score
        else:
            logger.debug(f"Judgment {judgment_id} HTTP {resp.status_code}")
            return latency_ms, False, 0.0

    except asyncio.TimeoutError:
        logger.debug(f"Judgment {judgment_id} timeout")
        return 0.0, False, 0.0
    except Exception as e:
        logger.debug(f"Judgment {judgment_id} error: {e}")
        return 0.0, False, 0.0


async def stress_test_level(
    client: httpx.AsyncClient,
    level: str,
    target_rps: int,
    duration_sec: int,
    max_workers: int = 10,
) -> Metrics:
    """
    Run stress test at a given RPS level.

    Args:
        level: "L1", "L2", etc.
        target_rps: Requests per second
        duration_sec: How long to run
        max_workers: Max concurrent requests
    """
    metrics = Metrics(level=level, duration_sec=duration_sec, target_rps=target_rps)

    logger.info(f"Starting {level}: {target_rps} RPS for {duration_sec}s (max {max_workers} workers)")

    judgment_id = 0
    start_time = time.time()
    interval = 1.0 / target_rps if target_rps > 0 else 0.1
    next_send = start_time

    pending = set()

    while time.time() - start_time < duration_sec:
        # Launch new requests to maintain target RPS
        while time.time() >= next_send and len(pending) < max_workers:
            judgment_id += 1
            task = asyncio.create_task(make_judgment(client, judgment_id))
            pending.add(task)
            next_send += interval

        # Collect completed requests
        if pending:
            done, pending = await asyncio.wait(pending, timeout=0.1, return_when=asyncio.FIRST_COMPLETED)
            for task in done:
                try:
                    latency_ms, success, q_score = await task
                    metrics.latencies.append(latency_ms)
                    metrics.q_scores.append(q_score)

                    if success:
                        metrics.success_count += 1
                    else:
                        metrics.error_count += 1
                except Exception:
                    metrics.error_count += 1
        else:
            await asyncio.sleep(0.01)

    # Wait for stragglers
    if pending:
        done, _ = await asyncio.wait(pending, timeout=5.0)
        for task in done:
            try:
                latency_ms, success, q_score = await task
                metrics.latencies.append(latency_ms)
                metrics.q_scores.append(q_score)
                if success:
                    metrics.success_count += 1
                else:
                    metrics.error_count += 1
            except Exception:
                metrics.error_count += 1

    metrics.compute()
    return metrics


async def stress_test_suite(
    kernel_url: str = "http://localhost:8000",
    levels: list[int] | None = None,
    duration_sec: int = 60,
) -> dict[str, Any]:
    """
    Run complete stress test suite across multiple RPS levels.

    Args:
        kernel_url: CYNIC kernel URL
        levels: RPS levels to test (default: [1, 10, 50, 100])
        duration_sec: Duration per level
    """
    if levels is None:
        levels = [1, 10, 50, 100]

    campaign_id = datetime.now().isoformat()[:19].replace(":", "-")
    logger.info(f"Starting stress test campaign {campaign_id}")

    # Verify kernel alive
    try:
        async with httpx.AsyncClient(base_url=kernel_url, timeout=10.0) as client:
            resp = await client.get("/health")
            resp.raise_for_status()
            logger.info("Kernel is alive ✓")
    except Exception as e:
        logger.error(f"Kernel not responding: {e}")
        return {"error": str(e), "campaign_id": campaign_id}

    results: list[Metrics] = []

    async with httpx.AsyncClient(base_url=kernel_url, timeout=30.0) as client:
        for i, target_rps in enumerate(levels, 1):
            level_name = f"L{i}"

            try:
                metrics = await stress_test_level(
                    client,
                    level=level_name,
                    target_rps=target_rps,
                    duration_sec=duration_sec,
                    max_workers=min(target_rps * 2, 50),  # 2x RPS workers, max 50
                )
                results.append(metrics)

                # Print summary
                print(f"\n{'='*70}")
                print(f"{level_name}: {target_rps} RPS")
                print(f"{'='*70}")
                print(f"Success: {metrics.success_count} | Error: {metrics.error_count} | Timeout: {metrics.timeout_count}")
                print(f"Actual RPS: {metrics.actual_rps:.1f} (target: {target_rps})")
                print(f"Latency (ms): Mean={metrics.latency_mean:.1f} | Median={metrics.latency_median:.1f} | P95={metrics.latency_p95:.1f} | P99={metrics.latency_p99:.1f} | Max={metrics.latency_max:.1f}")
                print(f"Q-Score: Mean={metrics.q_score_mean:.1f} | Min={metrics.q_score_min:.1f} | Max={metrics.q_score_max:.1f}")

                # Break if too many errors
                error_rate = metrics.error_count / (metrics.success_count + metrics.error_count) if (metrics.success_count + metrics.error_count) > 0 else 0
                if error_rate > 0.2:  # >20% errors = something broke
                    logger.warning(f"Error rate {error_rate:.1%} > 20%, stopping test")
                    break

            except Exception as e:
                logger.error(f"Level {level_name} failed: {e}")
                break

    # Save results
    output_path = Path.home() / ".cynic" / "stress_tests" / f"{campaign_id}.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    campaign_result = {
        "campaign_id": campaign_id,
        "timestamp": datetime.now().isoformat(),
        "kernel_url": kernel_url,
        "results": [r.to_dict() for r in results],
    }

    with open(output_path, "w") as f:
        json.dump(campaign_result, f, indent=2)

    logger.info(f"Results saved to {output_path}")

    # Final summary
    print(f"\n{'='*70}")
    print("STRESS TEST CAMPAIGN SUMMARY")
    print(f"{'='*70}")
    print(f"Campaign ID: {campaign_id}")
    print(f"Levels tested: {len(results)}")
    for metrics in results:
        print(f"  {metrics.level}: {metrics.target_rps} RPS -> {metrics.actual_rps:.1f} RPS (latency p99={metrics.latency_p99:.1f}ms, errors={metrics.error_count})")
    print(f"{'='*70}\n")

    return campaign_result


def main() -> None:
    """CLI entry point."""
    import argparse

    parser = argparse.ArgumentParser(description="CYNIC Stress Test — Phase 4")
    parser.add_argument("--kernel-url", default="http://localhost:8000", help="CYNIC kernel URL")
    parser.add_argument("--levels", nargs="+", type=int, default=[1, 10, 50, 100], help="RPS levels to test")
    parser.add_argument("--duration", type=int, default=60, help="Duration per level (seconds)")

    args = parser.parse_args()

    try:
        result = asyncio.run(stress_test_suite(
            kernel_url=args.kernel_url,
            levels=args.levels,
            duration_sec=args.duration,
        ))

        if "error" in result:
            logger.error(result["error"])
            sys.exit(1)

    except KeyboardInterrupt:
        logger.info("Stress test interrupted")
        sys.exit(0)
    except Exception as e:
        logger.error(f"Stress test failed: {e}", exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
