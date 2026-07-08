"""Runtime adapter for the K15 observation consumer."""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path
from typing import Optional

import requests

from organs.contracts.src.adapters.agent_tasks import agent_task_visible as kernel_agent_task_visible
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, normalize_kernel_base_url
from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
HEURISTICS_DIR = PROJECT_ROOT / "services" / "cynic-python" / "heuristics"
if str(HEURISTICS_DIR) not in sys.path:
    sys.path.insert(0, str(HEURISTICS_DIR))

from twitter_dog import TwitterDog
from twitter_signal_extractor import TwitterSignalExtractor


class K15ObservationConsumer:
    """Read observations from kernel, score with Dogs, dispatch high-signal tasks."""

    def __init__(
        self,
        kernel_url: str = "",
        api_key: str = None,
        domain_filter: str = "twitter",
    ):
        self.kernel_url = self._normalize_kernel_url(kernel_url)
        self.api_key = api_key or os.environ.get("CYNIC_API_KEY", "")
        self.domain_filter = domain_filter
        self.twitter_dog = TwitterDog()
        self.signal_extractor = TwitterSignalExtractor()
        self.known_high_signal_patterns = {
            "@gcrtrd": {"min_signal": 1},
            "rug": {"min_signal": 2},
            "exploit": {"min_signal": 2},
            "honeyp": {"min_signal": 3},
        }

    def headers(self):
        """Build request headers with auth."""
        return kernel_auth_headers(self.api_key)

    @staticmethod
    def _normalize_kernel_url(raw: str) -> str:
        return normalize_kernel_base_url(raw or os.environ.get("CYNIC_REST_ADDR", ""))

    def fetch_observations(self, limit: int = 100) -> list:
        """Fetch recent observations from kernel."""
        try:
            response = requests.get(
                f"{self.kernel_url}/observations?limit={limit}",
                headers=self.headers(),
                timeout=10,
            )
            if response.status_code == 200:
                return response.json() or []
            print(f"Error fetching observations: {response.status_code}")
            return []
        except Exception as exc:
            print(f"Exception fetching observations: {exc}")
            return []

    def score_observation(self, obs: dict) -> dict:
        """Score an observation with TwitterDog heuristics."""
        try:
            content = obs.get("context", "")
            synthetic_tweet = {
                "tweet_id": obs.get("_id", "obs_" + str(time.time())),
                "text": content,
                "author_followers_count": obs.get("metadata", {}).get("followers", 50000),
                "author_statuses_count": obs.get("metadata", {}).get("tweet_count", 1000),
                "engagement_rate": obs.get("metadata", {}).get("engagement_rate", 0.02),
            }
            signals = self.signal_extractor.extract(synthetic_tweet)
            verdict = self.twitter_dog.judge(synthetic_tweet, signal_score=obs.get("signal_score"))
            signal_score = self._compute_signal_score(content, signals)
            high_signal = self._is_high_signal(content, signal_score, verdict)
            verdict_type = "HOWL" if verdict.q_score > 0.528 else "GROWL" if verdict.q_score > 0.382 else "BARK"

            return {
                "observation_id": obs.get("_id", "unknown"),
                "signal_score": signal_score,
                "verdict": verdict_type,
                "q_score": verdict.q_score,
                "reasoning": verdict.reasoning,
                "high_signal": high_signal,
            }
        except Exception as exc:
            print(f"Error scoring observation {obs.get('_id')}: {exc}")
            return {
                "observation_id": obs.get("_id", "unknown"),
                "signal_score": 0.0,
                "verdict": "GROWL",
                "q_score": 0.4,
                "reasoning": {"error": str(exc)},
                "high_signal": False,
            }

    def _compute_signal_score(self, content: str, signals) -> float:
        """Compute signal score on a 0-10 scale."""
        score = 3.0
        if signals.has_rug_allegations:
            score += 2.0
        if signals.has_creator_criticism:
            score += 1.5
        if signals.has_buy_pressure_spam:
            score += 1.0
        if signals.liquidity_discussion_active:
            score += 1.0
        if signals.follower_count < 1000:
            score += 1.0
        elif signals.follower_count > 100000:
            score -= 1.0
        return max(0.0, min(10.0, score))

    def _is_high_signal(self, content: str, signal_score: float, verdict) -> bool:
        """Determine if observation should trigger a task."""
        for pattern, config in self.known_high_signal_patterns.items():
            if pattern.lower() in content.lower():
                return signal_score >= config.get("min_signal", 5)
        if verdict.q_score <= 0.382:
            return True
        return signal_score >= 6.0

    def dispatch_task(self, observation: dict, score: dict) -> Optional[str]:
        """Dispatch high-signal observation as agent task."""
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
            response = requests.post(
                f"{self.kernel_url}/agent-tasks",
                json={
                    "kind": "hermes",
                    "domain": "social-signal",
                    "content": task_content,
                    "agent_id": "k15-consumer",
                },
                headers=self.headers(),
                timeout=10,
            )
            if response.status_code in [200, 201]:
                task_id = response.json().get("task_id")
                if task_id and not kernel_agent_task_visible(
                    self.kernel_url,
                    self.api_key,
                    str(task_id),
                    kind="hermes",
                    domain="social-signal",
                ):
                    print(f"Error dispatching task: read-back failed for {task_id}")
                    return None
                print(f"Dispatched task {task_id} for observation {score['observation_id']}")
                return task_id
            print(f"Error dispatching task: {response.status_code} {response.text}")
            return None
        except Exception as exc:
            print(f"Exception dispatching task: {exc}")
            return None

    def run(self, poll_interval: int = 60, max_iterations: int = None):
        """Run consumer loop."""
        iteration = 0
        total_observed = 0
        total_high_signal = 0
        total_tasks = 0
        print(f"K15 Consumer starting (interval={poll_interval}s)")

        while True:
            iteration += 1
            if max_iterations is not None and iteration > max_iterations:
                break

            try:
                observations = self.fetch_observations(limit=100)
                if not observations:
                    print(f"[{iteration}] No observations fetched")
                    time.sleep(poll_interval)
                    continue

                print(f"[{iteration}] Fetched {len(observations)} observations")
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
                    f"[{iteration}] Processed: {total_observed} total, "
                    f"{total_high_signal} high-signal, {total_tasks} tasks dispatched"
                )
                time.sleep(poll_interval)

            except KeyboardInterrupt:
                print("Interrupted by user")
                break
            except Exception as exc:
                print(f"Error in main loop: {exc}")
                time.sleep(poll_interval)

        print(
            f"Consumer stopped. Stats: {total_observed} observed, "
            f"{total_high_signal} high-signal, {total_tasks} tasks"
        )
