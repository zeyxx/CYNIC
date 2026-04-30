#!/usr/bin/env python3
"""K15 Infrastructure Consumer — Route probe failures to recovery decisions.

This consumer handles infrastructure-domain observations (probes, health checks).
Routes based on failure_type, not social heuristics.

Failure classification (from inference_router.rs probe_node):
  - "timeout": node took >5s to respond
  - "unreachable": cannot connect (network error)
  - "parse_error": response unparseable
  - "mismatch": expected_model != actual_model
  - "none": healthy (skip)

Recovery actions:
  - timeout/unreachable: trigger /inference/remediate-dog on next healthy node
  - mismatch: alert (human decision needed)
  - parse_error: probe logs, next check

Usage:
  python k15_infrastructure_consumer.py --kernel-url http://<TAILSCALE_CORE>:3030 \
                                       --api-key $CYNIC_API_KEY \
                                       --poll-interval 60

Cron: Every 5min (future: systemd hermes-infrastructure-monitor.timer)
"""

import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional

import requests

# Exponential backoff for rate limiting (429)
INITIAL_BACKOFF = 2  # seconds
MAX_BACKOFF = 60  # seconds
BACKOFF_MULTIPLIER = 1.5


class K15InfrastructureConsumer:
    """Consume infrastructure observations, route failures to recovery."""

    def __init__(
        self,
        kernel_url: str = "http://<TAILSCALE_CORE>:3030",
        api_key: str = None,
    ):
        self.kernel_url = kernel_url.rstrip("/")
        self.api_key = api_key or os.environ.get("CYNIC_API_KEY", "")

    def headers(self):
        """Build request headers with auth."""
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def fetch_observations(self, domain: str = "infrastructure", limit: int = 50) -> tuple:
        """Fetch recent infrastructure observations from kernel.

        Returns (obs_list, should_backoff) where should_backoff=True if 429 rate limit hit.
        """
        try:
            # Filter by domain via query (future: add to API)
            url = f"{self.kernel_url}/observations?limit={limit}"
            resp = requests.get(url, headers=self.headers(), timeout=10)
            if resp.status_code == 200:
                obs_list = resp.json() or []
                # Client-side filter: only infrastructure domain
                return [o for o in obs_list if o.get("domain") == domain], False
            elif resp.status_code == 429:
                print(f"Rate limit (429): kernel at capacity, backing off")
                return [], True
            else:
                print(f"Error fetching observations: {resp.status_code}")
                return [], False
        except Exception as e:
            print(f"Exception fetching observations: {e}")
            return [], False

    def classify_failure(self, obs: dict) -> Optional[dict]:
        """Extract failure type and target from observation.

        Returns {
            "failure_type": "timeout" | "unreachable" | "parse_error" | "mismatch",
            "dog": "qwen-9b-core",
            "node": "cynic-core",
            "target": "cynic-core/qwen-9b-core",
            "latency_ms": 1234,
        }
        or None if not degraded.
        """
        tags = obs.get("tags", [])
        context = obs.get("context", "")

        # Look for failure_type tag
        failure_type = None
        for tag in tags:
            if tag.startswith("failure_type:"):
                failure_type = tag.split(":")[-1]
                break

        if not failure_type or failure_type == "none":
            return None  # Healthy, skip

        # Parse target: "{node}/{dog}" format
        target = obs.get("target", "unknown")
        parts = target.split("/")
        node = parts[0] if len(parts) > 0 else "unknown"
        dog = parts[1] if len(parts) > 1 else "unknown"

        # Extract latency from context: "failure_type=X, latency_ms=NNNN"
        latency_ms = 0
        try:
            for part in context.split(","):
                if "latency_ms=" in part:
                    latency_ms = int(part.split("=")[-1].strip())
        except:
            pass

        return {
            "failure_type": failure_type,
            "dog": dog,
            "node": node,
            "target": target,
            "latency_ms": latency_ms,
            "observation_id": obs.get("_id", "unknown"),
        }

    def decide_recovery(self, failure: dict) -> Optional[dict]:
        """Decide if/how to recover.

        Returns recovery action or None if no action needed.
        """
        failure_type = failure["failure_type"]

        if failure_type == "timeout":
            # Node is responding slowly. Try remediation if latency >3s.
            if failure["latency_ms"] > 3000:
                return {
                    "action": "remediate",
                    "target": failure["target"],
                    "reason": f"timeout: latency {failure['latency_ms']}ms",
                }

        elif failure_type == "unreachable":
            # Node is offline. Alert, don't auto-remediate (might need restart).
            return {
                "action": "alert",
                "target": failure["target"],
                "severity": "critical",
                "reason": "unreachable: network error",
            }

        elif failure_type == "mismatch":
            # Wrong model running. Human decision needed.
            return {
                "action": "alert",
                "target": failure["target"],
                "severity": "warning",
                "reason": "model mismatch: expected vs actual differ",
            }

        elif failure_type == "parse_error":
            # Response unparseable. Probably transient.
            return {
                "action": "log",
                "target": failure["target"],
                "reason": "parse_error: will retry next cycle",
            }

        return None

    def execute_recovery(self, recovery: dict) -> tuple:
        """Execute recovery action.

        Returns (success: bool, should_backoff: bool) where should_backoff=True if 429 rate limit.
        """
        action = recovery["action"]

        if action == "remediate":
            # POST /inference/remediate-dog with target
            try:
                url = f"{self.kernel_url}/inference/remediate-dog"
                body = {"target": recovery["target"]}
                resp = requests.post(url, json=body, headers=self.headers(), timeout=15)
                if resp.status_code in [200, 201]:
                    print(f"✓ Remediate {recovery['target']}: {resp.status_code}")
                    return True, False
                elif resp.status_code == 429:
                    print(f"✗ Remediate {recovery['target']}: 429 (rate limit, backing off)")
                    return False, True
                else:
                    print(f"✗ Remediate {recovery['target']}: {resp.status_code}")
                    return False, False
            except Exception as e:
                print(f"✗ Remediate {recovery['target']}: {e}")
                return False, False

        elif action == "alert":
            # Log with severity
            severity = recovery.get("severity", "info")
            print(
                f"[ALERT] {severity.upper()}: {recovery['target']} — {recovery['reason']}"
            )
            return True, False

        elif action == "log":
            # Just log
            print(f"[LOG] {recovery['target']} — {recovery['reason']}")
            return True, False

        return False, False

    def run(self, poll_interval: int = 60, max_iterations: int = None):
        """Run consumer loop with exponential backoff on 429 rate limits."""
        iteration = 0
        total_obs = 0
        total_degraded = 0
        total_recovery = 0
        current_backoff = INITIAL_BACKOFF

        print(f"K15 Infrastructure Consumer starting (interval={poll_interval}s)")

        while True:
            iteration += 1
            if max_iterations and iteration > max_iterations:
                break

            try:
                # Fetch observations
                observations, fetch_rate_limited = self.fetch_observations(domain="infrastructure", limit=50)

                # If we got rate limited on fetch, back off
                if fetch_rate_limited:
                    print(f"[{iteration}] Rate limited (429), backing off {current_backoff}s")
                    time.sleep(current_backoff)
                    current_backoff = min(current_backoff * BACKOFF_MULTIPLIER, MAX_BACKOFF)
                    continue

                # Reset backoff on successful fetch
                if observations:
                    current_backoff = INITIAL_BACKOFF

                if not observations:
                    print(f"[{iteration}] No infrastructure observations")
                    time.sleep(poll_interval)
                    continue

                print(f"[{iteration}] Fetched {len(observations)} infrastructure observations")

                # Classify and route
                rate_limited_on_recovery = False
                for obs in observations:
                    total_obs += 1
                    failure = self.classify_failure(obs)

                    if failure:
                        total_degraded += 1
                        recovery = self.decide_recovery(failure)

                        if recovery:
                            total_recovery += 1
                            success, rate_limited = self.execute_recovery(recovery)
                            if rate_limited:
                                rate_limited_on_recovery = True

                # If any recovery hit 429, back off
                if rate_limited_on_recovery:
                    print(f"[{iteration}] Recovery rate limited, backing off {current_backoff}s")
                    time.sleep(current_backoff)
                    current_backoff = min(current_backoff * BACKOFF_MULTIPLIER, MAX_BACKOFF)
                    continue

                # Log progress
                print(
                    f"[{iteration}] Processed: {total_obs} total, "
                    f"{total_degraded} degraded, {total_recovery} recovery actions"
                )

                time.sleep(poll_interval)

            except KeyboardInterrupt:
                print("Interrupted by user")
                break
            except Exception as e:
                print(f"Error in main loop: {e}")
                time.sleep(poll_interval)

        print(
            f"Consumer stopped. Stats: {total_obs} observed, {total_degraded} degraded, {total_recovery} recovery"
        )


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description="K15 Infrastructure Consumer — Route probe failures to recovery"
    )
    parser.add_argument("--kernel-url", default="http://<TAILSCALE_CORE>:3030", help="Kernel URL")
    parser.add_argument("--api-key", default=None, help="API key (or $CYNIC_API_KEY)")
    parser.add_argument("--poll-interval", type=int, default=60, help="Poll interval (seconds)")
    parser.add_argument("--max-iter", type=int, default=None, help="Max iterations (for testing)")

    args = parser.parse_args()

    consumer = K15InfrastructureConsumer(
        kernel_url=args.kernel_url,
        api_key=args.api_key,
    )
    consumer.run(poll_interval=args.poll_interval, max_iterations=args.max_iter)


if __name__ == "__main__":
    main()
