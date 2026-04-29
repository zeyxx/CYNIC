"""K15 Consumer — Read observations, score with domain Dogs, dispatch high-signal tasks.

This consumer closes the K15 producer-consumer loop:
1. Poll kernel /observations
2. Score each observation with TwitterDog (domain-aware heuristics)
3. Filter high-signal (≥3 or matches known patterns like @gcrtrd)
4. POST to /agent-tasks for Hermes to execute

Usage:
  python k15_observation_consumer.py --kernel-url http://localhost:3030 \
                                     --api-key $CYNIC_API_KEY \
                                     --poll-interval 60

Cron: Every 1h (systemd hermes-feedback-loop.timer wires this)
K15 Falsification: /health.observations_consumed increments
"""

import json
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional

import requests

# Import local heuristics
sys.path.insert(0, str(Path(__file__).parent.parent / "heuristics"))
from twitter_dog import TwitterDog
from twitter_signal_extractor import TwitterSignalExtractor


class K15ObservationConsumer:
    """Read observations from kernel, score with Dogs, dispatch high-signal tasks."""

    def __init__(
        self,
        kernel_url: str = "http://localhost:3030",
        api_key: str = None,
        domain_filter: str = "twitter",
    ):
        self.kernel_url = kernel_url.rstrip("/")
        self.api_key = api_key or os.environ.get("CYNIC_API_KEY", "")
        self.domain_filter = domain_filter

        # Initialize Dogs
        self.twitter_dog = TwitterDog()
        self.signal_extractor = TwitterSignalExtractor()

        # Patterns for high-signal sources (e.g., @gcrtrd)
        self.known_high_signal_patterns = {
            "@gcrtrd": {"min_signal": 1},  # Any mention of gcrtrd = worth investigating
            "rug": {"min_signal": 2},      # Rug allegations = signal
            "exploit": {"min_signal": 2},
            "honeyp": {"min_signal": 3},  # Honeypot = high signal
        }

    def headers(self):
        """Build request headers with auth."""
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def fetch_observations(self, limit: int = 100) -> list:
        """Fetch recent observations from kernel."""
        try:
            url = f"{self.kernel_url}/observations?limit={limit}"
            resp = requests.get(url, headers=self.headers(), timeout=10)
            if resp.status_code == 200:
                return resp.json() or []
            else:
                print(f"Error fetching observations: {resp.status_code}")
                return []
        except Exception as e:
            print(f"Exception fetching observations: {e}")
            return []

    def score_observation(self, obs: dict) -> dict:
        """Score an observation with TwitterDog heuristics.

        Returns: {
            "observation_id": str,
            "signal_score": float (0-10),
            "verdict": str ("HOWL", "GROWL", "BARK"),
            "q_score": float,
            "reasoning": dict,
            "high_signal": bool,
        }
        """
        try:
            # Check if observation is tweet-like
            content = obs.get("context", "")
            target = obs.get("target", "")

            # Build synthetic tweet object for TwitterDog
            synthetic_tweet = {
                "tweet_id": obs.get("_id", "obs_" + str(time.time())),
                "text": content,
                "author_followers_count": obs.get("metadata", {}).get("followers", 100),
                "author_statuses_count": obs.get("metadata", {}).get("tweet_count", 10),
                "engagement_rate": obs.get("metadata", {}).get("engagement_rate", 0.02),
            }

            # Extract signals
            signals = self.signal_extractor.extract(synthetic_tweet)

            # Score with TwitterDog
            verdict = self.twitter_dog.judge(synthetic_tweet, signal_score=obs.get("signal_score"))

            # Compute signal score (0-10 scale)
            signal_score = self._compute_signal_score(content, signals)

            # Determine if high-signal
            high_signal = self._is_high_signal(content, signal_score, verdict)

            # Map q_score to verdict
            verdict_type = "HOWL" if verdict.q_score > 0.528 else "GROWL" if verdict.q_score > 0.382 else "BARK"

            return {
                "observation_id": obs.get("_id", "unknown"),
                "signal_score": signal_score,
                "verdict": verdict_type,
                "q_score": verdict.q_score,
                "reasoning": verdict.reasoning,
                "high_signal": high_signal,
            }
        except Exception as e:
            print(f"Error scoring observation {obs.get('_id')}: {e}")
            return {
                "observation_id": obs.get("_id", "unknown"),
                "signal_score": 0.0,
                "verdict": "GROWL",
                "q_score": 0.4,
                "reasoning": {"error": str(e)},
                "high_signal": False,
            }

    def _compute_signal_score(self, content: str, signals) -> float:
        """Compute signal score (0-10).

        High signal: rug allegations, exploits, creator criticism, low followers
        """
        score = 5.0  # Neutral baseline

        # Red flags (positive signal)
        if signals.has_rug_allegations:
            score += 2.0
        if signals.has_creator_criticism:
            score += 1.5
        if signals.has_buy_pressure_spam:
            score += 1.0

        # Positive signals
        if signals.liquidity_discussion_active:
            score += 1.0

        # Community size (smaller = higher signal risk)
        if signals.follower_count < 1000:
            score += 1.0
        elif signals.follower_count > 100000:
            score -= 1.0

        # Clamp to 0-10
        return max(0.0, min(10.0, score))

    def _is_high_signal(self, content: str, signal_score: float, verdict) -> bool:
        """Determine if observation should trigger a task.

        Thresholds:
          - Pattern match (e.g., @gcrtrd): min_signal from config
          - BARK verdict (q_score ≤0.382): always high signal
          - General heuristic: signal ≥6.0 (high confidence)
        """
        # Pattern matching for known sources
        for pattern, config in self.known_high_signal_patterns.items():
            if pattern.lower() in content.lower():
                return signal_score >= config.get("min_signal", 5)

        # BARK is always worth investigating (rug allegations, exploits, etc.)
        if verdict.q_score <= 0.382:
            return True

        # General heuristic: signal ≥6.0 for non-BARK tweets (high confidence)
        # This filters out neutral market analysis (signal ≈5.0)
        return signal_score >= 6.0

    def dispatch_task(self, observation: dict, score: dict) -> Optional[str]:
        """Dispatch high-signal observation as agent task.

        Returns task_id on success, None on failure.
        """
        if not score["high_signal"]:
            return None

        task_content = f"""
Observation Signal Analysis
ID: {observation.get('_id')}
Content: {observation.get('context', '')[:500]}
Signal Score: {score['signal_score']:.1f}/10
Verdict: {score['verdict']} (q={score['q_score']:.3f})

Actions:
1. Verify signal against live data (Helius, CultScreener)
2. If BARK: update rug patterns in SKILL.md
3. If GROWL: monitor for follow-up signals from same source
4. If HOWL: record positive pattern (rare)

Reasoning:
{json.dumps(score['reasoning'], indent=2)}
""".strip()

        try:
            url = f"{self.kernel_url}/agent-tasks"
            body = {
                "kind": "hermes",
                "domain": "social-signal",
                "content": task_content,
                "agent_id": "k15-consumer",
            }
            resp = requests.post(url, json=body, headers=self.headers(), timeout=10)
            if resp.status_code in [200, 201]:
                task_id = resp.json().get("task_id")
                print(f"✓ Dispatched task {task_id} for observation {score['observation_id']}")
                return task_id
            else:
                print(f"Error dispatching task: {resp.status_code} {resp.text}")
                return None
        except Exception as e:
            print(f"Exception dispatching task: {e}")
            return None

    def run(self, poll_interval: int = 60, max_iterations: int = None):
        """Run consumer loop.

        Args:
            poll_interval: seconds between polls
            max_iterations: for testing, limit iterations
        """
        iteration = 0
        total_observed = 0
        total_high_signal = 0
        total_tasks = 0

        print(f"K15 Consumer starting (interval={poll_interval}s)")

        while True:
            iteration += 1
            if max_iterations and iteration > max_iterations:
                break

            try:
                # Fetch observations
                observations = self.fetch_observations(limit=100)
                if not observations:
                    print(f"[{iteration}] No observations fetched")
                    time.sleep(poll_interval)
                    continue

                print(f"[{iteration}] Fetched {len(observations)} observations")

                # Score and dispatch
                for obs in observations:
                    score = self.score_observation(obs)
                    total_observed += 1

                    if score["high_signal"]:
                        total_high_signal += 1
                        task_id = self.dispatch_task(obs, score)
                        if task_id:
                            total_tasks += 1

                # Log progress
                print(
                    f"[{iteration}] Processed: {total_observed} total, "
                    f"{total_high_signal} high-signal, {total_tasks} tasks dispatched"
                )

                time.sleep(poll_interval)

            except KeyboardInterrupt:
                print("Interrupted by user")
                break
            except Exception as e:
                print(f"Error in main loop: {e}")
                time.sleep(poll_interval)

        print(f"Consumer stopped. Stats: {total_observed} observed, {total_high_signal} high-signal, {total_tasks} tasks")


def main():
    import argparse

    parser = argparse.ArgumentParser(description="K15 Observation Consumer")
    parser.add_argument("--kernel-url", default="http://localhost:3030", help="Kernel URL")
    parser.add_argument("--api-key", default=None, help="API key (or $CYNIC_API_KEY)")
    parser.add_argument("--domain", default="twitter", help="Domain filter")
    parser.add_argument("--poll-interval", type=int, default=60, help="Poll interval (seconds)")
    parser.add_argument("--max-iter", type=int, default=None, help="Max iterations (for testing)")

    args = parser.parse_args()

    consumer = K15ObservationConsumer(
        kernel_url=args.kernel_url,
        api_key=args.api_key,
        domain_filter=args.domain,
    )
    consumer.run(poll_interval=args.poll_interval, max_iterations=args.max_iter)


if __name__ == "__main__":
    main()
