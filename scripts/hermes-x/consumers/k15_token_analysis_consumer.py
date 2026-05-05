#!/usr/bin/env python3
"""K15 Token Analysis Consumer — Route token-analysis observations to agent tasks.

Separate from TwitterDog consumer because token verdicts are already domain-scored.
This consumer just thresholds on records_count and verdict distribution.

K15 Seam 2: observations (token-analysis) → agent-tasks (token-judgment)

Usage:
  python k15_token_analysis_consumer.py --kernel-url http://localhost:3030 \
                                        --api-key $CYNIC_API_KEY \
                                        --poll-interval 60

Cron: Every 30m (systemd hermes-k15-token-consumer.timer wires this)
K15 Falsification: /health.observations_consumed_token increments
"""

import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional


try:
    import requests
except ImportError:
    requests = None


class K15TokenAnalysisConsumer:
    """Read token-analysis observations from kernel, dispatch high-signal tasks."""

    def __init__(
        self,
        kernel_url: str = "http://localhost:3030",
        api_key: str = None,
        domain: str = "token-analysis",
        min_records: int = 10,  # Minimum token records per batch to trigger task
    ):
        self.kernel_url = kernel_url.rstrip("/")
        self.api_key = api_key or os.environ.get("CYNIC_API_KEY", "")
        self.domain = domain
        self.min_records = min_records

    def headers(self):
        """Build request headers with auth."""
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def fetch_observations(self, limit: int = 100) -> list:
        """Fetch token-analysis observations from kernel.

        Fetches observations and filters by tool + domain in-memory.
        """
        if requests is None:
            print("requests library not available")
            return []

        try:
            # Fetch all observations, filter for domain_verdict_builder tool + token-analysis domain
            url = f"{self.kernel_url}/observations?limit={limit*2}"
            resp = requests.get(url, headers=self.headers(), timeout=10)
            if resp.status_code == 200:
                all_obs = resp.json() or []
                # Filter: tool == "domain_verdict_builder" AND domain == "token-analysis"
                filtered = [o for o in all_obs if o.get("tool") == "domain_verdict_builder" and o.get("domain") == "token-analysis"]
                return filtered[:limit]
            else:
                print(f"Error fetching observations: {resp.status_code}")
                return []
        except Exception as e:
            print(f"Exception fetching observations: {e}")
            return []

    def score_observation(self, obs: dict) -> dict:
        """Score token-analysis observation.

        Token observations are already domain-scored (verdicts aggregated).
        Just evaluate records_count and verdict distribution.

        Returns: {
            "observation_id": str,
            "records_count": int,
            "verdict_distribution": dict,
            "high_signal": bool,
            "reasoning": str,
        }
        """
        try:
            context = obs.get("context", "{}")
            if isinstance(context, str):
                context = json.loads(context)

            records_count = context.get("records_count", 0)
            verdicts = context.get("verdicts", {})

            # High signal: ≥min_records AND any disputed verdicts (not all same)
            verdict_types = len(set(verdicts.keys()))
            high_signal = (records_count >= self.min_records) and (verdict_types > 1 or verdicts.get("BARK", 0) > 0)

            reasoning = f"Token batch: {records_count} records, verdicts: {verdicts}"

            return {
                "observation_id": obs.get("id", "unknown"),
                "records_count": records_count,
                "verdict_distribution": verdicts,
                "high_signal": high_signal,
                "reasoning": reasoning,
            }
        except Exception as e:
            print(f"Error scoring observation {obs.get('id')}: {e}")
            return {
                "observation_id": obs.get("id", "unknown"),
                "records_count": 0,
                "verdict_distribution": {},
                "high_signal": False,
                "reasoning": f"Error: {str(e)}",
            }

    def dispatch_task(self, observation: dict, score: dict) -> Optional[str]:
        """Dispatch high-signal token observation as agent task."""
        if not score["high_signal"]:
            return None

        if requests is None:
            print("requests library not available")
            return None

        task_content = f"""Token Analysis Batch
ID: {score['observation_id']}
Records: {score['records_count']}
Verdicts: {json.dumps(score['verdict_distribution'], indent=2)}

Actions:
1. Validate verdict consistency across records
2. If BARK dominant: flag for emergency analysis
3. If mixed (BARK+GROWL): investigate root cause

Reasoning: {score['reasoning']}""".strip()

        try:
            url = f"{self.kernel_url}/agent-tasks"
            body = {
                "kind": "hermes",
                "domain": "token-judgment",
                "content": task_content,
                "agent_id": "k15-token-consumer",
            }
            # Try with explicit json=body (serialized by requests)
            resp = requests.post(url, json=body, headers=self.headers(), timeout=10)

            if resp.status_code in [200, 201]:
                task_id = resp.json().get("task_id")
                print(f"✓ Dispatched task {task_id} for token observation {score['observation_id']}")
                return task_id
            elif resp.status_code == 500:
                # Kernel error (storage/DB issue)
                print(f"Kernel error: {resp.status_code} — task storage unavailable (kernel may be degraded)")
                return None
            else:
                print(f"Error dispatching task: {resp.status_code} {resp.text}")
                return None
        except Exception as e:
            print(f"Exception dispatching task: {e}")
            import traceback
            traceback.print_exc()
            return None

    def run(self, poll_interval: int = 60, max_iterations: int = None):
        """Run consumer loop."""
        iteration = 0
        total_observed = 0
        total_high_signal = 0
        total_tasks = 0

        print(f"K15 Token Analysis Consumer starting (interval={poll_interval}s, domain={self.domain})")

        while True:
            iteration += 1
            if max_iterations and iteration > max_iterations:
                break

            try:
                observations = self.fetch_observations(limit=100)
                if not observations:
                    print(f"[{iteration}] No token-analysis observations fetched")
                    time.sleep(poll_interval)
                    continue

                print(f"[{iteration}] Fetched {len(observations)} token-analysis observations")

                for obs in observations:
                    score = self.score_observation(obs)
                    total_observed += 1

                    if score["high_signal"]:
                        total_high_signal += 1
                        task_id = self.dispatch_task(obs, score)
                        if task_id:
                            total_tasks += 1
                        time.sleep(2)

                print(
                    f"[{iteration}] Token analysis: {total_observed} total, "
                    f"{total_high_signal} high-signal, {total_tasks} tasks dispatched"
                )

                time.sleep(poll_interval)

            except KeyboardInterrupt:
                print("Interrupted by user")
                break
            except Exception as e:
                print(f"Error in main loop: {e}")
                time.sleep(poll_interval)

        print(f"Token consumer stopped. Stats: {total_observed} observed, {total_high_signal} high-signal, {total_tasks} tasks")


def main():
    import argparse

    parser = argparse.ArgumentParser(description="K15 Token Analysis Consumer")
    parser.add_argument("--kernel-url", default="http://localhost:3030", help="Kernel URL")
    parser.add_argument("--api-key", default=None, help="API key (or $CYNIC_API_KEY)")
    parser.add_argument("--poll-interval", type=int, default=60, help="Poll interval (seconds)")
    parser.add_argument("--min-records", type=int, default=10, help="Min records per batch for task")
    parser.add_argument("--max-iter", type=int, default=None, help="Max iterations (for testing)")

    args = parser.parse_args()

    consumer = K15TokenAnalysisConsumer(
        kernel_url=args.kernel_url,
        api_key=args.api_key,
        min_records=args.min_records,
    )
    consumer.run(poll_interval=args.poll_interval, max_iterations=args.max_iter)


if __name__ == "__main__":
    main()
