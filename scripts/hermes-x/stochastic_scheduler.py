"""
Stochastic scheduler for Organ X search executor.
Converts behavioral state into search scheduling decisions with adaptive timing.
"""

import json
import random
import time
from dataclasses import dataclass
from typing import Optional

from behavioral_state import BehavioralState

PHI = 1.6180339887

@dataclass
class SearchDecision:
    """Output of scheduling decision."""
    skip: bool
    domain: Optional[str]
    query: Optional[str]
    keyword: Optional[str]
    reason: str
    scheduled_wait_secs: float
    env_state: str
    backoff_level: int
    consecutive_zeros: int
    timestamp: str


class StochasticScheduler:
    """Decides whether to search, which domain, and timing."""

    def __init__(self, state: BehavioralState, config: dict):
        self.state = state
        self.config = config
        self.domains = list(config.get("domains", {}).keys())

    def decide(self) -> SearchDecision:
        """
        Produce a search decision based on current behavioral state.
        """
        from datetime import datetime

        timestamp = datetime.utcnow().isoformat() + "Z"

        # Skip if blocked or max backoff
        if self.state.env_state == "blocked" or self.state.backoff_level >= 4:
            return SearchDecision(
                skip=True,
                domain=None,
                query=None,
                keyword=None,
                reason=f"env_state={self.state.env_state}, backoff_level={self.state.backoff_level}",
                scheduled_wait_secs=0,
                env_state=self.state.env_state,
                backoff_level=self.state.backoff_level,
                consecutive_zeros=self.state.consecutive_zeros,
                timestamp=timestamp,
            )

        # Compute stochastic timing
        base_interval = 60.0
        backoff_mult = PHI ** self.state.backoff_level  # 1.0, 1.618, 2.618, 4.236, 6.854
        jitter = random.gauss(0, 15)  # ±15 second variance
        scheduled_wait = max(30.0, base_interval * backoff_mult + jitter)

        # Choose domain via weighted probability
        if not self.domains:
            return SearchDecision(
                skip=True,
                domain=None,
                query=None,
                keyword=None,
                reason="No domains configured",
                scheduled_wait_secs=0,
                env_state=self.state.env_state,
                backoff_level=self.state.backoff_level,
                consecutive_zeros=self.state.consecutive_zeros,
                timestamp=timestamp,
            )

        weights = [
            self.state.domain_weights.get(d, 0.2 / len(self.domains))
            for d in self.domains
        ]
        domain = random.choices(self.domains, weights=weights, k=1)[0]

        # Choose keyword via per-cycle entropy (not daily seed)
        cycle_entropy = int(time.time() * 1000) % (2**31)
        rng = random.Random(cycle_entropy ^ hash(domain))

        domain_cfg = self.config.get("domains", {}).get(domain, {})
        keywords = domain_cfg.get("keywords", [])

        if not keywords:
            return SearchDecision(
                skip=True,
                domain=domain,
                query=None,
                keyword=None,
                reason=f"No keywords for domain {domain}",
                scheduled_wait_secs=scheduled_wait,
                env_state=self.state.env_state,
                backoff_level=self.state.backoff_level,
                consecutive_zeros=self.state.consecutive_zeros,
                timestamp=timestamp,
            )

        keyword = rng.choice(keywords)
        query = keyword  # Plain query, executor will URL-encode

        return SearchDecision(
            skip=False,
            domain=domain,
            query=query,
            keyword=keyword,
            reason=f"backoff={self.state.backoff_level}, consecutive_zeros={self.state.consecutive_zeros}",
            scheduled_wait_secs=scheduled_wait,
            env_state=self.state.env_state,
            backoff_level=self.state.backoff_level,
            consecutive_zeros=self.state.consecutive_zeros,
            timestamp=timestamp,
        )

    @staticmethod
    def update_state_after_search(
        decision: SearchDecision,
        results_count: int,
        http_status: Optional[int] = None,
    ) -> tuple:
        """
        Determine new backoff_level and env_state after a search completes.
        Returns (new_backoff_level, new_env_state)
        """
        backoff = decision.backoff_level
        env = decision.env_state

        # Detect blockers via HTTP status
        if http_status in (403, 429):
            env = "blocked"
            backoff = min(4, backoff + 1)
        elif results_count > 0:
            # Positive signal: results returned
            backoff = max(0, backoff - 1)
            env = "ok"
        # else: zero results, keep existing backoff/env

        return backoff, env
